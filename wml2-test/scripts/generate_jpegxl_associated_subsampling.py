#!/usr/bin/env python3
"""Generate associated subsampled-alpha oracles with independent C libjxl and LittleCMS.
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
pairs=[(8,8),(9,16),(10,9),(12,10),(16,12),(16,16),(16,8)]
for channels in [1,3]:
 for shift in [1,2,3]:
  for index,(bits,alpha_bits) in enumerate(pairs):
   orientation=(index+shift+(3 if channels==3 else 0))%8+1
   icc=channels==3 and ((bits,alpha_bits,shift) in [(9,16,1),(16,8,3)])
   w,h=33,17;name=f'jxl-associated-subsampled-{channels}c{bits}a{alpha_bits}s{shift}o{orientation}';values=[]
   for y in range(h):
    for x in range(w):
     maxcode=(1<<alpha_bits)-1
     alpha=0 if x<9 else maxcode if x>24 else [1,maxcode//2,maxcode-1][(x+y)%3]
     color=[(x*(7919+193*c)+y*413+12345*(c+1))%65536 for c in range(channels)]
     values+=color+[(alpha*65535+maxcode//2)//maxcode]
   source=work/(name+'.source16le');source.write_bytes(struct.pack('<'+'H'*len(values),*values));path=dest/(name+'.jxl')
   run([work/'encoder',source,path,w,h,channels,bits,alpha_bits,orientation,f'associated-subsampled-{shift}',dest/'jxl-gamma2.icc' if icc else '-'])
   straight=dest/(name+'.rgba16le');profile=work/(name+'.icc');result=run([work/'reference',path,straight,profile,'straight'])
   assert f'image {bits} {alpha_bits} {channels} 1' in result.stderr and f'extra 0 0 {alpha_bits} 0 {shift} 1' in result.stderr
   assert [int(v) for v in result.stdout.split()]==([h,w] if orientation>=5 else [w,h])
   raw=straight.read_bytes();samples=struct.unpack('<'+'H'*(w*h*4),raw)
   record=dict(name=name,color_channels=channels,color_bits=bits,alpha_bits=alpha_bits,dim_shift=shift,declared_alpha=[(w+(1<<shift)-1)>>shift,(h+(1<<shift)-1)>>shift],orientation=orientation,embedded_icc=icc,coded=[w,h],output=[int(v) for v in result.stdout.split()],jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256(raw).hexdigest())
   if icc:
    so=C.create_string_buffer(profile.read_bytes());destination=(root/'wml2/src/srgb.icc').read_bytes();do=C.create_string_buffer(destination)
    sp=cms.cmsOpenProfileFromMem(so,len(so.raw)-1);dp=cms.cmsOpenProfileFromMem(do,len(destination));assert sp and dp
    tr=cms.cmsCreateTransform(sp,(4<<16)|(3<<3)|2,dp,(4<<16)|(3<<3)|2,1,0x100);assert tr
    rgb=[v for i,v in enumerate(samples) if i%4!=3];src=(C.c_uint16*len(rgb))(*rgb);out=(C.c_uint16*len(rgb))();cms.cmsDoTransform(tr,src,out,w*h)
    managed=bytes((out[i*3+c]*255+32767)//65535 if c<3 else (samples[i*4+3]*255+32767)//65535 for i in range(w*h) for c in range(4));(dest/(name+'.srgb.rgba')).write_bytes(managed)
    record['managed_sha256']=hashlib.sha256(managed).hexdigest();record['profile_sha256']=hashlib.sha256(profile.read_bytes()).hexdigest()
    cms.cmsDeleteTransform(tr);cms.cmsCloseProfile(sp);cms.cmsCloseProfile(dp)
   records.append(record)
# Tiny images become one alpha sample at8x; zero and opaque must remain exact.
for shift in [1,2,3]:
 for alpha in [0,65535]:
  name=f'jxl-associated-subsampled-flat-s{shift}a{alpha}';path=dest/(name+'.jxl');source=work/(name+'.source16le')
  source.write_bytes(struct.pack('<140H',*([12345,23456,34567,alpha]*35)))
  run([work/'encoder',source,path,7,5,3,16,16,6,f'associated-subsampled-{shift}','-'])
  run([work/'reference',path,dest/(name+'.rgba16le'),work/(name+'.icc'),'straight'])
  values=struct.unpack('<140H',(dest/(name+'.rgba16le')).read_bytes());assert all(values[i]==alpha for i in range(3,140,4))
  records.append(dict(name=name,color_channels=3,color_bits=16,alpha_bits=16,dim_shift=shift,coded=[7,5],orientation=6,output=[5,7],flat_alpha=alpha,jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256((dest/(name+'.rgba16le')).read_bytes()).hexdigest()))
path=dest/'jxl-subsampled-refused-associated-subsampled-1.jxl'
run([work/'reference',path,dest/'jxl-associated-subsampled-original.rgba16le',work/'original.icc','straight'])
records.append(dict(name=path.stem,color_bits=16,alpha_bits=8,dim_shift=1,coded=[33,17],orientation=6,kind='historical associated subsampled refusal now positive',output=[17,33],jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256((dest/'jxl-associated-subsampled-original.rgba16le').read_bytes()).hexdigest()))
path=dest/'jxl-associated-subsampled-layers-refused.jxl'
run([work/'encoder',work/'jxl-associated-subsampled-3c16a8s1o3.source16le',path,33,17,3,16,8,6,'associated-subsampled-layers','-'])
run([work/'reference',path,work/'layers.rgba16le',work/'layers.icc','straight'])
records.append(dict(name=path.stem,refused=True,kind='independently decodable multi-layer association+subsampling refusal',jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
assert {r['orientation'] for r in records if not r.get('refused')}==set(range(1,9))
(root/'docs/jpeg-xl-associated-subsampling-oracle.json').write_text(json.dumps(dict(reference='C libjxl0.11.2 explicit unpremultiply after alpha resampling; LittleCMS2.16 RGB16 relative NOOPTIMIZE, alpha copied',generator_sha256=hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),encoder_sha256=hashlib.sha256((root/'wml2-test/scripts/jpegxl_alpha_fixture_encoder.c').read_bytes()).hexdigest(),reference_sha256=hashlib.sha256((root/'wml2-test/scripts/jpegxl_alpha_reference.c').read_bytes()).hexdigest(),cases=records),indent=2)+'\n')
lines=[]
for r in records:
 if r.get('refused'):continue
 name=r['name'];oracle='jxl-associated-subsampled-original' if r.get('kind') else name
 lines.append(f'(include_bytes!("../fixtures/review/{name}.jxl").as_slice(),include_bytes!("../fixtures/review/{oracle}.rgba16le").as_slice(),{r['color_bits']}u8,{r['alpha_bits']}u8,{r['dim_shift']}u8,{r['coded'][0]}usize,{r['coded'][1]}usize,{r['orientation']}u8),')
(root/'wml2/tests/jpeg_xl/associated_subsampling_cases.rs').write_text('// Generated independent C associated alpha-resampling oracle.\n[\n'+'\n'.join(lines)+'\n]\n')
print(len(records),'C oracle cases generated')
