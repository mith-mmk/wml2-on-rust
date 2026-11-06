#!/usr/bin/env python3
"""Bounded before/after container admission probe using existing C pixel oracles."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import zlib

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--before', type=Path, required=True)
p.add_argument('--after', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
root = Path(__file__).resolve().parents[2]
out = a.output.resolve()
if not out.name.startswith('.test'):
    p.error('output must start .test')
out.mkdir(parents=True, exist_ok=False)
fixtures = root / 'wml2/tests/fixtures/review'
binaries = {'before': a.before.resolve(), 'after': a.after.resolve()}
def sha(data):
    return hashlib.sha256(data).hexdigest()
def box(kind, data):
    return (len(data)+8).to_bytes(4, 'big') + kind + data
sig = b'\0\0\0\x0cJXL \r\n\x87\n'
ftyp = box(b'ftyp', b'jxl \0\0\0\0jxl ')
raw = (fixtures/'jxl-rgba8.jxl').read_bytes()
assert raw[:2] == b'\xff\x0a'
empty = (fixtures/'jxl-brob-empty.br').read_bytes()
for name, records in [('opaque-1', box(b'test', b'0123456789abcdef')),
                      ('opaque-1024', box(b'test', b'0123456789abcdef')*1024),
                      ('empty-brob-1024', box(b'brob', b'test'+empty)*1024)]:
    (out/(name+'.jxl')).write_bytes(sig+ftyp+records+box(b'jxlc', raw))
hdr = (fixtures/'jxl-float-hdr-pq-f32-rgba-o6.jxl').read_bytes()
# Preserve original Level/container signaling when the C fixture has a container.
hdr_wrapped = hdr + box(b'test', b'0123456789abcdef') if hdr.startswith(sig) else sig+ftyp+box(b'test', b'0123456789abcdef')+box(b'jxlc', hdr)
(out/'hdr-opaque.jxl').write_bytes(hdr_wrapped)
cases = [('bare', fixtures/'jxl-rgba8.jxl', 'ordinary', 300),
         ('opaque-1', out/'opaque-1.jxl', 'ordinary', 300),
         ('plain', fixtures/'jxl-brob-metadata-plain.jxl', 'ordinary', 100),
         ('brob', fixtures/'jxl-brob-metadata.jxl', 'ordinary', 100),
         ('opaque-1024', out/'opaque-1024.jxl', 'ordinary', 20),
         ('empty-brob-1024', out/'empty-brob-1024.jxl', 'ordinary', 10),
         ('hdr-opaque', out/'hdr-opaque.jxl', 'hdr', 300)]
cpu = None
if hasattr(os, 'sched_getaffinity'):
    cpu = min(os.sched_getaffinity(0))
    os.sched_setaffinity(0, {cpu})
rows = []
for name, file, mode, n in cases:
    row = dict(name=name, input_sha256=sha(file.read_bytes()), mode=mode, samples=n)
    times = {'before': [], 'after': []}
    hashes = set()
    for which, exe in binaries.items():
        row[which+'_floor'] = json.loads(subprocess.check_output([str(exe), str(file), mode, 'floor'], cwd=root, text=True))['workspace_bytes']
    row['floor_delta'] = row['after_floor']-row['before_floor']
    for block in range(3):
        for which in (['before', 'after'] if block%2 == 0 else ['after', 'before']):
            args = [str(binaries[which]), str(file), mode, str(n)]
            if block == 0:
                args.append(str(out/(name+'-'+which+'.raw')))
            text = subprocess.check_output(args, cwd=root, text=True)
            (out/f'{name}-{which}-{block}.csv').write_text(text)
            samples = list(csv.DictReader(text.splitlines()))
            hashes.update(r['hash'] for r in samples)
            times[which].append(statistics.median(int(r['ns']) for r in samples))
    assert len(hashes) == 1
    actual = (out/(name+'-after.raw')).read_bytes()
    assert actual == (out/(name+'-before.raw')).read_bytes()
    expected = zlib.decompress((fixtures/'jxl-float-hdr-pq-f32-rgba-o6.f32le.zlib').read_bytes()) if mode == 'hdr' else (fixtures/'jxl-rgba8-djxl.bin').read_bytes()
    assert actual == expected
    row.update(block_medians_ns=times, ratio=statistics.median(times['after'])/statistics.median(times['before']), independent_pixel_bytes_exact=True)
    rows.append(row)
assert rows[0]['floor_delta'] == 0
assert rows[1]['floor_delta'] > 0 and rows[-1]['floor_delta'] == rows[1]['floor_delta']
assert rows[4]['floor_delta'] == rows[1]['floor_delta']*256
assert rows[5]['floor_delta'] == rows[1]['floor_delta']*512
report = dict(method=f'identical release feature graph/probe; CPU{cpu};3 alternating pairs,3 warmups; times exclude serialization/hash; floors check exact/one-under',
              probe_sha256=sha((root/'wml2-test/scripts/jpegxl_format_budget_probe.rs').read_bytes()),
              generator_sha256=sha(Path(__file__).read_bytes()), binaries_sha256={k: sha(v.read_bytes()) for k, v in binaries.items()}, cases=rows)
(out/'report.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report, indent=2))
