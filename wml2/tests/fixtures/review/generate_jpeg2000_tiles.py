"""Deterministic reversible Gray/RGB multi-tile fixtures, OpenJPEG oracle only.

Run from the repository root. All outputs belong to this generator.
Supported manifest fields (14 whitespace-separated fields per encoding):
encoded_sha filename source_sha source_filename width height cbwidth cbheight
levels components mct tilewidth tileheight tilecount.
The separate unsupported manifest adds field 15: misaligned, tiny, or count.
Each manifest starts with one Pillow/OpenJPEG version line.

T.800 B-15/B.7: band origins are ceil((origin-high*2^(depth-1))/2^depth).
If origin is aligned to 2^levels, each band origin simplifies to origin>>depth.
Code-block grids are anchored at global zero, not at each tile's origin.

OpenJPEG emits neutral planes as present packets with no included blocks
(one 0x80 header byte each). The flat case replaces those body-less headers
with standard 0x00 empty-packet headers without changing lengths, then checks
the resulting files through OpenJPEG again. No entropy coding is implemented.
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
                if pattern == 'flat':
                    value = 128
                elif pattern == 'noise':
                    value = (x * 73 + y * 151 + x * y * 17 + 31) % 256
                elif pattern == 'mixed':
                    region = (x // 16 + y // 16) % 3
                    value = (128 if region == 0 else 128 + (x + y) % 16
                             if region == 1 else (x * 13 + y * 11) % 256)
                else:
                    value = (x * 9 + y * 7) % 256
                output.append(value)
            elif pattern == 'flat':
                output.extend((128, 128, 128))
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
        assert length >= 8
        if encoded[position + 4:position + 8] == b'jp2c':
            return encoded[position + 8:position + length]
        position += length
    raise AssertionError('missing JP2 codestream')


def assert_headers(encoded, width, height, cbwidth, cbheight, levels, components,
                   mct, tilewidth, tileheight, tilecount, *, flat=False):
    data, position, found_siz, found_cod = codestream(encoded), 2, False, False
    assert data[:2] == bytes.fromhex('ff4f')
    while data[position:position + 2] != bytes.fromhex('ff90'):
        marker = data[position:position + 2]
        length = int.from_bytes(data[position + 2:position + 4], 'big')
        assert length >= 2
        body = data[position + 4:position + 2 + length]
        if marker == bytes.fromhex('ff51'):
            assert int.from_bytes(body[2:6], 'big') == width
            assert int.from_bytes(body[6:10], 'big') == height
            assert body[10:18] == bytes(8) and body[26:34] == bytes(8)
            assert int.from_bytes(body[18:22], 'big') == tilewidth
            assert int.from_bytes(body[22:26], 'big') == tileheight
            assert int.from_bytes(body[34:36], 'big') == components
            assert body[36:] == bytes([7, 1, 1]) * components
            found_siz = True
        elif marker == bytes.fromhex('ff52'):
            assert len(body) == 10 and body[:5] == bytes([0, 0, 0, 1, mct])
            assert body[5] == levels
            assert (1 << (body[6] + 2), 1 << (body[7] + 2)) == (cbwidth, cbheight)
            assert body[8:] == bytes([0, 1])
            found_cod = True
        position += length + 2
    assert found_siz and found_cod
    for tile in range(tilecount):
        assert data[position:position + 2] == bytes.fromhex('ff90')
        assert data[position + 2:position + 4] == bytes.fromhex('000a')
        assert int.from_bytes(data[position + 4:position + 6], 'big') == tile
        length = int.from_bytes(data[position + 6:position + 10], 'big')
        assert length >= 14
        assert data[position + 10:position + 14] == bytes.fromhex('0001ff93')
        if flat:
            assert data[position + 14:position + length] == bytes((levels + 1) * components)
        position += length
    assert data[position:] == bytes.fromhex('ffd9')


def empty_flat_packets(encoded, levels, components, tilecount):
    data = codestream(encoded)
    stream_offset = encoded.find(data)
    assert stream_offset >= 0
    position = 2
    while data[position:position + 2] != bytes.fromhex('ff90'):
        position += int.from_bytes(data[position + 2:position + 4], 'big') + 2
    result = bytearray(encoded)
    packets = (levels + 1) * components
    for _ in range(tilecount):
        length = int.from_bytes(data[position + 6:position + 10], 'big')
        body = data[position + 14:position + length]
        assert body == bytes([0x80]) * packets or body == bytes(packets)
        result[stream_offset + position + 14:stream_offset + position + length] = bytes(packets)
        position += length
    return bytes(result)


supported, unsupported = [VERSIONS], [VERSIONS]


def case(width, height, tilewidth, tileheight, levels, cbwidth, cbheight, pattern,
         *, rejection=None):
    tiles_x = (width + tilewidth - 1) // tilewidth
    tiles_y = (height + tileheight - 1) // tileheight
    tilecount, factor = tiles_x * tiles_y, 1 << levels
    aligned = all(x % factor == 0 for x in range(0, width, tilewidth))
    aligned &= all(y % factor == 0 for y in range(0, height, tileheight))
    large_enough = min(tilewidth, width - (tiles_x - 1) * tilewidth) >= factor
    large_enough &= min(tileheight, height - (tiles_y - 1) * tileheight) >= factor
    assert 1 < tilecount and width <= 1024 and height <= 1024
    if rejection is None:
        assert aligned and large_enough and tilecount <= 256
    elif rejection == 'misaligned':
        assert not aligned and large_enough and tilecount <= 256
    elif rejection == 'tiny':
        assert aligned and not large_enough and tilecount <= 256
    else:
        assert rejection == 'count' and aligned and large_enough and tilecount > 256
    for components, mode in [(1, 'gray'), (3, 'rgb')]:
        stem = (f'jp2_tiles_{width}x{height}_d{levels}_{mode}'
                f'_b{cbwidth}x{cbheight}_t{tilewidth}x{tileheight}_{pattern}')
        if rejection is not None:
            stem = f'unsupported_{stem}_{rejection}'
        source_path = ROOT / (stem + ('.gray' if components == 1 else '.rgb'))
        raw = pixels(width, height, components, pattern)
        source_path.write_bytes(raw)
        source = Image.frombytes('L' if components == 1 else 'RGB', (width, height), raw)
        source_hash = hashlib.sha256(raw).hexdigest()
        for mct in ([0] if components == 1 else [0, 1]):
            for ext in ['j2k', 'jp2']:
                path = ROOT / f'{stem}_mct{mct}.{ext}'
                source.save(path, format='JPEG2000', no_jp2=ext == 'j2k',
                            num_resolutions=levels + 1, codeblock_size=(cbwidth, cbheight),
                            tile_size=(tilewidth, tileheight), tile_offset=(0, 0),
                            irreversible=False, mct=mct)
                encoded = path.read_bytes()
                if pattern == 'flat':
                    assert_headers(encoded, width, height, cbwidth, cbheight, levels,
                                   components, mct, tilewidth, tileheight, tilecount)
                    encoded = empty_flat_packets(encoded, levels, components, tilecount)
                    path.write_bytes(encoded)
                with Image.open(path) as oracle:
                    assert oracle.size == (width, height)
                    assert oracle.mode == ('L' if components == 1 else 'RGB')
                    assert oracle.tobytes() == raw, path
                assert_headers(encoded, width, height, cbwidth, cbheight, levels,
                               components, mct, tilewidth, tileheight, tilecount,
                               flat=pattern == 'flat')
                fields = [hashlib.sha256(encoded).hexdigest(), path.name,
                          source_hash, source_path.name, width, height, cbwidth,
                          cbheight, levels, components, mct, tilewidth, tileheight, tilecount]
                if rejection is None:
                    supported.append(' '.join(map(str, fields)))
                else:
                    unsupported.append(' '.join(map(str, fields + [rejection])))


for levels in range(6):
    factor = 1 << levels
    tilewidth, tileheight = (7, 9) if levels == 0 else (5 * factor, 7 * factor)
    edgewidth, edgeheight = factor + 3, factor + 5
    for index, (cbwidth, cbheight) in enumerate([(4, 4), (16, 16), (64, 64)]):
        case(2 * tilewidth + edgewidth, 2 * tileheight + edgeheight,
             tilewidth, tileheight, levels, cbwidth, cbheight,
             ['noise', 'mixed', 'ramp'][index])
for levels, cbwidth, cbheight in [(0, 4, 64), (1, 64, 4), (3, 8, 32), (5, 32, 8)]:
    factor = 1 << levels
    tilewidth, tileheight = 5 * factor, 7 * factor
    case(2 * tilewidth + factor + 3, 2 * tileheight + factor + 5,
         tilewidth, tileheight, levels, cbwidth, cbheight, 'noise')
case(64, 64, 4, 4, 2, 16, 16, 'mixed')  # 256 tiles, deepest LL singleton
case(1024, 1024, 64, 64, 5, 64, 64, 'ramp')  # max canvas and 256 tiles
case(49, 37, 49, 13, 0, 4, 16, 'noise')  # one column, clipped bottom
case(49, 37, 13, 37, 0, 16, 4, 'noise')  # one row, clipped right
case(48, 48, 16, 16, 3, 64, 64, 'flat')  # all component packets empty, MCT0/1
case(31, 27, 13, 11, 2, 16, 16, 'noise', rejection='misaligned')
case(35, 33, 15, 14, 1, 4, 64, 'mixed', rejection='misaligned')
case(33, 35, 16, 16, 2, 16, 16, 'ramp', rejection='tiny')
case(65, 65, 4, 4, 0, 4, 4, 'noise', rejection='count')
assert len(supported) - 1 == 162 and len(unsupported) - 1 == 24
assert all(len(row.split()) == 14 for row in supported[1:])
assert all(len(row.split()) == 15 for row in unsupported[1:])
(ROOT / 'jpeg2000-tiles-manifest.txt').write_text('\n'.join(supported) + '\n')
(ROOT / 'jpeg2000-tiles-unsupported-manifest.txt').write_text('\n'.join(unsupported) + '\n')
print(VERSIONS, 'supported encodings:', len(supported) - 1,
      'unsupported encodings:', len(unsupported) - 1)
