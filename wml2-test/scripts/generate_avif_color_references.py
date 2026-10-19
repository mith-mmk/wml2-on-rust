"""Independent libavif16/LittleCMS16 AVIF references; never calls WML2.

The checked-in AVIF inputs are frozen. Default mode verifies their oracles;
--write refreshes them. Matching libavif development headers/runtime and
LittleCMS2 are development tools, not WML2 runtime dependencies.
"""
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
from pathlib import Path
import struct
import subprocess

REPO = Path(__file__).resolve().parents[2]

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture-dir', type=Path, default=REPO / 'wml2/tests/fixtures/review')
    parser.add_argument('--work-dir', type=Path, default=REPO / '.test-avif-color-reference')
    parser.add_argument('--avif-header', type=Path)
    parser.add_argument('--avif-library', help='Exact Unix soname when no development symlink exists')
    parser.add_argument('--cc', default='cc')
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    work = args.work_dir.resolve()
    if not work.name.startswith('.test'):
        parser.error('working directory must begin with .test')
    work.mkdir(exist_ok=True)
    fixtures = args.fixture_dir.resolve()
    if not fixtures.is_dir():
        parser.error('fixture directory must already exist')
    executable = work / 'avif-color-reference'
    command = [args.cc, '-O2', str(Path(__file__).with_name('avif_color_reference.c'))]
    if args.avif_header:
        command.append('-DWML2_AVIF_HEADER=' + json.dumps(str(args.avif_header.resolve())))
    command += ['-Wl,-l:' + args.avif_library] if args.avif_library else ['-lavif']
    subprocess.run(command + ['-o', str(executable)], check=True)
    version = subprocess.check_output([str(executable), '--version'], text=True).strip()
    runtime, header = (piece.split('=', 1)[1] for piece in version.split())
    if runtime != header:
        raise RuntimeError('reference runtime and header versions must match: ' + version)

    cms = C.CDLL(ctypes.util.find_library('lcms2'))
    cms.cmsOpenProfileFromMem.argtypes = [C.c_void_p, C.c_uint32]
    cms.cmsOpenProfileFromMem.restype = C.c_void_p
    cms.cmsGetColorSpace.argtypes = [C.c_void_p]
    cms.cmsGetColorSpace.restype = C.c_uint32
    cms.cmsChannelsOf.argtypes = [C.c_uint32]
    cms.cmsChannelsOf.restype = C.c_uint32
    cms.cmsCreateTransform.argtypes = [C.c_void_p, C.c_uint32, C.c_void_p, C.c_uint32, C.c_uint32, C.c_uint32]
    cms.cmsCreateTransform.restype = C.c_void_p
    cms.cmsDoTransform.argtypes = [C.c_void_p, C.c_void_p, C.c_void_p, C.c_uint32]
    cms.cmsDeleteTransform.argtypes = [C.c_void_p]
    cms.cmsCloseProfile.argtypes = [C.c_void_p]
    cms.cmsGetEncodedCMMversion.restype = C.c_uint32
    destination = (REPO / 'wml2/src/srgb.icc').read_bytes()
    manifest_path = fixtures / 'avif-icc-origins.json'
    manifest = json.loads(manifest_path.read_text())
    failures = []
    for item in manifest:
        name = item['name']
        source_path = fixtures / (name + '.avif')
        if digest(source_path.read_bytes()) != item['avif_sha256']:
            raise RuntimeError(name + ': source hash differs from frozen input')
        raw_path, icc_path = work / (name + '.rgba16le'), work / (name + '.icc')
        subprocess.run([str(executable), str(source_path), str(raw_path), str(icc_path)], check=True)
        raw, profile = raw_path.read_bytes(), icc_path.read_bytes()
        if digest(profile) != item['icc_sha256']:
            raise RuntimeError(name + ': source ICC differs from frozen profile')
        if len(raw) != item['size'][0] * item['size'][1] * 8:
            raise RuntimeError(name + ': reference dimensions differ')
        samples = struct.unpack('<' + 'H' * (len(raw) // 2), raw)
        pixels = len(samples) // 4
        source_owner, destination_owner = C.create_string_buffer(profile), C.create_string_buffer(destination)
        source_profile = cms.cmsOpenProfileFromMem(source_owner, len(profile))
        destination_profile = cms.cmsOpenProfileFromMem(destination_owner, len(destination))
        if not source_profile or not destination_profile:
            raise RuntimeError(name + ': reference ICC parse failed')
        channels = cms.cmsChannelsOf(cms.cmsGetColorSpace(source_profile))
        if channels not in (1, 3):
            raise RuntimeError('reference corpus expects Gray/RGB')
        source = [samples[i * 4 + c] for i in range(pixels) for c in range(channels)]
        source = (C.c_uint16 * len(source))(*source)
        target = (C.c_uint16 * (pixels * 3))()
        # Stable LCMS2 format macros: COLORSPACE_SH, CHANNELS_SH, BYTES_SH.
        source_type = ((3 if channels == 1 else 4) << 16) | (channels << 3) | 2
        target_type = (4 << 16) | (3 << 3) | 2
        transform = cms.cmsCreateTransform(source_profile, source_type, destination_profile,
                                           target_type, 1, 0x0100)  # Relative, NOOPTIMIZE
        if not transform:
            raise RuntimeError(name + ': reference transform compilation failed')
        cms.cmsDoTransform(transform, source, target, pixels)
        raw8 = bytes((value * 255 + 32767) // 65535 for value in samples)
        managed8 = bytes((target[i * 3 + c] * 255 + 32767) // 65535 if c < 3
                         else (samples[i * 4 + 3] * 255 + 32767) // 65535
                         for i in range(pixels) for c in range(4))
        cms.cmsDeleteTransform(transform)
        cms.cmsCloseProfile(source_profile)
        cms.cmsCloseProfile(destination_profile)
        for suffix, content in [('raw.rgba', raw8), ('srgb.rgba', managed8)]:
            path = fixtures / (name + '.' + suffix)
            if args.write:
                path.write_bytes(content)
            elif not path.exists() or path.read_bytes() != content:
                failures.append(path.name)
        if args.write:
            item['raw_sha256'], item['srgb_sha256'] = digest(raw8), digest(managed8)
            item['oracle'] = 'libavif RGBA16 straight alpha; relative LittleCMS16; final RGBA8 quantization'
            item['reference_detail'] = {'libavif': runtime, 'lcms_encoded_version': cms.cmsGetEncodedCMMversion(),
                                        'raw16_sha256': digest(raw), 'destination_icc_sha256': digest(destination)}
            item.pop('measured_raw16_max_delta', None)
    if args.write:
        manifest_path.write_text(json.dumps(manifest, indent=2) + '\n')
    if failures:
        raise RuntimeError('reference mismatch: ' + ', '.join(failures))
    print(f'{len(manifest)} independent AVIF references verified; {version}; LCMS{cms.cmsGetEncodedCMMversion()}')

if __name__ == '__main__':
    main()
