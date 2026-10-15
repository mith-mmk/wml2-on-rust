"""Reproducible reversible Gray8/RGB8 code-block geometry fixtures.

Pillow/OpenJPEG only encodes fixtures and independently checks source pixels.
Run from the repository root. Existing fixtures are read, never changed.
Manifest: encoding SHA/name, source SHA/name, w/h, block w/h, levels, nc, MCT.
"""
import hashlib
from pathlib import Path

from PIL import Image, __version__, features

ROOT = Path(__file__).resolve().parent
VERSIONS = f'Pillow {__version__} OpenJPEG {features.version_codec("jpg_2000")}'


def pixels(width, height, components, pattern):
    palette = [(0, 0, 0), (255, 255, 255), (255, 0, 0), (0, 255, 0),
               (0, 0, 255), (1, 2, 3), (3, 2, 1), (127, 128, 129)]
    output = bytearray()
    for y in range(height):
        for x in range(width):
            if components == 1:
                if pattern == 'noise':
                    value = (x * 73 + y * 151 + x * y * 17 + 31) % 256
                elif pattern == 'mixed':
                    region = (x // 16 + y // 16) % 3
                    value = (128 if region == 0 else 128 + (x + y) % 16
                             if region == 1 else (x * 13 + y * 11) % 256)
                else:
                    value = (x * 9 + y * 7) % 256
                output.append(value)
            elif pattern == 'mixed':
                output.extend(palette[(x + 3 * y) % len(palette)])
            elif pattern == 'ramp':
                output.extend(((x + y) % 256, (x * 3 + y) % 256,
                               (x + y * 5) % 256))
            else:
                output.extend(((x * 73 + y * 151 + x * y * 17 + 31) % 256,
                               (x * 11 + y * 97 + x * y * 31 + 83) % 256,
                               (x * 137 + y * 37 + x * y * 7 + 197) % 256))
    return bytes(output)


def codestream(encoded):
    if encoded[:2] == bytes.fromhex('ff4f'):
        return encoded
    position = 0
    while position < len(encoded):
        length = int.from_bytes(encoded[position:position + 4], 'big')
        kind = encoded[position + 4:position + 8]
        assert length >= 8
        if kind == b'jp2c':
            return encoded[position + 8:position + length]
        position += length
    raise AssertionError('missing JP2 codestream')


def assert_coding(encoded, width, height, cbwidth, cbheight, levels, components, mct):
    data, position = codestream(encoded), 2
    assert data[:2] == bytes.fromhex('ff4f')
    found_siz = False
    while position < len(data):
        marker = data[position:position + 2]
        length = int.from_bytes(data[position + 2:position + 4], 'big')
        assert length >= 2
        body = data[position + 4:position + 2 + length]
        if marker == bytes.fromhex('ff51'):
            assert int.from_bytes(body[2:6], 'big') == width
            assert int.from_bytes(body[6:10], 'big') == height
            assert int.from_bytes(body[34:36], 'big') == components
            assert body[36:] == bytes([7, 1, 1]) * components
            found_siz = True
        elif marker == bytes.fromhex('ff52'):
            assert found_siz and len(body) == 10
            assert body[:5] == bytes([0, 0, 0, 1, mct])
            assert body[5] == levels
            assert (1 << (body[6] + 2), 1 << (body[7] + 2)) == (cbwidth, cbheight)
            assert body[8:] == bytes([0, 1])
            return
        position += length + 2
    raise AssertionError('missing COD')


manifest = [VERSIONS]


def case(width, height, cbwidth, cbheight, levels, pattern, *, default_block=False,
         existing_sources=None):
    for components, mode in [(1, 'gray'), (3, 'rgb')]:
        stem = f'jp2_blocks_{width}x{height}_d{levels}_{mode}_b{cbwidth}x{cbheight}_{pattern}'
        suffix = '.gray' if components == 1 else '.rgb'
        if existing_sources is None:
            source_path = ROOT / (stem + suffix)
            raw = pixels(width, height, components, pattern)
            source_path.write_bytes(raw)
        else:
            source_path = ROOT / existing_sources[components]
            raw = source_path.read_bytes()
        assert len(raw) == width * height * components
        source = Image.frombytes('L' if components == 1 else 'RGB', (width, height), raw)
        source_hash = hashlib.sha256(raw).hexdigest()
        for mct in ([0] if components == 1 else [0, 1]):
            for ext in ['j2k', 'jp2']:
                path = ROOT / f'{stem}_mct{mct}.{ext}'
                options = dict(format='JPEG2000', no_jp2=ext == 'j2k',
                               num_resolutions=levels + 1, irreversible=False, mct=mct)
                if not default_block:
                    options['codeblock_size'] = (cbwidth, cbheight)
                source.save(path, **options)
                with Image.open(path) as oracle:
                    assert oracle.size == (width, height)
                    assert oracle.mode == ('L' if components == 1 else 'RGB')
                    assert oracle.tobytes() == raw, path
                encoded = path.read_bytes()
                assert_coding(encoded, width, height, cbwidth, cbheight, levels, components, mct)
                fields = [hashlib.sha256(encoded).hexdigest(), path.name,
                          source_hash, source_path.name, width, height, cbwidth,
                          cbheight, levels, components, mct]
                manifest.append(' '.join(map(str, fields)))


geometries = [(1 << (bx + 2), 1 << (by + 2))
              for bx in range(9) for by in range(9 - bx)]
assert len(geometries) == 45
assert sum(w * h == 4096 for w, h in geometries) == 9
for index, (cbwidth, cbheight) in enumerate(geometries):
    width, height = min(1024, cbwidth + 3), min(1024, cbheight + 5)
    levels = (0 if cbwidth * cbheight == 4096
              else min(index % 6, min(width, height).bit_length() - 1))
    case(width, height, cbwidth, cbheight, levels, ['noise', 'mixed', 'ramp'][index % 3])
assert len(manifest) - 1 == 270
for levels in range(6):
    case(257, 129, 64, 64, levels, 'noise')
case(65, 67, 64, 64, 1, 'mixed', default_block=True)
case(1024, 1024, 64, 64, 5, 'ramp', existing_sources={
    1: 'jp2_deep_1024x1024_d5_ramp.gray',
    3: 'jp2_rgb_1024x1024_d5_ramp.rgb',
})
assert len(manifest) - 1 == 318
assert all(len(line.split()) == 11 for line in manifest[1:])
(ROOT / 'jpeg2000-blocks-manifest.txt').write_text('\n'.join(manifest) + '\n')
print(VERSIONS, 'encodings:', len(manifest) - 1, 'geometries:', len(geometries))
