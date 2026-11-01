#!/usr/bin/env python3
"""Linux-only allocation attribution, with separately observable CI phases."""
import argparse
import hashlib
import json
import pathlib
import platform
import re
import subprocess
import tarfile

parser = argparse.ArgumentParser()
parser.add_argument('--output', type=pathlib.Path, required=True)
parser.add_argument('--phase', choices=['all', 'build', 'matrix', 'source', 'symbols'], default='all')
args = parser.parse_args()
if not args.output.name.startswith('.test'):
    parser.error('output must start with .test')
if platform.system() != 'Linux' or platform.machine() != 'x86_64':
    parser.error('attribution harness requires Linux x86_64')
root = pathlib.Path(__file__).resolve().parents[2]
out = args.output.resolve()
out.mkdir(parents=True, exist_ok=True)
exe = out / 'probe'
probe = root / 'wml2-test/scripts/jpegxl_palette_cache_probe.rs'

def run(command):
    return subprocess.run([str(x) for x in command], cwd=root, capture_output=True,
                          text=True, check=True).stdout

def artifacts():
    return [json.loads(line) for line in (out / 'build.json').read_text().splitlines()
            if line.startswith('{')]

def artifact(name):
    values = [f for item in artifacts() if item.get('reason') == 'compiler-artifact'
              and item['target']['name'] == name for f in item['filenames'] if f.endswith('.rlib')]
    assert len(values) == 1, (name, values)
    return pathlib.Path(values[0])

if args.phase in ['all', 'build']:
    # Pin the diagnostic profile rather than relying on runner/cache defaults.
    build = run(['cargo', 'build', '-p', 'wml2', '--locked', '--target-dir', out / '.test-target',
                 '--features', 'jpeg-xl,image-buffer-ex,color-management', '--message-format=json',
                 '--config', 'profile.dev.debug=2', '--config', 'profile.dev.opt-level=0',
                 '--config', 'profile.dev.strip="none"'])
    (out / 'build.json').write_text(build)
    run(['rustc', '--edition', '2024', '-g', '-C', 'relocation-model=static', '-C',
         'link-arg=-no-pie', '--crate-name', 'palette_cache_probe', probe,
         '--extern', 'wml2=' + str(artifact('wml2')), '--extern', 'bin_rs=' + str(artifact('bin_rs')),
         '-L', 'dependency=' + str(artifact('bin_rs').parent), '-o', exe])
    if args.phase == 'build':
        raise SystemExit(0)

cases = {}
stacks = {}
schedules = [('success', [4008, 0, 0]), ('native', [4008, 0, 0]), ('begin', [0, 0, 0]),
             ('init', [4008, 0, 0]), ('draw', [4008, 0, 0]), ('early_error', [0, 0, 0]),
             ('late_error', [4008, 0, 0]), ('budget', [0, 0, 0]),
             ('depths', [4008, 4008, 0, 0]), ('threads', [4008, 0])]
for mode, expected in schedules:
    path = out / (mode + '.log')
    if args.phase in ['all', 'matrix']:
        path.write_text(run([exe, mode]))
    text = path.read_text()
    values = []
    for line in text.splitlines():
        if line.startswith('mode='):
            assert 'external_frees=0,overflow=false' in line
            sizes = json.loads(re.search(r'live=(\[.*?\])', line).group(1))
            value = int(re.search(r'sum=(\d+)', line).group(1))
            assert sizes == ([24, 768, 1500, 1716] if value else [])
            values.append(value)
        if args.phase in ['all', 'symbols'] and mode == 'success' and line.startswith('trace'):
            size = re.search(r'bytes=(\d+)', line).group(1)
            addresses = re.search(r'ip=\[(.*?)\]', line).group(1).split(', ')
            # Include inlined callers; physical unwind frames alone omit them.
            symbols = run(['addr2line', '-i', '-f', '-C', '-e', exe] + ['0x' + v for v in addresses])
            (out / ('stack-' + size + '.txt')).write_text(symbols)
            assert 'ImplicitPalette::new' in symbols or (size == '24' and 'Palette::new::' in symbols)
            assert 'once_lock::OnceLock' in symbols
            stacks[size] = [s for s in symbols.splitlines()
                            if 'jxl::' in s or 'once_lock' in s or '/palette.rs:' in s]
    assert values == expected, (mode, values)
    cases[mode] = values
if args.phase == 'matrix':
    (out / 'matrix.json').write_text(json.dumps(cases, indent=2) + '\n')
    raise SystemExit(0)

if args.phase in ['all', 'source']:
    # Inspect the exact package Cargo compiled, including configured registry locations.
    packages = {item['manifest_path'] for item in artifacts()
                if item.get('reason') == 'compiler-artifact' and item['package_id'].endswith('#jxl@0.8.0')}
    assert len(packages) == 1, packages
    folder = pathlib.Path(next(iter(packages))).parent
    relative = 'src/frame/modular/transforms/palette.rs'
    data = (folder / relative).read_bytes()
    archive = folder.parents[2] / 'cache' / folder.parent.name / 'jxl-0.8.0.crate'
    with tarfile.open(archive) as tar:
        assert tar.extractfile('jxl-0.8.0/' + relative).read() == data
    digest = hashlib.sha256(data).hexdigest()
    assert digest == '358253013cf07ab3c7cfd845d559a4baadcb08be94986336c0a886ead625db1d'
    source = dict(crate='jxl', version='0.8.0', file=relative, sha256=digest, matches_locked_archive=True)
    (out / 'source.json').write_text(json.dumps(source, indent=2) + '\n')
    if args.phase == 'source':
        raise SystemExit(0)
else:
    source = json.loads((out / 'source.json').read_text())

report = dict(
    platform='Linux x86_64; ' + run(['rustc', '--version']).strip()
             + '; explicit dev opt0/debug2; jpeg-xl,image-buffer-ex,color-management',
    source=source, probe_sha256=hashlib.sha256(probe.read_bytes()).hexdigest(), cases=cases,
    allocation_stacks=stacks,
    attribution='Process-global static OnceLock<Box<ImplicitPalette>> by min(source bit depth,24),25 slots; '
                'three fixed RGB i32 arrays plus3-pointer boxed object. Retained until process exit, '
                'not image ownership or TLS. No general leak-absence inference.',
    requested_bytes=dict(arrays=[1716, 768, 1500], descriptor64=24, per_slot64=4008, all_slots64=100200,
                         descriptor32_static=12, per_slot32_static=3996, all_slots32_static=99900),
    method='Fixed TLS pointer table; address-matched alloc/free/realloc, empty baseline per measured call, '
           'no allocation by table. GCC unwinder captures allocation sites; addr2line including inline '
           'frames symbolizes outside measured interval. Separate fresh process for every mode. Thread '
           'creation outside measurement. No RSS/allocator-overhead/all-thread guarantee.')
(out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(dict(cases=cases, source=source), indent=2))
