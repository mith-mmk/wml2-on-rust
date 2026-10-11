"""Synthetic RGB8 sources; Pillow/OpenJPEG only encodes fixtures and checks pixels.

Run from the repository root. No production decoder dependency is added.
"""
import argparse
import hashlib
from pathlib import Path
from PIL import Image, features, __version__

parser = argparse.ArgumentParser()
parser.add_argument('--mct', type=int, nargs='+', choices=[0, 1], default=[0])
args = parser.parse_args()
root = Path('wml2/tests/fixtures/review')
cases = [(1, 1, 0), (1, 33, 0), (33, 1, 0), (2, 2, 1), (3, 5, 0),
         (3, 5, 1), (16, 16, 0), (16, 16, 1), (16, 16, 4), (17, 19, 1),
         (17, 19, 4), (31, 33, 0), (32, 32, 5), (33, 35, 5), (65, 49, 2),
         (65, 49, 5), (257, 129, 3), (257, 129, 5), (1024, 1024, 5)]
manifest = [f'Pillow {__version__} OpenJPEG {features.version_codec("jpg_2000")}']
for w, h, level in cases:
    for pattern in (['ramp'] if w == 1024 else ['noise', 'mixed']):
        pixels = bytearray()
        palette = [(0, 0, 0), (255, 255, 255), (255, 0, 0), (0, 255, 0),
                   (0, 0, 255), (1, 2, 3), (3, 2, 1), (127, 128, 129)]
        for y in range(h):
            for x in range(w):
                rgb = ((x * 73 + y * 151 + x * y * 17 + 31) % 256,
                       (x * 11 + y * 97 + x * y * 31 + 83) % 256,
                       (x * 137 + y * 37 + x * y * 7 + 197) % 256)
                if pattern == 'mixed':
                    rgb = palette[(x + 3 * y) % len(palette)]
                elif pattern == 'ramp':
                    rgb = ((x + y) % 256, (x * 3 + y) % 256, (x + y * 5) % 256)
                pixels.extend(rgb)
        pixels = bytes(pixels)
        stem = f'jp2_rgb_{w}x{h}_d{level}_{pattern}'
        source = Image.frombytes('RGB', (w, h), pixels)
        raw = root / f'{stem}.rgb'
        raw.write_bytes(pixels)
        manifest.append(f'{hashlib.sha256(pixels).hexdigest()} {raw.name}')
        for mct in args.mct:
            for ext in ['j2k', 'jp2']:
                path = root / f'{stem}_mct{mct}.{ext}'
                source.save(path, format='JPEG2000', no_jp2=ext == 'j2k',
                            num_resolutions=level + 1, codeblock_size=(16, 16),
                            irreversible=False, mct=mct)
                with Image.open(path) as oracle:
                    assert oracle.mode == 'RGB' and oracle.tobytes() == pixels, path
                encoded = path.read_bytes()
                cod = encoded.index(b'\xff\x52')
                assert encoded[cod + 8] == mct, path
                manifest.append(f'{hashlib.sha256(encoded).hexdigest()} {path.name}')
(root / 'jpeg2000-rgb-manifest.txt').write_text('\n'.join(manifest) + '\n')
print(manifest[0], 'sources:', 37, 'encodings:', 74 * len(args.mct))
