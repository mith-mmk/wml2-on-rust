#!/usr/bin/env python3
"""Seeded planar U16 -> OpenJPEG reversible streams -> independent PGX."""
import argparse
import hashlib
import json
from pathlib import Path
import random
import struct
import subprocess


def run(*args):
    return subprocess.run(args, check=True, capture_output=True).stdout


def box(kind, data):
    return struct.pack('>I4s', len(data)+8, kind)+data


def children(data):
    pos = 0
    while pos < len(data):
        length, kind = struct.unpack_from('>I4s', data, pos)
        assert length >= 8
        yield kind, data[pos+8:pos+length]
        pos += length
    assert pos == len(data)


def declare_rgb(data, definitions):
    result = b''
    for kind, payload in children(data):
        if kind == b'jp2h':
            payload = b''.join(box(k, b'\x01\0\0'+struct.pack('>I', 16) if k == b'colr' else v)
                               for k, v in children(payload) if k != b'cdef')
            if definitions is not None:
                payload += box(b'cdef', struct.pack('>H', len(definitions)) +
                               b''.join(struct.pack('>HHH', *d) for d in definitions))
        result += box(kind, payload)
    return result


def pgx(path):
    header, raw = path.read_bytes().split(b'\n', 1)
    magic, endian, sign, depth, w, h = header.split()
    assert magic == b'PG' and sign == b'+'
    w, h, depth = int(w), int(h), int(depth)
    return depth, w, h, struct.unpack(('>' if endian == b'ML' else '<')+f'{w*h}H', raw)


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
    cases = [('edges', 9, 7, 2, []), ('noise', 17, 13, 3, []),
             ('tiles', 33, 29, 2, ['-t', '17,15', '-d', '3,5', '-T', '1,2']),
             ('layers', 17, 13, 2, ['-p', 'RPCL', '-r', '8,4,1', '-TP', 'L']),
             ('singleton', 7, 1, 0, ['-t', '3,2', '-d', '1,1']),
             ('order', 9, 7, 2, [])]
    cases += [(order, 17, 13, 2, ['-p', order, '-r', '8,4,1', '-TP', 'L'])
              for order in ['RLCP', 'PCRL', 'CPRL']]
    for bits in [12, 16]:
        maximum = (1 << bits)-1
        edges = [0, 1, 255, 256, 257, maximum//2, maximum-1, maximum]
        for nc, kind in [(3, 'rgb'), (4, 'rgba')]:
            for name, w, h, levels, extra in cases:
                stem = f'jp2_ex_{kind}{bits}_{name}'
                rng = random.Random(40000+bits*nc+w+h)
                # Deliberately retain nonzero colour at alpha zero.
                planes = [[edges[(i*(2*c+1)+c+2) % len(edges)] if name == 'edges'
                           else rng.randrange(maximum+1) for i in range(w*h)] for c in range(3)]
                alpha = [edges[i % len(edges)] for i in range(w*h)] if nc == 4 else [maximum]*(w*h)
                expected = [v for i in range(w*h) for v in [planes[0][i], planes[1][i], planes[2][i], alpha[i]]]
                source = root/f'{stem}.rgba16le'
                source.write_bytes(struct.pack(f'<{len(expected)}H', *expected))
                permutation = [2, 0, 1] if name == 'order' else [0, 1, 2]
                coded = [planes[c] for c in permutation] + ([alpha] if nc == 4 else [])
                original = work/'source.rawl'
                original.write_bytes(b''.join(struct.pack(f'<{w*h}H', *v) for v in coded))
                definitions = [(i, 0, c+1) for i, c in enumerate(permutation)]
                if nc == 4:
                    definitions.append((3, 1, 0))
                for mct in [0, 1]:
                    for ext in (['j2k', 'jp2'] if nc == 3 and name != 'order' else ['jp2']):
                        path = root/f'{stem}_mct{mct}.{ext}'
                        run(args.compress, '-i', str(original), '-o', str(path), '-n', str(levels+1),
                            '-F', f'{w},{h},{nc},{bits},u', '-mct', str(mct), '-b', '16,16',
                            '-threads', '1', '-C', 'WML2 original EX colour fixture', *extra)
                        if ext == 'jp2':
                            path.write_bytes(declare_rgb(path.read_bytes(), definitions if nc == 4 or name == 'order' else None))
                        target = work/'decoded.pgx'
                        run(args.decompress, '-i', str(path), '-o', str(target), '-threads', '1')
                        refs = []
                        for c in range(nc):
                            decoded = work/f'decoded_{c}.pgx'
                            assert pgx(decoded) == (bits, w, h, tuple(expected[c::4])), (path, c)
                            reference = root/f'{stem}_{c}.pgx'
                            reference.write_bytes(decoded.read_bytes())
                            refs.append(reference.name)
                        rows.append(dict(input=path.name, source=source.name, pgx=refs, bits=bits,
                                         width=w, height=h, components=nc, mct=mct,
                                         sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    negatives = []
    for nc in [3, 4]:
        for reason, bits, signed, extra in [('signed16',16,True,[]), ('irreversible16',16,False,['-I']), ('depth10',10,False,[])]:
            original = work/'negative.rawl'
            original.write_bytes(struct.pack('<'+str(16*16*nc)+'H', *[(i*997+17)&((1<<bits)-1) for i in range(16*16*nc)]))
            path = root/f'jp2_ex_color_unsupported_{nc}_{reason}.jp2'
            run(args.compress,'-i',str(original),'-o',str(path),'-n','3','-F',f'16,16,{nc},{bits},{"s" if signed else "u"}','-mct','0','-threads','1',*extra)
            path.write_bytes(declare_rgb(path.read_bytes(), [(0,0,1),(1,0,2),(2,0,3),(3,1,0)] if nc==4 else None))
            run(args.decompress,'-i',str(path),'-o',str(work/'negative.pgx'),'-threads','1')
            negatives.append(dict(input=path.name,reason=reason,components=nc,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    (root/'jpeg2000-ex-color-rejected.json').write_text(json.dumps(negatives,indent=2)+'\n')
    (root/'jpeg2000-ex-color-manifest.json').write_text(json.dumps(dict(
        provenance='Original seeded U16, OpenJPEG CLI 2.5.3 encode and independent per-component PGX decode. CDEF explicitly declares RGB channel order and straight whole-image alpha; tooling only.', files=rows), indent=2)+'\n')
    (root/'jpeg2000-ex-color-manifest.txt').write_text(''.join(
        f"{r['input']} {r['source']} {r['bits']} {r['width']} {r['height']} {r['components']}\n" for r in rows))
    print(f'{len(rows)} source/PGX-exact colour fixtures')


if __name__ == '__main__':
    main()
