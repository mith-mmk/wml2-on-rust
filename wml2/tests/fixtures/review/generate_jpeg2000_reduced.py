#!/usr/bin/env python3
"""Independent OpenJPEG native-resolution references; all work lives below .test*."""
import argparse
import hashlib
import gzip
import json
from pathlib import Path
import re
import resource
import runpy
import struct
import subprocess

ROOT = Path(__file__).resolve().parent
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def sha(data):
    return hashlib.sha256(data).hexdigest()


def codestream(data):
    if data.startswith(b'\xff\x4f'):
        return data
    pos = 0
    while pos < len(data):
        length, kind = struct.unpack_from('>I4s', data, pos)
        header = 8
        if length == 1:
            length = struct.unpack_from('>Q', data, pos + 8)[0]
            header = 16
        if length == 0:
            length = len(data) - pos
        assert length >= header
        if kind == b'jp2c':
            return data[pos + header:pos + length]
        pos += length
    raise ValueError('missing jp2c')


def main_segments(data):
    pos = 2
    while data[pos:pos + 2] != b'\xff\x90':
        marker, length = struct.unpack_from('>HH', data, pos)
        yield marker, data[pos:pos + length + 2]
        pos += length + 2


def geometry(data):
    stream = codestream(data)
    payload = stream[6:6 + struct.unpack_from('>H', stream, 4)[0] - 2]
    x1, y1, x0, y0, tw, th, tx, ty = struct.unpack_from('>8I', payload, 2)
    nc = struct.unpack_from('>H', payload, 34)[0]
    bits = (payload[36] & 127) + 1
    assert all(payload[36 + c * 3:39 + c * 3] == bytes([bits - 1, 1, 1]) for c in range(nc))
    styles = [(0, 1)] * nc

    def apply(marker, item, styles):
        b = item[4:]
        if marker == 0xff52:
            styles[:] = [(b[5], b[9])] * nc
        elif marker == 0xff53:
            assert nc < 257
            styles[b[0]] = (b[2], b[6])

    pos = 2
    for marker, item in main_segments(stream):
        apply(marker, item, styles)
        pos += len(item)
    tiles = {}
    while stream[pos:pos + 2] == b'\xff\x90':
        tile = struct.unpack_from('>H', stream, pos + 4)[0]
        length = struct.unpack_from('>I', stream, pos + 6)[0]
        end = pos + length if length else len(stream) - 2
        active = tiles.setdefault(tile, list(styles))
        q = pos + 12
        while stream[q:q + 2] != b'\xff\x93':
            marker, size = struct.unpack_from('>HH', stream, q)
            apply(marker, stream[q:q + size + 2], active)
            q += size + 2
        pos = end
    assert tiles
    return dict(bits=bits, nc=nc, source_width=x1-x0, source_height=y1-y0,
                x0=x0, y0=y0, x1=x1, y1=y1, min_levels=min(s[0] for tile in tiles.values() for s in tile),
                filters=sorted({s[1] for tile in tiles.values() for s in tile}))


def packed_reference(work, bits, nc, expected):
    planes = []
    for c in range(nc):
        header, raw = (work / f'reference_{c}.pgx').read_bytes().split(b'\n', 1)
        magic, endian, sign, depth, w, h = header.split()
        assert magic == b'PG' and sign == b'+' and int(depth) == bits
        assert (int(w), int(h)) == expected, (header, expected)
        values = tuple(raw) if bits == 8 else struct.unpack(('>' if endian == b'ML' else '<') + str(expected[0]*expected[1]) + 'H', raw)
        planes.append(values)
    maximum = (1 << bits) - 1
    if bits != 8 and nc == 1:
        values = planes[0]
    else:
        values = []
        for i in range(expected[0] * expected[1]):
            values.extend([planes[0][i]] * 3 if nc == 1 else [planes[c][i] for c in range(3)])
            values.append(planes[3][i] if nc == 4 else maximum)
    return bytes(values) if bits == 8 else struct.pack('<' + str(len(values)) + 'H', *values)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compress', required=True)
    parser.add_argument('--decompress', required=True)
    parser.add_argument('--work-dir', type=Path, required=True)
    args = parser.parse_args()
    work = args.work_dir.resolve()
    assert work.name.startswith('.test')
    work.mkdir(exist_ok=True)
    declare = runpy.run_path(str(ROOT / 'generate_jpeg2000_ex_color.py'))['declare_rgb']
    wrap = runpy.run_path(str(ROOT / 'generate_jpeg2000_ex_gray.py'))['wrap']
    inputs = []
    encoding_failures = []
    generated = {}
    for kind, w, h, origin, tile in [('odd',33,29,(3,5),False),('empty_edge_tiles',63,49,(3,5),True),('singleton_output',5,33,(1,1),False)]:
        for bits in [8, 12, 16]:
            for nc in [1, 3, 4]:
                levels = 2 if kind == 'singleton_output' else 3
                raw = work / 'source.rawl'
                maximum = (1 << bits) - 1
                values = []
                for c in range(nc):
                    for y in range(h):
                        for x in range(w):
                            values.append((0 if (x+y)%5 == 0 else maximum) if c == 3 else (x*(17+c*2)+y*(31-c*3)) & maximum)
                raw.write_bytes(bytes(values) if bits == 8 else struct.pack('<'+str(len(values))+'H', *values))
                for wavelet in ['53', '97']:
                    name = f'jp2_reduced_{kind}_{bits}b_{nc}c_{wavelet}.jp2'
                    path = work / name
                    cmd = [args.compress,'-i',str(raw),'-o',str(path),'-F',f'{w},{h},{nc},{bits},u','-n',str(levels+1),'-d',f'{origin[0]},{origin[1]}','-mct',str(int(nc>1)),'-threads','1']
                    if tile: cmd += ['-t','16,16','-T','1,1']
                    if wavelet == '97': cmd += ['-I']
                    encoded = subprocess.run(cmd, capture_output=True, text=True)
                    if encoded.returncode:
                        assert kind == 'empty_edge_tiles' and wavelet == '97', encoded.stderr
                        encoding_failures.append(dict(input=name,exit=encoded.returncode,stderr=encoded.stderr,command=cmd))
                        continue
                    data = path.read_bytes()
                    if nc > 1: data = declare(data, [(0,0,1),(1,0,2),(2,0,3),(3,1,0)] if nc == 4 else None)
                    (ROOT / name).write_bytes(data)
                    generated[kind,bits,nc,wavelet] = data
                    inputs.append(dict(input=name,provenance='OpenJPEG encoded deterministic source',encode_command=cmd))
    # OpenJPEG's9/7 encoder asserts on the one-pixel edge. Zero coefficients have
    # empty entropy packets valid for either filter; keep this derived provenance.
    for bits in [8,12,16]:
        for nc in [1,3,4]:
            raw = work / 'zero.rawl'
            midpoint = 1 << (bits - 1)
            raw.write_bytes((bytes([midpoint]) if bits == 8 else struct.pack('<H', midpoint)) * (63*49*nc))
            base = work / 'zero53.jp2'
            cmd = [args.compress,'-i',str(raw),'-o',str(base),'-F',f'63,49,{nc},{bits},u','-n','4','-d','3,5','-T','1,1','-t','16,16','-mct',str(int(nc>1)),'-threads','1']
            subprocess.run(cmd, check=True, capture_output=True)
            template = base.read_bytes()
            stream = codestream(template)
            qcd = next(item for m,item in main_segments(codestream(generated['odd',bits,nc,'97'])) if m == 0xff5c)
            out = b'\xff\x4f'
            pos = 2
            for marker,item in main_segments(stream):
                pos += len(item)
                if marker == 0xff52: item = item[:13]+b'\0'+item[14:]
                if marker == 0xff5c: item = qcd
                out += item
            data = wrap(out+stream[pos:], template)
            if nc > 1: data = declare(data, [(0,0,1),(1,0,2),(2,0,3),(3,1,0)] if nc == 4 else None)
            name = f'jp2_reduced_empty_zero_{bits}b_{nc}c_97.jp2'
            (ROOT / name).write_bytes(data)
            inputs.append(dict(input=name,provenance='Derived zero-entropy5/3 packets with valid independently encoded9/7 COD/QCD; independently decoded, not claimed independently9/7 encoded',encode_command=cmd))
    for group in ['gray','color','97']:
        for item in json.loads((ROOT / f'jpeg2000-ex-{group}-manifest.json').read_text())['files']:
            oracle = re.sub(r'(_qcc_[01])_1(?=\.)', r'\1_0', item['input'])
            inputs.append(dict(input=item['input'],oracle=oracle,provenance='Existing independent EX corpus; canonical QCD-before-QCC order for equivalent permutations' if oracle != item['input'] else 'Existing independent EX corpus'))
    for item in json.loads((ROOT/'jpeg2000-general-manifest.json').read_text())['files']:
        if re.fullmatch(r'jp2_general_(L|RGB)_(LRCP|RLCP|RPCL|PCRL|CPRL)_[01]\.jp2', item['input']):
            inputs.append(dict(input=item['input'],provenance='Existing8-bit multi-layer/all-five-progression/precinct corpus'))
    blob = bytearray()
    entries = []
    failures = []
    empty = []
    for item in inputs:
        data = (ROOT/item['input']).read_bytes()
        g = geometry(data)
        oracle = ROOT/item.get('oracle',item['input'])
        for reduce in range(1,g['min_levels']+1):
            factor = 1 << reduce
            shape = ((g['x1']+factor-1)//factor-(g['x0']+factor-1)//factor,
                     (g['y1']+factor-1)//factor-(g['y0']+factor-1)//factor)
            if not all(shape):
                empty.append(dict(input=item['input'],reduce=reduce,geometry=g))
                continue
            cmd = [args.decompress,'-i',str(oracle),'-o',str(work/'reference.pgx'),'-r',str(reduce),'-threads','1']
            decoded = subprocess.run(cmd, capture_output=True, text=True)
            if decoded.returncode:
                failures.append(dict(input=item['input'],reduce=reduce,exit=decoded.returncode,stderr=decoded.stderr,command=cmd))
                continue
            reference = packed_reference(work,g['bits'],g['nc'],shape)
            row = dict(item,**g,reduce=reduce,width=shape[0],height=shape[1],offset=len(blob),length=len(reference),source_sha256=sha(data),oracle_sha256=sha(oracle.read_bytes()),reference_sha256=sha(reference),decode_command=cmd)
            entries.append(row)
            blob.extend(reference)
    (ROOT/'jpeg2000-reduced-refs.bin').write_bytes(blob)
    (ROOT/'jpeg2000-reduced-manifest.txt').write_text(''.join(' '.join(str(row[k]) for k in ['input','offset','length','bits','nc','reduce','width','height'])+'\n' for row in entries))
    report = dict(provenance='OpenJPEG2.5.3 native opj_decompress-r PGX; no full-resolution resize oracle',reference_blob_sha256=sha(blob),reference_bytes=len(blob),input_profiles=len(inputs),entries=entries,encode_failures=encoding_failures,decode_failures=failures,empty_global_controls=empty)
    provenance = gzip.compress((json.dumps(report,indent=2)+'\n').encode(), mtime=0)
    (ROOT/'jpeg2000-reduced-provenance.json.gz').write_bytes(provenance)
    summary = dict(provenance=report['provenance'],input_profiles=len(inputs),references=len(entries),
                   reference_bytes=len(blob),reference_blob_sha256=sha(blob),
                   provenance_file='jpeg2000-reduced-provenance.json.gz',provenance_sha256=sha(provenance),
                   encode_failures=encoding_failures,decode_failures=failures,empty_global_controls=empty)
    (ROOT/'jpeg2000-reduced-manifest.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(dict(inputs=len(inputs),references=len(entries),reference_bytes=len(blob),encode_failures=len(encoding_failures),decode_failures=len(failures),empty_global_controls=len(empty))))


if __name__ == '__main__':
    main()
