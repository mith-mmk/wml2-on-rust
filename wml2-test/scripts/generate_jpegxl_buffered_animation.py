#!/usr/bin/env python3
"""Revalidate frozen PR67 C oracles; add Main5-rate variants without overwrites."""
import argparse,pathlib,json,subprocess,hashlib,copy
p=argparse.ArgumentParser();p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('work directory must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review';work=a.work.resolve()
manifest=json.loads((root/'docs/jpeg-xl-buffered-animation-oracle.json').read_text());original=[c for c in manifest['oracle']['cases']if not c.get('main5_variant') and not c.get('new_source')]
def run(args):return subprocess.run([str(x)for x in args],capture_output=True,text=True,check=True)
assert run([work/'reference','--version']).stdout.strip()=='runtime=11002 header=11002'
for name,mode in [('internal','internal'),('icc16','reference'),('icc16-variable','replace')]:
 old=next(c for c in original if c['name']==name);c=copy.deepcopy(old);c['name']='main5-'+name;c['file']='jxl-animation-'+c['name']+'.jxl';c['tps_numerator']=100;c['main5_variant']=True
 run([work/'encoder',dest/c['file'],mode,c['color_bits'],c['source_orientation'],100,c['tps_denominator'],c['loop_count'],dest/'jxl-gamma2.icc'if c['color_bits']==16 else'-'])
 c['sha256']=hashlib.sha256((dest/c['file']).read_bytes()).hexdigest();ref=work/c['file'][:-4]
 run([work/'reference',dest/c['file'],ref])
 for i,f in enumerate(c['frames']):
  data=pathlib.Path(str(ref)+f'.{i}.rgba16le').read_bytes();(dest/(c['file'][:-4]+f'.{i}.rgba16le')).write_bytes(data);f['rgba16_sha256']=hashlib.sha256(data).hexdigest();f.pop('srgb_sha256',None);f.pop('milliseconds_exact',None)
 original.append(c)
for name,bits,ori,frames in [('rgb8',8,1,3),('rgb16',16,8,3),('layers64',8,1,64),('layers65',8,1,65)]:
 filename='jxl-animation-main5-'+name+'.jxl';mode='rgb-replace'if name.startswith('rgb')else'replace';args=[work/'encoder',dest/filename,mode,bits,ori,100,1,0,dest/'jxl-gamma2.icc'if bits==16 else'-']
 if frames>3:args.append(frames)
 run(args);ref=work/filename[:-4];result=run([work/'reference',dest/filename,ref]);entries=[]
 for i in range(frames):
  data=pathlib.Path(str(ref)+f'.{i}.rgba16le').read_bytes();ticks=5 if frames>3 else[5,17,3][i]
  if frames<=64:(dest/(filename[:-4]+f'.{i}.rgba16le')).write_bytes(data)
  entries.append(dict(index=i,ticks=ticks,last=i+1==frames,rgba16_sha256=hashlib.sha256(data).hexdigest()))
 original.append(dict(name='main5-'+name,file=filename,sha256=hashlib.sha256((dest/filename).read_bytes()).hexdigest(),color_bits=bits,source_orientation=ori,tps_numerator=100,tps_denominator=1,loop_count=0,have_timecodes=False,have_preview=False,frames=entries,new_source=True,too_many_layers=frames>64))
lines=[]
for c in original:
 name=c['file'];stem=name[:-4];ref=work/stem
 assert hashlib.sha256((dest/name).read_bytes()).hexdigest()==c['sha256']
 run([work/'reference',dest/name,ref])
 for i,f in enumerate(c['frames']):
  if c.get('too_many_layers'):continue
  assert pathlib.Path(str(ref)+f'.{i}.rgba16le').read_bytes()==(dest/(stem+f'.{i}.rgba16le')).read_bytes()
 rejected=c.get('too_many_layers',False)or c['have_preview']or c['have_timecodes']or any(f['ticks']>0 and f['ticks']*c['tps_denominator']*120<c['tps_numerator']for f in c['frames'])
 c['buffered_main5_supported']=not rejected
 if rejected:continue
 args=[f'include_bytes!("../fixtures/review/{name}").as_slice()', '&['+','.join(f'include_bytes!("../fixtures/review/{stem}.{i}.rgba16le").as_slice()'for i in range(len(c['frames'])))+'] as &[&[u8]]','&['+','.join(str(f['ticks'])+'u32'for f in c['frames'])+'] as &[u32]',str(c['tps_numerator'])+'u32',str(c['tps_denominator'])+'u32',str(c['loop_count'])+'u32',str(c['source_orientation'])+'u8',str(c['color_bits'])+'u8']
 lines.append('('+','.join(args)+'),')
manifest['oracle']['cases']=original;manifest['new_generator_sha256']=hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest();manifest['C_sources_sha256']={name:hashlib.sha256((root/'wml2-test/scripts'/name).read_bytes()).hexdigest()for name in ['jpegxl_animation_encoder.c','jpegxl_animation_reference.c']}
(root/'docs/jpeg-xl-buffered-animation-oracle.json').write_text(json.dumps(manifest,indent=2)+'\n')
(root/'wml2/tests/jpeg_xl/animation_cases.rs').write_text('// Independently C-revalidated PR67 and Main5-rate variants.\n[\n'+'\n'.join(lines)+'\n]\n')
print(len(lines),'supported /',len(original)-len(lines),'refused C-validated animation inputs')
