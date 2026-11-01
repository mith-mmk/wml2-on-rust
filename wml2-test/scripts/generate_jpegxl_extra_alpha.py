#!/usr/bin/env python3
"""Independent C associated/uniformly reconstructed Main5 extra-plane oracle."""
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
   if kinds[j]=='A' and ('zero-alpha'in mode or'opaque-alpha'in mode):code=0 if'zero-alpha'in mode else maximum
   codes[j].append(code);values.append((code*65535+maximum//2)//maximum)
 source=work/(name+'.source16le');source.write_bytes(struct.pack('<'+'H'*len(values),*values));path=dest/(name+'.jxl')
 run([work/'encoder',source,path,channels,bits,o,kinds,','.join(map(str,ebits)),dest/'jxl-gamma2.icc'if icc else'-',mode])
 return path,codes

assert 'runtime=11002 header=11002' in run([work/'reference','--version']).stdout
configs=[];depths=[8,9,10,12,16]
for channels in [1,3]:
 for i,bits in enumerate(depths):
  for j,abits in enumerate(depths):
   n=2+(i*5+j)%3;ai=(2*i+j+(channels==3))%n;kinds=''.join('A'if k==ai else'D'if k%2==0 else'M'for k in range(n))
   ebits=[abits if k==ai else depths[(i+j+k)%5]for k in range(n)]
   configs.append((channels,bits,kinds,ebits,(i*5+j)%8+1,0,True,'associated'))
 for shift in [1,2,3]:
  for associated in [False,True]:
   for j,(bits,abits)in enumerate([(8,16),(9,12),(10,9),(12,10),(16,8)]):
    n=2+j%3;ai=(j+shift+(channels==3))%n;kinds=''.join('A'if k==ai else'D'if k%2==0 else'M'for k in range(n));ebits=[abits if k==ai else depths[(j+k)%5]for k in range(n)]
    configs.append((channels,bits,kinds,ebits,(j*3+shift+(3 if associated else 0))%8+1,shift,associated,('associated'if associated else'straight')+f'-s{shift}'))
for associated in [False,True]:
 for label in ['zero-alpha','opaque-alpha']:
  configs.append((3,8,'AMDM',[12,16,8,9],4,3,associated,('associated'if associated else'straight')+'-s3-'+label))
for b in [8,16]:configs.append((3,b,'DMDA',[16,9,12,b],6,0,True,'associated-colored'))
configs += [(3,16,'DMDM',[8,9,12,16],5,1,False,'straight-s1'),(1,12,'MDM',[10,16,8],6,3,False,'straight-s3')]
configs += [(3,8,'DAMD',[12,9,16,8],7,0,False,'straight-f4'),(3,8,'DAMD',[12,9,16,8],7,0,True,'associated-f4')]
for index,(channels,bits,kinds,ebits,o,shift,associated,mode)in enumerate(configs):
 factor=int(mode.split('-f')[1])if'-f'in mode else 1<<shift
 name=f'jxl-extra-alpha-{index:03d}-{mode}';icc=channels==3 and (index%3==0 or len(kinds)==4)
 path,codes=encode(name,channels,bits,kinds,ebits,o,icc,mode)
 result=run([work/'reference',path,dest/(name+'.rgba16le'),work/(name+'.icc'),work/(name+'.norm'),'straight'])
 for i,(kind,b)in enumerate(zip(kinds,ebits)):
  ds=shift if kind=='A'else 0;flag=int(associated and kind=='A');assert f'extra {i} {0 if kind=="A"else 1 if kind=="D"else 3} {b} 0 {ds} {flag}'in result.stderr
  norm=struct.unpack('<35H',(work/(name+('.norm'if len(kinds)==1 else f'.norm.{i}'))).read_bytes())
  if factor>1:output=norm
  else:
   maximum=(1<<b)-1;output=tuple((v*maximum+32767)//65535 for v in norm);assert output==tuple(oriented(codes[i],o))
  (dest/(name+f'.p{i}.codes16le')).write_bytes(struct.pack('<35H',*output))
 files=[path,dest/(name+'.rgba16le')]+[dest/(name+f'.p{i}.codes16le')for i in range(len(kinds))]
 records.append(dict(name=name,channels=channels,color_bits=bits,kinds=kinds,extra_bits=ebits,orientation=o,dim_shift=shift,effective_upsampling=factor,associated=associated,embedded_icc=icc,sha256={f.name:hashlib.sha256(f.read_bytes()).hexdigest()for f in files}))
manifest=dict(reference='Independent C libjxl0.11.2 straight RGBA16 and every separately reconstructed normalized U16 extra; full-resolution codes independently recover authored data, reconstructed16 samples use C directly without claiming native coded-plane access',source_sha256={str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest()for f in [pathlib.Path(__file__),root/'wml2-test/scripts/jpegxl_multi_fixture_encoder.c',root/'wml2-test/scripts/jpegxl_extra_reference.c']},cases=records)
(root/'docs/jpeg-xl-extra-alpha-oracle.json').write_text(json.dumps(manifest,indent=2)+'\n')
lines=[]
for r in records:
 n=r['name'];args=[f'include_bytes!("../fixtures/review/{n}{suffix}").as_slice()'for suffix in ['.jxl','.rgba16le']]
 args += ['&['+','.join(f'include_bytes!("../fixtures/review/{n}.p{i}.codes16le").as_slice()'for i in range(len(r['kinds'])))+'] as &[&[u8]]']
 args += ['&['+','.join(str(b)+'u8'for b in r['extra_bits'])+'] as &[u8]']
 args += ['&['+','.join('Alpha'if k=='A'else'Depth'if k=='D'else'SelectionMask'for k in r['kinds'])+'] as &[JxlExtraChannelKind]']
 args += [str(r['orientation'])+'u8',str(r['color_bits'])+'u8',str(r['dim_shift'])+'u8',str(r['associated']).lower(),str(r['effective_upsampling'])+'u8']
 lines.append('('+','.join(args)+'),')
(root/'wml2/tests/jpeg_xl/extra_alpha_cases.rs').write_text('// Generated independent C associated/reconstructed extras.\n[\n'+'\n'.join(lines)+'\n]\n')
print(len(records),'independent C extra Alpha cases')
