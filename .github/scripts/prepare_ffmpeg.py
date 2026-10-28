#!/usr/bin/env python3
"""Ubuntu-only fallback, caching authenticated distro packages, not executables.

The runner's own tools always get the first chance. Resolve a pinned FFmpeg
version and the complete apt transaction before restoring packages. The cache
key also includes the runner image, architecture and these preparation scripts.
Apt still verifies packages against its signed indices on every installation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
from urllib.parse import unquote, urlparse

import check_ffmpeg

ROOT = Path(__file__).resolve().parents[2]
STATE = ROOT / '.test-ffmpeg-state.json'
ARCHIVES = ROOT / '.test-ffmpeg-debs'
APT = ['sudo', 'apt-get', '-o', 'Acquire::Retries=1', '-o', 'Acquire::http::Timeout=20',
       '-o', 'Acquire::https::Timeout=20', '-o', 'APT::Update::Error-Mode=any']


def run(command, timeout):
    # Coreutils timeout forwards signals to the child process group, including
    # sudo/apt children. A Python timeout alone could leave apt running behind.
    return subprocess.check_output(['timeout', '--kill-after=10s', f'{timeout}s', *command],
                                   text=True, timeout=timeout + 20,
                                   env={**os.environ, 'LC_ALL': 'C'})


def output(**values):
    for key, value in values.items():
        print(f'{key}={value}')
    if destination := os.environ.get('GITHUB_OUTPUT'):
        with open(destination, 'a', encoding='utf-8') as stream:
            for key, value in values.items():
                stream.write(f'{key}={value}\n')


def package_hashes(transaction, options):
    """Bind each cached archive to SHA256 from the freshly signed apt indices.

    Do not assume apt will rehash a pre-existing archive. Consumer jobs build
    this manifest independently, not from a producer artifact or cache.
    """
    packages = {}
    for line in transaction.splitlines():
        if not line.startswith("'"):
            continue
        uri, filename, size, _digest = shlex.split(line)
        if not uri.startswith(('https://archive.ubuntu.com/ubuntu/', 'https://security.ubuntu.com/ubuntu/')):
            raise ValueError('unexpected package origin')
        if Path(filename).name != filename or not filename.endswith('.deb') or not size.isdigit():
            raise ValueError('invalid package filename or size')
        package = filename.split('_')[0]
        if not re.fullmatch(r'[a-z0-9][a-z0-9+.-]*', package):
            raise ValueError('invalid package name')
        metadata = run(['apt-cache', *options, 'show', package], 30)
        candidates = set()
        for record in metadata.split('\n\n'):
            fields = dict(line.split(': ', 1) for line in record.splitlines() if ': ' in line and not line.startswith(' '))
            index_path = fields.get('Filename', '')
            digest = fields.get('SHA256', '')
            if (index_path and unquote(urlparse(uri).path).endswith('/' + index_path)
                    and fields.get('Size') == size and re.fullmatch(r'[0-9a-f]{64}', digest)):
                candidates.add(digest)
        if len(candidates) != 1 or filename in packages:
            raise ValueError('package has no unambiguous authenticated SHA256')
        packages[filename] = {'size': int(size), 'sha256': candidates.pop()}
    if not packages:
        raise ValueError('empty package download transaction')
    return packages


def validate_archives(packages):
    for archive in ARCHIVES.glob('*.deb'):
        expected = packages.get(archive.name)
        if not archive.is_symlink() and expected and archive.stat().st_size == expected['size']:
            with archive.open('rb') as stream:
                if hashlib.file_digest(stream, 'sha256').hexdigest() == expected['sha256']:
                    continue
        print(f'Discarding unverified cached package: {archive.name}', flush=True)
        archive.unlink()


def plan():
    if check_ffmpeg.usable():
        output(ready='true', package_key='unused')
        return
    # Never install from extra third-party repositories configured on a runner.
    # Use the official distro's signed noble and noble-security repositories.
    os_release = Path('/etc/os-release').read_text()
    if 'ID=ubuntu\n' not in os_release or 'VERSION_ID="24.04"' not in os_release:
        raise RuntimeError('FFmpeg fallback requires Ubuntu 24.04')
    architecture = run(['dpkg', '--print-architecture'], 10).strip()
    if architecture != 'amd64':
        raise RuntimeError('FFmpeg fallback requires amd64')
    sources = ROOT / '.test-ffmpeg.sources'
    sources.write_text(
        'Types: deb\nURIs: https://archive.ubuntu.com/ubuntu\n'
        'Suites: noble noble-updates\nComponents: main universe\n'
        'Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n\n'
        'Types: deb\nURIs: https://security.ubuntu.com/ubuntu\n'
        'Suites: noble-security\nComponents: main universe\n'
        'Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n', encoding='utf-8')
    lists = ROOT / '.test-ffmpeg-lists'
    lists.mkdir(exist_ok=True)
    options = ['-o', f'Dir::Etc::sourcelist={sources}', '-o', 'Dir::Etc::sourceparts=-',
               '-o', f'Dir::State::lists={lists}', '-o', 'Dir::Cache::pkgcache=',
               '-o', 'Dir::Cache::srcpkgcache=']
    print(run(APT + options + ['update'], 180), flush=True)
    policy = run(['apt-cache', *options, 'policy', 'ffmpeg'], 30)
    match = re.search(r'^\s*Candidate:\s*([0-9][A-Za-z0-9.+:~\-]*)\s*$', policy, re.MULTILINE)
    if not match:
        raise RuntimeError('no authenticated distro FFmpeg candidate')
    version = match[1]
    transaction = run(APT + options + ['--print-uris', '--yes', '--reinstall',
                                      '--no-install-recommends', 'install', f'ffmpeg={version}'], 60)
    packages = package_hashes(transaction, options)
    # Include exact dependency versions/URIs/checksums as well as FFmpeg's version.
    material = [transaction, json.dumps(packages, sort_keys=True), version, architecture, os.environ.get('ImageVersion', 'unknown')]
    material += [Path(__file__).read_text(), Path(check_ffmpeg.__file__).read_text()]
    key = hashlib.sha256('\n'.join(material).encode()).hexdigest()
    STATE.write_text(json.dumps({'version': version, 'options': options, 'packages': packages}), encoding='utf-8')
    output(ready='false', package_key='ubuntu-24.04-amd64-ffmpeg-' + key)


def install():
    state = json.loads(STATE.read_text(encoding='utf-8'))
    ARCHIVES.mkdir(exist_ok=True)
    validate_archives(state['packages'])
    command = APT + state['options'] + ['-o', f'Dir::Cache::archives={ARCHIVES}',
              '-o', 'APT::Keep-Downloaded-Packages=true',
              '--yes', '--reinstall', '--no-install-recommends', 'install', f'ffmpeg={state["version"]}']
    for attempt in range(2):
        try:
            print(run(command, 480), flush=True)
            break
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            if attempt:
                raise
            # Only discard package files inside our fixed workspace cache.
            # Apt can fetch a corrupt/missing package again from signed sources.
            for package in ARCHIVES.glob('*.deb'):
                package.unlink()
    version = run(['dpkg-query', '-W', '-f=${Version}', 'ffmpeg'], 10).strip()
    if version != state['version']:
        raise RuntimeError('installed FFmpeg does not match the pinned transaction')
    os.environ['PATH'] = '/usr/bin:' + os.environ['PATH']
    if not check_ffmpeg.usable():
        raise RuntimeError('installed FFmpeg failed the required encode/probe/raw-pixel checks')
    if destination := os.environ.get('GITHUB_PATH'):
        with open(destination, 'a', encoding='utf-8') as stream:
            stream.write('/usr/bin\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['plan', 'install'])
    args = parser.parse_args()
    {'plan': plan, 'install': install}[args.phase]()
