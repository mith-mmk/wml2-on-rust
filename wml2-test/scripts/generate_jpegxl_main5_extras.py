#!/usr/bin/env python3
"""Independent C libjxl0.11.2 Main5 extras/Alpha oracle with authored source codes."""
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
   if kinds[j]=='A' and mode in ['zero-alpha','opaque-alpha']:code=0 if mode=='zero-alpha'else maximum
   codes[j].append(code);values.append((code*65535+maximum//2)//maximum)
 source=work/(name+'.source16le');source.write_bytes(struct.pack('<'+'H'*len(values),*values));path=dest/(name+'.jxl')
 run([work/'encoder',source,path,channels,bits,o,kinds,','.join(map(str,ebits)),dest/'jxl-gamma2.icc'if icc else'-',mode])
 return path,codes
assert 'runtime=11002 header=11002' in run([work/'reference','--version']).stdout
configs=[]
for channels in [1,3]:
 for i,bits in enumerate([8,9,10,12,16]):
  for j,abits in enumerate([8,9,10,12,16]):
   n=1+(i*5+j)%4;ai=(i*2+j+(channels==3))%n
   kinds=''.join('A'if k==ai else 'D'if k%2==0 else 'M'for k in range(n))
   ebits=[abits if k==ai else [8,9,10,12,16][(i+j+k)%5] for k in range(n)]
   configs.append((channels,bits,kinds,ebits,(i*5+j)%8+1))
configs += [(3,16,'DMDM',[8,9,12,16],5),(1,8,'MMMM',[16,16,16,16],6),(3,12,'DDDD',[8,10,12,16],7),(1,10,'MDM',[16,9,8],8)]
assert {k.index('A')for _,_,k,_,_ in configs if len(k)==4 and 'A'in k}=={0,1,2,3}
configs=[(*c,'normal')for c in configs]+[(3,8,'AMDM',[12,16,8,9],4,'zero-alpha'),(3,8,'AMDM',[12,16,8,9],4,'opaque-alpha')]
for channels,bits,kinds,ebits,o,mode in configs:
 name=f'jxl-main5-{channels}c{bits}-{kinds}-'+ '-'.join(map(str,ebits))+f'-o{o}'+(''if mode=='normal'else'-'+mode);icc=channels==3 and (len(kinds)==4 or (bits==16 and 'A'in kinds))
 path,codes=encode(name,channels,bits,kinds,ebits,o,icc,mode)
 result=run([work/'reference',path,dest/(name+'.rgba16le'),work/(name+'.icc'),work/(name+'.norm')])
 # Independent decoded color/alpha must retain authored straight coordinates,
 # including colored zero-alpha pixels; no WML2 output is used as oracle.
 source=struct.unpack('<'+'H'*(35*(channels+len(kinds))),(work/(name+'.source16le')).read_bytes())
 rgb=[oriented([source[x*(channels+len(kinds))+c]for x in range(35)],o)for c in range(channels)]
 alpha=[65535]*35 if 'A'not in kinds else oriented([source[x*(channels+len(kinds))+channels+kinds.index('A')]for x in range(35)],o)
 expected=[v for x in range(35)for v in ([rgb[c if channels==3 else 0][x]for c in range(3)]+[alpha[x]])]
 actual=struct.unpack('<140H',(dest/(name+'.rgba16le')).read_bytes());assert all(abs(a-b)<=1 for a,b in zip(actual,expected))
 for i,(kind,b)in enumerate(zip(kinds,ebits)):
  assert f'extra {i} {0 if kind=="A"else 1 if kind=="D"else 3} {b} 0 0 0'in result.stderr and f'name plane-{i}'in result.stderr
  norm=struct.unpack('<35H',(work/(name+('.norm' if len(kinds)==1 else f'.norm.{i}'))).read_bytes());maximum=(1<<b)-1;recovered=[(v*maximum+32767)//65535 for v in norm]
  assert recovered==oriented(codes[i],o)
  (dest/(name+f'.p{i}.codes16le')).write_bytes(struct.pack('<35H',*recovered))
 files=[path,dest/(name+'.rgba16le')]+[dest/(name+f'.p{i}.codes16le')for i in range(len(kinds))]
 records.append(dict(name=name,channels=channels,color_bits=bits,kinds=kinds,extra_bits=ebits,orientation=o,coded=[7,5],output=[int(v)for v in result.stdout.split()],embedded_icc=icc,sha256={f.name:hashlib.sha256(f.read_bytes()).hexdigest()for f in files}))
for label,kinds,ebits,mode in [('five','DMMMD',[8,16,9,10,12],'normal'),('spot','ADSM',[8,12,16,9],'normal'),('float','ADM',[8,12,16],'float'),('subsampled','ADM',[8,12,16],'subsampled'),('layers','ADM',[8,12,16],'layers'),('associated','ADM',[8,12,16],'associated'),('multiple-alpha','AAMD',[8,16,12,9],'normal')]:
 name='jxl-main5-refused-'+label;path,_=encode(name,3,8,kinds,ebits,1,mode=mode)
 result=run([work/'rgba-reference',path,work/(name+'.rgba16le'),work/(name+'.icc')])
 for i,b in enumerate(ebits):assert f'extra {i} 'in result.stderr
 records.append(dict(name=name,refused=True,kind=label,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
manifest=dict(reference='Independent C libjxl0.11.2; per-plane normalized decode recovered to source codes; exact authored source code and independent orientation map agreement',source_sha256={str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest()for f in [pathlib.Path(__file__),root/'wml2-test/scripts/jpegxl_multi_fixture_encoder.c',root/'wml2-test/scripts/jpegxl_extra_reference.c']},cases=records)
(root/'docs/jpeg-xl-main5-extras-oracle.json').write_text(json.dumps(manifest,indent=2)+'\n')
lines=[]
for r in records:
 if r.get('refused'):continue
 n=r['name'];args=[f'include_bytes!("../fixtures/review/{n}{suffix}").as_slice()'for suffix in ['.jxl','.rgba16le']]
 args += ['&['+','.join(f'include_bytes!("../fixtures/review/{n}.p{i}.codes16le").as_slice()'for i in range(len(r['kinds'])))+'] as &[&[u8]]']
 args += ['&['+','.join(str(b)+'u8'for b in r['extra_bits'])+'] as &[u8]']
 args += ['&['+','.join('Alpha'if k=='A'else'Depth'if k=='D'else'SelectionMask'for k in r['kinds'])+'] as &[JxlExtraChannelKind]']
 args += [str(r['orientation'])+'u8',str(r['color_bits'])+'u8']
 lines.append('('+','.join(args)+'),')
(root/'wml2/tests/jpeg_xl/main5_extra_cases.rs').write_text('// Generated independent C Main5 extras and straight Alpha.\n[\n'+'\n'.join(lines)+'\n]\n')
print(len(records),'independent C Main5 extras cases')
