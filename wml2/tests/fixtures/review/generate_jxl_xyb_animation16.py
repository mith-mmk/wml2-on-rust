"""Generate a real 16-bit XYB animation and independent 16-bit libjxl oracle.

python generate_jxl_xyb_animation16.py --tools /path/to/libjxl/bin
Requires official cjxl/djxl (recorded: v0.12.0 4128790). Uses only Python stdlib.
All temporary files stay in .test-jxl-xyb16 beside the fixtures. No Rust decoder
is involved in source generation or reference pixels. Outputs are adjacent.
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
STEM = ROOT / 'jxl-animation-xyb-rgba16'
WORK = ROOT / '.test-jxl-xyb16'


def pack(fmt, *values):
    return struct.pack('>' + fmt, *values)


def chunk(kind, data):
    return pack('I', len(data)) + kind + data + pack('I', zlib.crc32(kind + data))


def source():
    out = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', pack('IIBBBBB', W, H, 16, 6, 0, 0, 0))
    out += chunk(b'sRGB', b'\x01') + chunk(b'acTL', pack('II', FRAMES, 3))
    seq = 0
    for f, delay in enumerate([5, 9, 13]):
        out += chunk(b'fcTL', pack('IIIIIHHBB', seq, W, H, 0, 0, delay, 100, 0, 0))
        seq += 1
        rows = bytearray()
        for y in range(H):
            rows.append(0)
            for x in range(W):
                rows += pack('4H', (x*1703+y*193+f*4111) % 65536,
                             (x*911+y*2347+f*9017) % 65536,
                             (x*2137+y*743+f*3137) % 65536,
                             (x*3457+y*1297+f*1741) % 65536)
        data = zlib.compress(rows, 9)
        if f == 0:
            out += chunk(b'IDAT', data)
        else:
            out += chunk(b'fdAT', pack('I', seq) + data)
            seq += 1
    return out + chunk(b'IEND', b'')


def codestream(data):
    # cjxl emits a jxlp container for animation even with --container=0.
    # Strip only the container framing; preserve all encoded image/frame bits.
    if data.startswith(b'\xff\x0a'):
        return data
    result = bytearray()
    pos = index = 0
    final = False
    while pos < len(data):
        size = int.from_bytes(data[pos:pos+4], 'big')
        kind = data[pos+4:pos+8]
        assert size >= 8 and pos + size <= len(data)
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
    args = argparse.ArgumentParser()
    args.add_argument('--tools', type=Path, required=True)
    tools = args.parse_args().tools
    cjxl, djxl = tools / 'cjxl', tools / 'djxl'
    if not cjxl.exists():
        cjxl, djxl = tools / 'cjxl.exe', tools / 'djxl.exe'
    versions = [subprocess.check_output([str(tool), '-V'], stderr=subprocess.STDOUT, text=True).strip()
                for tool in (cjxl, djxl)]
    assert all('v0.12.0 4128790' in v for v in versions), versions
    WORK.mkdir(exist_ok=True)
    original = source()
    (WORK / 'source.apng').write_bytes(original)
    subprocess.run([str(cjxl), str(WORK / 'source.apng'), str(WORK / 'encoded.jxl'),
                    '-d', '1', '-e', '7'], check=True)
    encoded = codestream((WORK / 'encoded.jxl').read_bytes())
    Path(str(STEM) + '.jxl').write_bytes(encoded)
    subprocess.run([str(djxl), str(STEM) + '.jxl', str(WORK / 'reference.pam'),
                    '--bits_per_sample=16', '--color_space=RGB_D65_SRG_Rel_SRG',
                    '--output_frames', '--icc_out=' + str(WORK / 'output.icc'),
                    '--orig_icc_out=' + str(WORK / 'source.icc')], check=True)
    reference = bytearray()
    non_8bit = []
    for f in range(FRAMES):
        header, pixels = (WORK / f'reference.pam-{f}.pam').read_bytes().split(b'ENDHDR\n', 1)
        for field in [b'WIDTH 32', b'HEIGHT 24', b'DEPTH 4', b'MAXVAL 65535', b'TUPLTYPE RGB_ALPHA']:
            assert field in header
        assert len(pixels) == W * H * 8
        values = struct.unpack('>' + 'H' * (W*H*4), pixels)
        non_8bit.append(sum(v % 257 != 0 for v in values))
        reference.extend(struct.pack('<' + 'H' * len(values), *values))
    Path(str(STEM) + '.rgba16le').write_bytes(reference)
    # Both are semantic sRGB/relative profiles. Record bytes for provenance,
    # rather than equating different ICC generators' serialization.
    sha = lambda b: hashlib.sha256(b).hexdigest()
    manifest = {
        'source': 'Deterministic synthetic RGBA16 APNG; generation formulas in this script; project MIT license',
        'encoder': 'libjxl cjxl 0.12.0 4128790 -d 1 -e 7; jxlp container stripped without changing codestream',
        'decoder': 'libjxl djxl 0.12.0 4128790 --bits_per_sample=16 --color_space=RGB_D65_SRG_Rel_SRG --output_frames',
        'release': 'https://github.com/libjxl/libjxl/releases/tag/v0.12.0',
        'recorded_original_windows_archive_sha256': '3025d7e308390796d20492322e606bc92decaee7b6bc99d3f7547870ae5db7de',
        'archive_note': 'Hash of the verified original Windows distribution; a rerun validates tool version, not distribution bytes',
        'source_apng_sha256': sha(original), 'codestream_sha256': sha(encoded),
        'reference_sha256': sha(reference),
        'source_icc_sha256': sha((WORK/'source.icc').read_bytes()),
        'output_icc_sha256': sha((WORK/'output.icc').read_bytes()),
        'dimensions': [W, H], 'frames': FRAMES, 'source_rgb_bits': 16, 'source_alpha_bits': 16,
        'color': 'RGB D65 sRGB primaries, sRGB transfer, relative intent; straight alpha',
        'timing': {'ticks_per_second': [1000, 1], 'frame_ticks': [50, 90, 130], 'loops': 0},
        'reference_samples_not_divisible_by_257_per_frame': non_8bit,
        'comparison': 'Compare native/EX u16 directly, no u8 normalization, CMS, premultiplication or background compositing',
        'rgb_tolerance_u16': 1, 'alpha_tolerance_u16': 0,
    }
    Path(str(STEM) + '.json').write_text(json.dumps(manifest, indent=2)+'\n')
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    main()
