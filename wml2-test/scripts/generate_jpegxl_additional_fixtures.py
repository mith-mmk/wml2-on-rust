#!/usr/bin/env python3
"""Independent orientation/depth and display-CMS references; libjxl/djxl only."""
import argparse,hashlib,json,pathlib,struct,subprocess,zlib
import ctypes as C
import ctypes.util
p=argparse.ArgumentParser();p.add_argument('--tools',type=pathlib.Path,required=True);p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('--work must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review';work=a.work.resolve();work.mkdir(parents=True,exist_ok=True)
def run(args):
 r=subprocess.run([str(v) for v in args],capture_output=True)
 if r.returncode:raise RuntimeError(r.stderr.decode(errors='replace'))
def digest(data):return hashlib.sha256(data).hexdigest()
cms=C.CDLL(ctypes.util.find_library('lcms2'))
cms.cmsOpenProfileFromMem.argtypes=[C.c_void_p,C.c_uint32];cms.cmsOpenProfileFromMem.restype=C.c_void_p
cms.cmsGetColorSpace.argtypes=[C.c_void_p];cms.cmsGetColorSpace.restype=C.c_uint32
cms.cmsChannelsOf.argtypes=[C.c_uint32];cms.cmsChannelsOf.restype=C.c_uint32
cms.cmsCreateTransform.argtypes=[C.c_void_p,C.c_uint32,C.c_void_p,C.c_uint32,C.c_uint32,C.c_uint32];cms.cmsCreateTransform.restype=C.c_void_p
cms.cmsDoTransform.argtypes=[C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32]
cms.cmsDeleteTransform.argtypes=[C.c_void_p];cms.cmsCloseProfile.argtypes=[C.c_void_p]
cms.cmsGetEncodedCMMversion.restype=C.c_uint32
encoder=work/'additional-encoder';run(['cc','-O2','-I'+str(a.tools/'include'),root/'wml2-test/scripts/jpegxl_fixture_encoder.c','-Wl,-l:libjxl.so.0.11','-o',encoder])
decoder=work/'color-reference';run(['cc','-O2','-I'+str(a.tools/'include'),root/'wml2-test/scripts/jpegxl_color_reference.c','-Wl,-l:libjxl.so.0.11','-o',decoder])
version=subprocess.check_output([str(decoder),'--version'],text=True).strip()
if version!='runtime=11002 header=11002':raise RuntimeError('reference tools require matching libjxl0.11.2: '+version)
def oracle(source,bits,managed=False):
 out=work/(source.stem+('-managed' if managed else '')+'.pam')
 profile_path=work/(source.stem+'-output.icc')
 cmd=[a.tools/'bin/djxl',source,out,'--bits_per_sample='+str(bits),'--icc_out='+str(profile_path),'--quiet']
 if managed:cmd.append('--color_space=RGB_D65_SRG_Rel_SRG')
 run(cmd);header,data=out.read_bytes().split(b'ENDHDR\n',1);fields=dict(line.split(maxsplit=1) for line in header.splitlines()[1:] if line and not line.startswith(b'#'))
 w,h,c=(int(fields[k]) for k in [b'WIDTH',b'HEIGHT',b'DEPTH']);source_max=int(fields[b'MAXVAL']);vals=list(data) if source_max<=255 else list(struct.unpack('>'+'H'*(len(data)//2),data));assert len(vals)==w*h*c
 # PAM may retain the coded maxval even when a container-depth request is made.
 # Normalize the independently decoded sample domain explicitly.
 m=(1<<bits)-1;vals=[(v*m+source_max//2)//source_max for v in vals]
 rgba=[];m=(1<<bits)-1
 for off in range(0,len(vals),c):
  v=vals[off:off+c];rgba.extend([v[0],v[0],v[0],v[1] if c==2 else m] if c<=2 else v+[m] if c==3 else v)
 if managed:
  assert bits==16
  raw_path=work/(source.stem+'.libjxl16')
  decoded=subprocess.check_output([str(decoder),str(source),str(raw_path),str(profile_path)],text=True).split();assert [int(v) for v in decoded]==[w,h]
  raw=raw_path.read_bytes();rgba=list(struct.unpack('<'+'H'*(len(raw)//2),raw));assert len(rgba)==w*h*4
  profile=profile_path.read_bytes();destination=(root/'wml2/src/srgb.icc').read_bytes();sp=C.create_string_buffer(profile);dp=C.create_string_buffer(destination)
  source_profile=cms.cmsOpenProfileFromMem(sp,len(profile));destination_profile=cms.cmsOpenProfileFromMem(dp,len(destination));assert source_profile and destination_profile
  channels=cms.cmsChannelsOf(cms.cmsGetColorSpace(source_profile));assert channels in [1,3]
  samples=(C.c_uint16*(w*h*channels))(*(rgba[i*4+j] for i in range(w*h) for j in range(channels)));target=(C.c_uint16*(w*h*3))()
  transform=cms.cmsCreateTransform(source_profile,((3 if channels==1 else 4)<<16)|(channels<<3)|2,destination_profile,(4<<16)|(3<<3)|2,1,0x100);assert transform
  cms.cmsDoTransform(transform,samples,target,w*h)
  for i in range(w*h):rgba[i*4:i*4+3]=target[i*3:i*3+3]
  cms.cmsDeleteTransform(transform);cms.cmsCloseProfile(source_profile);cms.cmsCloseProfile(destination_profile)
 return w,h,bytes(rgba) if bits==8 else struct.pack('<'+'H'*len(rgba),*rgba)
manifest={'encoder':'libjxl0.11.2 C API,lossless,effort3,keep invisible','oracle':'djxl0.11.2 PAM normalized by MAXVAL; managed libjxl C RGBA16 and actual output ICC,then LittleCMS16 to fixed sRGB,relative intent,NOOPTIMIZE,final RGBA8 rounding','lcms_encoded_version':cms.cmsGetEncodedCMMversion(),'libjxl_version':version,'orientations':[],'depths':[],'managed':[]}
source=work/'orientation.raw';source.write_bytes(bytes([17,34,51,255,200,10,30,127,9,230,77,0,32,64,96,1,222,111,8,254,250,240,16,255]))
for orientation in range(1,9):
 name=f'jxl-orientation{orientation}';output=dest/(name+'.jxl');run([encoder,source,output,2,3,4,8,f'orientation{orientation}','-']);w,h,rgba=oracle(output,8);(dest/(name+'.rgba')).write_bytes(rgba)
 manifest['orientations'].append({'file':output.name,'expected':name+'.rgba','orientation':orientation,'size':[w,h],'sha256':digest(output.read_bytes()),'rgba_sha256':digest(rgba)})
for bits in [9,10]:
 m=(1<<bits)-1;values=[v for i in range(35) for v in [(i*37+1)%(m+1),(i*71+17)%(m+1),(i*97+31)%(m+1),[0,1,m//2,m-1,m][i%5]]];source=work/f'rgba{bits}.raw';source.write_bytes(struct.pack('<'+'H'*len(values),*values));name=f'jxl-rgba{bits}';output=dest/(name+'.jxl');run([encoder,source,output,7,5,4,bits,'lossless','-']);w,h,rgba16=oracle(output,16);(dest/(name+'.rgba16le')).write_bytes(rgba16);(dest/(name+'.source16le')).write_bytes(source.read_bytes());manifest['depths'].append({'file':output.name,'coded_bits':bits,'output_bits':16,'size':[w,h],'sha256':digest(output.read_bytes()),'rgba16_sha256':digest(rgba16)})
for file in ['jxl-icc-gray16.jxl','jxl-icc-lossless.jxl','jxl-icc-lossy.jxl','jxl-jbrd-bench.jxl','jxl-jbrd-gray.jxl']:
 w,h,rgba16=oracle(dest/file,16,True);values=struct.unpack('<'+'H'*(len(rgba16)//2),rgba16);rgba=bytes((v*255+32767)//65535 for v in values);name=pathlib.Path(file).stem+'.srgb.rgba';compressed=file.startswith('jxl-jbrd-');name+=('.zlib' if compressed else '');(dest/name).write_bytes(zlib.compress(rgba,9) if compressed else rgba);manifest['managed'].append({'file':file,'expected':name,'compression':'zlib' if compressed else None,'size':[w,h],'sha256':digest((dest/file).read_bytes()),'rgba_sha256':digest(rgba)})
(dest/'jxl-additional-fixtures.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(manifest,indent=2))
