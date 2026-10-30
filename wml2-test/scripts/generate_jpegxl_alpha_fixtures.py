#!/usr/bin/env python3
"""Independent integer-alpha qualification; requires C libjxl0.11.2 and LittleCMS2.16."""
from pathlib import Path
import struct,subprocess,json,hashlib,argparse,ctypes as C,ctypes.util
p=argparse.ArgumentParser();p.add_argument('--include',type=Path,required=True);p.add_argument('--work',type=Path,required=True);args=p.parse_args()
if not args.work.name.startswith('.test'):p.error('--work must start with .test')
root=Path(__file__).resolve().parents[2];out=args.work.resolve();out.mkdir(parents=True,exist_ok=True);dest=root/'wml2/tests/fixtures/review';manifest={'oracle':'independent C libjxl0.11.2 normalized RGBA16 / source samples; original alpha association preserved','cases':[]}
def run(cmd):
 result=subprocess.run([str(x) for x in cmd],text=True,capture_output=True,check=True)
 if str(cmd[0])==str(out/'reference') and len(cmd)>2:
  lines=result.stderr.splitlines();image=[int(v) for v in lines[0].split()[1:]];extras=[]
  for line in lines[1:]:
   fields=[int(v) for v in line.split()[1:]];extras.append(dict(zip(['index','type','bits','exponent_bits','dim_shift','associated'],fields)))
  manifest.setdefault('c_headers',{})[Path(cmd[1]).name]={'color_bits':image[0],'alpha_bits':image[1],'color_channels':image[2],'output_orientation':image[3],'extras':extras}
 return result.stdout.strip()
def norm(v,bits):return (v*65535+((1<<bits)-1)//2)//((1<<bits)-1)
for src,name in [('jpegxl_alpha_fixture_encoder.c','encoder'),('jpegxl_alpha_reference.c','reference')]:
 run(['cc','-Wall','-Wextra','-Werror','-O2','-I'+str(args.include),root/'wml2-test/scripts'/src,'-Wl,-l:libjxl.so.0.11','-o',out/name])
assert run([out/'reference','--version'])=='runtime=11002 header=11002'
def orient(values,channels,orientation):
 result=[0]*len(values);w,h=7,5;ow=h if orientation>=5 else w
 for y in range(h):
  for x in range(w):
   dx,dy=[(x,y),(w-1-x,y),(w-1-x,h-1-y),(x,h-1-y),(y,x),(h-1-y,x),(h-1-y,w-1-x),(y,w-1-x)][orientation-1]
   result[(dy*ow+dx)*channels:(dy*ow+dx+1)*channels]=values[(y*w+x)*channels:(y*w+x+1)*channels]
 return result
for channels in [1,3]:
 for bi,bits in enumerate([8,9,10,12,16]):
  for ai,alpha in enumerate([8,9,10,12,16]):
   orientation=(bi*5+ai)%8+1;w,h=7,5;values=[];codes=[]
   for i in range(w*h):
    p=[(i*(97+31*c)+13+23*c)%((1<<bits)-1) for c in range(channels)]
    av=[0,1,((1<<alpha)-1)//2,((1<<alpha)-1)-1,(1<<alpha)-1][i%5]
    codes+=p+[av];values += [norm(v,bits) for v in p]+[norm(av,alpha)]
   name=f'jxl-alpha-{channels}c{bits}a{alpha}o{orientation}';source=out/(name+'.source16le');source.write_bytes(struct.pack('<'+'H'*len(values),*values));path=dest/(name+'.jxl')
   run([out/'encoder',source,path,w,h,channels,bits,alpha,orientation,'straight','-']);dims=[int(v) for v in run([out/'reference',path,dest/(name+'.rgba16le'),out/(name+'.icc')]).split()]
   header=manifest['c_headers'][path.name];assert header['color_bits']==bits and header['alpha_bits']==alpha and header['extras']==[dict(index=0,type=0,bits=alpha,exponent_bits=0,dim_shift=0,associated=0)]
   oracle=struct.unpack('<'+'H'*(w*h*4),(dest/(name+'.rgba16le')).read_bytes());original=orient(codes,channels+1,orientation)
   for i,pixel in enumerate(zip(*[iter(original)]*(channels+1))):
    for c in range(4):
     depth=alpha if c==3 else bits;code=pixel[-1] if c==3 else pixel[c if channels==3 else 0];actual=oracle[i*4+c]
     assert (actual*((1<<depth)-1)+32767)//65535==code,(name,i,c,actual,code)
   manifest['cases'].append(dict(file=path.name,color_channels=channels,color_bits=bits,alpha_bits=alpha,orientation=orientation,coded=[w,h],output=dims,sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256((dest/(name+'.rgba16le')).read_bytes()).hexdigest()))
# RGB ICC cases use distinct depths and all alpha extrema. The independent
# decoder's actual output ICC, never a WML2 profile, feeds LittleCMS below.
for bits,alpha,orientation in [(8,16,6),(16,8,8)]:
 source=next(out.glob(f'jxl-alpha-3c{bits}a{alpha}*.source16le'));name=f'jxl-alpha-icc-c{bits}a{alpha}o{orientation}';path=dest/(name+'.jxl')
 run([out/'encoder',source,path,7,5,3,bits,alpha,orientation,'straight',dest/'jxl-gamma2.icc']);run([out/'reference',path,dest/(name+'.rgba16le'),out/(name+'.icc')])
 cms=C.CDLL(ctypes.util.find_library('lcms2'));cms.cmsOpenProfileFromMem.argtypes=[C.c_void_p,C.c_uint32];cms.cmsOpenProfileFromMem.restype=C.c_void_p;cms.cmsCreateTransform.argtypes=[C.c_void_p,C.c_uint32,C.c_void_p,C.c_uint32,C.c_uint32,C.c_uint32];cms.cmsCreateTransform.restype=C.c_void_p;cms.cmsDoTransform.argtypes=[C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32];cms.cmsDeleteTransform.argtypes=[C.c_void_p];cms.cmsCloseProfile.argtypes=[C.c_void_p];cms.cmsGetEncodedCMMversion.restype=C.c_uint32;assert cms.cmsGetEncodedCMMversion()==2160
 profile=(out/(name+'.icc')).read_bytes();destination=(root/'wml2/src/srgb.icc').read_bytes();so=C.create_string_buffer(profile);do=C.create_string_buffer(destination);sp=cms.cmsOpenProfileFromMem(so,len(profile));dp=cms.cmsOpenProfileFromMem(do,len(destination));assert sp and dp
 tr=cms.cmsCreateTransform(sp,(4<<16)|(3<<3)|2,dp,(4<<16)|(3<<3)|2,1,0x100);assert tr
 raw=struct.unpack('<'+'H'*(35*4),(dest/(name+'.rgba16le')).read_bytes());rgb=[v for i,v in enumerate(raw) if i%4!=3];src=(C.c_uint16*len(rgb))(*rgb);dst=(C.c_uint16*len(rgb))();cms.cmsDoTransform(tr,src,dst,35);managed=bytes((dst[i*3+c]*255+32767)//65535 if c<3 else (raw[i*4+3]*255+32767)//65535 for i in range(35) for c in range(4));(dest/(name+'.srgb.rgba')).write_bytes(managed);cms.cmsDeleteTransform(tr);cms.cmsCloseProfile(sp);cms.cmsCloseProfile(dp)
 manifest.setdefault('icc',[]).append(dict(file=path.name,color_bits=bits,alpha_bits=alpha,orientation=orientation,rgba16_sha256=hashlib.sha256((dest/(name+'.rgba16le')).read_bytes()).hexdigest(),srgb_sha256=hashlib.sha256(managed).hexdigest(),oracle='C libjxl0.11.2 output ICC -> LittleCMS2.16 RGB16 relative/NOOPTIMIZE; alpha copied; final8-bit rounding'))
source=next(out.glob('jxl-alpha-3c16a8*.source16le'))
for mode,alpha in [('associated',8),('subsampled',8),('depth',8),('spot',8),('multiple',8),('multiple-alpha',8),('float-alpha',16),('straight',4)]:
 name='jxl-alpha-refused-'+('alpha4' if mode=='straight' else mode);path=dest/(name+'.jxl');run([out/'encoder',source,path,7,5,3,16,alpha,6,mode,'-']);run([out/'reference',path,out/(name+'.rgba16le'),out/(name+'.icc')]);manifest.setdefault('refused',[]).append(dict(file=path.name,kind=mode,alpha_bits=alpha,c_decodes=True,sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256((out/(name+'.rgba16le')).read_bytes()).hexdigest()))
# Explicitly retain both associated and unassociated C oracles for evaluation.
path=dest/'jxl-alpha-refused-associated.jxl';run([out/'reference',path,dest/'jxl-alpha-associated.asis.rgba16le',out/'associated.icc']);run([out/'reference',path,dest/'jxl-alpha-associated.straight.rgba16le',out/'associated-straight.icc','straight'])
# The old independently generated mixed-depth fixture remains accepted too.
run([out/'reference',dest/'jxl-mixed-alpha.jxl',dest/'jxl-alpha-original-mixed.rgba16le',out/'original.icc'])
case_lines=[]
for c in manifest['cases']:
 name=c['file'][:-4]
 case_lines.append('    (include_bytes!("../fixtures/review/'+name+'.jxl").as_slice(), include_bytes!("../fixtures/review/'+name+'.rgba16le").as_slice(), '+str(c['color_bits'])+', '+str(c['alpha_bits'])+', '+str(c['orientation'])+'),')
(root/'wml2/tests/jpeg_xl/alpha_cases.rs').write_text('// Generated independent C qualification matrix. See generate_jpegxl_alpha_fixtures.py.\n[\n'+'\n'.join(case_lines)+'\n]\n')
(dest/'jxl-alpha-fixtures.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('C encoded/decoded',len(manifest['cases']),'cases')
