#!/usr/bin/env python3
"""Trusted small-input streaming evaluation. Not a shipping streaming API.
Requires matching C libjxl0.11.2 headers/runtime; default replays frozen inputs.
"""
import argparse, hashlib, json, pathlib, struct, subprocess
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',type=pathlib.Path,required=True)
p.add_argument('--include',type=pathlib.Path,required=True)
p.add_argument('--target-dir',type=pathlib.Path)
p.add_argument('--write-fixtures',action='store_true')
a=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[2];out=a.output.resolve();target=(a.target_dir or out/'.test-target').resolve()
if not out.name.startswith('.test') or not target.name.startswith('.test'):p.error('working directories must start with .test')
out.mkdir(parents=True,exist_ok=True);fixtures=root/'wml2/tests/fixtures/review';scripts=root/'wml2-test/scripts'
def sha(b):return hashlib.sha256(b).hexdigest()
def run(command,label,check=True):
 r=subprocess.run(list(map(str,command)),cwd=root,capture_output=True,text=True);(out/(label+'.log')).write_text(r.stdout+r.stderr)
 if check:r.check_returncode()
 return r
run(['cargo','build','-p','wml2','--locked','--release','--no-default-features','--features','jpeg-xl,image-buffer-ex,color-management','--target-dir',target],'build')
jxl=list((target/'release/deps').glob('libjxl-*.rlib'));assert len(jxl)==1
run(['cc','-O2','-Wall','-Wextra','-Werror','-I'+str(a.include),scripts/'jpegxl_progressive_fixture.c','-Wl,-l:libjxl.so.0.11','-o',out/'fixture'],'compile-C')
run(['rustc','--edition','2024','-C','opt-level=3',scripts/'jpegxl_progressive_probe.rs','--extern',f'jxl_codec={jxl[0]}','--extern',f'wml2={target/"release/libwml2.rlib"}','-L',f'dependency={target/"release/deps"}','-o',out/'probe'],'compile-Rust')
def box(kind,data):return struct.pack('>I',len(data)+8)+kind+data
signature=b'\0\0\0\x0cJXL \r\n\x87\n';ftyp=box(b'ftyp',b'jxl \0\0\0\0jxl ')
manifest_path=fixtures/'jxl-progressive-fixtures.json';old=json.loads(manifest_path.read_text()) if manifest_path.exists() else None
report={'base_sha':'6a48db924b338c5ae7095bea6105fb4dbd436e95','scope':'Trusted small fixture evaluation; no production API, input/metadata/codec workspace qualification or inside-call cancellation claim','C_libjxl':'0.11.2','jxl_rs':'0.7.4 defaults=false','cases':[],'negatives':[],'root_lock_sha256':sha((root/'Cargo.lock').read_bytes()),'rustc':run(['rustc','-vV'],'rustc').stdout}
manifest={'oracle':'Independent C libjxl0.11.2 RGBA16 little endian, relative sRGB reconstruction; RGB<=3u16 against trusted pure Rust core, final managed RGB<=2u8 and alpha exact','construction':'32x24 deterministic RGB8 VarDCT, distance1, effort3, progressive AC and quantized AC enabled; DC0 or DC1. Count actual coded frame passes with the pinned header parser. Containers are structural wrappers around the same C codestream.','cases':[]}
for name,dc in [('ac',0),('dc',1)]:
 rawpath=fixtures/f'jxl-progressive-{name}.jxl'
 if a.write_fixtures:run([out/'fixture','encode',rawpath,dc],'encode-'+name)
 raw=rawpath.read_bytes();assert raw[:2]==b'\xff\x0a'
 parts=[raw[:7],raw[7:len(raw)//2],raw[len(raw)//2:]]
 wrappers={'raw':raw,'jxlc':signature+ftyp+box(b'jxlc',raw),'jxlp':signature+ftyp+b''.join(box(b'jxlp',struct.pack('>I',i|(0x80000000 if i==2 else 0))+part) for i,part in enumerate(parts)),'trailer':signature+ftyp+box(b'jxlc',raw)+box(b'xml ',b'<x:xmpmeta xmlns:x="adobe:ns:meta/">bounded</x:xmpmeta>')}
 run([out/'fixture','decode',rawpath,out/(name+'.C.rgba16le')],'C-'+name)
 oracle=(out/(name+'.C.rgba16le')).read_bytes();assert len(oracle)==32*24*8
 oraclepath=fixtures/f'jxl-progressive-{name}.rgba16le'
 if a.write_fixtures:oraclepath.write_bytes(oracle)
 else:assert oraclepath.read_bytes()==oracle,'C oracle changed'
 for kind,data in wrappers.items():
  label=name+'-'+kind;path=rawpath if kind=='raw' else fixtures/f'jxl-progressive-{label}.jxl'
  if a.write_fixtures:path.write_bytes(data)
  else:assert path.read_bytes()==data,'container changed'
  if not a.write_fixtures:assert old and next(c for c in old['cases'] if c['name']==label)['sha256']==sha(data),'input changed'
  independent=out/(label+'.C.rgba16le');run([out/'fixture','decode',path,independent],'C-'+label);assert independent.read_bytes()==oracle
  r=run([out/'probe',path,out/(label+'.rgba16le')],'Rust-'+label)
  stats=json.loads(next(line for line in r.stdout.splitlines() if line.startswith('{')));policy=next(line for line in r.stdout.splitlines() if line.startswith('policy '))
  actual=(out/(label+'.rgba16le')).read_bytes();assert len(actual)==len(oracle);words=struct.unpack('<'+'H'*(len(oracle)//2),oracle);values=struct.unpack('<'+'H'*(len(actual)//2),actual);diff=max(abs(v-w) for v,w in zip(values,words));assert diff<=3,(label,diff)
  managed=(out/(label+'.rgba16le.srgb.rgba')).read_bytes();assert len(managed)==32*24*4;expected=bytes((v*255+32767)//65535 for v in words);rgb=max(abs(v-w) for i,(v,w) in enumerate(zip(managed,expected)) if i%4!=3);alpha=all(v==w for i,(v,w) in enumerate(zip(managed,expected)) if i%4==3);assert rgb<=2 and alpha
  assert stats['bytewise_updates']>=2 and stats['provisional_different_from_final']>=2 and stats['successive_different_updates']>=1
  if kind=='raw':
   headers=[line for line in r.stdout.splitlines() if line.startswith('coded ')];assert any('passes=2' in line for line in headers);assert len(headers)==1+dc
  entry={'name':label,'file':path.name,'sha256':sha(data),'oracle':oraclepath.name,'oracle_sha256':sha(oracle)};manifest['cases'].append(entry)
  report['cases'].append(dict(**entry,**stats,native_max_difference=diff,managed_RGB_max_difference=rgb,managed_alpha_exact=alpha,policy=policy,core_trailing_observation=next(line for line in r.stdout.splitlines() if line.startswith('trailing ')),headers=[line for line in r.stdout.splitlines() if line.startswith('coded ')]))
 # Late invalid envelopes, not arbitrary entropy corruption claimed detected.
 bad={'raw-truncated':raw[:-1],'raw-junk':raw+b'\xff','jxlc-truncated':wrappers['jxlc'][:-1],'jxlp-truncated':wrappers['jxlp'][:-1],'jxlp-order':signature+ftyp+box(b'jxlp',struct.pack('>I',0x80000001)+raw),'trailer-short':wrappers['jxlc']+b'\0\0\0\x10xml','trailer-oversize':wrappers['jxlc']+box(b'xml ',b'a')[:-1]}
 for kind,data in bad.items():
  label=name+'-'+kind;path=out/(label+'.bad.jxl');path.write_bytes(data);prefix=out/(label+'.negative');r=run([out/'probe',path,prefix,'--negative'],'negative-Rust-'+label);assert not list(out.glob(prefix.name+'*')),'negative published files'
  c=run([out/'fixture','decode',path,out/(label+'.negative-C.rgba16le')],'negative-C-'+label,False)
  report['negatives'].append({'name':label,'WML2_rejected':True,'C_rejected':c.returncode!=0,'core_observation':r.stdout.strip(),'output_files':0})
if a.write_fixtures:manifest_path.write_text(json.dumps(manifest,indent=2)+'\n')
else:assert old==manifest,'manifest changed'
report['total_two_chunk_boundaries']=sum(c['all_two_chunk_boundaries'] for c in report['cases']);report['decision']='HOLD production resumable API until bounded incremental preflight, container+EOF final validation, owned provisional publication/backpressure and cumulative live codec+metadata+snapshot accounting are qualified'
report['source_sha256']={f.name:sha(f.read_bytes()) for f in [scripts/'jpegxl_progressive_fixture.c',scripts/'jpegxl_progressive_probe.rs',pathlib.Path(__file__).resolve(),root/'wml2/tests/common/jpegxl_progressive.rs']}
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
