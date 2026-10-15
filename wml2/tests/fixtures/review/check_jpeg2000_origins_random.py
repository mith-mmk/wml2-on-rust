"""Seeded unaligned-tile pixel comparisons through the public Rust API.

OpenJPEG (via Pillow) only encodes fixtures and supplies independent pixels.
Usage: python3 .../check_jpeg2000_origins_random.py /path/to/jpeg2000_verify
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
parser.add_argument('--cases', type=int, default=24)
args = parser.parse_args()
assert args.cases > 0
repo = Path(__file__).resolve().parents[4]
work = repo / '.test-jpeg2000-origins-random'
work.mkdir(exist_ok=True)
rng = random.Random(0x4f524947)
shapes = [(1 << x, 1 << y) for x in range(2, 11) for y in range(2, 11) if x + y <= 12]
rows = []
compatibility_only = []
source_exact = 0
attempt = 0
while source_exact < args.cases:
    index = attempt
    attempt += 1
    assert attempt <= args.cases * 10, 'oracle source-exact attempt bound exceeded'
    levels = source_exact % 6
    geometry_rows = []
    geometry_exact = True
    factor = 1 << levels
    tile_w, tile_h = factor + rng.randint(1, 5), factor + rng.randint(1, 6)
    nx, ny = rng.randint(1, 3), rng.randint(1, 3)
    if nx * ny == 1:
        nx = 2
    w = (nx - 1) * tile_w + rng.randint(1, tile_w)
    h = (ny - 1) * tile_h + rng.randint(1, tile_h)
    cb = rng.choice(shapes)
    for components, mode in [(1, 'L'), (3, 'RGB')]:
        source = rng.randbytes(w * h * components)
        (work / f'{index}-{components}.source').write_bytes(source)
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
                    roundtrip_exact = oracle.tobytes() == source
                    geometry_exact &= roundtrip_exact
                    expected = oracle.convert('RGB').tobytes()
                subprocess.run([str(args.verifier.resolve()), str(path), str(ppm), '1'],
                               check=True, stdout=subprocess.DEVNULL)
                with Image.open(ppm) as decoded:
                    assert decoded.size == (w,h) and decoded.mode == 'RGB'
                    assert decoded.tobytes() == expected, path
                geometry_rows.append({'case':index, 'width':w, 'height':h, 'levels':levels,
                             'block':cb, 'tile':[tile_w,tile_h], 'tile_count':nx*ny,
                             'components':components, 'mct':mct, 'container':ext,
                             'source_sha256':hashlib.sha256(source).hexdigest(),
                             'encoded_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
                             'rgb_sha256':hashlib.sha256(expected).hexdigest(),
                             'encoder_source_exact':roundtrip_exact})
    if geometry_exact:
        rows.extend(geometry_rows)
        source_exact += 1
    else:
        compatibility_only.extend(geometry_rows)
report = {'seed':0x4f524947, 'geometries':args.cases, 'pixel_comparisons':len(rows),
          'all_passed':True, 'attempted_geometries':attempt,
          'source_exact_geometries':source_exact, 'compatibility_only_comparisons':len(compatibility_only),
          'encoder_roundtrip_mismatches':sum(not r['encoder_source_exact'] for r in compatibility_only),
          'oracle_limit':'Known encoder-issued coefficient mismatches are recorded separately; every attempted file still matches Rust to OpenJPEG output.',
          'pillow':__version__, 'openjpeg':features.version_codec('jpg_2000'),
          'decoder_sha256':hashlib.sha256((repo/'wml2/src/jpeg2000.rs').read_bytes()).hexdigest(),
          'cases':rows, 'compatibility_only_cases':compatibility_only}
(work/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k not in ['cases','compatibility_only_cases']}))
