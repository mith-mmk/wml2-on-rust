#!/usr/bin/env python3
"""Generate bounded product F16-source/F32-output fixtures using the #68-derived C oracle only.
Compile jpegxl_f32_reference.c against official libjxl0.11.2 headers/runtime.
"""
import argparse, hashlib, json, pathlib, struct, subprocess, zlib
p=argparse.ArgumentParser();p.add_argument('--reference',type=pathlib.Path,required=True);p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
assert a.work.name.startswith('.test')
a.work.mkdir(parents=True,exist_ok=True);reference=a.reference.resolve()
root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review'
values=[-65504.0,-1.0,-0.0,2**-24,-2**-24,2**-14,0.25,1.0,1.5,65504.0,2**-23,0.00390625]
rows=[]
for name,c,storage,transfer,profile,accepted in [
 ('rgb',3,'f16','linear','-',True),('rgba',4,'f16','linear','-',True),
 ('gray',1,'f16','linear','-',True),('grayalpha',2,'f16','linear','-',True),
 ('grayicc',2,'f16','srgb','jxl-gray-gamma2.icc',True),('icc',4,'f16','srgb','jxl-gamma2.icc',True),('oriented',4,'f16o6','linear','-',True),
 ('float32-refused',4,'f32','linear','-',False),('pq-refused',4,'f16','pq','-',False),('hlg-refused',4,'f16','hlg','-',False),
 ('nonfinite-refused',4,'f16','linear','-',False),('alpha-refused',4,'f16','linear','-',False),
]:
 samples=[]
 for i in range(15):
  colors=[values[((i if c<=2 else i*3)+k)%len(values)] for k in range(1 if c<=2 else 3)]
  if name=='nonfinite-refused':colors[0]=float('nan') if i%2 else float('inf')
  samples.extend(colors)
  if c in (2,4):samples.append((-0.25 if i%2 else 1.25) if name=='alpha-refused' else [0.0,-0.0,1.0,0.5,2**-24][i%5])
 raw=struct.pack('<'+('e' if storage.startswith('f16') else 'f')*len(samples),*samples)
 source=a.work/(name+'.source');source.write_bytes(raw)
 file=dest/('jxl-native-f16-'+name+'.jxl');output=a.work/(name+'.f32le')
 subprocess.run([str(reference),'encode',str(source),str(file),'5','3',str(c),storage,transfer,'255',str(dest/profile) if profile!='-' else '-'],check=True,capture_output=True)
 result=subprocess.run([str(reference),'decode',str(file),str(output),'f32','0'],check=True,capture_output=True,text=True)
 data=output.read_bytes();meta=json.loads(result.stdout)
 if accepted:
  kept=a.work/(name+'.kept.f32le')
  subprocess.run([str(reference),'decode',str(file),str(kept),'f32','keep'],check=True,capture_output=True)
  assert kept.read_bytes()==struct.pack('<'+'f'*(len(raw)//2),*struct.unpack('<'+'e'*(len(raw)//2),raw)), 'C exact16-to32 source promotion: '+name
 scalar=struct.unpack('<'+'f'*(len(data)//4),data);rgba=[]
 for i in range(0,len(scalar),c):
  v=scalar[i:i+c];rgba.extend([v[0],v[0],v[0],v[1] if c==2 else 1.0] if c<=2 else [*v[:3],v[3] if c==4 else 1.0])
 expanded=struct.pack('<'+'f'*len(rgba),*rgba)
 (dest/(file.stem+'.f32le.zlib')).write_bytes(zlib.compress(expanded,9))
 original=pathlib.Path(str(output)+'.icc').read_bytes();reconstructed=pathlib.Path(str(output)+'.data.icc').read_bytes()
 if profile!='-':assert original==reconstructed==(dest/profile).read_bytes()
 rows.append(dict(file=file.name,accepted=accepted,source_promoted_exactly=accepted,input_sha256=hashlib.sha256(file.read_bytes()).hexdigest(),rgba_sha256=hashlib.sha256(expanded).hexdigest(),source_profile_sha256=hashlib.sha256(original).hexdigest(),output_profile_sha256=hashlib.sha256(reconstructed).hexdigest(),header=meta))
(root/'docs/jpeg-xl-native-f16-validation.json').write_text(json.dumps({'oracle':'C libjxl0.11.2, original/data ICC queried separately; generated trusted tiny lossless original-profile fixtures; F16 sources decoded to F32; no backend F16 output requested','helper_sha256':hashlib.sha256((root/'wml2-test/scripts/jpegxl_f32_reference.c').read_bytes()).hexdigest(),'generator_sha256':hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),'cases':rows},indent=2)+'\n')
