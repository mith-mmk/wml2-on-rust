"""Seeded independent-PLT multiple tile-part pixel comparisons through the public Rust API.

OpenJPEG (via Pillow) only encodes fixtures and supplies independent pixels.
Usage: python3 .../check_jpeg2000_parts_random.py /path/to/jpeg2000_verify
"""
import argparse
import hashlib
import json
from io import BytesIO
from pathlib import Path
import random
import subprocess
import sys

sys.dont_write_bytecode = True

from PIL import Image, __version__, features
from generate_jpeg2000_parts import split_plt

parser = argparse.ArgumentParser()
parser.add_argument('verifier', type=Path)
parser.add_argument('--cases', type=int, default=24)
args = parser.parse_args()
assert args.cases > 0
repo = Path(__file__).resolve().parents[4]
work = repo / '.test-jpeg2000-parts-random'
work.mkdir(exist_ok=True)
rng = random.Random(0x50415254)
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
    strategy = ['packet', 'pair', 'alternate'][source_exact % 3]
    geometry_rows = []
    geometry_exact = True
    factor = 1 << levels
    tile_w, tile_h = factor + rng.randint(1, 5), factor + rng.randint(1, 6)
    nx, ny = rng.randint(1, 3), rng.randint(1, 3)
    if nx * ny == 1:
        nx = 2
    w = (nx - 1) * tile_w + rng.randint(1, tile_w)
    h = (ny - 1) * tile_h + rng.randint(1, tile_h)
    x0 = rng.randint(tile_w, 32768 - w)
    y0 = rng.randint(tile_h, 32768 - h)
    tx0 = x0 - rng.randrange(tile_w)
    ty0 = y0 - rng.randrange(tile_h)
    # Include the clipped first tile in the actual tile-count bound.
    count = ((x0+w-tx0+tile_w-1)//tile_w) * ((y0+h-ty0+tile_h-1)//tile_h)
    assert count <= 256
    cb = rng.choice(shapes)
    for components, mode in [(1, 'L'), (3, 'RGB')]:
        source = rng.randbytes(w * h * components)
        (work / f'{index}-{components}.source').write_bytes(source)
        image = Image.frombytes(mode, (w, h), source)
        for mct in ([0] if components == 1 else [0, 1]):
            for ext in ['j2k', 'jp2']:
                path = work / f'{index}-{components}-{mct}.{ext}'
                ppm = work / 'decoded.ppm'
                original = work / f'{index}-{components}-{mct}.plt.{ext}'
                image.save(original, format='JPEG2000', no_jp2=ext == 'j2k', plt=True,
                           num_resolutions=levels+1, codeblock_size=cb,
                           tile_size=(tile_w, tile_h), tile_offset=(tx0, ty0), offset=(x0,y0),
                           irreversible=False, mct=mct)
                with Image.open(original) as oracle:
                    assert oracle.size == (w,h) and oracle.mode == mode
                    roundtrip_exact = oracle.tobytes() == source
                    geometry_exact &= roundtrip_exact
                    expected = oracle.convert('RGB').tobytes()
                encoded, proof = split_plt(original.read_bytes(), strategy)
                path.write_bytes(encoded)
                with Image.open(BytesIO(encoded)) as multipart:
                    assert multipart.size == (w, h) and multipart.mode == mode
                    assert multipart.convert('RGB').tobytes() == expected
                subprocess.run([str(args.verifier.resolve()), str(path), str(ppm), '1'],
                               check=True, stdout=subprocess.DEVNULL)
                with Image.open(ppm) as decoded:
                    assert decoded.size == (w,h) and decoded.mode == 'RGB'
                    assert decoded.tobytes() == expected, path
                geometry_rows.append({'case':index, 'width':w, 'height':h, 'levels':levels,
                             'block':cb, 'tile':[tile_w,tile_h], 'tile_count':count, 'image_origin':[x0,y0], 'tile_origin':[tx0,ty0],
                             'components':components, 'mct':mct, 'container':ext,
                             'source_sha256':hashlib.sha256(source).hexdigest(),
                             'encoded_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
                             'rgb_sha256':hashlib.sha256(expected).hexdigest(),
                             'encoder_source_exact':roundtrip_exact, 'strategy':strategy,
                             'max_parts_per_tile':proof['max_parts_per_tile'], 'peak_join_bytes':proof['peak_join_bytes'],
                             'plt_proof':proof})
    if geometry_exact:
        rows.extend(geometry_rows)
        source_exact += 1
    else:
        compatibility_only.extend(geometry_rows)
report = {'seed':0x50415254, 'geometries':args.cases, 'pixel_comparisons':len(rows),
          'multipart_comparisons':sum(r['max_parts_per_tile'] > 1 for r in rows),
          'all_passed':True, 'attempted_geometries':attempt,
          'source_exact_geometries':source_exact, 'compatibility_only_comparisons':len(compatibility_only),
          'encoder_roundtrip_mismatches':sum(not r['encoder_source_exact'] for r in compatibility_only),
          'oracle_limit':'Known encoder-issued coefficient mismatches are recorded separately; every attempted file still matches Rust to OpenJPEG output.',
          'pillow':__version__, 'openjpeg':features.version_codec('jpg_2000'),
          'decoder_sha256':hashlib.sha256((repo/'wml2/src/jpeg2000.rs').read_bytes()).hexdigest(),
          'cases':rows, 'compatibility_only_cases':compatibility_only}
(work/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k not in ['cases','compatibility_only_cases']}))
