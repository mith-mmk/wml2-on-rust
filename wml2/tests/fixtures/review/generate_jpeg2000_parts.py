"""Source-exact multipart fixtures from independent OpenJPEG PLT boundaries.

Import-safe. split_plt(encoded, strategy='packet') is exported for random tests
and returns (repackaged_encoding, independent_proof). No Rust parser is used.
Pillow/OpenJPEG emits PLT packet lengths. Every PLT varint/Zplt and the sum of
packet lengths are validated; PLT is removed and only SOT/TPsot/TNsot/Psot/JP2
lengths are rewritten. Joined tile bodies are byte-identical to encoder bodies.
Original PLT input and final multipart output must both match original pixels.

T.800 A.4 SOT identifies tile and part index/total, Psot includes SOT through
body end. A.7.3 PLT independently includes packet-header+body lengths. B.11
tile-parts contain complete packets; packet LRCP order is preserved. Grouped
raster parts and known totals1..18 are this decoder's deliberately bounded
profile, not general Part1 requirements. Unknown totals/interleaving are valid
unsupported examples; bad cuts/missing/inconsistent indices are malformed.

Supported manifest fields20: encoded_sha name source_sha source_name w h cbw
cbh levels components mct tw th tilecount image_x0 image_y0 grid_x0 grid_y0
max_parts_per_tile peak_join_body_bytes. Negative manifest adds reason(field21).
Packet-length proofs and negative oracle outcomes/hashes are persisted in JSON.
Deep tiny nonneutral OpenJPEG encoder limitations are documented by the earlier
origins generator. This matrix uses neutral tiny cases rather than weak assertions.
"""
import hashlib
import json
from io import BytesIO
from pathlib import Path

from PIL import Image, __version__, features

ROOT = Path(__file__).resolve().parent
VERSIONS = f'Pillow {__version__} OpenJPEG {features.version_codec("jpg_2000")}'


def sha(data):
    return hashlib.sha256(data).hexdigest()


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


def stream_offset(encoded):
    if encoded[:2] == bytes.fromhex('ff4f'):
        return 0
    p = 0
    while p < len(encoded):
        n = int.from_bytes(encoded[p:p + 4], 'big')
        assert n >= 8
        if encoded[p + 4:p + 8] == b'jp2c':
            assert p + n == len(encoded)
            return p + 8
        p += n
    raise AssertionError('missing codestream')


def wrap(encoded, offset, stream):
    if offset == 0:
        return bytes(stream)
    prefix = bytearray(encoded[:offset])
    prefix[offset - 8:offset - 4] = (len(stream) + 8).to_bytes(4, 'big')
    return bytes(prefix) + bytes(stream)


def read_plt(encoded):
    """Read ONLY SIZ/COD/SOT/PLT framing, never entropy or packet header bits."""
    offset = stream_offset(encoded)
    data, p, profile = encoded[offset:], 2, {}
    assert data[:2] == bytes.fromhex('ff4f')
    while data[p:p + 2] != bytes.fromhex('ff90'):
        n = int.from_bytes(data[p + 2:p + 4], 'big')
        assert n >= 2
        body = data[p + 4:p + 2 + n]
        if data[p:p + 2] == bytes.fromhex('ff51'):
            assert body[:2] == bytes(2)
            xe, ye, xo, yo, tw, th, gx, gy = [int.from_bytes(body[i:i + 4], 'big') for i in range(2, 34, 4)]
            c = int.from_bytes(body[34:36], 'big')
            assert c in [1, 3] and body[36:] == bytes([7, 1, 1]) * c
            profile.update(w=xe - xo, h=ye - yo, xo=xo, yo=yo, gx=gx, gy=gy, tw=tw, th=th, components=c)
        elif data[p:p + 2] == bytes.fromhex('ff52'):
            assert len(body) == 10 and body[:4] == bytes([0, 0, 0, 1]) and body[8:] == bytes([0, 1])
            profile.update(levels=body[5], mct=body[4], cbw=1 << (body[6] + 2), cbh=1 << (body[7] + 2))
        p += n + 2
    main = data[:p]
    nx = (profile['xo'] + profile['w'] - profile['gx'] + profile['tw'] - 1) // profile['tw']
    ny = (profile['yo'] + profile['h'] - profile['gy'] + profile['th'] - 1) // profile['th']
    profile['tilecount'] = nx * ny
    tiles = []
    for isot in range(nx * ny):
        assert data[p:p + 4] == bytes.fromhex('ff90000a')
        assert int.from_bytes(data[p + 4:p + 6], 'big') == isot
        assert data[p + 10:p + 12] == bytes([0, 1])
        end = p + int.from_bytes(data[p + 6:p + 10], 'big')
        q, lengths, raw_markers, next_z = p + 12, [], [], 0
        while data[q:q + 2] != bytes.fromhex('ff93'):
            assert data[q:q + 2] == bytes.fromhex('ff58')
            n = int.from_bytes(data[q + 2:q + 4], 'big')
            body = data[q + 4:q + 2 + n]
            assert n >= 3 and body[0] == next_z
            next_z += 1
            raw_markers.append(body.hex())
            value = 0
            for byte in body[1:]:
                value = (value << 7) | (byte & 127)
                if byte < 128:
                    assert value > 0
                    lengths.append(value)
                    value = 0
            assert value == 0 and (not body[1:] or body[-1] < 128)
            q += n + 2
        payload = data[q + 2:end]
        assert lengths and sum(lengths) == len(payload)
        x0 = max(profile['gx'] + isot % nx * profile['tw'], profile['xo'])
        y0 = max(profile['gy'] + isot // nx * profile['th'], profile['yo'])
        x1 = min(profile['gx'] + (isot % nx + 1) * profile['tw'], profile['xo'] + profile['w'])
        y1 = min(profile['gy'] + (isot // nx + 1) * profile['th'], profile['yo'] + profile['h'])
        packet_count = profile['components'] * sum(
            (x1 + (1 << d) - 1) // (1 << d) > (x0 + (1 << d) - 1) // (1 << d)
            and (y1 + (1 << d) - 1) // (1 << d) > (y0 + (1 << d) - 1) // (1 << d)
            for d in range(profile['levels'], -1, -1))
        assert len(lengths) == packet_count and len(lengths) <= 18
        tiles.append(dict(isot=isot, payload=payload, packet_lengths=lengths, plt_hex=raw_markers))
        p = end
    assert data[p:] == bytes.fromhex('ffd9')
    return offset, main, profile, tiles


def make_part(isot, tp, tn, payload):
    return (bytes.fromhex('ff90000a') + isot.to_bytes(2, 'big')
            + (14 + len(payload)).to_bytes(4, 'big') + bytes([tp, tn])
            + bytes.fromhex('ff93') + payload)


def chunks(tile, strategy):
    lengths, body = tile['packet_lengths'], tile['payload']
    group = 2 if strategy == 'pair' else 1
    if strategy == 'single' or (strategy == 'alternate' and tile['isot'] % 2 == 1):
        group = len(lengths)
    result, p = [], 0
    for i in range(0, len(lengths), group):
        n = sum(lengths[i:i + group])
        result.append(body[p:p + n])
        p += n
    assert p == len(body) and all(result)
    return result


def split_plt(encoded, strategy='packet'):
    """Independent, import-safe PLT splitter; returns (bytes, proof dict)."""
    assert strategy in ['packet', 'pair', 'alternate', 'single']
    offset, main, profile, tiles = read_plt(encoded)
    result = bytearray(main)
    proof = dict(tool_versions=VERSIONS, encoder_plt_sha256=sha(encoded), profile=profile,
                 strategy=strategy, max_parts_per_tile=0, peak_join_bytes=0, tiles=[])
    for tile in tiles:
        bodies = chunks(tile, strategy)
        assert len(bodies) <= 18 and b''.join(bodies) == tile['payload']
        proof['max_parts_per_tile'] = max(proof['max_parts_per_tile'], len(bodies))
        if len(bodies) > 1:
            proof['peak_join_bytes'] = max(proof['peak_join_bytes'], len(tile['payload']))
        for tp, payload in enumerate(bodies):
            result.extend(make_part(tile['isot'], tp, len(bodies), payload))
        packet_ends, part_ends, total = [], [], 0
        for n in tile['packet_lengths']:
            total += n
            packet_ends.append(total)
        total = 0
        for body in bodies:
            total += len(body)
            part_ends.append(total)
        assert set(part_ends) <= set(packet_ends)
        proof['tiles'].append(dict(isot=tile['isot'], body_sha256=sha(tile['payload']),
                                  body_bytes=len(tile['payload']), plt_hex=tile['plt_hex'],
                                  packet_lengths=tile['packet_lengths'], packet_ends=packet_ends,
                                  part_ends=part_ends, part_count=len(bodies)))
    result.extend(bytes.fromhex('ffd9'))
    output = wrap(encoded, offset, result)
    proof['multipart_sha256'] = sha(output)
    return output, proof


def oracle(data, raw, w, h, components):
    with Image.open(BytesIO(data)) as image:
        assert image.size == (w, h) and image.mode == ('L' if components == 1 else 'RGB')
        assert image.tobytes() == raw


def row(name, source_name, data, raw, profile, max_parts, peak):
    keys = ['w', 'h', 'cbw', 'cbh', 'levels', 'components', 'mct', 'tw', 'th', 'tilecount', 'xo', 'yo', 'gx', 'gy']
    return [sha(data), name, sha(raw), source_name] + [profile[k] for k in keys] + [max_parts, peak]


def encode(raw, w, h, components, mct, origin, grid, tile, levels, block, ext):
    image = Image.frombytes('L' if components == 1 else 'RGB', (w, h), raw)
    out = BytesIO()
    image.save(out, format='JPEG2000', no_jp2=ext == 'j2k', offset=origin, tile_offset=grid,
               tile_size=tile, num_resolutions=levels + 1, codeblock_size=block,
               irreversible=False, mct=mct, plt=True)
    data = out.getvalue()
    oracle(data, raw, w, h, components)
    return data


def generate():
    manifest, negative, proofs, outcomes = [VERSIONS], [VERSIONS], {}, {}

    def case(w, h, origin, grid, tile, levels, block, pattern, strategy):
        for components, mode in [(1, 'gray'), (3, 'rgb')]:
            stem = (f'jp2_parts_{w}x{h}_d{levels}_{mode}_b{block[0]}x{block[1]}'
                    f'_t{tile[0]}x{tile[1]}_o{origin[0]}x{origin[1]}'
                    f'_g{grid[0]}x{grid[1]}_{pattern}_{strategy}')
            raw = pixels(w, h, components, pattern)
            source = stem + ('.gray' if components == 1 else '.rgb')
            (ROOT / source).write_bytes(raw)
            for mct in ([0] if components == 1 else [0, 1]):
                for ext in ['j2k', 'jp2']:
                    original = encode(raw, w, h, components, mct, origin, grid, tile, levels, block, ext)
                    data, proof = split_plt(original, strategy)
                    oracle(data, raw, w, h, components)
                    name = f'{stem}_mct{mct}.{ext}'
                    (ROOT / name).write_bytes(data)
                    fields = row(name, source, data, raw, proof['profile'],
                                 proof['max_parts_per_tile'], proof['peak_join_bytes'])
                    assert len(fields) == 20
                    manifest.append(' '.join(map(str, fields)))
                    proofs[name] = proof

    for levels in range(1, 6):
        f = 1 << levels
        tw, th = max(12, 2 * f + 3), max(14, 2 * f + 5)
        case(3 * tw - 8, 3 * th - 10, (5, 7), (2, 3), (tw, th), levels,
             (4, 4) if levels % 2 == 0 else (4, 64), 'noise', 'packet')
        tw, th = f + 3, f + 5
        case(tw + 2, th + 3, (9 + tw - 1, 11 + th - 2), (9, 11), (tw, th),
             levels, (16, 16), 'noise' if levels == 1 else 'flat', 'pair')
        tw, th = f + 4, f + 6
        w, h = 3 * tw - 4 - levels % 2, 3 * th - 6 - (levels % 2 == 0)
        origin = (32768 - w, 32768 - h)
        case(w, h, origin, (origin[0] - 2, origin[1] - 3), (tw, th), levels,
             (64, 64) if levels % 2 == 0 else (64, 4), 'mixed', 'alternate')
    case(1024, 1024, (37, 41), (5, 9), (256, 256), 5, (64, 64), 'ramp', 'pair')
    case(79, 78, (101, 205), (100, 203), (5, 5), 1, (16, 16), 'mixed', 'packet')
    case(1, 1, (31, 47), (30, 46), (32, 32), 5, (4, 4), 'flat', 'packet')
    case(17, 19, (0, 0), (0, 0), (8, 8), 1, (4, 4), 'noise', 'packet')
    case(19, 22, (3, 5), (1, 2), (16, 16), 3, (8, 32), 'flat', 'alternate')
    case(33, 35, (2, 4), (0, 0), (64, 64), 5, (16, 16), 'mixed', 'packet')
    assert len(manifest) - 1 == 126

    for components, mode in [(1, 'gray'), (3, 'rgb')]:
        for mct in ([0] if components == 1 else [0, 1]):
            for ext in ['j2k', 'jp2']:
                for interleave in [False, True]:
                    w, h, tile = (32, 16, (16, 16)) if interleave else (16, 16, (16, 16))
                    raw = pixels(w, h, components, 'noise')
                    original = encode(raw, w, h, components, mct, (0, 0), (0, 0), tile, 1, (16, 16), ext)
                    offset, main, profile, tiles = read_plt(original)
                    groups = [[make_part(t['isot'], i, len(t['packet_lengths']), b)
                               for i, b in enumerate(chunks(t, 'packet'))] for t in tiles]
                    variants = {}
                    if interleave:
                        variants['unsupported_interleaved'] = [groups[t][i]
                            for i in range(max(map(len, groups))) for t in range(len(groups)) if i < len(groups[t])]
                    else:
                        payload = tiles[0]['payload']
                        first_packet = tiles[0]['packet_lengths'][0]
                        assert first_packet >= 16  # one LL8x8 block, cut last MQ byte
                        for reason, cut in [('malformed_mid_header', 1), ('malformed_mid_body', first_packet - 1)]:
                            variants[reason] = [make_part(0, 0, 2, payload[:cut]), make_part(0, 1, 2, payload[cut:])]
                        variants['malformed_missing_part'] = groups[0][:1] + groups[0][2:]
                        variants['unsupported_unknown_TNsot'] = [p[:11] + bytes(1) + p[12:] for p in groups[0]]
                        variants['malformed_inconsistent_TNsot'] = [groups[0][0][:11] + bytes([len(groups[0]) + 1]) + groups[0][0][12:]] + groups[0][1:]
                        variants['malformed_TPsot_sequence'] = [groups[0][0][:10] + bytes([1]) + groups[0][0][11:]] + groups[0][1:]
                        assert len(payload) >= 19
                        bodies = [payload[len(payload) * i // 19:len(payload) * (i + 1) // 19] for i in range(19)]
                        variants['malformed_19_parts'] = [make_part(0, i, 19, body) for i, body in enumerate(bodies)]
                    for reason, parts in variants.items():
                        data = wrap(original, offset, main + b''.join(parts) + bytes.fromhex('ffd9'))
                        stem = f'unsupported_jp2_parts_{w}x{h}_d1_{mode}_mct{mct}_{reason}'
                        source_name = stem + ('.gray' if components == 1 else '.rgb')
                        name = stem + '.' + ext
                        (ROOT / source_name).write_bytes(raw)
                        (ROOT / name).write_bytes(data)
                        counts, sizes = {}, {}
                        for part in parts:
                            isot = int.from_bytes(part[4:6], 'big')
                            counts[isot] = counts.get(isot, 0) + 1
                            sizes[isot] = sizes.get(isot, 0) + len(part) - 14
                        max_parts = max(counts.values())
                        peak = max((sizes[i] for i in counts if counts[i] > 1), default=0)
                        fields = row(name, source_name, data, raw, profile, max_parts, peak) + [reason]
                        assert len(fields) == 21
                        negative.append(' '.join(map(str, fields)))
                        status = 'source-exact'
                        try:
                            oracle(data, raw, w, h, components)
                        except Exception as error:
                            status = 'rejected-or-different: ' + type(error).__name__
                        if reason.startswith('unsupported_'):
                            assert status == 'source-exact', (name, status)
                        outcomes[name] = dict(reason=reason, classification='valid-unsupported' if reason.startswith('unsupported_') else 'malformed',
                                              oracle_status=status, encoded_sha256=sha(data), source_sha256=sha(raw),
                                              original_plt_sha256=sha(original), packet_lengths=[t['packet_lengths'] for t in tiles])
    assert len(negative) - 1 == 48
    (ROOT / 'jpeg2000-parts-manifest.txt').write_text('\n'.join(manifest) + '\n')
    (ROOT / 'jpeg2000-parts-unsupported-manifest.txt').write_text('\n'.join(negative) + '\n')
    (ROOT / 'jpeg2000-parts-proof.json').write_text(json.dumps(dict(tool_versions=VERSIONS, positive=proofs, negative=outcomes), indent=2) + '\n')
    print(VERSIONS, 'supported encodings:', len(manifest) - 1, 'negative encodings:', len(negative) - 1)


if __name__ == '__main__':
    generate()
