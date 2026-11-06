#!/usr/bin/env python3
"""Independent C libjxl float HDR fixtures; no WML2 output is used as oracle."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import zlib

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--reference', type=Path, required=True)
p.add_argument('--work', type=Path, required=True)
a = p.parse_args()
if not a.work.name.startswith('.test'):
    p.error('work must start .test')
a.work.mkdir(parents=True, exist_ok=True)
root = Path(__file__).resolve().parents[2]
dest = root / 'wml2/tests/fixtures/review'
reference = a.reference.resolve()

def sha(data):
    return hashlib.sha256(data).hexdigest()

def run(*args):
    return subprocess.run([str(reference), *map(str, args)], check=True, capture_output=True, text=True)

rows = []
for transfer in ['pq', 'hlg']:
    for bits in [16, 32]:
        values = [0.0, -0.0, 2.0 ** (-24 if bits == 16 else -149),
                  2.0 ** (-14 if bits == 16 else -126), 0.5, 1.0, 0.25, 0.75]
        for suffix, channels in [('rgb', 3), ('rgba-o6', 4)]:
            name = f'jxl-float-hdr-{transfer}-f{bits}-{suffix}'
            source = a.work / (name + '.raw')
            samples = []
            for i in range(15):
                samples.extend(values[(i * 3 + c) % len(values)] for c in range(3))
                if channels == 4:
                    samples.append(values[i % len(values)])
            source.write_bytes(struct.pack('<' + ('e' if bits == 16 else 'f') * len(samples), *samples))
            file = dest / (name + '.jxl')
            storage = f'f{bits}' + ('o6' if channels == 4 else '')
            run('encode', source, file, 5, 3, channels, storage, transfer, '1000:0.02:1:0.5', '-')
            output = a.work / (name + '.f32le')
            header = json.loads(run('decode', file, output, 'f32', '0').stdout)
            kept = a.work / (name + '.kept')
            run('decode', file, kept, 'f32', 'keep')
            # Check independent C ORIGINAL/DATA identity and authored source bits.
            expected = struct.pack('<' + 'f' * len(samples), *samples)
            assert kept.read_bytes() == expected
            assert header['version'] == 11002 and header['bits'] == bits
            assert header['exponent_bits'] == (5 if bits == 16 else 8)
            assert header['uses_original_profile'] and header['structured_profile'] and header['data_structured']
            assert header['transfer'] == header['data_transfer'] == (16 if transfer == 'pq' else 18)
            assert header['primaries'] == header['data_primaries'] == 9
            raw = output.read_bytes()
            if channels == 3:
                raw = b''.join(raw[i:i+12] + struct.pack('<f', 1.0) for i in range(0, len(raw), 12))
            (dest / (name + '.f32le.zlib')).write_bytes(zlib.compress(raw, 9))
            rows.append(dict(file=file.name, source_sha256=sha(source.read_bytes()),
                             input_sha256=sha(file.read_bytes()), rgba_sha256=sha(raw), header=header))

negative = []
for label, component, value, storage, transfer in [
    ('negative', 0, -0.25, 'f32', 'pq'),
    ('over-one', 1, 1.25, 'f32', 'hlg'),
    ('nan', 2, float('nan'), 'f32', 'pq'),
    ('infinite', 0, float('inf'), 'f32', 'hlg'),
    ('alpha-negative', 3, -0.25, 'f32', 'pq'),
    ('alpha-nan', 3, float('nan'), 'f32', 'hlg'),
    ('f16-nan', 1, float('nan'), 'f16', 'pq'),
    ('associated', 0, 0.25, 'f32-associated', 'pq'),
    ('p3', 0, 0.25, 'f32', 'pq-p3'),
]:
    name = 'jxl-float-hdr-refused-' + label
    source = a.work / (name + '.raw')
    samples = [0.25, 0.5, 0.75, 1.0] * 15
    samples[component] = value
    source.write_bytes(struct.pack('<' + ('e' if storage == 'f16' else 'f') * 60, *samples))
    file = dest / (name + '.jxl')
    run('encode', source, file, 5, 3, 4, storage, transfer, '1000', '-')
    decoded = subprocess.run([str(reference), 'decode', str(file), str(a.work / (name+'.out')), 'f32', '0'], capture_output=True)
    assert decoded.returncode == 0, (label, decoded.stderr)
    negative.append(dict(file=file.name, input_sha256=sha(file.read_bytes()),
                         c_decode_status=decoded.returncode, reason=label))
report = dict(oracle='C libjxl0.11.2, structured ORIGINAL/DATA F32 without intensity conversion; exact authored F16/F32 source values including subnormals and signed zero',
              helper_sha256=sha((root/'wml2-test/scripts/jpegxl_hdr_encoded_reference.c').read_bytes()),
              generator_sha256=sha(Path(__file__).read_bytes()), cases=rows, negative=negative)
(root/'docs/jpeg-xl-float-hdr-oracle.json').write_text(json.dumps(report, indent=2)+'\n')
print(f'{len(rows)} independent positive fixtures, {len(negative)} refused cases')
