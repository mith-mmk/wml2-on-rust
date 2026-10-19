#!/usr/bin/env python3
"""Pin official JPEG XL still/JPEG-reconstruction fixtures and pixel oracles.

Requires numpy and djxl 0.11.2. Uses official float samples and ICC objects,
never WML2 output. No Git hooks or corpus-wide object downloads are needed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import struct
import urllib.request
import zlib

import numpy as np

REVISION = 'b1d0f990b03e57bf6d137c365cd5dc8b470b9191'
CASES = [
    ('bench_oriented_brg', 'bench', 606, 500, 3, 5,
     'e223fed907c6238622b2b6ec1c80609050c9d2db4d759cf3ca6f0db304cbb82a'),
    ('grayscale_jpeg', 'gray', 200, 200, 1, 1,
     '346cc92b55864b0efb9f85cdddd02b586473ff18b03718e5a02fa03cb3e86844'),
]
p = argparse.ArgumentParser()
p.add_argument('--source', type=Path, required=True, help='official conformance Git checkout')
p.add_argument('--tools', type=Path, required=True, help='libjxl prefix containing bin/djxl')
p.add_argument('--work', type=Path, required=True)
a = p.parse_args()
if not a.work.name.startswith('.test'):
    p.error('--work must start with .test')
revision = subprocess.check_output(['git', '-C', str(a.source), 'rev-parse', 'HEAD'], text=True).strip()
if revision != REVISION:
    p.error('conformance checkout must be pinned to ' + REVISION)
root = Path(__file__).resolve().parents[2]
dest = root / 'wml2/tests/fixtures/review'
a.work.mkdir(parents=True, exist_ok=True)
version = subprocess.run([str(a.tools / 'bin/djxl'), '--version'], capture_output=True, text=True, check=True)
if 'v0.11.2' not in (version.stdout + version.stderr).split():
    p.error('fixture oracle must use djxl0.11.2')
cases = []
files = {}


def save(name, data):
    (dest / name).write_bytes(data)
    files[name] = hashlib.sha256(data).hexdigest()


for source, short, width, height, channels, orientation, input_sha in CASES:
    base = a.source / 'testcases' / source
    config = json.loads(subprocess.check_output([
        'git', '-C', str(a.source), 'show', f'{REVISION}:testcases/{source}/test.json']))
    data = (base / 'input.jxl').read_bytes()
    assert hashlib.sha256(data).hexdigest() == input_sha
    prefix = 'jxl-jbrd-' + short
    save(prefix + '.jxl', data)
    objects = {}
    for name in ['original.icc', 'reference_image.npy']:
        sha = config['sha256sums'][name]
        url = 'https://storage.googleapis.com/jxl-conformance/objects/' + sha
        cache = a.work / (source + '-' + name)
        blob = cache.read_bytes() if cache.exists() else urllib.request.urlopen(url, timeout=60).read()
        assert hashlib.sha256(blob).hexdigest() == sha
        cache.write_bytes(blob)
        objects[name] = dict(sha256=sha, url=url)
    save(prefix + '.icc', (a.work / (source + '-original.icc')).read_bytes())
    floats = np.load(a.work / (source + '-reference_image.npy'), allow_pickle=False)
    assert floats.dtype == np.dtype('float32')
    assert floats.shape == (1, height, width, channels)
    # EX is native integer8 and ordinary is RGBA8, so compare in the declared
    # integer domain, not against the float reference's unquantized RMS bound.
    rounded = np.clip(np.floor(floats * 255 + 0.5), 0, 255).astype(np.uint8).tobytes()
    save(prefix + '-reference.bin.zlib', zlib.compress(rounded, 9))
    probes = bytearray()
    for y in [0, height // 4, height // 2, height * 3 // 4, height - 1]:
        for x in [0, width // 4, width // 2, width * 3 // 4, width - 1]:
            offset = (y * width + x) * channels
            probes.extend(struct.pack('<HH', x, y) + rounded[offset:offset + channels])
    save(prefix + '-probes.bin', probes)
    pam = a.work / (source + '.pam')
    subprocess.run([str(a.tools / 'bin/djxl'), str(base / 'input.jxl'), str(pam),
                    '--bits_per_sample=8', '--quiet'], check=True, capture_output=True)
    header, pixels = pam.read_bytes().split(b'ENDHDR\n', 1)
    fields = dict(line.split(maxsplit=1) for line in header.splitlines()[1:] if line)
    assert [int(fields[k]) for k in [b'WIDTH', b'HEIGHT', b'DEPTH', b'MAXVAL']] == [width, height, channels, 255]
    assert len(pixels) == width * height * channels
    save(prefix + '-djxl.bin.zlib', zlib.compress(pixels, 9))
    cases.append(dict(source=source, prefix=prefix, width=width, height=height,
                      channels=channels, bits=8, coded_orientation=orientation,
                      input_sha256=input_sha, official_objects=objects,
                      oracle_sha256=hashlib.sha256(pixels).hexdigest(),
                      rounded_reference_sha256=hashlib.sha256(rounded).hexdigest(),
                      source_url=f'https://github.com/libjxl/conformance/tree/{REVISION}/testcases/{source}'))

manifest = dict(repository='https://github.com/libjxl/conformance', revision=REVISION,
                decoder='djxl 0.11.2; default output profile; oriented integer8 PAM',
                reference='Official float32 reference: clamp and floor(sample*255+0.5); output stays in official ICC profile',
                tolerance='At most2/255 per output channel; source ICC bytes must match exactly',
                licenses='Both source images are CC0; test bitstreams and decoded references may be reproduced freely for testing; see testcases/README.md at pinned revision',
                cases=cases, files=files)
(dest / 'jxl-jbrd-fixtures.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(dict(cases=len(cases), total_bytes=sum((dest / f).stat().st_size for f in files))))
