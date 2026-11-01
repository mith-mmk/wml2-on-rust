#!/usr/bin/env python3
"""Independent C libjxl0.11.2 two-plane oracle with authored source codes."""
import argparse,pathlib,struct,subprocess,json,hashlib
p=argparse.ArgumentParser();p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('work directory must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review';work=a.work.resolve();records=[]
def run(args):return subprocess.run([str(x)for x in args],capture_output=True,text=True,check=True)
def oriented(v,o):
 out=[0]*35;w,h=7,5;ow=h if o>=5 else w
 for y in range(h):
  for x in range(w):
   dx,dy=[(x,y),(w-1-x,y),(w-1-x,h-1-y),(x,h-1-y),(y,x),(h-1-y,x),(h-1-y,w-1-x),(y,w-1-x)][o-1]
   out[dy*ow+dx]=v[y*w+x]
 return out
def encode(name,channels,bits,kinds,ebits,o,icc=False,mode='normal'):
 values=[];codes=[[]for _ in kinds]
 for i in range(35):
  maximum=(1<<bits)-1
  values += [(((i*(997+31*c)+12345+c*193)%maximum)*65535+maximum//2)//maximum for c in range(channels)]
  for j,b in enumerate(ebits):
   maximum=(1<<b)-1;code=[0,1,maximum//2,maximum-1,maximum,(i*(193+j*37)+17+j*71)%maximum][(i+j*3)%6]
   if mode=='float'and j==len(kinds)-1:code=maximum if i%2 else 0
   codes[j].append(code);values.append((code*65535+maximum//2)//maximum)
 source=work/(name+'.source16le');source.write_bytes(struct.pack('<'+'H'*len(values),*values));path=dest/(name+'.jxl')
 run([work/'encoder',source,path,channels,bits,o,kinds,','.join(map(str,ebits)),dest/'jxl-gamma2.icc'if icc else'-',mode])
 return path,codes
assert 'runtime=11002 header=11002' in run([work/'reference','--version']).stdout
configs=[]
for channels in [1,3]:
 for i,b0 in enumerate([8,9,10,12,16]):
  for j,b1 in enumerate([8,9,10,12,16]):
   configs.append((channels,[8,9,10,12,16][(i+2*j)%5],'DM'if(i+j+(channels==3))%2==0 else'MD',[b0,b1],(i*5+j)%8+1))
configs += [(3,16,'DD',[8,16],5),(1,8,'MM',[16,8],6),(3,12,'DD',[8,8],7),(1,10,'MM',[16,16],8)]
for channels,bits,kinds,ebits,o in configs:
 name=f'jxl-two-{channels}c{bits}-{kinds}-{ebits[0]}-{ebits[1]}-o{o}';icc=channels==3 and ebits==[8,16]
 path,codes=encode(name,channels,bits,kinds,ebits,o,icc)
 result=run([work/'reference',path,dest/(name+'.rgba16le'),work/(name+'.icc'),work/(name+'.norm')])
 for i,(kind,b)in enumerate(zip(kinds,ebits)):
  assert f'extra {i} {1 if kind=="D"else 3} {b} 0 0 0'in result.stderr and f'name plane-{i}'in result.stderr
  norm=struct.unpack('<35H',(work/(name+f'.norm.{i}')).read_bytes());maximum=(1<<b)-1;recovered=[(v*maximum+32767)//65535 for v in norm]
  assert recovered==oriented(codes[i],o)
  (dest/(name+f'.p{i}.codes16le')).write_bytes(struct.pack('<35H',*recovered))
 files=[path,dest/(name+'.rgba16le')]+[dest/(name+f'.p{i}.codes16le')for i in range(2)]
 records.append(dict(name=name,channels=channels,color_bits=bits,kinds=kinds,extra_bits=ebits,orientation=o,coded=[7,5],output=[int(v)for v in result.stdout.split()],embedded_icc=icc,sha256={f.name:hashlib.sha256(f.read_bytes()).hexdigest()for f in files}))
for label,kinds,ebits,mode in [('three','DMM',[8,16,9],'normal'),('spot','DS',[8,16],'normal'),('float','DM',[8,16],'float'),('subsampled','DM',[8,16],'subsampled'),('layers','DM',[8,16],'layers')]:
 name='jxl-two-refused-'+label;path,_=encode(name,3,8,kinds,ebits,1,mode=mode)
 result=run([work/'rgba-reference',path,work/(name+'.rgba16le'),work/(name+'.icc')])
 for i,b in enumerate(ebits):assert f'extra {i} 'in result.stderr
 records.append(dict(name=name,refused=True,kind=label,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
manifest=dict(reference='Independent C libjxl0.11.2; per-plane normalized decode recovered to source codes; exact authored source code and independent orientation map agreement',source_sha256={str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest()for f in [pathlib.Path(__file__),root/'wml2-test/scripts/jpegxl_multi_fixture_encoder.c',root/'wml2-test/scripts/jpegxl_extra_reference.c']},cases=records)
(root/'docs/jpeg-xl-two-plane-oracle.json').write_text(json.dumps(manifest,indent=2)+'\n')
lines=[]
for r in records:
 if r.get('refused'):continue
 n=r['name'];args=[f'include_bytes!("../fixtures/review/{n}{suffix}").as_slice()'for suffix in ['.jxl','.rgba16le','.p0.codes16le','.p1.codes16le']]
 kinds=[('Depth'if k=='D'else'SelectionMask')for k in r['kinds']]
 args += [f'[{r["extra_bits"][0]}u8,{r["extra_bits"][1]}u8]',f'[{kinds[0]},{kinds[1]}]',f'{r["orientation"]}u8',f'{r["color_bits"]}u8']
 lines.append('('+','.join(args)+'),')
(root/'wml2/tests/jpeg_xl/two_plane_cases.rs').write_text('// Generated independent C two-plane cases.\n[\n'+'\n'.join(lines)+'\n]\n')
print(len(records),'independent C two-plane cases')
