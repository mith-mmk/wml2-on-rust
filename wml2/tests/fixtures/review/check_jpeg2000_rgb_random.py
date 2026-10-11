"""Reproducible RGB/Gray OpenJPEG comparisons through the Rust public API.

Usage: python .../check_jpeg2000_rgb_random.py /path/to/jpeg2000_verify
Requires Pillow/OpenJPEG only for test input generation and the pixel oracle.
"""
import argparse
import hashlib
import json
import random
import subprocess
from pathlib import Path
from PIL import Image, features, __version__

parser = argparse.ArgumentParser()
parser.add_argument('binary')
parser.add_argument('--cases', type=int, default=128)
args = parser.parse_args()
if args.cases < 1:
    parser.error('cases must be positive')
binary = str(Path(args.binary).resolve())
work = Path('.test-jpeg2000-rgb-random')
work.mkdir(exist_ok=True)
rng = random.Random(200083)
rows = []
for index in range(args.cases):
    w, h = rng.randint(1, 96), rng.randint(1, 96)
    level = min(index % 6, min(w, h).bit_length() - 1)
    # The same geometry is checked with independent RGB components, RCT, Gray,
    # and both file containers so neither mode is tied to a particular shape.
    for mode, mct in [('RGB', 0), ('RGB', 1), ('L', 0)]:
        pixels = rng.randbytes(w * h * (3 if mode == 'RGB' else 1))
        source = Image.frombytes(mode, (w, h), pixels)
        for ext in ['j2k', 'jp2']:
            path = work / f'input.{ext}'
            source.save(path, format='JPEG2000', no_jp2=ext == 'j2k',
                        num_resolutions=level + 1, codeblock_size=(16, 16),
                        irreversible=False, mct=mct)
            with Image.open(path) as oracle:
                assert oracle.tobytes() == pixels, (index, mode, mct, ext)
                expected = oracle.convert('RGB').tobytes()
            output = work / 'output.ppm'
            subprocess.run([binary, str(path), str(output), '1'],
                           stdout=subprocess.DEVNULL, check=True)
            with Image.open(output) as got:
                assert got.size == (w, h) and got.tobytes() == expected, (
                    index, w, h, level, mode, mct, ext)
            rows.append({'case': index, 'width': w, 'height': h, 'level': level,
                         'mode': mode, 'mct': mct, 'container': ext,
                         'source_sha256': hashlib.sha256(pixels).hexdigest()})
report = {'seed': 200083, 'geometries': args.cases, 'comparisons': len(rows),
          'matched': True, 'pillow': __version__,
          'openjpeg': features.version_codec('jpg_2000'), 'cases': rows}
(work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
print({k: v for k, v in report.items() if k != 'cases'})
