#!/usr/bin/env python3
"""Original U16 samples -> OpenJPEG J2K/JP2 -> independent PGX samples.

Only fixture tooling uses OpenJPEG. All work directories start .test. The
canonical QCD-before-QCC output is the independent oracle for reordered QCC
variants because some OpenJPEG versions mishandle the reverse tile order.
"""
import argparse
import hashlib
import json
from pathlib import Path
import random
import struct
import subprocess


def run(*args):
    return subprocess.run(args, check=True, capture_output=True).stdout


def pgx(path):
    header, raw = path.read_bytes().split(b'\n', 1)
    magic, endian, sign, depth, w, h = header.split()
    assert magic == b'PG' and sign == b'+'
    depth, w, h = int(depth), int(w), int(h)
    assert depth in [12, 16]
    values = struct.unpack(('>' if endian == b'ML' else '<') + f'{w*h}H', raw)
    return depth, w, h, values


def segment(kind, payload):
    return struct.pack('>HH', kind, len(payload) + 2) + payload


def qcc_variant(raw, reverse, tile):
    pos = 2
    segments = []
    quant = None
    while raw[pos:pos+2] != b'\xff\x90':
        marker, length = struct.unpack_from('>HH', raw, pos)
        item = raw[pos:pos+length+2]
        if marker == 0xff5c:
            quant = item[4:]
        segments.append((marker, item))
        pos += length + 2
    assert quant and quant[0] & 31 == 0
    # Give the default a different valid coefficient precision. The component
    # override retains the encoder's actual precision and packet interpretation.
    alternate = quant[:1] + bytes(v - 8 for v in quant[1:])
    qcd = segment(0xff5c, alternate)
    qcc = segment(0xff5d, b'\0' + quant)
    changes = qcc + qcd if reverse else qcd + qcc
    if not tile:
        return b'\xff\x4f' + b''.join(changes if m == 0xff5c else item for m, item in segments) + raw[pos:]
    prefix = raw[:pos]
    while raw[pos:pos+2] == b'\xff\x90':
        length = struct.unpack_from('>I', raw, pos + 6)[0]
        assert length and raw[pos+12:pos+14] == b'\xff\x93'
        part = bytearray(raw[pos:pos+length])
        if part[10] == 0:
            struct.pack_into('>I', part, 6, length + len(changes))
            part[12:12] = changes
        prefix += part
        pos += length
    return prefix + raw[pos:]


def wrap(raw, template):
    pos = 0
    result = b''
    while pos < len(template):
        length, kind = struct.unpack_from('>I4s', template, pos)
        assert length >= 8
        item = template[pos:pos+length]
        result += struct.pack('>I4s', len(raw)+8, b'jp2c') + raw if kind == b'jp2c' else item
        pos += length
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--compress', required=True)
    p.add_argument('--decompress', required=True)
    p.add_argument('--work-dir', type=Path, required=True)
    args = p.parse_args()
    work = args.work_dir.resolve()
    assert work.name.startswith('.test')
    work.mkdir(exist_ok=True)
    root = Path(__file__).resolve().parent
    rows = []
    for depth in [12, 16]:
        maximum = (1 << depth) - 1
        cases = [('zero', 1, 1, 0, []), ('max', 1, 1, 0, []),
                 ('edges', 9, 7, 2, ['-b', '4,4']), ('ramp', 17, 13, 3, []),
                 ('noise', 33, 29, 3, []),
                 ('tiles', 33, 29, 2, ['-t', '17,15', '-d', '3,5', '-T', '1,2']),
                 ('singleton', 7, 1, 0, ['-t', '3,2', '-d', '1,1']),
                 ('modes', 33, 29, 3, ['-M', '38', '-SOP', '-EPH', '-PLT']),
                 ('precincts', 33, 29, 3, ['-c', '[16,16]']),
                 ('deep', 65, 65, 6, []),
                 ('bench', 128, 96, 4, ['-t', '64,48'])]
        cases += [(order, 33, 29, 3, ['-p', order, '-r', '8,4,1', '-TP', 'L'])
                  for order in ['LRCP', 'RLCP', 'RPCL', 'PCRL', 'CPRL']]
        for name, w, h, levels, extra in cases:
            stem = f'jp2_ex_gray{depth}_{name}'
            rng = random.Random(200000 + depth * 100 + w + h)
            edges = [0, 1, 2, 255, 256, 257, maximum // 2, maximum - 1, maximum]
            values = [0 if name == 'zero' else maximum if name == 'max'
                      else edges[i % len(edges)] if name == 'edges'
                      else (i * maximum // (w*h-1)) if name == 'ramp'
                      else rng.randrange(maximum+1) for i in range(w*h)]
            source = struct.pack(f'<{w*h}H', *values)
            (root / f'{stem}.u16le').write_bytes(source)
            # RAWL + explicit precision avoids OpenJPEG's PGX input loader
            # reducing a flat zero image to one bit. PGX is the decoded oracle.
            original = work / f'{stem}.rawl'
            original.write_bytes(source)
            encoded = {}
            for ext in ['j2k', 'jp2']:
                path = root / f'{stem}.{ext}'
                command = [args.compress, '-i', str(original), '-o', str(path), '-n', str(levels+1),
                           '-F', f'{w},{h},1,{depth},u', '-b', '16,16', '-mct', '0', '-threads', '1', '-C', 'WML2 original Gray12/16 EX fixture', *extra]
                run(*command)
                encoded[ext] = path.read_bytes()
                target = work / f'{stem}_{ext}.pgx'
                run(args.decompress, '-i', str(path), '-o', str(target), '-threads', '1')
                decoded = target.with_name(target.stem + '_0.pgx')
                assert pgx(decoded) == (depth, w, h, tuple(values)), path
                reference = root / f'{stem}.pgx'
                reference.write_bytes(decoded.read_bytes())
                rows.append(dict(input=path.name, source=f'{stem}.u16le', pgx=reference.name,
                                 depth=depth, width=w, height=h, options=command[7:],
                                 sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
            if name == 'noise':
                for tile in [False, True]:
                    for reverse in [False, True]:
                        data = qcc_variant(encoded['j2k'], reverse, tile)
                        for ext in ['j2k', 'jp2']:
                            path = root / f'{stem}_qcc_{int(tile)}_{int(reverse)}.{ext}'
                            path.write_bytes(data if ext == 'j2k' else wrap(data, encoded['jp2']))
                            if not reverse:
                                target = work / 'qcc.pgx'
                                run(args.decompress, '-i', str(path), '-o', str(target), '-threads', '1')
                                assert pgx(work / 'qcc_0.pgx') == (depth, w, h, tuple(values))
                            rows.append(dict(input=path.name, source=f'{stem}.u16le', pgx=f'{stem}.pgx',
                                             depth=depth, width=w, height=h,
                                             oracle='canonical QCD-before-QCC PGX; equivalent order permutation',
                                             sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    negatives = []
    for name, components, bits, signed, extra in [
        ('signed16', 1, 16, True, []), ('rgb16', 3, 16, False, []),
        ('rgba16', 4, 16, False, []), ('irreversible16', 1, 16, False, ['-I']),
        ('gray10', 1, 10, False, []), ('roi16', 1, 16, False, ['-ROI', 'c=0,U=3']),
    ]:
        w, h = 32, 24
        maximum = (1 << bits) - 1
        values = [(i * 997 + 17) & maximum for i in range(w*h*components)]
        original = work / 'negative.rawl'
        original.write_bytes(struct.pack(f'<{len(values)}H', *values))
        for ext in ['j2k', 'jp2']:
            path = root / f'jp2_ex_unsupported_{name}.{ext}'
            run(args.compress, '-i', str(original), '-o', str(path), '-n', '3',
                '-F', f'{w},{h},{components},{bits},{"s" if signed else "u"}',
                '-mct', '0', '-threads', '1', *extra)
            # Verify that this is an independently readable stream, even though
            # the WML2 EX profile intentionally does not support its declaration.
            run(args.decompress, '-i', str(path), '-o', str(work / 'negative.pgx'), '-threads', '1')
            negatives.append(dict(input=path.name, reason=name, sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    (root / 'jpeg2000-ex-gray-rejected.json').write_text(json.dumps(negatives, indent=2) + '\n')
    (root / 'jpeg2000-ex-gray-manifest.txt').write_text(''.join(
        f"{r['input']} {r['source']} {r['depth']} {r['width']} {r['height']}\n" for r in rows))
    (root / 'jpeg2000-ex-gray-manifest.json').write_text(json.dumps(dict(
        provenance='Original seeded U16 values, OpenJPEG CLI 2.5.3 encode and PGX decode; not a production dependency',
        files=rows), indent=2) + '\n')
    print(f'{len(rows)} exact source/PGX fixtures')


if __name__ == '__main__':
    main()
