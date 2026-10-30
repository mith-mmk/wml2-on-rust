#!/usr/bin/env python3
"""Evaluate pinned decoder cores on trusted fixtures, not a WML2 migration.

Outputs source/tool identity, independent-oracle comparisons and optionally
paired core-only allocation/time measurements. No untrusted input admission is
provided by the Rust probe. WML2's 0.7.4 runtime dependency is not edited.
"""
import argparse
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import statistics
import struct
import subprocess
import zlib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--bench', action='store_true', help='three alternating core-only pairs; run without other builds/tests')
a = parser.parse_args()
if not a.output.name.startswith('.test'):
    parser.error('--output must start with .test')
root = Path(__file__).resolve().parents[2]
out = a.output.resolve()
out.mkdir(parents=True, exist_ok=True)
fixtures = root / 'wml2/tests/fixtures/review'
manifest = out / 'Cargo.toml'
# JSON strings are valid TOML basic string literals for these absolute paths.
manifest.write_text('''[package]
name = "jxl08-evaluation"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
jxl07 = { package = "jxl", version = "=0.7.4", default-features = false }
jxl08 = { package = "jxl", version = "=0.8.0", default-features = false }
icc_profile = { package = "icc-profile", version = "=0.0.7" }
[[bin]]
name = "probe"
path = ''' + json.dumps(str(root / 'wml2-test/scripts/jxl08_probe.rs')) + '\n')
target = Path(os.environ.get('CARGO_TARGET_DIR', str(out / '.test-target'))).resolve()
build = subprocess.run(['cargo', 'build', '--release', '--manifest-path', str(manifest), '--target-dir', str(target)], text=True, capture_output=True)
(out / 'build.log').write_text(build.stdout + build.stderr)
build.check_returncode()
exe = target / 'release/probe'
temporary = out / '.test-pixels.bin'
report = {'checkout_sha': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
          'rustc': subprocess.check_output(['rustc', '-vV'], text=True),
          'scope': 'trusted-fixture core probe only; no WML2 admission or migrated API',
          'default_features': False,
          'fixture_manifest_sha256': {name: hashlib.sha256((fixtures/name).read_bytes()).hexdigest() for name in ['jxl-fixtures.json','jxl-additional-fixtures.json']},
          'comparisons': [], 'benchmarks': []}

def run(item, version, managed=False):
    data = (fixtures / item['file']).read_bytes()
    assert hashlib.sha256(data).hexdigest() == item['sha256'], item['file']
    cmd = [str(exe), str(fixtures/item['file']), version, '1', str(temporary)]
    if managed:
        cmd.append(str(root/'wml2/src/srgb.icc'))
    result = subprocess.run(cmd, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr)
    row = next(csv.DictReader(io.StringIO(result.stdout)))
    return temporary.read_bytes(), (int(row['width']), int(row['height']))

def compare(item, name, expected, size, tolerance, managed=False):
    for version in ['07', '08']:
        record = {'case': name, 'version': version, 'tolerance': tolerance}
        try:
            actual, dimensions = run(item, version, managed)
            if not managed:
                actual = [v[0] for v in struct.iter_unpack('<H', actual)]
            assert dimensions == tuple(size), dimensions
            assert len(actual) == len(expected), (len(actual),len(expected))
            delta = max(abs(x-y) for x,y in zip(actual,expected))
            record.update(max_difference=delta, passed=delta <= tolerance)
        except Exception as error:
            record.update(passed=False, error=str(error))
        report['comparisons'].append(record)
        print(json.dumps(record), flush=True)

original = json.loads((fixtures/'jxl-fixtures.json').read_text())
for item in original['cases']:
    raw = (fixtures/item['oracle']).read_bytes()
    bits = item['bits']; channels = item['oracle_channels']; maximum = (1<<bits)-1
    values = list(raw) if bits == 8 else [v[0] for v in struct.iter_unpack('<H',raw)]
    expected = []
    for at in range(0,len(values),channels):
        p = values[at:at+channels]
        pixel = [p[0],p[0],p[0],p[1] if channels==2 else maximum] if channels<=2 else p[:3]+[p[3] if channels==4 else maximum]
        expected.extend((v*65535+maximum//2)//maximum for v in pixel)
    tolerance = (514 if bits==8 else 32) if item['mode']=='lossy' else 1
    compare(item,item['file']+'/native16',expected,(item['width'],item['height']),tolerance)
additional = json.loads((fixtures/'jxl-additional-fixtures.json').read_text())
for item in additional['orientations']:
    compare(item,item['file']+'/orientation', [v*257 for v in (fixtures/item['expected']).read_bytes()], item['size'],1)
for item in additional['depths']:
    expected = [v[0] for v in struct.iter_unpack('<H',(fixtures/Path(item['file']).with_suffix('.rgba16le')).read_bytes())]
    compare(item,item['file']+'/normalized16',expected,item['size'],1)
for item in additional['managed']:
    expected = (fixtures/item['expected']).read_bytes()
    if item.get('compression')=='zlib':
        expected=zlib.decompress(expected)
    compare(item,item['file']+'/srgb',expected,item['size'],2,True)

if a.bench:
    for name, iterations in [('jxl-icc-lossless',150),('jxl-jbrd-bench',13),('jxl-sunset-logo',13)]:
        for pair in range(3):
            for version in (['07','08'] if pair%2==0 else ['08','07']):
                result=subprocess.check_output([str(exe),str(fixtures/(name+'.jxl')),version,str(iterations)],text=True)
                (out/f'{name}-{pair}-{version}.csv').write_text(result)
                rows=list(csv.DictReader(io.StringIO(result)))[3:]
                report['benchmarks'].append({'case':name,'pair':pair,'version':version,'warm_samples':len(rows),
                    **{key:statistics.median(int(row[key]) for row in rows) for key in ['ns','allocated_bytes','peak_extra_live_bytes']}})
report['passed']=all(r['passed'] for r in report['comparisons'])
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
temporary.unlink(missing_ok=True) # only this generated, reproducible scratch file
print(json.dumps({'passed':report['passed'],'comparisons':len(report['comparisons'])}))
if not report['passed']:
    raise SystemExit(1)
