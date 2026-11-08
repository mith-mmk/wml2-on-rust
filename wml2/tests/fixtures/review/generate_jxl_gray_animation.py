"""Reproduce bounded SDR Gray animation fixtures using official libjxl 0.12.0.

python generate_jxl_gray_animation.py --tools /path/to/libjxl/bin
Python stdlib only. Source/oracles never use WML2 or jxl-rs. Synthetic sources
are covered by the project MIT license. All scratch files live in .test*.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import zlib

W, H, FRAMES = 32, 24, 3
ROOT = Path(__file__).resolve().parent
WORK = ROOT / '.test-jxl-gray'


def pack(fmt, *v):
    return struct.pack('>' + fmt, *v)


def chunk(kind, data):
    return pack('I', len(data)) + kind + data + pack('I', zlib.crc32(kind + data))


def source(bits, alpha, icc=None):
    out = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', pack('IIBBBBB', W, H, bits, 4 if alpha else 0, 0, 0, 0))
    out += chunk(b'iCCP', b'Gray gamma2\x00\x00' + zlib.compress(icc)) if icc else chunk(b'sRGB', b'\x01')
    out += chunk(b'acTL', pack('II', FRAMES, 3))
    seq = 0
    for f, delay in enumerate([5, 9, 13]):
        out += chunk(b'fcTL', pack('IIIIIHHBB', seq, W, H, 0, 0, delay, 100, 0, 0))
        seq += 1
        raw = bytearray()
        for y in range(H):
            raw.append(0)
            for x in range(W):
                values = [(x*1703+y*193+f*4111) % (1 << bits)]
                if alpha:
                    values.append((x*3457+y*1297+f*1741) % (1 << bits))
                raw += struct.pack('>' + ('B' if bits == 8 else 'H')*len(values), *values)
        data = zlib.compress(raw, 9)
        if f == 0:
            out += chunk(b'IDAT', data)
        else:
            out += chunk(b'fdAT', pack('I', seq) + data)
            seq += 1
    return out + chunk(b'IEND', b'')


def codestream(data):
    if data.startswith(b'\xff\x0a'):
        return data
    result = bytearray()
    pos = index = 0
    final = False
    while pos < len(data):
        size = int.from_bytes(data[pos:pos+4], 'big')
        kind = data[pos+4:pos+8]
        assert size >= 8 and pos+size <= len(data)
        payload = data[pos+8:pos+size]
        if kind == b'jxlp':
            order = int.from_bytes(payload[:4], 'big')
            assert not final and order & 0x7fffffff == index
            final = bool(order & 0x80000000)
            index += 1
            result.extend(payload[4:])
        pos += size
    assert final and result.startswith(b'\xff\x0a')
    return bytes(result)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tools', type=Path, required=True)
    tools = parser.parse_args().tools.resolve()
    cjxl, djxl = tools/'cjxl', tools/'djxl'
    if not cjxl.exists():
        cjxl, djxl = tools/'cjxl.exe', tools/'djxl.exe'
    versions = [subprocess.check_output([str(t), '-V'], stderr=subprocess.STDOUT, text=True).strip() for t in [cjxl, djxl]]
    assert all('v0.12.0 4128790' in v for v in versions), versions
    WORK.mkdir(exist_ok=True)
    sha = lambda b: hashlib.sha256(b).hexdigest()
    cases = []
    for bits, alpha, icc in [(8, False, None), (8, True, None), (16, False, None), (16, True, None),
                             (16, False, (ROOT/'jxl-gray-gamma2.icc').read_bytes())]:
        name = f'jxl-animation-gray{"alpha" if alpha else ""}{bits}' + ('-icc-refused' if icc else '')
        stem = ROOT/name
        original = source(bits, alpha, icc)
        (WORK/'source.apng').write_bytes(original)
        subprocess.run([str(cjxl), str(WORK/'source.apng'), str(WORK/'encoded.jxl'), '-d', '0', '-e', '7', '--keep_invisible=1'], check=True)
        encoded = codestream((WORK/'encoded.jxl').read_bytes())
        Path(str(stem)+'.jxl').write_bytes(encoded)
        case = {'name': name, 'bits': bits, 'alpha': alpha, 'source_apng_sha256': sha(original),
                'codestream_sha256': sha(encoded), 'supported': not bool(icc)}
        if not icc:
            subprocess.run([str(djxl), str(stem)+'.jxl', str(WORK/'reference.pam'), '--bits_per_sample=16',
                            '--color_space=RGB_D65_SRG_Rel_SRG', '--output_frames'], check=True)
            reference = bytearray()
            for f in range(FRAMES):
                header, data = (WORK/f'reference.pam-{f}.pam').read_bytes().split(b'ENDHDR\n', 1)
                depth = 4 if alpha else 3
                for field in [b'WIDTH 32', b'HEIGHT 24', f'DEPTH {depth}'.encode(), f'MAXVAL {(1 << bits)-1}'.encode()]:
                    assert field in header, header
                assert len(data) == W*H*depth*(bits//8)
                values = ([v*257 for v in data] if bits == 8 else
                          struct.unpack('>'+'H'*(len(data)//2), data))
                if not alpha:
                    values = [v for i in range(0, len(values), 3) for v in [*values[i:i+3], 65535]]
                reference.extend(struct.pack('<'+'H'*len(values), *values))
            Path(str(stem)+'.rgba16le').write_bytes(reference)
            case.update(reference_sha256=sha(reference), reference_bytes=len(reference))
        cases.append(case)
    manifest = {'source': 'Synthetic Gray/GrayAlpha APNG; project MIT license; formulas in generator',
                'release': 'https://github.com/libjxl/libjxl/releases/tag/v0.12.0',
                'tools': versions, 'encoder_arguments': '-d 0 -e 7 --keep_invisible=1; strip jxlp framing only',
                'oracle': 'djxl --bits_per_sample=16 --color_space=RGB_D65_SRG_Rel_SRG --output_frames; PAM 8-bit expanded by 257 or true big-endian u16 to RGBA16LE, opaque alpha appended to RGB',
                'dimensions': [W,H], 'frames': FRAMES, 'timing': {'ticks_per_second': [1000,1], 'ticks': [50,90,130], 'loops': 0},
                'profile': 'Simple Gray D65/sRGB transfer, relative intent; refused case embeds arbitrary Gray gamma2 ICC',
                'cases': cases}
    (ROOT/'jxl-animation-gray.json').write_text(json.dumps(manifest, indent=2)+'\n')
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    main()
