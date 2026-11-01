#!/usr/bin/env python3
"""C-only structured PQ/HLG integer16 -> encoded F32 reference fixtures."""
import argparse,hashlib,json,pathlib,struct,subprocess,zlib
p=argparse.ArgumentParser();p.add_argument('--reference',type=pathlib.Path,required=True);p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('work must start with .test')
a.work.mkdir(parents=True,exist_ok=True);reference=a.reference.resolve();root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review'
rows=[];codes=[0,1,64,1024,10000,32768,40000,65534,65535]
for transfer in ['pq','hlg']:
 for suffix,c,storage,tone in [('rgb',3,'u16','255'),('rgba',4,'u16','1000:0.02:1:0.5'),('oriented',4,'u16o6','2000:0.005:0:10')]:
  name=transfer+'-'+suffix;values=[]
  for i in range(15):
   values.extend(codes[(i*3+k)%len(codes)]for k in range(3))
   if c==4:values.append([0,1,16384,32768,65535][i%5])
  source=a.work/(name+'.u16le');source.write_bytes(struct.pack('<'+'H'*len(values),*values));file=dest/('jxl-encoded-hdr-'+name+'.jxl');output=a.work/(name+'.f32le')
  subprocess.run([str(reference),'encode',str(source),str(file),'5','3',str(c),storage,transfer,tone,'-'],check=True,capture_output=True)
  r=subprocess.run([str(reference),'decode',str(file),str(output),'f32','0'],check=True,capture_output=True,text=True);meta=json.loads(r.stdout);assert meta['version']==11002 and meta['structured_profile'] and meta['data_structured'] and meta['transfer']==meta['data_transfer']==(16 if transfer=='pq'else 18)
  kept=a.work/(name+'.kept');subprocess.run([str(reference),'decode',str(file),str(kept),'f32','keep'],check=True,capture_output=True)
  samples=struct.unpack('<'+'f'*(len(kept.read_bytes())//4),kept.read_bytes());error=max(abs(v-q/65535)for v,q in zip(samples,values));assert error<=2**-24
  raw=output.read_bytes();samples=struct.unpack('<'+'f'*(len(raw)//4),raw);rgba=[]
  for i in range(0,len(samples),c):rgba.extend([*samples[i:i+3],samples[i+3]if c==4 else 1.0])
  data=struct.pack('<'+'f'*len(rgba),*rgba);(dest/(file.stem+'.f32le.zlib')).write_bytes(zlib.compress(data,9))
  rows.append(dict(file=file.name,source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),input_sha256=hashlib.sha256(file.read_bytes()).hexdigest(),rgba_sha256=hashlib.sha256(data).hexdigest(),maximum_normalized_code_error=error,header=meta))
negative=[]
for name,transfer,profile in [('p3-refused','pq-p3','-'),('icc-refused','pq',str(a.work/'pq-rgb.f32le.icc'))]:
 file=dest/('jxl-encoded-hdr-'+name+'.jxl');output=a.work/(name+'.f32le')
 subprocess.run([str(reference),'encode',str(a.work/'pq-rgb.u16le'),str(file),'5','3','3','u16',transfer,'255',profile],check=True,capture_output=True)
 subprocess.run([str(reference),'decode',str(file),str(output),'f32','0'],check=True,capture_output=True)
 negative.append(dict(file=file.name,sha256=hashlib.sha256(file.read_bytes()).hexdigest(),scope='C-decodable but deliberately outside structured Rec.2020/D65 contract'))
file=dest/'jxl-encoded-hdr-associated-refused.jxl'
subprocess.run([str(reference),'encode',str(a.work/'pq-rgba.u16le'),str(file),'5','3','4','u16-associated','pq','255','-'],check=True,capture_output=True)
subprocess.run([str(reference),'decode',str(file),str(a.work/'associated.f32le'),'f32','0'],check=True,capture_output=True)
negative.append(dict(file=file.name,sha256=hashlib.sha256(file.read_bytes()).hexdigest(),scope='C-decodable associated alpha; explicit refusal remains'))
base=(dest/'jxl-encoded-hdr-pq-rgba.jxl').read_bytes();bits=[(v>>j)&1 for v in base for j in range(8)];needle=[(0x63d0>>j)&1 for j in range(16)]
positions=[i for i in range(16,min(1024,len(bits))-16)if bits[i:i+16]==needle];assert len(positions)==1
for name,value in [('infinite',0x7c00),('zero',0)]:
 mutated=bits[:];mutated[positions[0]:positions[0]+16]=[(value>>j)&1 for j in range(16)];data=bytes(sum(v<<j for j,v in enumerate(mutated[i:i+8]))for i in range(0,len(bits),8));file=dest/('jxl-encoded-hdr-tone-'+name+'-bad.jxl');file.write_bytes(data)
 r=subprocess.run([str(reference),'decode',str(file),str(a.work/'bad.f32le'),'f32','0'],capture_output=True);assert r.returncode!=0
 negative.append(dict(file=file.name,sha256=hashlib.sha256(data).hexdigest(),source_sha256=hashlib.sha256(base).hexdigest(),bit=positions[0],scope='Malformed intensity-target half-field mutation; independent C decoder also rejects'))
(root/'docs/jpeg-xl-encoded-hdr-validation.json').write_text(json.dumps({'oracle':'C libjxl0.11.2, ORIGINAL/DATA structured profiles and F32 encoded coordinates; virtual ICC bytes not used as source signal','helper_sha256':hashlib.sha256((root/'wml2-test/scripts/jpegxl_hdr_encoded_reference.c').read_bytes()).hexdigest(),'generator_sha256':hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),'negative':negative,'cases':rows},indent=2)+'\n')
