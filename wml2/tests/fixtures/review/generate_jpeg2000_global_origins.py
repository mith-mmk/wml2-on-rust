"""Nonzero image/tile-grid origins; deterministic source + OpenJPEG oracles.

Supported manifest has 18 whitespace-separated fields per encoding:
encoded_sha name source_sha source_name w h cbw cbh levels components mct tw th
tilecount image_x0 image_y0 tile_grid_x0 tile_grid_y0. Negative manifest adds
field19: extent_x or extent_y. Both begin with one Pillow/OpenJPEG version line.
Only new jp2_global_origins_* / unsupported_jp2_global_origins_* files are written.

T.800 B.3: count=ceil((extent-gridOrigin)/tileSize), tile start=max(gridOrigin+
p*tileSize,imageOrigin), tile end=min(gridOrigin+(p+1)*tileSize,extent).
Canvas width is Xsiz-XOsiz; output positions subtract image origin. B.5 uses
absolute tile bounds for the existing low/high band ceil divisions. B.6 precinct
grids start globally at zero: positive extent<=32768 stays in one implicit
precinct at every resolution. Code-block grids also retain global zero.

All final files decode exactly to their independent raw source through
Pillow12.3/OpenJPEG2.5.4. Existing deep/tiny nonneutral encoder limitations
are documented in generate_jpeg2000_origins.py; deeper tiny cases here are
neutral128. Flat body-less 0x80 packets are normalized to standard 0x00 empty
headers with unchanged lengths and then decoded again by OpenJPEG. No failed
roundtrip is accepted, no MQ encoding or production decoder dependency is added.
"""
import hashlib
from pathlib import Path

from PIL import Image, __version__, features

ROOT = Path(__file__).resolve().parent
VERSIONS = f'Pillow {__version__} OpenJPEG {features.version_codec("jpg_2000")}'


def ceil_div(a, b):
    return -((-a) // b)


def pixels(w, h, components, pattern):
    palette = [(0, 0, 0), (255, 255, 255), (255, 0, 0), (0, 255, 0),
               (0, 0, 255), (1, 2, 3), (3, 2, 1), (127, 128, 129)]
    out = bytearray()
    for y in range(h):
        for x in range(w):
            if pattern == 'flat':
                rgb = (128, 128, 128)
            elif pattern == 'mixed':
                rgb = palette[(x + 3 * y) % len(palette)]
            elif pattern == 'ramp':
                rgb = ((x * 9 + y * 7) % 256, (x * 3 + y) % 256, (x + y * 5) % 256)
            else:
                rgb = ((x * 73 + y * 151 + x * y * 17 + 31) % 256,
                       (x * 11 + y * 97 + x * y * 31 + 83) % 256,
                       (x * 137 + y * 37 + x * y * 7 + 197) % 256)
            out.extend(rgb[:components])
    return bytes(out)


def stream_offset(encoded, w, h, components):
    if encoded[:2] == bytes.fromhex('ff4f'):
        return 0
    ihdr = encoded.index(b'ihdr') + 4
    assert int.from_bytes(encoded[ihdr:ihdr + 4], 'big') == h
    assert int.from_bytes(encoded[ihdr + 4:ihdr + 8], 'big') == w
    assert int.from_bytes(encoded[ihdr + 8:ihdr + 10], 'big') == components
    position = 0
    while position < len(encoded):
        length = int.from_bytes(encoded[position:position + 4], 'big')
        assert length >= 8
        if encoded[position + 4:position + 8] == b'jp2c':
            assert position + length == len(encoded)
            return position + 8
        position += length
    raise AssertionError('missing codestream')


def headers(encoded, w, h, components, mct, levels, block, tile, origin, grid,
            *, flat=False):
    offset = stream_offset(encoded, w, h, components)
    data, position, siz, cod = encoded[offset:], 2, False, False
    xo, yo = origin
    gx, gy = grid
    tw, th = tile
    xe, ye = xo + w, yo + h
    assert data[:2] == bytes.fromhex('ff4f')
    while data[position:position + 2] != bytes.fromhex('ff90'):
        marker = data[position:position + 2]
        length = int.from_bytes(data[position + 2:position + 4], 'big')
        assert length >= 2
        body = data[position + 4:position + 2 + length]
        if marker == bytes.fromhex('ff51'):
            assert body[:2] == bytes(2)
            values = [int.from_bytes(body[i:i + 4], 'big') for i in range(2, 34, 4)]
            assert values == [xe, ye, xo, yo, tw, th, gx, gy], values
            assert int.from_bytes(body[34:36], 'big') == components
            assert body[36:] == bytes([7, 1, 1]) * components
            siz = True
        elif marker == bytes.fromhex('ff52'):
            assert len(body) == 10 and body[:5] == bytes([0, 0, 0, 1, mct])
            assert body[5] == levels
            assert (1 << (body[6] + 2), 1 << (body[7] + 2)) == block
            assert body[8:] == bytes([0, 1])
            cod = True
        elif marker == bytes.fromhex('ff5c'):
            assert len(body) == 2 + 3 * levels and body[0] & 31 == 0
            assert all(v & 7 == 0 for v in body[1:])
        elif marker == bytes.fromhex('ff5d'):
            assert len(body) == 3 + 3 * levels and body[0] < components
            assert body[1] & 31 == 0 and all(v & 7 == 0 for v in body[2:])
        position += length + 2
    assert siz and cod
    nx, ny = ceil_div(xe - gx, tw), ceil_div(ye - gy, th)
    ranges = []
    for index in range(nx * ny):
        assert data[position:position + 2] == bytes.fromhex('ff90')
        assert data[position + 2:position + 4] == bytes.fromhex('000a')
        assert int.from_bytes(data[position + 4:position + 6], 'big') == index
        length = int.from_bytes(data[position + 6:position + 10], 'big')
        assert length >= 14
        assert data[position + 10:position + 14] == bytes.fromhex('0001ff93')
        x0, y0 = max(gx + index % nx * tw, xo), max(gy + index // nx * th, yo)
        x1, y1 = min(gx + (index % nx + 1) * tw, xe), min(gy + (index // nx + 1) * th, ye)
        assert xo <= x0 < x1 <= xe and yo <= y0 < y1 <= ye
        packets = components * sum(
            ceil_div(x1, 1 << d) > ceil_div(x0, 1 << d)
            and ceil_div(y1, 1 << d) > ceil_div(y0, 1 << d)
            for d in range(levels, -1, -1))
        if flat:
            assert data[position + 14:position + length] == bytes(packets)
        ranges.append((offset + position + 14, offset + position + length, packets))
        position += length
    assert data[position:] == bytes.fromhex('ffd9')
    return ranges


supported, unsupported = [VERSIONS], [VERSIONS]


def case(w, h, origin, grid, tile, levels, block, pattern, *, rejection=None):
    xo, yo = origin
    gx, gy = grid
    tw, th = tile
    xe, ye = xo + w, yo + h
    count = ceil_div(xe - gx, tw) * ceil_div(ye - gy, th)
    assert 1 <= w <= 1024 and 1 <= h <= 1024 and 1 <= count <= 256
    assert 0 <= gx <= xo < xe and 0 <= gy <= yo < ye
    assert gx + tw > xo and gy + th > yo
    assert min(tw, th) >= 1 << levels
    if rejection is None:
        assert xe <= 32768 and ye <= 32768
    else:
        assert (rejection == 'extent_x' and xe > 32768) or (rejection == 'extent_y' and ye > 32768)
    for components, mode in [(1, 'gray'), (3, 'rgb')]:
        stem = (f'jp2_global_origins_{w}x{h}_d{levels}_{mode}_b{block[0]}x{block[1]}'
                f'_t{tw}x{th}_o{xo}x{yo}_g{gx}x{gy}_{pattern}')
        if rejection:
            stem = f'unsupported_{stem}_{rejection}'
        source_path = ROOT / (stem + ('.gray' if components == 1 else '.rgb'))
        raw = pixels(w, h, components, pattern)
        source_path.write_bytes(raw)
        source = Image.frombytes('L' if components == 1 else 'RGB', (w, h), raw)
        for mct in ([0] if components == 1 else [0, 1]):
            for ext in ['j2k', 'jp2']:
                path = ROOT / f'{stem}_mct{mct}.{ext}'
                source.save(path, format='JPEG2000', no_jp2=ext == 'j2k', offset=origin,
                            tile_offset=grid, tile_size=tile, num_resolutions=levels + 1,
                            codeblock_size=block, irreversible=False, mct=mct)
                encoded = path.read_bytes()
                ranges = headers(encoded, w, h, components, mct, levels, block, tile, origin, grid)
                if pattern == 'flat':
                    changed = bytearray(encoded)
                    for start, end, packets in ranges:
                        assert encoded[start:end] in [bytes([0x80]) * packets, bytes(packets)]
                        changed[start:end] = bytes(packets)
                    encoded = bytes(changed)
                    path.write_bytes(encoded)
                    headers(encoded, w, h, components, mct, levels, block, tile, origin, grid, flat=True)
                with Image.open(path) as oracle:
                    assert oracle.size == (w, h)
                    assert oracle.mode == ('L' if components == 1 else 'RGB')
                    assert oracle.tobytes() == raw, path
                fields = [hashlib.sha256(encoded).hexdigest(), path.name,
                          hashlib.sha256(raw).hexdigest(), source_path.name,
                          w, h, *block, levels, components, mct, tw, th, count, xo, yo, gx, gy]
                target = supported if rejection is None else unsupported
                target.append(' '.join(map(str, fields + ([rejection] if rejection else []))))


for levels in range(6):
    factor = 1 << levels
    tw, th = max(12, 2 * factor + 3), max(14, 2 * factor + 5)
    case(3 * tw - 8, 3 * th - 10, (5, 7), (2, 3), (tw, th), levels,
         (4, 4) if levels % 2 == 0 else (4, 64), 'noise')
    tw, th = factor + 3, factor + 5
    case(tw + 2, th + 3, (9 + tw - 1, 11 + th - 2), (9, 11), (tw, th),
         levels, (16, 16), 'noise' if levels < 2 else 'flat')
    tw, th = factor + 4, factor + 6
    w = 3 * tw - 4 - (levels % 2)
    h = 3 * th - 6 - (levels % 2 == 0)
    origin = (32768 - w, 32768 - h)
    grid = (origin[0] - 2, origin[1] - 3)
    case(w, h, origin, grid, (tw, th), levels,
         (64, 64) if levels % 2 == 0 else (64, 4), 'mixed')
case(1024, 1024, (37, 41), (5, 9), (256, 256), 5, (64, 64), 'ramp')
case(79, 78, (101, 205), (100, 203), (5, 5), 1, (16, 16), 'mixed')
case(1, 1, (31, 47), (30, 46), (32, 32), 5, (4, 4), 'flat')
case(17, 19, (32768, 15), (32766, 12), (32, 32), 1, (16, 16), 'noise', rejection='extent_x')
case(17, 19, (15, 32768), (12, 32766), (32, 32), 1, (16, 16), 'noise', rejection='extent_y')
assert len(supported) - 1 == 126 and len(unsupported) - 1 == 12
assert all(len(row.split()) == 18 for row in supported[1:])
assert all(len(row.split()) == 19 for row in unsupported[1:])
(ROOT / 'jpeg2000-global-origins-manifest.txt').write_text('\n'.join(supported) + '\n')
(ROOT / 'jpeg2000-global-origins-unsupported-manifest.txt').write_text('\n'.join(unsupported) + '\n')
print(VERSIONS, 'supported encodings:', len(supported) - 1,
      'unsupported encodings:', len(unsupported) - 1)
