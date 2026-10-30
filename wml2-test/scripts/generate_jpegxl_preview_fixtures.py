#!/usr/bin/env python3
"""Assemble independently C-encoded frames into embedded-preview codestreams.
C libjxl decodes BOTH preview and main, then separately decodes main alone;
Rust/WML2 never supplies expected pixels. Header field locations below belong
to fingerprinted C libjxl0.11.2 source fixtures, not a generic JXL serializer.
"""
import argparse, ctypes as C, ctypes.util, hashlib, json, pathlib, struct, subprocess, zlib
p=argparse.ArgumentParser();p.add_argument('--include',type=pathlib.Path,required=True);p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('--work must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review';work=a.work.resolve();work.mkdir(parents=True,exist_ok=True)
def run(cmd):return subprocess.check_output([str(v) for v in cmd],text=True).strip()
encoder=work/'encoder';reference=work/'preview-reference'
for src,out in [('jpegxl_fixture_encoder.c',encoder),('jpegxl_preview_reference.c',reference)]:run(['cc','-O2','-I'+str(a.include),root/'wml2-test/scripts'/src,'-Wl,-l:libjxl.so.0.11','-o',out])
version=run([reference,'--version']);assert version=='runtime=11002 header=11002',version

def digest(data):return hashlib.sha256(data).hexdigest()
def raw(data):
 if data[:2]==b'\xff\x0a':return data
 assert data[:12]==b'\0\0\0\x0cJXL \r\n\x87\n';out=b'';at=12
 while at<len(data):
  n=int.from_bytes(data[at:at+4],'big');kind=data[at+4:at+8];payload=data[at+8:at+n];assert n>=8
  if kind==b'jxlc':out+=payload
  elif kind==b'jxlp':out+=payload[4:]
  at+=n
 assert at==len(data) and out[:2]==b'\xff\x0a';return out
def bits(data):return [(v>>j)&1 for v in data for j in range(8)]
def pack(values):
 values=values+[0]*((-len(values))%8)
 return bytes(sum(v<<j for j,v in enumerate(values[i:i+8])) for i in range(0,len(values),8))
def little(value,n):return [(value>>i)&1 for i in range(n)]
def preview_dimension(size):
 value=size-1
 for selector,(base,n) in enumerate([(0,6),(64,8),(320,10),(1344,12)]):
  if base<=value<base+(1<<n):return little(selector,2)+little(value-base,n)
 raise ValueError(size)
def splice(main,layout,preview,preview_byte,w,h,orientation=1):
 start,end,header_end,frames_bit=layout;a=bits(raw(main));preview=raw(preview)
 assert a[start:start+2]==[0,0] and a[end-2:end]==[0,0]  # nondefault metadata, extra_fields=false, no extensions
 size=[0]+preview_dimension(h)+[0,0,0]+preview_dimension(w)  # !div8, unknown aspect
 values=a[:start]+[0,1]+little(orientation-1,3)+[0,1]+size+[0]
 values+=a[start+2:end-2]+[1]+a[end-2:header_end]+a[header_end:frames_bit] # default tone mapping, original ICC bits
 header=pack(values);body=preview[preview_byte:];result=header+body+raw(main)[(frames_bit+7)//8:]
 return result,{'preview_start':len(header),'main_start':len(header)+len(body),'preview_dimensions_bit':start+7,'preview_dimensions_bits':len(size),'orientation_bit':start+2}
def encode(name,w,h,channels,bits_,mode):
 values=[v for y in range(h) for x in range(w) for v in ([int((x+y)*255/max(w+h-2,1)),[255,80,0,1,220,145][(x+y)%6]] if channels==2 else [x*251//max(w-1,1),y*241//max(h-1,1),220 if (x//4+y//4)%2 else 20,[255,80,0,1,220,145][(x+y)%6]])]
 if bits_==16:values=[v*257 for v in values]
 source=bytes(values) if bits_==8 else struct.pack('<'+'H'*len(values),*values)
 source_file=work/(name+'.source');source_file.write_bytes(source);out=work/(name+'.jxl');run([encoder,source_file,out,w,h,channels,bits_,mode,'-']);return out.read_bytes(),source
preview16,source16=encode('preview16',2,3,4,16,'lossless')
gray,gray_source=encode('preview-gray',2,3,2,8,'lossless')
large,large_source=encode('preview-large',32,32,4,8,'lossless')
display,display_source=encode('display-main',64,48,4,8,'lossy')
lossy,lossy_source=encode('preview-lossy',8,8,4,8,'lossy')
rgba=(dest/'jxl-rgba8.jxl').read_bytes();small=(dest/'jxl-orientation1.jxl').read_bytes();icc=(dest/'jxl-icc-lossless.jxl').read_bytes();gray_main=(dest/'jxl-grayalpha8.jxl').read_bytes()
# The normalized C output profile feeds independent LittleCMS for sRGB oracle.
cms=C.CDLL(ctypes.util.find_library('lcms2'))
cms.cmsOpenProfileFromMem.argtypes=[C.c_void_p,C.c_uint32];cms.cmsOpenProfileFromMem.restype=C.c_void_p
cms.cmsCreateTransform.argtypes=[C.c_void_p,C.c_uint32,C.c_void_p,C.c_uint32,C.c_uint32,C.c_uint32];cms.cmsCreateTransform.restype=C.c_void_p
cms.cmsDoTransform.argtypes=[C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32];cms.cmsDeleteTransform.argtypes=[C.c_void_p];cms.cmsCloseProfile.argtypes=[C.c_void_p]
cms.cmsGetEncodedCMMversion.restype=C.c_uint32
cms.cmsGetColorSpace.argtypes=[C.c_void_p];cms.cmsGetColorSpace.restype=C.c_uint32
cms.cmsChannelsOf.argtypes=[C.c_uint32];cms.cmsChannelsOf.restype=C.c_uint32
def srgb(pixels,profile):
 source=C.create_string_buffer(profile);destination_bytes=(root/'wml2/src/srgb.icc').read_bytes();destination=C.create_string_buffer(destination_bytes)
 sp=cms.cmsOpenProfileFromMem(source,len(profile));dp=cms.cmsOpenProfileFromMem(destination,len(destination_bytes));assert sp and dp
 values=struct.unpack('<'+'H'*(len(pixels)//2),pixels);count=len(values)//4
 channels=cms.cmsChannelsOf(cms.cmsGetColorSpace(sp));assert channels in [1,3]
 samples=(C.c_uint16*(count*channels))(*(values[i*4+c] for i in range(count) for c in range(channels)));output=(C.c_uint16*(count*3))()
 transform=cms.cmsCreateTransform(sp,((3 if channels==1 else 4)<<16)|(channels<<3)|2,dp,(4<<16)|(3<<3)|2,1,0);assert transform
 cms.cmsDoTransform(transform,samples,output,count);cms.cmsDeleteTransform(transform);cms.cmsCloseProfile(sp);cms.cmsCloseProfile(dp)
 return bytes((value*255+32767)//65535 for i in range(count) for value in [output[i*3],output[i*3+1],output[i*3+2],values[i*4+3]])
cases=[('rgba8',rgba,(31,44,45,45),small,7,2,3,1),('icc16',icc,(31,69,70,1940),preview16,10,2,3,1),('oriented-icc16',icc,(31,69,70,1940),preview16,10,2,3,6),('grayalpha8',gray_main,(31,58,59,59),gray,9,2,3,1),('larger',small,(42,55,56,56),large,5,32,32,1),('lossy',display,(25,38,39,39),lossy,5,8,8,1)]
manifest={'oracle':version+' preview + main normalized RGBA16 and output ICC; main-only decode agreement; independent LittleCMS sRGB before one final8bit rounding','lcms_encoded_version':cms.cmsGetEncodedCMMversion(),'construction':'Bit splice of C-encoded standalone frame payloads; checked header fields. Both preview/main decoded by C libjxl; no WML2/Rust expected pixels. Shared global depth/profile/alpha/orientation, different preview samples and dimensions.','source_fingerprints':{name:digest(data) for name,data in [('jxl-rgba8.jxl',rgba),('jxl-orientation1.jxl',small),('jxl-icc-lossless.jxl',icc),('jxl-grayalpha8.jxl',gray_main)]},'cases':[]}
for name,main,layout,preview,preview_byte,w,h,orientation in cases:
 data,positions=splice(main,layout,preview,preview_byte,w,h,orientation);stem='jxl-preview-'+name;path=dest/(stem+'.jxl');path.write_bytes(data)
 main_out=work/(stem+'.rgba16');profile_out=work/(stem+'.icc');preview_out=work/(stem+'.preview.rgba16')
 fields=[int(v) for v in run([reference,path,main_out,profile_out,preview_out]).split()];assert fields[2]==1
 pixels=main_out.read_bytes();preview_pixels=preview_out.read_bytes();assert len(pixels)==fields[0]*fields[1]*8 and len(preview_pixels)==w*h*8
 # C main-only skip must agree byte-for-byte with decoding preview too.
 run([reference,path,work/'skip-main.rgba16',work/'skip-profile.icc','-']);assert (work/'skip-main.rgba16').read_bytes()==pixels
 native=struct.unpack('<'+'H'*(len(pixels)//2),pixels);rgba8=bytes((v*255+32767)//65535 for v in native)
 (dest/(stem+'.rgba.zlib')).write_bytes(zlib.compress(rgba8,9));(dest/(stem+'.rgba16le.zlib')).write_bytes(zlib.compress(pixels,9))
 managed=srgb(pixels,profile_out.read_bytes());(dest/(stem+'.srgb.rgba.zlib')).write_bytes(zlib.compress(managed,9))
 assert pixels[:min(len(pixels),len(preview_pixels))]!=preview_pixels[:min(len(pixels),len(preview_pixels))]
 if name=='rgba8':assert rgba8==(dest/'jxl-rgba8-djxl.bin').read_bytes()
 if name=='icc16':assert pixels==(dest/'jxl-icc-lossless-djxl.bin').read_bytes()
 if name=='lossy':
  # Keep equivalent no-preview C source for performance/display comparison.
  (dest/'jxl-preview-lossy-main-only.jxl').write_bytes(display)
  run([reference,dest/'jxl-preview-lossy-main-only.jxl',work/'plain-main.rgba16',work/'plain-profile.icc','-']);assert (work/'plain-main.rgba16').read_bytes()==pixels
 manifest['cases'].append(dict(file=path.name,bytes=len(data),sha256=digest(data),output_dimensions=fields[:2],coded_preview_dimensions=[w,h],reported_preview_dimensions=fields[3:5],coded_bits=fields[5],alpha_bits=fields[6],orientation=orientation,rgba_sha256=digest(rgba8),rgba16_sha256=digest(pixels),srgb_sha256=digest(managed),preview_rgba16_sha256=digest(preview_pixels),**positions))
# A main-only reader deliberately does not validate skipped entropy. Confirm
# that this byte mutation is rejected when independently requesting the preview.
case=manifest['cases'][0];damaged=bytearray((dest/case['file']).read_bytes());damaged[case['preview_start']+6:case['main_start']]=b'\xff'*(case['main_start']-case['preview_start']-6)
damaged_path=work/'damaged-preview.jxl';damaged_path.write_bytes(damaged)
run([reference,damaged_path,work/'damaged-main.rgba16',work/'damaged-profile.icc','-'])
assert (work/'damaged-main.rgba16').read_bytes()==(work/'jxl-preview-rgba8.rgba16').read_bytes()
probe=subprocess.run([str(v) for v in [reference,damaged_path,work/'bad-main.rgba16',work/'bad-profile.icc',work/'bad-preview.rgba16']],capture_output=True,text=True)
assert probe.returncode!=0
manifest['skipped_entropy']={'mutation':'Fill RGBA8 preview section bytes with ff, retaining header/TOC/extents','main_only_c_decode':'exact unchanged main RGBA16','c_preview_request_rejected':True,'c_preview_error':probe.stderr[-500:]}
# Syntactically valid oversized preview declaration; deliberately not a full
# oversized-pixel oracle. Admission must reject it before TOC/pixel work.
data,positions=splice(rgba,(31,44,45,45),small,7,4096,4096)
(dest/'jxl-preview-oversized-header.jxl').write_bytes(data)
manifest['oversized_header']={'file':'jxl-preview-oversized-header.jxl','sha256':digest(data),'declared_preview_dimensions':[4096,4096],'preview_start':positions['preview_start'],'scope':'Header-admission negative fixture; body retains small frame geometry and is not claimed as a valid oversized preview encoding.'}
(dest/'jxl-preview-fixtures.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(manifest,indent=2))
