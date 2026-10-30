#!/usr/bin/env python3
"""C libjxl0.11.2 metadata/pixel oracles for intrinsic dimensions.
The display-axis normalization is an explicit WML2 contract: C BasicInfo keeps
stored intrinsic axes even when it orients the decoded pixel canvas.
"""
import argparse,ctypes as C,ctypes.util,hashlib,json,pathlib,struct,subprocess,zlib
p=argparse.ArgumentParser();p.add_argument('--include',type=pathlib.Path,required=True);p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('--work must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review';work=a.work.resolve();work.mkdir(parents=True,exist_ok=True)
def run(cmd):return subprocess.check_output([str(v) for v in cmd],text=True).strip()
encoder=work/'encoder';pixels=work/'pixels';reference=work/'dimensions'
for src,out in [('jpegxl_fixture_encoder.c',encoder),('jpegxl_color_reference.c',pixels),('jpegxl_intrinsic_reference.c',reference)]:run(['cc','-O2','-I'+str(a.include),root/'wml2-test/scripts'/src,'-Wl,-l:libjxl.so.0.11','-o',out])
assert run([pixels,'--version'])=='runtime=11002 header=11002'
source=bytes([17,34,51,255,200,10,30,127,9,230,77,0,32,64,96,1,222,111,8,254,250,240,16,255]);(work/'source.raw').write_bytes(source)
def digest(data):return hashlib.sha256(data).hexdigest()
def raw(data):
 if data[:2]==b'\xff\x0a':return data
 out=b'';at=12
 while at<len(data):
  n=int.from_bytes(data[at:at+4],'big');kind=data[at+4:at+8];payload=data[at+8:at+n];assert n>=8
  if kind==b'jxlc':out+=payload
  elif kind==b'jxlp':out+=payload[4:]
  at+=n
 assert at==len(data);return out
class Reader:
 def __init__(self,data):self.data=data;self.at=0
 def read(self,n):
  assert self.at+n<=len(self.data)*8
  value=sum(((self.data[(self.at+i)//8]>>((self.at+i)%8))&1)<<i for i in range(n));self.at+=n;return value
 def size(self):
  small=self.read(1);y=8*(1+self.read(5)) if small else 1+self.read([9,13,18,30][self.read(2)]);ratio=self.read(3)
  if ratio:x=[None,(1,1),(6,5),(4,3),(3,2),(16,9),(5,4),(2,1)][ratio];x=y*x[0]//x[1]
  else:x=8*(1+self.read(5)) if small else 1+self.read([9,13,18,30][self.read(2)])
  return [x,y]
def prefix(data):
 b=Reader(raw(data));assert b.read(16)==0x0aff;coded=b.size();ori=1;intrinsic=None
 if not b.read(1) and b.read(1):
  ori=1+b.read(3)
  if b.read(1):intrinsic=b.size()
  assert b.read(1)==0 # these fixtures do not contain preview
  assert b.read(1)==0 # or animation
 return dict(coded=coded,orientation=ori,source_intrinsic=intrinsic,dimension_prefix_bytes=(b.at+7)//8)
manifest={'oracle':'C libjxl0.11.2 independent BasicInfo with/without keep_orientation plus full normalized RGBA16; existing saved djxl orientation references','contract':'coded axes from input; output and intrinsic_display swap axes for orientations5..8; stored C intrinsic fields stay raw and are recorded separately; absent declaration stays None, not C fallback','cases':[]}
def record(name,data,complete=True):
 path=dest/(name+'.jxl');path.write_bytes(data);meta=prefix(data)
 kept=[int(v) for v in run([reference,path,1]).split()];oriented=[int(v) for v in run([reference,path,0]).split()]
 assert kept[:2]==meta['coded'] and kept[2]==meta['orientation']
 if meta['source_intrinsic'] is not None:assert kept[3:]==oriented[3:]==meta['source_intrinsic']
 info=dict(file=path.name,bytes=len(data),sha256=digest(data),c_basic_kept=kept,c_basic_oriented=oriented,**meta)
 if complete:
  dimensions=[int(v) for v in run([pixels,path,work/(name+'.rgba16'),work/(name+'.icc')]).split()];assert dimensions==oriented[:2]
  values=struct.unpack('<'+'H'*(len((work/(name+'.rgba16')).read_bytes())//2),(work/(name+'.rgba16')).read_bytes());rgba=bytes((v*255+32767)//65535 for v in values)
  (dest/(name+'.rgba')).write_bytes(rgba)
  if name=='jxl-intrinsic-icc16':(dest/(name+'.rgba16le')).write_bytes((work/(name+'.rgba16')).read_bytes())
  info['rgba_sha256']=digest(rgba);info['rgba16_sha256']=digest((work/(name+'.rgba16')).read_bytes())
  if name.startswith('jxl-intrinsic-orientation'):assert rgba==(dest/f'jxl-orientation{meta["orientation"]}.rgba').read_bytes()
 else:info['scope']='Header-only valid dimension declaration; main frame body is not claimed valid for the huge coded canvas.'
 manifest['cases'].append(info)
for orientation in range(1,9):
 path=work/f'orientation{orientation}.jxl';run([encoder,work/'source.raw',path,2,3,4,8,'orientation'+str(orientation),'-',19,11]);record('jxl-intrinsic-orientation'+str(orientation),path.read_bytes())
path=work/'icc16.jxl';run([encoder,dest/'jxl-rgba16.bin',path,7,5,4,16,'orientation6',dest/'jxl-gamma2.icc',19,11]);record('jxl-intrinsic-icc16',path.read_bytes())
# Rotate the frozen independent LittleCMS gamma2-to-sRGB oracle; do not use WML2
# decoding/conversion to generate expected managed pixels.
srgb=(dest/'jxl-icc-lossless.srgb.rgba').read_bytes();assert len(srgb)==7*5*4
rotated=bytearray(7*5*4)
for y in range(5):
 for x in range(7):
  dst=(x*5+4-y)*4;rotated[dst:dst+4]=srgb[(y*7+x)*4:(y*7+x+1)*4]
(dest/'jxl-intrinsic-icc16.srgb.rgba').write_bytes(rotated)
manifest['managed_reference']={'source':'jxl-icc-lossless.srgb.rgba (existing independent LittleCMS reference)','orientation':6,'mapping':'output(x,y)=(4-source_y,source_x)','sha256':digest(rotated)}
# Explicitly absent via original independent C-encoded fixtures, no copied
# fallback intrinsic size in the typed query.
manifest['absent']=[{'file':f'jxl-orientation{i}.jxl',**prefix((dest/f'jxl-orientation{i}.jxl').read_bytes())} for i in range(1,9)]
def bits(data):return [(v>>j)&1 for v in data for j in range(8)]
def little(v,n):return [(v>>j)&1 for j in range(n)]
def pack(a):
 a=a+[0]*((-len(a))%8);return bytes(sum(v<<j for j,v in enumerate(a[k:k+8])) for k in range(0,len(a),8))
def size(w,h,ratio=0):
 def axis(v):
  for selector,n in enumerate([9,13,18,30]):
   if 0<=v-1<(1<<n):return little(selector,2)+little(v-1,n)
  raise ValueError(v)
 return [0]+axis(h)+little(ratio,3)+(axis(w) if ratio==0 else [])
base=raw((dest/'jxl-orientation1.jxl').read_bytes());b=bits(base);assert b[42:44]==[0,0] and b[53:55]==[0,0] # fingerprinted C header fields
# Reuse the C-encoded frame unchanged; only insert a valid SizeHeader. C then
# independently decodes all six pixels, including format-maximum display size.
def synthetic(intrinsic,coded=None):
 start=b[:42] if coded is None else b[:16]+size(*coded)
 return pack(start+[0,1]+[0,0,0]+[1]+intrinsic+[0,0]+b[44:53]+[1]+b[53:56])+base[7:]
record('jxl-intrinsic-large-display',synthetic(size(1<<20,1<<20)))
record('jxl-intrinsic-huge-coded-header',synthetic(size(19,11),coded=(1<<20,1<<20)),False)
# Retain pinned jxl0.7.4 Size-reader restrictions. C may accept these
# oversized display metadata values; do not mislabel that difference as proof
# of universally invalid syntax.
for name,description,encoded in [('area','Both dimensions2^30 exceed the pinned Size reader2^40 area bound',size(1<<30,1<<30)),('axis','Derived2:1 width2^31 exceeds the pinned Size reader2^30 axis bound',size(1,1<<30,7))]:
 data=synthetic(encoded);path=dest/('jxl-intrinsic-oversized-'+name+'.jxl');path.write_bytes(data)
 result=subprocess.run([str(reference),str(path),'1'],capture_output=True,text=True)
 manifest.setdefault('invalid',[]).append(dict(file=path.name,sha256=digest(data),description=description,c_basic_info_rejected=result.returncode!=0,c_basic_fields=result.stdout.strip(),scope='Expected rejection by the pinned Size reader, not a claim that C rejects every oversized display declaration.'))
(dest/'jxl-intrinsic-fixtures.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(manifest,indent=2))
