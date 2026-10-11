"""Paired public-API benchmark. Run from repository root after other builds/tests.
Usage: python .../benchmark_jpeg2000_multi.py BASE_BINARY HEAD_BINARY
Linux CPU affinity and POSIX process CPU time; no production decoder dependency.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('base_binary')
parser.add_argument('head_binary')
args = parser.parse_args()
executables = {'base': str(Path(args.base_binary).resolve()),
               'head': str(Path(args.head_binary).resolve())}
work = Path('.test-jpeg2000-benchmark')
work.mkdir(exist_ok=True)
cpu = min(os.sched_getaffinity(0))
os.sched_setaffinity(0, {cpu})
rows = []
for pattern in ['noise', 'ramp', 'checker']:
    for version in ['base', 'head', 'head', 'base']:
        path = f'wml2/tests/fixtures/review/jp2_16x16_d1_{pattern}.jp2'
        output = work / f'{version}.ppm'
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        result = subprocess.check_output([executables[version], path, str(output), '20000'],
                                         text=True).strip()
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        cpu_seconds = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
        rows.append({'input': path, 'pattern': pattern, 'version': version,
                     'result': result, 'process_cpu_us_per_call': cpu_seconds * 1e6 / 20101,
                     'sha256': hashlib.sha256(output.read_bytes()).hexdigest()})
    assert len({r['sha256'] for r in rows if r['pattern'] == pattern}) == 1
for name in ['65x49_d1_noise', '257x129_d1_mixed', '1024x1024_d1_ramp']:
    path = f'wml2/tests/fixtures/review/jp2_multi_{name}.jp2'
    output = work / 'large.ppm'
    result = subprocess.check_output([executables['head'], path, str(output), '20'],
                                     text=True).strip()
    rows.append({'input': path, 'version': 'head', 'result': result,
                 'sha256': hashlib.sha256(output.read_bytes()).hexdigest()})
report = {'cpu_affinity': cpu, 'base_order': 'B/H/H/B',
          'small_iterations': 20000, 'warmups': 100,
          'process_cpu_note': 'whole child including warmups, output export and startup / 20101 calls',
          'rows': rows}
(work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
for row in rows:
    print(row['version'], row['result'], row.get('process_cpu_us_per_call', ''))
