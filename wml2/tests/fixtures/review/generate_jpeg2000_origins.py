"""Odd-origin and tiny-edge Gray/RGB tile fixtures; OpenJPEG is oracle only.

Supported manifest: encoded_sha filename source_sha source_filename width height
cbwidth cbheight levels components mct tilewidth tileheight tilecount (14 fields).
Unsupported manifest adds rejection reason at field 15; its files use unsupported_.
Each manifest begins with one tool-version line. All global origins are zero.

T.800 B-15 gives low bounds ceil(a/2^d), ceil(b/2^d), and high bounds
ceil((a-2^(d-1))/2^d), ceil((b-2^(d-1))/2^d). Empty reduced resolutions
have no packets (B.6). F.3.6 halves odd-start singleton coefficients.
Nominal tile dimensions allow Pillow's requested decomposition, but clipped
tiles can have 1/2/3 samples or empty deepest LL axes.

Flat fixtures use standard 0x00 empty packet headers: OpenJPEG's body-less
0x80 present/no-included-block headers are replaced without changing lengths.
The modified file is independently decoded through OpenJPEG before hashing.

Oracle limitation (Pillow12.3.0/OpenJPEG2.5.4): direct NL2 encoding of 6x8
Gray tile5x7 noise corrupts the clipped odd-origin x=5 column. Quarantine:
repository .test-origins-oracle/jp2_origins_6x8_d2_gray_b4x4_t5x7_noise_mct0.j2k
encoded SHA256 14be5a9163cf36c6b3755fecf23a97498a079db2313e7e73550364a4f8bee6db;
source .gray SHA256 72d96c42e5e93d9c5589dd331d4021b01f931748057c0a049ebe318f23f590d6;
oracle openjpeg.gray SHA256 76c0f0e00cc800147da672aa2aeb89d0386667ec5950f458e6f2178ce629f7e1.
Affected source column y0..6 [140,120,100,80,60,40,20] becomes
[120,100,80,60,40,57,74] in both OpenJPEG and the independently written Rust
decoder. Encoded HL coefficients [-16,-96,-176,-108] contradict direct
forward5/3 expected [24,-56,-136,-216] (HH zero). Thus coefficients issued
by the encoder are wrong; no mismatched source is used as a positive oracle.

Canonical NL2..5 cases encode at NL1 first: first tile neutral128, only the
rightmost one-sample column nonneutral at odd x=2^NL+1. All reduced-resolution
x extents of that column are zero, so its sole finest-resolution packet is
unchanged when promoted. COD NL and reversible QCD/QCC coarse band exponents
are extended; the neutral first tile receives NL+1 empty packets per component
and corrected Psot. JP2 codestream length is updated. Both the original NL1
file and the promoted file are source-exact through OpenJPEG. No MQ encoding
or alternate decoder is added. Tiny multi-axis deep-empty LL is also covered
by source-exact neutral fixtures; arbitrary nonneutral encoder failures remain
an explicitly documented oracle limitation, not weakened assertions.
"""
import hashlib
from pathlib import Path

from PIL import Image, __version__, features

ROOT = Path(__file__).resolve().parent
VERSIONS = f'Pillow {__version__} OpenJPEG {features.version_codec("jpg_2000")}'


def ceil_div(value, divisor):
    return -((-value) // divisor)


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


def stream_offset(encoded):
    if encoded[:2] == bytes.fromhex('ff4f'):
        return 0
    position = 0
    while position < len(encoded):
        length = int.from_bytes(encoded[position:position + 4], 'big')
        assert length >= 8
        if encoded[position + 4:position + 8] == b'jp2c':
            assert position + length == len(encoded)
            return position + 8
        position += length
    raise AssertionError('missing codestream')


def packet_count(x0, y0, x1, y1, levels, components):
    return components * sum(
        ceil_div(x1, 1 << depth) > ceil_div(x0, 1 << depth)
        and ceil_div(y1, 1 << depth) > ceil_div(y0, 1 << depth)
        for depth in range(levels, -1, -1))


def promote_neutral_first_tile(encoded, levels, components):
    offset = stream_offset(encoded)
    data, position = encoded[offset:], 2
    main = bytearray(data[:2])
    while data[position:position + 2] != bytes.fromhex('ff90'):
        marker = data[position:position + 2]
        length = int.from_bytes(data[position + 2:position + 4], 'big')
        body = bytearray(data[position + 4:position + 2 + length])
        if marker == bytes.fromhex('ff52'):
            assert body[5] == 1
            body[5] = levels
        elif marker == bytes.fromhex('ff5c'):
            assert len(body) == 5 and body[0] & 31 == 0
            body = body[:2] + body[2:] * levels
        elif marker == bytes.fromhex('ff5d'):
            assert len(body) == 6 and body[1] & 31 == 0
            body = body[:3] + body[3:] * levels
        main.extend(marker)
        main.extend((len(body) + 2).to_bytes(2, 'big'))
        main.extend(body)
        position += length + 2
    length = int.from_bytes(data[position + 6:position + 10], 'big')
    assert data[position + 14:position + length] == bytes([0x80]) * (2 * components)
    first = bytearray(data[position:position + 14])
    first[6:10] = (14 + (levels + 1) * components).to_bytes(4, 'big')
    main.extend(first)
    main.extend(bytes((levels + 1) * components))
    main.extend(data[position + length:])
    if offset == 0:
        return bytes(main)
    result = bytearray(encoded[:offset])
    result[offset - 8:offset - 4] = (len(main) + 8).to_bytes(4, 'big')
    result.extend(main)
    return bytes(result)


def headers(encoded, width, height, cbwidth, cbheight, levels, components, mct,
            tilewidth, tileheight, *, flat=False):
    offset = stream_offset(encoded)
    data, position, siz, cod = encoded[offset:], 2, False, False
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
            siz = True
        elif marker == bytes.fromhex('ff52'):
            assert len(body) == 10 and body[:5] == bytes([0, 0, 0, 1, mct])
            assert body[5] == levels
            assert (1 << (body[6] + 2), 1 << (body[7] + 2)) == (cbwidth, cbheight)
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
    nx = ceil_div(width, tilewidth)
    count = nx * ceil_div(height, tileheight)
    ranges = []
    for tile in range(count):
        assert data[position:position + 2] == bytes.fromhex('ff90')
        assert data[position + 2:position + 4] == bytes.fromhex('000a')
        assert int.from_bytes(data[position + 4:position + 6], 'big') == tile
        length = int.from_bytes(data[position + 6:position + 10], 'big')
        assert length >= 14
        assert data[position + 10:position + 14] == bytes.fromhex('0001ff93')
        x0, y0 = tile % nx * tilewidth, tile // nx * tileheight
        packets = packet_count(x0, y0, min(width, x0 + tilewidth),
                               min(height, y0 + tileheight), levels, components)
        if flat:
            assert data[position + 14:position + length] == bytes(packets)
        ranges.append((offset + position + 14, offset + position + length, packets))
        position += length
    assert data[position:] == bytes.fromhex('ffd9')
    return ranges


supported, unsupported = [VERSIONS], [VERSIONS]


def case(width, height, tilewidth, tileheight, levels, cbwidth, cbheight, pattern,
         *, rejection=None, canonical=False):
    count = ceil_div(width, tilewidth) * ceil_div(height, tileheight)
    assert width <= 1024 and height <= 1024 and 1 < count
    assert min(tilewidth, tileheight) >= 1 << levels
    assert (count <= 256) == (rejection is None)
    for components, mode in [(1, 'gray'), (3, 'rgb')]:
        stem = (f'jp2_origins_{width}x{height}_d{levels}_{mode}'
                f'_b{cbwidth}x{cbheight}_t{tilewidth}x{tileheight}_{pattern}')
        if canonical:
            assert levels >= 2 and width == tilewidth + 1 and height == tileheight
            assert tilewidth == (1 << levels) + 1 and tilewidth % 2 == 1
            stem += '_canonical'
        if rejection is not None:
            stem = f'unsupported_{stem}_{rejection}'
        source_path = ROOT / (stem + ('.gray' if components == 1 else '.rgb'))
        raw = pixels(width, height, components, pattern)
        if canonical:
            column = raw
            neutral = bytearray([128]) * len(raw)
            for y in range(height):
                position = (y * width + width - 1) * components
                neutral[position:position + components] = column[position:position + components]
            raw = bytes(neutral)
        source_path.write_bytes(raw)
        source = Image.frombytes('L' if components == 1 else 'RGB', (width, height), raw)
        for mct in ([0] if components == 1 else [0, 1]):
            for ext in ['j2k', 'jp2']:
                path = ROOT / f'{stem}_mct{mct}.{ext}'
                source.save(path, format='JPEG2000', no_jp2=ext == 'j2k',
                            num_resolutions=2 if canonical else levels + 1,
                            codeblock_size=(cbwidth, cbheight),
                            tile_size=(tilewidth, tileheight), tile_offset=(0, 0),
                            irreversible=False, mct=mct)
                encoded = path.read_bytes()
                if canonical:
                    with Image.open(path) as original:
                        assert original.tobytes() == raw, ('NL1 canonical', path)
                    encoded = promote_neutral_first_tile(encoded, levels, components)
                    path.write_bytes(encoded)
                ranges = headers(encoded, width, height, cbwidth, cbheight, levels,
                                 components, mct, tilewidth, tileheight)
                if pattern == 'flat':
                    changed = bytearray(encoded)
                    for start, end, packets in ranges:
                        body = encoded[start:end]
                        assert body == bytes([0x80]) * packets or body == bytes(packets)
                        changed[start:end] = bytes(packets)
                    encoded = bytes(changed)
                    path.write_bytes(encoded)
                    headers(encoded, width, height, cbwidth, cbheight, levels,
                            components, mct, tilewidth, tileheight, flat=True)
                with Image.open(path) as oracle:
                    assert oracle.size == (width, height)
                    assert oracle.mode == ('L' if components == 1 else 'RGB')
                    assert oracle.tobytes() == raw, path
                fields = [hashlib.sha256(encoded).hexdigest(), path.name,
                          hashlib.sha256(raw).hexdigest(), source_path.name,
                          width, height, cbwidth, cbheight, levels, components, mct,
                          tilewidth, tileheight, count]
                if rejection is None:
                    supported.append(' '.join(map(str, fields)))
                else:
                    unsupported.append(' '.join(map(str, fields + [rejection])))


for levels in range(6):
    factor = 1 << levels
    tw, th = (3, 5) if levels == 0 else (factor + 1, factor + 3)
    for edge, block in [(1, 4), (2, 16), (3, 64)]:
        if levels >= 2 and edge == 1:
            case(tw + 1, th, tw, th, levels, block, block, 'noise', canonical=True)
        elif levels >= 3:
            case(2 * factor + edge, 2 * (factor + 3), 2 * factor, factor + 3,
                 levels, block, block, ['noise', 'mixed', 'ramp'][edge - 1])
        else:
            case(tw + edge, th + edge, tw, th, levels, block, block,
                 ['noise', 'mixed', 'ramp'][edge - 1])
    tw, th = 2 * factor, factor + 3
    case(tw + 1, 3 * th, tw, th, levels, 4, 64, 'noise')
    tw, th = factor + 1, 2 * factor
    case(3 * tw, th + 3, tw, th, levels, 64, 4, 'mixed')
for levels in [1, 3, 5]:
    factor = 1 << levels
    case(factor + 2, factor + 4, factor + 1, factor + 3, levels, 16, 16, 'flat')
case(80, 80, 5, 5, 2, 16, 16, 'mixed')  # 256 odd-origin tiles
case(1024, 1024, 129, 131, 5, 64, 64, 'ramp')
case(65, 65, 4, 4, 0, 4, 4, 'noise', rejection='count')
assert len(supported) - 1 == 210 and len(unsupported) - 1 == 6
assert all(len(row.split()) == 14 for row in supported[1:])
assert all(len(row.split()) == 15 for row in unsupported[1:])
(ROOT / 'jpeg2000-origins-manifest.txt').write_text('\n'.join(supported) + '\n')
(ROOT / 'jpeg2000-origins-unsupported-manifest.txt').write_text('\n'.join(unsupported) + '\n')
print(VERSIONS, 'supported encodings:', len(supported) - 1,
      'unsupported encodings:', len(unsupported) - 1)
