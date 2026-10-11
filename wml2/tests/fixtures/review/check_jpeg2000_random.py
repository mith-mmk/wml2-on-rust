"""Additional independent OpenJPEG oracles, outside Cargo/production code.
Usage: python .../check_jpeg2000_random.py /absolute/path/to/jpeg2000_verify
Requires Pillow with OpenJPEG. Writes only to a .test* folder in the cwd.
"""
import json
import random
import subprocess
import sys
from pathlib import Path
from PIL import Image, features, __version__

binary = str(Path(sys.argv[1]).resolve())
work = Path('.test-jpeg2000-random')
work.mkdir(exist_ok=True)
rng = random.Random(200053)
for index in range(256):
    width, height = rng.randint(1, 16), rng.randint(1, 16)
    level = rng.randint(0, 1) if min(width, height) > 1 else 0
    gray = bytes(rng.randrange(256) for _ in range(width * height))
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
report = {'cases': 256, 'seed': 200053, 'matched': True,
          'pillow': __version__, 'openjpeg': features.version('jpg_2000')}
(work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
