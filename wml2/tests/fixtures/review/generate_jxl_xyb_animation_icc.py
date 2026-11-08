"""Generate XYB RGBA16 with distinct source/reconstruction ICCs and a libjxl oracle.

python generate_jxl_xyb_animation_icc.py --tools /path/to/libjxl/bin
Only Python stdlib and official cjxl/djxl 0.12.0 are needed. No Rust decoder is
used for source or reference generation. Temporary files live in .test-jxl-xyb-icc.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import runpy
import struct
import subprocess
import zlib

ROOT = Path(__file__).resolve().parent
WORK = ROOT / '.test-jxl-xyb-icc'
STEM = ROOT / 'jxl-animation-xyb-icc16'
W, H, FRAMES = 32, 24, 3


def rgb_profile():
    # Existing LittleCMS-generated RGB matrix/TRC profile, with separate gamma
    # 2.0, 2.25, 2.5 curves. This cannot collapse to one JXL simple transfer
    # function, so cjxl preserves the embedded ICC. No LUT or HDR signaling.
    data = bytearray((ROOT / 'jxl-gamma2.icc').read_bytes())
    for tag, gamma in [(b'gTRC', 576), (b'bTRC', 640)]:
        offset = len(data)
        data += b'curv' + bytes(4) + struct.pack('>IH', 1, gamma) + bytes(2)
        for i in range(int.from_bytes(data[128:132], 'big')):
            pos = 132 + 12 * i
            if data[pos:pos+4] == tag:
                data[pos+4:pos+12] = struct.pack('>II', offset, 14)
    data[:4] = len(data).to_bytes(4, 'big')
    data[84:100] = bytes(16)  # Clear the optional old ICC profile ID.
    return bytes(data)


def assert_linear_srgb(icc):
    tags = {}
    for i in range(int.from_bytes(icc[128:132], 'big')):
        tag, offset, size = struct.unpack('>4sII', icc[132+12*i:144+12*i])
        tags[tag] = icc[offset:offset+size]
    assert icc[16:20] == b'RGB '
    assert tags[b'cicp'] == b'cicp' + bytes(4) + bytes([1, 8, 0, 1])
    linear = b'para' + bytes(4) + struct.pack('>HH5i', 3, 0, 65536, 65536, 0, 65536, 0)
    assert all(tags[tag] == linear for tag in [b'rTRC', b'gTRC', b'bTRC'])


def linear_reference(data):
    # Read the small NPY v1 float32 array without depending on NumPy.
    assert data[:8] == b'\x93NUMPY\x01\x00'
    length = int.from_bytes(data[8:10], 'little')
    header = ast.literal_eval(data[10:10+length].decode('ascii'))
    assert header == {'descr': '<f4', 'fortran_order': False, 'shape': (FRAMES, H, W, 4)}
    pixels = data[10+length:]
    assert len(pixels) == FRAMES * H * W * 4 * 4
    result = bytearray()
    for index, (value,) in enumerate(struct.iter_unpack('<f', pixels)):
        if index % 4 != 3:
            value = 12.92 * value if value <= 0.0031308 else 1.055 * value ** (1 / 2.4) - 0.055
        sample = int(max(0.0, min(1.0, value)) * 65535 + 0.5)
        result += struct.pack('<H', sample)
    return bytes(result)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tools', type=Path, required=True)
    tools = parser.parse_args().tools
    cjxl, djxl = tools / 'cjxl', tools / 'djxl'
    if not cjxl.exists():
        cjxl, djxl = tools / 'cjxl.exe', tools / 'djxl.exe'
    versions = [subprocess.check_output([str(t), '-V'], stderr=subprocess.STDOUT, text=True).strip()
                for t in (cjxl, djxl)]
    assert all('v0.12.0 4128790' in v for v in versions), versions
    helpers = runpy.run_path(str(ROOT / 'generate_jxl_xyb_animation16.py'))
    original = helpers['source']()
    profile = rgb_profile()
    source = bytearray(original[:8])
    pos = 8
    while pos < len(original):
        size = int.from_bytes(original[pos:pos+4], 'big')
        part = original[pos:pos+size+12]
        if original[pos+4:pos+8] == b'sRGB':
            part = helpers['chunk'](b'iCCP', b'RGB matrix TRC\0\0' + zlib.compress(profile))
        source += part
        pos += size + 12
    WORK.mkdir(exist_ok=True)
    (WORK / 'source.apng').write_bytes(source)
    subprocess.run([str(cjxl), str(WORK / 'source.apng'), str(WORK / 'encoded.jxl'),
                    '-d', '1', '-e', '7'], check=True)
    encoded = helpers['codestream']((WORK / 'encoded.jxl').read_bytes())
    Path(str(STEM) + '.jxl').write_bytes(encoded)
    # Ask for the codec's default floating-point output, and verify its actual
    # linear-sRGB ICC. Do not relabel source ICC coordinates or quantize linear
    # light to 16 bits before applying the sRGB transfer function.
    subprocess.run([str(djxl), str(STEM) + '.jxl', str(WORK / 'reference.npy'),
                    '--output_frames', '--icc_out=' + str(WORK / 'output.icc'),
                    '--orig_icc_out=' + str(WORK / 'source.icc')], check=True)
    output_icc = (WORK / 'output.icc').read_bytes()
    assert_linear_srgb(output_icc)
    assert (WORK / 'source.icc').read_bytes() == profile
    reference = linear_reference((WORK / 'reference.npy').read_bytes())
    Path(str(STEM) + '.rgba16le').write_bytes(reference)
    sha = lambda b: hashlib.sha256(b).hexdigest()
    manifest = {
        'source': 'Deterministic synthetic RGBA16 APNG from generate_jxl_xyb_animation16.py, with standard RGB matrix/TRC ICC; project MIT license',
        'encoder': versions[0] + '; -d 1 -e 7; container stripped without changing codestream',
        'decoder': versions[1] + '; --output_frames NPY float32; actual linear-sRGB output ICC checked before standard sRGB transfer and u16 rounding',
        'release': 'https://github.com/libjxl/libjxl/releases/tag/v0.12.0',
        'source_apng_sha256': sha(source), 'codestream_sha256': sha(encoded),
        'source_icc_sha256': sha(profile), 'libjxl_linear_output_icc_sha256': sha(output_icc),
        'libjxl_float_reference_sha256': sha((WORK / 'reference.npy').read_bytes()),
        'srgb16_reference_sha256': sha(reference),
        'dimensions': [W, H], 'frames': FRAMES, 'rgb_bits': 16, 'alpha_bits': 16,
        'source_color': 'RGB matrix/TRC; gamma 2.0/2.25/2.5; no LUT/HDR',
        'wml2_output_color': 'jxl 0.8 embedded-ICC XYB fallback sRGB; distinct from original ICC',
        'timing': {'ticks_per_second': [1000, 1], 'frame_ticks': [50, 90, 130], 'loops': 0},
        'comparison': 'Every native and EX u16 sample against independently reconstructed sRGB; ordinary and managed paths after final rounding; alpha never color transformed',
        'rgb_tolerance_u16': 1, 'alpha_tolerance_u16': 0,
        'not_claimed': 'Arbitrary ICC/LUT correctness, HDR, browser-wide color equivalence, or lossless RGB agreement after lossy encoding',
    }
    Path(str(STEM) + '.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    main()
