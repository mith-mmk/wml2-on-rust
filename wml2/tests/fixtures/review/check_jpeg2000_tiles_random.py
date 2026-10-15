"""Seeded multi-tile pixel comparisons through the public Rust API.

OpenJPEG (via Pillow) only encodes fixtures and supplies independent pixels.
Usage: python3 .../check_jpeg2000_tiles_random.py /path/to/jpeg2000_verify
"""
import argparse
import hashlib
import json
from pathlib import Path
import random
import subprocess

from PIL import Image, __version__, features

parser = argparse.ArgumentParser()
parser.add_argument('verifier', type=Path)
parser.add_argument('--cases', type=int, default=18)
args = parser.parse_args()
assert args.cases > 0
repo = Path(__file__).resolve().parents[4]
work = repo / '.test-jpeg2000-tiles-random'
work.mkdir(exist_ok=True)
rng = random.Random(0x54494c45)
shapes = [(1 << x, 1 << y) for x in range(2, 11) for y in range(2, 11) if x + y <= 12]
rows = []
for index in range(args.cases):
    levels = index % 6
    factor = 1 << levels
    tile_w, tile_h = factor * rng.randint(1, 3), factor * rng.randint(1, 3)
    nx, ny = rng.randint(1, 3), rng.randint(1, 3)
    if nx * ny == 1:
        nx = 2
    w = (nx - 1) * tile_w + rng.randint(factor, tile_w)
    h = (ny - 1) * tile_h + rng.randint(factor, tile_h)
    cb = rng.choice(shapes)
    for components, mode in [(1, 'L'), (3, 'RGB')]:
        source = rng.randbytes(w * h * components)
        image = Image.frombytes(mode, (w, h), source)
        for mct in ([0] if components == 1 else [0, 1]):
            for ext in ['j2k', 'jp2']:
                path = work / f'{index}-{components}-{mct}.{ext}'
                ppm = work / 'decoded.ppm'
                image.save(path, format='JPEG2000', no_jp2=ext == 'j2k',
                           num_resolutions=levels+1, codeblock_size=cb,
                           tile_size=(tile_w, tile_h), tile_offset=(0, 0),
                           irreversible=False, mct=mct)
                with Image.open(path) as oracle:
                    assert oracle.size == (w,h) and oracle.mode == mode
                    assert oracle.tobytes() == source
                    expected = oracle.convert('RGB').tobytes()
                subprocess.run([str(args.verifier.resolve()), str(path), str(ppm), '1'],
                               check=True, stdout=subprocess.DEVNULL)
                with Image.open(ppm) as decoded:
                    assert decoded.size == (w,h) and decoded.mode == 'RGB'
                    assert decoded.tobytes() == expected, path
                rows.append({'case':index, 'width':w, 'height':h, 'levels':levels,
                             'block':cb, 'tile':[tile_w,tile_h], 'tile_count':nx*ny,
                             'components':components, 'mct':mct, 'container':ext,
                             'source_sha256':hashlib.sha256(source).hexdigest(),
                             'encoded_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
                             'rgb_sha256':hashlib.sha256(expected).hexdigest()})
report = {'seed':0x54494c45, 'geometries':args.cases, 'pixel_comparisons':len(rows),
          'all_passed':True, 'pillow':__version__, 'openjpeg':features.version_codec('jpg_2000'),
          'decoder_sha256':hashlib.sha256((repo/'wml2/src/jpeg2000.rs').read_bytes()).hexdigest(),
          'cases':rows}
(work/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k != 'cases'}))
