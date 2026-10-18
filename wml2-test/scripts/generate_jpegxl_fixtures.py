#!/usr/bin/env python3
"""Generate tiny, independently encoded JPEG XL fixtures with libjxl 0.11.2.
No WML2/jxl-rs output is used as a lossless expected value.
Usage: python3 ... --tools /path/to/libjxl-tools/usr --work .test-jxl-fixtures
"""
import argparse,hashlib,json,pathlib,struct,subprocess
import ctypes,ctypes.util
p=argparse.ArgumentParser();p.add_argument('--tools',type=pathlib.Path,required=True);p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('--work must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review';work=a.work.resolve();work.mkdir(exist_ok=True,parents=True)
def run(cmd):
 result=subprocess.run([str(v) for v in cmd],capture_output=True)
 if result.returncode: raise RuntimeError(result.stderr.decode(errors='replace'))
encoder=work/'encoder'
run(['cc','-O2','-I'+str(a.tools/'include'),root/'wml2-test/scripts/jpegxl_fixture_encoder.c','-Wl,-l:libjxl.so.0.11','-o',encoder])
# A real matrix RGB profile, with neutral gamma2 curves, independently made
# from LittleCMS sRGB primaries (the older synthetic ICC fixtures have dummy primaries).
from PIL import ImageCms
icc=bytearray(ImageCms.ImageCmsProfile(ImageCms.createProfile('sRGB')).tobytes())
icc[24:36]=struct.pack('>6H',2026,1,1,0,0,0);icc[84:100]=bytes(16)
for i in range(struct.unpack_from('>I',icc,128)[0]):
 off=132+12*i;tag=icc[off:off+4]
 if tag in [b'rTRC',b'gTRC',b'bTRC']:
  start,size=struct.unpack_from('>II',icc,off+4)
  curve=b'curv'+bytes(4)+struct.pack('>IH',1,512)
  icc[start:start+len(curve)]=curve
  struct.pack_into('>I',icc,off+8,len(curve))
(dest/'jxl-gamma2.icc').write_bytes(icc)
# Real Gray gamma2 profile from LittleCMS, exercising native Gray ICC retention
# and the explicit refusal to replicate arbitrary Gray ICC samples into RGBA.
cms_library=ctypes.util.find_library('lcms2')
if not cms_library:p.error('fixture regeneration requires a system LittleCMS2 library')
cms=ctypes.CDLL(cms_library)
cms.cmsD50_xyY.restype=ctypes.c_void_p
cms.cmsBuildGamma.argtypes=[ctypes.c_void_p,ctypes.c_double];cms.cmsBuildGamma.restype=ctypes.c_void_p
cms.cmsCreateGrayProfile.argtypes=[ctypes.c_void_p,ctypes.c_void_p];cms.cmsCreateGrayProfile.restype=ctypes.c_void_p
cms.cmsSaveProfileToMem.argtypes=[ctypes.c_void_p,ctypes.c_void_p,ctypes.POINTER(ctypes.c_uint32)];cms.cmsSaveProfileToMem.restype=ctypes.c_int
cms.cmsFreeToneCurve.argtypes=[ctypes.c_void_p];cms.cmsCloseProfile.argtypes=[ctypes.c_void_p]
curve=cms.cmsBuildGamma(None,2.0);assert curve
profile=cms.cmsCreateGrayProfile(cms.cmsD50_xyY(),curve);assert profile
size=ctypes.c_uint32();assert cms.cmsSaveProfileToMem(profile,None,ctypes.byref(size))
buffer=ctypes.create_string_buffer(size.value);assert cms.cmsSaveProfileToMem(profile,buffer,ctypes.byref(size))
cms.cmsCloseProfile(profile);cms.cmsFreeToneCurve(curve)
gray_icc=bytearray(buffer.raw);gray_icc[24:36]=struct.pack('>6H',2026,1,1,0,0,0);gray_icc[84:100]=bytes(16)
(dest/'jxl-gray-gamma2.icc').write_bytes(gray_icc)
cases=[]
for channels,name in [(1,'gray'),(2,'grayalpha'),(3,'rgb'),(4,'rgba')]:
 for bits in [8,12,16]:
  maxv=(1<<bits)-1
  samples=[]
  for i in range(35):
   for c in range(channels):
    v=(i*197+c*41+1)%maxv
    if c==channels-1 and channels in [2,4]: v=[0,1,maxv//2,maxv-1,maxv][i%5]
    samples.append(v)
  raw=bytes(samples) if bits==8 else struct.pack('<'+'H'*len(samples),*samples)
  source=work/f'{name}{bits}.raw';source.write_bytes(raw)
  output=dest/f'jxl-{name}{bits}.jxl';run([encoder,source,output,7,5,channels,bits,'lossless','-'])
  expected=dest/f'jxl-{name}{bits}.bin';expected.write_bytes(raw)
  cases.append(dict(file=output.name,expected=expected.name,width=7,height=5,channels=channels,bits=bits,mode='lossless'))
# Rejection profiles and orientation derive from an independent source above.
source=work/'rgba16.raw'
for mode in ['premult','animation','orientation','hdr']:
 output=dest/f'jxl-{mode}.jxl';run([encoder,source,output,7,5,4,16,mode,'-'])
run([encoder,work/'gray16.raw',dest/'jxl-float.jxl',7,5,1,16,'float','-'])
run([encoder,source,dest/'jxl-mixed-alpha.jxl',7,5,4,16,'mixed-alpha','-'])
run([encoder,source,dest/'jxl-depth10.jxl',7,5,4,10,'lossless','-'])
# Embedded non-sRGB profile exercises source/output profile identity (lossless)
# and different XYB output profile (lossy).
for mode in ['lossless','lossy']:
 output=dest/f'jxl-icc-{mode}.jxl';run([encoder,source,output,7,5,4,16,mode,dest/'jxl-gamma2.icc'])
 cases.append(dict(file=output.name,width=7,height=5,channels=4,bits=16,mode=mode))
output=dest/'jxl-icc-gray16.jxl';run([encoder,work/'gray16.raw',output,7,5,1,16,'lossless',dest/'jxl-gray-gamma2.icc'])
cases.append(dict(file=output.name,expected='jxl-gray16.bin',width=7,height=5,channels=1,bits=16,mode='lossless'))
# Larger lossy pattern, no alpha ambiguity. Oracle PNG -> raw RGBA via Pillow.
samples=bytes(v for y in range(24) for x in range(32) for v in ((x*7+y*3)%256,(y*11+x)%256,(x*x+y*5)%256))
source=work/'lossy.raw';source.write_bytes(samples);output=dest/'jxl-lossy.jxl';run([encoder,source,output,32,24,3,8,'lossy','-'])
cases.append(dict(file=output.name,width=32,height=24,channels=3,bits=8,mode='lossy'))
# djxl writes PAM samples big endian for >8bit; PNM output maxval describes
# the requested precision. Retain decoded channel samples as LE oracle bytes.
for case in cases:
 output=dest/case['file'];oracle=work/(output.stem+'.pam')
 cmd=[a.tools/'bin/djxl',output,oracle,'--bits_per_sample='+str(case['bits']),'--quiet']
 if output.stem=='jxl-icc-lossy': cmd.append('--color_space=RGB_D65_SRG_Rel_SRG')
 run(cmd)
 data=oracle.read_bytes();header,pixels=data.split(b'ENDHDR\n',1)
 fields=dict(line.split(maxsplit=1) for line in header.splitlines()[1:] if line and not line.startswith(b'#'))
 channels=int(fields[b'DEPTH']);bits=case['bits'];w=case['width'];h=case['height']
 vals=list(pixels) if bits==8 else list(struct.unpack('>'+'H'*(len(pixels)//2),pixels))
 assert len(vals)==w*h*channels,(case,len(vals),channels)
 oracle_path=dest/(output.stem+'-djxl.bin');oracle_path.write_bytes(bytes(vals) if bits==8 else struct.pack('<'+'H'*len(vals),*vals))
 case['oracle']=oracle_path.name;case['oracle_channels']=channels
 case['sha256']=hashlib.sha256(output.read_bytes()).hexdigest()
manifest={'encoder':'libjxl 0.11.2 C API, effort3, single encoder process','decoder':'djxl 0.11.2','source':'deterministic integer patterns in generate_jpegxl_fixtures.py; original samples are expected for lossless','cases':cases}
manifest['files']={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(dest.glob('jxl-*')) if p.suffix in ['.jxl','.bin','.icc']}
manifest['lossy_icc_oracle_color_space']='RGB_D65_SRG_Rel_SRG (matches jxl integer XYB fallback)'
manifest['icc_profiles']={'jxl-gamma2.icc':'Pillow LittleCMS sRGB matrix with neutral gamma2 curves','jxl-gray-gamma2.icc':'LittleCMS2 cmsCreateGrayProfile, D50, cmsBuildGamma(2.0); fixed timestamp and zero profile ID'}
(dest/'jxl-fixtures.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'cases':len(cases),'output':str(dest)},indent=2))
