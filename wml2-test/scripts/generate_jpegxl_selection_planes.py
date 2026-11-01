#!/usr/bin/env python3
"""Independent libjxl0.11.2 integer SelectionMask-plane qualification; no WML2 oracle."""
import argparse,pathlib,struct,subprocess,json,hashlib
p=argparse.ArgumentParser();p.add_argument('--work',type=pathlib.Path,required=True);a=p.parse_args()
if not a.work.name.startswith('.test'):p.error('work directory must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];dest=root/'wml2/tests/fixtures/review';work=a.work.resolve();records=[]
def run(args):return subprocess.run([str(x)for x in args],capture_output=True,text=True,check=True)
def oriented(values,channels,o):
 out=[0]*len(values);w,h=7,5;ow=h if o>=5 else w
 for y in range(h):
  for x in range(w):
   dx,dy=[(x,y),(w-1-x,y),(w-1-x,h-1-y),(x,h-1-y),(y,x),(h-1-y,x),(h-1-y,w-1-x),(y,w-1-x)][o-1]
   out[(dy*ow+dx)*channels:(dy*ow+dx+1)*channels]=values[(y*w+x)*channels:(y*w+x+1)*channels]
 return out
assert 'runtime=11002 header=11002' in run([work/'reference','--version']).stdout
for channels in [1,3]:
 for ci,bits in enumerate([8,9,10,12,16]):
  for di,selection in enumerate([8,9,10,12,16]):
   o=(ci*5+di)%8+1;icc=channels==3 and (bits,selection) in [(8,16),(16,8)];named=(bits,selection)==(9,12)
   name=f'jxl-selection-{channels}c{bits}d{selection}o{o}';values=[];codes=[]
   maxselection=(1<<selection)-1;maxcolor=(1<<bits)-1
   for i in range(35):
    colors=[(i*(97+c*31)+13+23*c)%maxcolor for c in range(channels)]
    code=[0,1,maxselection//2,maxselection-1,maxselection,(i*193+17)%maxselection][i%6];codes.append(code)
    values += [(v*65535+maxcolor//2)//maxcolor for v in colors]+[(code*65535+maxselection//2)//maxselection]
   source=work/(name+'.source16le');source.write_bytes(struct.pack('<'+'H'*len(values),*values));path=dest/(name+'.jxl')
   run([work/'encoder',source,path,7,5,channels,bits,selection,o,'selection-named'if named else'selection',dest/'jxl-gamma2.icc'if icc else'-'])
   result=run([work/'reference',path,dest/(name+'.rgba16le'),work/(name+'.icc'),work/(name+'.normalized16le')])
   assert f'image {bits} 0 {channels} 1' in result.stderr and f'extra 0 3 {selection} 0 0 0' in result.stderr
   if named:assert 'name camera-Z' in result.stderr
   normalized=struct.unpack('<35H',(work/(name+'.normalized16le')).read_bytes());recovered=[(v*maxselection+32767)//65535 for v in normalized]
   assert recovered==oriented(codes,1,o)
   plane=struct.pack('<35H',*recovered);(dest/(name+'.codes16le')).write_bytes(plane)
   record=dict(name=name,color_channels=channels,color_bits=bits,extra_kind='SelectionMask',extra_bits=selection,dim_shift=0,orientation=o,coded=[7,5],output=[int(v)for v in result.stdout.split()],name_from_c='camera-Z'if named else'',embedded_icc=icc,jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),rgba16_sha256=hashlib.sha256((dest/(name+'.rgba16le')).read_bytes()).hexdigest(),source_code_sha256=hashlib.sha256(plane).hexdigest())
   records.append(record)
for mode in ['selection-float','selection-subsampled','selection-multiple']:
 name='jxl-selection-refused-'+mode;path=dest/(name+'.jxl')
 run([work/'encoder',work/'jxl-selection-3c16d16o1.source16le',path,7,5,3,16,16,1,mode,'-'])
 result=run([work/'rgba-reference',path,work/(name+'.rgba16le'),work/(name+'.icc')])
 if mode=='selection-multiple':assert 'extra 0 3 16 0 0 0' in result.stderr and 'extra 1 3 16 0 0 0' in result.stderr
 records.append(dict(name=name,refused=True,kind='independently C-decoded unsupported SelectionMask storage',jxl_sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
(root/'docs/jpeg-xl-selection-plane-oracle.json').write_text(json.dumps(dict(reference='Independent C libjxl0.11.2 color RGBA16 and normalized extra U16; source-code recovery verified against authored integer codes and independent orientation map',generator_sha256=hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),encoder_sha256=hashlib.sha256((root/'wml2-test/scripts/jpegxl_alpha_fixture_encoder.c').read_bytes()).hexdigest(),reference_sha256=hashlib.sha256((root/'wml2-test/scripts/jpegxl_extra_reference.c').read_bytes()).hexdigest(),cases=records),indent=2)+'\n')
lines=[]
for r in records:
 if r.get('refused'):continue
 n=r['name'];lines.append(f'(include_bytes!("../fixtures/review/{n}.jxl").as_slice(),include_bytes!("../fixtures/review/{n}.rgba16le").as_slice(),include_bytes!("../fixtures/review/{n}.codes16le").as_slice(),{r["color_bits"]}u8,{r["extra_bits"]}u8,{r["orientation"]}u8),')
(root/'wml2/tests/jpeg_xl/selection_cases.rs').write_text('// Generated independent C SelectionMask-plane cases.\n[\n'+'\n'.join(lines)+'\n]\n')
print(len(records),'independent C SelectionMask cases')
