#!/usr/bin/env python3
"""Conservative PR selection. Unknown/shared changes and non-PR events run all checks."""
import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import tomllib

FEATURES = [
    'default', 'minimal', 'jpeg-xl', 'jpeg-xl,image-buffer-ex',
    'image-buffer-ex', 'image-buffer-ex,high-bit-depth',
    'png,image-buffer-ex,high-bit-depth', 'tiff,image-buffer-ex,high-bit-depth',
    'high-bit-depth', 'color-management', 'jpeg2000', 'jpeg2000,image-buffer-ex',
    'avif,image-buffer-ex', 'avifenc,image-buffer-ex,high-bit-depth',
    'avifenc,image-buffer-ex,high-bit-depth,multithread', 'avif,high-bit-depth',
    'avifenc,high-bit-depth', 'avifenc,high-bit-depth,multithread',
    'jpeg,idct_llm', 'jpeg,idct_aan', 'jpeg,idct_slower',
    'jpeg,idct_llm,multithread', 'png,psd,tiff',
]
CODECS = {'avif', 'jpeg2000', 'jpeg-xl'}
AVIF_PACKAGES = {'avif-rust', 'avifenc-rust'}
BASE_FEATURES = {'default', 'minimal', 'image-buffer-ex',
                 'image-buffer-ex,high-bit-depth', 'high-bit-depth', 'color-management'}


def git(*args):
    return subprocess.check_output(['git', *args])


def manifest_avif_only(before, after):
    """Only existing registry AVIF version requirements may change, not features."""
    a, b = copy.deepcopy(before), copy.deepcopy(after)
    changed = False
    for alias, package in [('avif_codec', 'avif-rust'), ('avifenc_codec', 'avifenc-rust')]:
        for manifest in [a, b]:
            dep = manifest['dependencies'][alias]
            if dep.get('package') != package or any(k in dep for k in ['git', 'path', 'registry']):
                return False
            if not isinstance(dep.get('version'), str):
                return False
        changed |= a['dependencies'][alias]['version'] != b['dependencies'][alias]['version']
        a['dependencies'][alias].pop('version')
        b['dependencies'][alias].pop('version')
    return changed and a == b


def lock_avif_only(before, after):
    """Reject changes to shared/transitive dependencies, edges, sources or package sets."""
    def normalized(value):
        value = copy.deepcopy(value)
        seen = set()
        for package in value['package']:
            if package['name'] in AVIF_PACKAGES:
                if package['name'] in seen or package.get('source') != 'registry+https://github.com/rust-lang/crates.io-index':
                    raise ValueError('ambiguous AVIF package source or duplicate')
                seen.add(package['name'])
                package.pop('version')
                package.pop('checksum')
        if seen != AVIF_PACKAGES:
            raise ValueError('missing AVIF package')
        value['package'].sort(key=lambda p: (p['name'], p.get('version', ''), p.get('source', '')))
        return value
    return before != after and normalized(before) == normalized(after)


def affected(paths, read_pair):
    codecs = set()
    for path in paths:
        if path == 'wml2/Cargo.toml':
            if not manifest_avif_only(*read_pair(path)):
                return CODECS, True, 'shared manifest/feature change'
            codecs.add('avif')
        elif path == 'Cargo.lock':
            if not lock_avif_only(*read_pair(path)):
                return CODECS, True, 'shared/transitive dependency change'
            codecs.add('avif')
        elif path.startswith(('wml2/src/avif/', 'wml2/tests/avif_')) or path in {
            'wml2/src/highres/avif.rs', 'wml2/src/highres/avif_encode.rs',
            'wml2/src/highres/avif_encode_bench.rs',
            'wml2/tests/highres_avif_native_bridge.rs', 'wml2/tests/highres_avif_encode.rs',
        }:
            codecs.add('avif')
        elif path == 'wml2/src/jpegxl.rs' or path.startswith('wml2/tests/jpeg_xl'):
            codecs.add('jpeg-xl')
        elif (path.startswith('wml2/src/jpeg2000') and path.endswith('.rs')) or path.startswith('wml2/tests/jpeg2000'):
            codecs.add('jpeg2000')
        elif (path.startswith('docs/') and path.endswith(('.md', '.json', '.png', '.json.gz'))) or path in {'README.md', 'README.ja.md'}:
            # Documentation only. Executable scripts, fixtures, submodules and
            # unknown extensions are deliberately NOT classified by their name.
            continue
        else:
            return CODECS, True, 'shared, unknown or executable path: ' + path
    return codecs, False, 'recognized codec/documentation changes'


def uses_avif(features, manifest):
    todo = ['default'] if features == 'default' else ([] if features == 'minimal' else features.split(','))
    seen = set()
    while todo:
        feature = todo.pop()
        if feature in seen:
            continue
        seen.add(feature)
        if feature in {'avif', 'avifenc', 'dep:avif_codec', 'dep:avifenc_codec'}:
            return True
        todo.extend(manifest.get('features', {}).get(feature, []))
    return False


def plan(event, base, head, manifest):
    full, codecs, reason = True, CODECS, 'non-PR events always run the full suite'
    if event == 'pull_request':
        try:
            # Merge-base catches changes across multiple commits. No rename
            # detection: both old and new paths must be classified safely.
            ancestor = git('merge-base', base, head).decode().strip()
            paths = git('diff', '--name-only', '--no-renames', '-z', ancestor, head).decode().split('\0')
            paths = [p for p in paths if p]
            if not paths:
                raise ValueError('empty or indeterminate diff')
            def pair(path):
                return tuple(tomllib.loads(git('show', f'{rev}:{path}').decode()) for rev in [ancestor, head])
            codecs, full, reason = affected(paths, pair)
        except (subprocess.CalledProcessError, ValueError, KeyError, TypeError, UnicodeError) as exc:
            full, codecs, reason = True, CODECS, 'selection unavailable; full suite: ' + type(exc).__name__
    selected = [f for f in FEATURES if full or f in BASE_FEATURES or any(c in f.split(',') or (c == 'avif' and 'avifenc' in f.split(',')) for c in codecs)]
    matrix = {'include': [{'rust': rust, 'features': f, 'avif': uses_avif(f, manifest)}
                          for rust in ['1.91.0', 'stable'] for f in selected]}
    return {'matrix': matrix, 'full': full, 'avif': full or 'avif' in codecs,
            'jpeg2000': full or 'jpeg2000' in codecs, 'jpegxl': full or 'jpeg-xl' in codecs,
            'tiff': full, 'reason': reason}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--event', default=os.environ.get('CI_EVENT', ''))
    parser.add_argument('--base', default=os.environ.get('CI_BASE', ''))
    parser.add_argument('--head', default=os.environ.get('CI_HEAD', 'HEAD'))
    args = parser.parse_args()
    manifest = tomllib.loads(Path('wml2/Cargo.toml').read_text())
    result = plan(args.event, args.base, args.head, manifest)
    print(json.dumps(result, indent=2))
    if output := os.environ.get('GITHUB_OUTPUT'):
        with open(output, 'a') as stream:
            for key, value in result.items():
                if key != 'reason':
                    stream.write(f'{key}={json.dumps(value, separators=(",", ":"))}\n')


if __name__ == '__main__':
    main()
