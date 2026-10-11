"""Additional independent OpenJPEG oracles, outside Cargo/production code.
Usage: python .../check_jpeg2000_random.py /absolute/path/to/jpeg2000_verify
Requires Pillow with OpenJPEG. Writes only to a .test* folder in the cwd.
"""
import json
import random
import subprocess
import argparse
from pathlib import Path
from PIL import Image, features, __version__

parser=argparse.ArgumentParser()
parser.add_argument('binary')
parser.add_argument('--max-dimension',type=int,default=16)
parser.add_argument('--cases',type=int,default=256)
args=parser.parse_args()
if not 1 <= args.max_dimension <= 1024 or args.cases < 1:
    parser.error('dimensions must be 1..1024 and cases positive')
binary = str(Path(args.binary).resolve())
work = Path('.test-jpeg2000-random')
work.mkdir(exist_ok=True)
rng = random.Random(200053)
for index in range(args.cases):
    width, height = rng.randint(1, args.max_dimension), rng.randint(1, args.max_dimension)
    level = rng.randint(0, 1) if min(width, height) > 1 else 0
    gray = bytes(rng.randrange(args.cases) for _ in range(width * height))
    source = Image.frombytes('L', (width, height), gray)
    path = work / ('input.jp2' if index % 2 else 'input.j2k')
    source.save(path, format='JPEG2000', no_jp2=index % 2 == 0,
                num_resolutions=level + 1, codeblock_size=(16, 16), irreversible=False)
    assert Image.open(path).tobytes() == gray
    output = work / 'output.ppm'
    subprocess.run([binary, str(path), str(output), '1'],
                   stdout=subprocess.DEVNULL, check=True)
    assert Image.open(output).tobytes() == bytes(v for g in gray for v in (g, g, g)), (
        index, width, height, level)
report = {'cases': args.cases, 'max_dimension':args.max_dimension, 'seed': 200053, 'matched': True,
          'pillow': __version__, 'openjpeg': features.version('jpg_2000')}
(work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
