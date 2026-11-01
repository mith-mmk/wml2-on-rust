#!/usr/bin/env python3
"""Generate associated-alpha oracles with independent C libjxl and LittleCMS.
Compile jpegxl_alpha_{fixture_encoder,reference}.c against libjxl0.11.2 first.
"""
import argparse,pathlib,struct,subprocess,hashlib,json,ctypes as C,ctypes.util
p=argparse.ArgumentParser();p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('work name must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];work=a.work.resolve();dest=root/'wml2/tests/fixtures/review';records=[]
def run(args):return subprocess.run([str(x) for x in args],capture_output=True,text=True,check=True)
assert 'runtime=11002 header=11002' in run([work/'reference','--version']).stdout
cms=C.CDLL(ctypes.util.find_library('lcms2'))
cms.cmsGetEncodedCMMversion.restype=C.c_uint32;assert cms.cmsGetEncodedCMMversion()==2160
cms.cmsOpenProfileFromMem.argtypes=[C.c_void_p,C.c_uint32];cms.cmsOpenProfileFromMem.restype=C.c_void_p
cms.cmsCreateTransform.argtypes=[C.c_void_p,C.c_uint32,C.c_void_p,C.c_uint32,C.c_uint32,C.c_uint32];cms.cmsCreateTransform.restype=C.c_void_p
cms.cmsDoTransform.argtypes=[C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32];cms.cmsDeleteTransform.argtypes=[C.c_void_p];cms.cmsCloseProfile.argtypes=[C.c_void_p]
for channels in [1,3]:
 for bits in [8,16]:
  for orientation in [1,6,8]:
   icc=channels==3 and orientation==8
   name=f'jxl-associated-{channels}c{bits}o{orientation}';values=[]
   for i in range(35):
    maxcode=(1<<bits)-1
    alpha=[0,1,maxcode//2,maxcode-1,maxcode][i%5]
    color=[(i*(7919+193*c)+12345*(c+1))%65536 for c in range(channels)]
    values+=color+[(alpha*65535+maxcode//2)//maxcode]
   source=work/(name+'.source16le');source.write_bytes(struct.pack('<'+'H'*len(values),*values));path=dest/(name+'.jxl')
   run([work/'encoder',source,path,7,5,channels,bits,bits,orientation,'associated',dest/'jxl-gamma2.icc' if icc else '-'])
   straight=dest/(name+'.rgba16le');profile=work/(name+'.icc')
   result=run([work/'reference',path,straight,profile,'straight'])
   # C default applies orientation and reports the oriented BasicInfo with tag1.
   assert f'image {bits} {bits} {channels} 1' in result.stderr and f'extra 0 0 {bits} 0 0 1' in result.stderr
   run([work/'reference',path,work/(name+'.asis'),work/(name+'.asis.icc')])
   assert [int(v) for v in result.stdout.split()]==([5,7] if orientation>=5 else [7,5])
   raw=straight.read_bytes();samples=struct.unpack('<140H',raw)
   # Association does not affect alpha; completely transparent black remains black.
   stored=struct.unpack('<140H',(work/(name+'.asis')).read_bytes())
   assert all(samples[i]==stored[i] for i in range(3,140,4))
   assert any(samples[i]!=stored[i] for i in range(140) if i%4!=3)
   for i in range(35):
    if samples[i*4+3]==0:assert samples[i*4:i*4+3]==(0,0,0)
   record=dict(name=name,color_channels=channels,bits=bits,orientation=orientation,embedded_icc=icc,coded=[7,5],output=[int(v) for v in result.stdout.split()],jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256(raw).hexdigest())
   if icc:
    so=C.create_string_buffer(profile.read_bytes());destination=(root/'wml2/src/srgb.icc').read_bytes();do=C.create_string_buffer(destination)
    sp=cms.cmsOpenProfileFromMem(so,len(so.raw)-1);dp=cms.cmsOpenProfileFromMem(do,len(destination));assert sp and dp
    tr=cms.cmsCreateTransform(sp,(4<<16)|(3<<3)|2,dp,(4<<16)|(3<<3)|2,1,0x100);assert tr
    rgb=[v for i,v in enumerate(samples) if i%4!=3];src=(C.c_uint16*len(rgb))(*rgb);out=(C.c_uint16*len(rgb))();cms.cmsDoTransform(tr,src,out,35)
    managed=bytes((out[i*3+c]*255+32767)//65535 if c<3 else (samples[i*4+3]*255+32767)//65535 for i in range(35) for c in range(4));(dest/(name+'.srgb.rgba')).write_bytes(managed)
    record['managed_sha256']=hashlib.sha256(managed).hexdigest();record['profile_sha256']=hashlib.sha256(profile.read_bytes()).hexdigest()
    cms.cmsDeleteTransform(tr);cms.cmsCloseProfile(sp);cms.cmsCloseProfile(dp)
   records.append(record)
# Independent colored zero-alpha stress: associated source coordinates need not
# be physically meaningful. Preserve the decoder's explicit normalization result.
for bits in [8,16]:
 name=f'jxl-associated-colored-zero-{bits}'
 source=work/f'jxl-associated-3c{bits}o6.source16le';path=dest/(name+'.jxl')
 run([work/'encoder',source,path,7,5,3,bits,bits,6,'associated-colored-zero','-'])
 run([work/'reference',path,dest/(name+'.rgba16le'),work/(name+'.icc'),'straight'])
 records.append(dict(name=name,bits=bits,kind='colored zero-alpha stress',jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256((dest/(name+'.rgba16le')).read_bytes()).hexdigest()))
path=dest/'jxl-associated-layers-refused.jxl'
run([work/'encoder',work/'jxl-associated-3c16o6.source16le',path,7,5,3,16,16,6,'associated-layers','-'])
run([work/'reference',path,work/'layers.rgba16le',work/'layers.icc','straight'])
records.append(dict(name=path.stem,kind='independently decodable multi-layer refusal',jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
path=dest/'jxl-premult.jxl'
run([work/'reference',path,dest/'jxl-associated-original-premult.rgba16le',work/'original-premult.icc','straight'])
records.append(dict(name='jxl-premult',kind='previously refused original matching-depth fixture',jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256((dest/'jxl-associated-original-premult.rgba16le').read_bytes()).hexdigest()))
(root/'docs/jpeg-xl-associated-alpha-oracle.json').write_text(json.dumps(dict(reference='C libjxl0.11.2 explicit unpremultiply; LittleCMS2.16 RGB16 relative NOOPTIMIZE, alpha copied',generator_sha256=hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),encoder_sha256=hashlib.sha256((root/'wml2-test/scripts/jpegxl_alpha_fixture_encoder.c').read_bytes()).hexdigest(),reference_sha256=hashlib.sha256((root/'wml2-test/scripts/jpegxl_alpha_reference.c').read_bytes()).hexdigest(),cases=records),indent=2)+'\n')
print(len(records),'C oracle cases generated')
