#!/usr/bin/env python3
"""Execute existing independent-oracle harnesses with bounded Node linear memory.
This is WASM execution, not Chromium display testing or total process bounds.
"""
import argparse,hashlib,json,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
if not a.output.name.startswith('.test'):p.error('--output must start with .test')
root=Path(__file__).resolve().parents[2];out=a.output.resolve()
# Never overwrite a checkout, another task's directory or unique user inputs.
owner=out/'.test-jxl-resource-owner';identity=str(root)+'/wasm-resource-v1'
if out==root or root.is_relative_to(out) or (out/'.git').exists():p.error('--output must not contain a checkout')
if out.exists() and any(out.iterdir()) and not owner.is_file():p.error('--output must be empty or owned by this probe')
if owner.exists() and owner.read_text()!=identity:p.error('--output belongs to another probe/check-out')
out.mkdir(parents=True,exist_ok=True);owner.write_text(identity);target=out/'.test-target'
def run(cmd,log):
 r=subprocess.run(list(map(str,cmd)),cwd=root,capture_output=True,text=True);(out/log).write_text(r.stdout+r.stderr);r.check_returncode();return r.stdout
features='jpeg-xl,image-buffer-ex,color-management'
run(['cargo','build','-p','wml2','--locked','--release','--target','wasm32-unknown-unknown','--target-dir',target,'--no-default-features','--features',features],'build.log')
deps=target/'wasm32-unknown-unknown/release'
for name,source in [('source-ex','jpegxl_wasm.rs'),('managed','jpegxl_browser.rs'),('typed-layer','jxl_resource_wasm.rs')]:
 wasm=out/(name+'.wasm')
 run(['rustc','--edition','2024','--target','wasm32-unknown-unknown','--crate-type','cdylib','-C','opt-level=3','-C','panic=abort',root/'wml2-test/scripts'/source,'--extern',f'wml2={deps/"libwml2.rlib"}','--extern','miniz_oxide='+str(next((deps/'deps').glob('libminiz_oxide-*.rlib'))),'-L',f'dependency={deps/"deps"}','-L',f'dependency={target/"release/deps"}','-o',wasm],name+'-build.log')
runner=out/'run.cjs';runner.write_text('''const fs=require('fs');
(async()=>{const module=await WebAssembly.compile(fs.readFileSync(process.argv[2]));
const imports=WebAssembly.Module.imports(module);if(imports.length)throw Error('unexpected host imports');
const single=process.argv[3]==='source-ex';const typed=process.argv[3]==='typed-layer';const records=[];
for(let i=0;i<(single?1:typed?2:14);i++){
 const {exports:e}=await WebAssembly.instantiate(module,{});const before=e.memory.buffer.byteLength;
 try {if(typed){const budget=i===0?256:512;const result=e.evaluate_layer(budget);const error=Buffer.from(new Uint8Array(e.memory.buffer,e.error_ptr(),e.error_len())).toString();records.push({case:'typed-layer',expanded_mib:budget,ok:result===0,returned_error:result===1,error,width:e.width(),height:e.height(),before,after:e.memory.buffer.byteLength});}
 else if(single){const n=e.verify_jpegxl();if(n!==61)throw Error('case count '+n);records.push({case:'61 source/EX oracle cases',ok:true,before,after:e.memory.buffer.byteLength});}
 else {e.render_case(i);records.push({case:i,ok:true,width:e.width(),height:e.height(),before,after:e.memory.buffer.byteLength});}}
 catch(error){records.push({case:single?'source-ex':typed?'typed-layer':i,expanded_mib:typed?(i===0?256:512):undefined,ok:false,before,after:e.memory.buffer.byteLength,width:e.width?.(),height:e.height?.(),error:String(error)});}
}
process.stdout.write(JSON.stringify({node:process.version,host_imports:imports,records,process_max_rss_kib:process.resourceUsage().maxRSS})+'\\n');
})().catch(e=>{console.error(e);process.exit(1)});
''')
def require_wasm_outcomes(name,pages,item):
 records=item['records']
 if name=='source-ex':
  # The source/EX reference harness must pass at both 64MiB and 32MiB.
  assert len(records)==1 and records[0].get('ok') is True,item
 elif name=='managed' and pages==1024:
  assert len(records)==14 and all(r.get('ok') is True for r in records),item
 elif name=='typed-layer':
  assert len(records)==2,item
  low,high=records
  assert low.get('expanded_mib')==256 and high.get('expanded_mib')==512,item
  assert low.get('ok') is False and low.get('returned_error') is True,item
  assert low.get('width')==0 and low.get('height')==0,item
  assert low.get('error')=='Out of memory: JPEG XL internal layer workspace exceeds decode limit',item
  if pages==1024:
   assert high.get('ok') is True and high.get('returned_error') is False,item
   assert (high.get('width'),high.get('height'))==(924,1386) and high.get('error')=='',item
  else:
   # This records one fixed fixture/engine allocation failure, not a hard
   # allocation guarantee for other streams, allocators or memory ceilings.
   assert pages==512,item
   assert high.get('ok') is False and high.get('returned_error') is True,item
   assert high.get('width')==0 and high.get('height')==0,item
   assert high.get('error')=='memory allocation failed because the memory allocator returned an error',item
report={'features':features,'target':'wasm32-unknown-unknown','scope':'fresh instance per managed case; existing Rust independent-oracle assertions; linear capacities are not live heap/RSS; Node memory ceiling applies to WASM pages only','artifacts':{},'runs':[]}
for name in ['source-ex','managed','typed-layer']:
 wasm=out/(name+'.wasm');report['artifacts'][name]={'bytes':wasm.stat().st_size,'sha256':hashlib.sha256(wasm.read_bytes()).hexdigest()}
 for pages in [1024,512]:
  item=json.loads(run(['node',f'--wasm-max-mem-pages={pages}',runner,wasm,name],name+f'-{pages}.log'));item.update(harness=name,linear_page_ceiling=pages,linear_byte_ceiling=pages*65536)
  require_wasm_outcomes(name,pages,item)
  report['runs'].append(item)
report['runtime_sha']=run(['git','rev-parse','HEAD'],'runtime-sha.log').strip()
report['runtime_source_sha256']=hashlib.sha256((root/'wml2/src/jpegxl.rs').read_bytes()).hexdigest()
report['harness_source_sha256']={name:hashlib.sha256((root/'wml2-test/scripts'/name).read_bytes()).hexdigest() for name in ['jpegxl_wasm.rs','jpegxl_browser.rs','jxl_resource_wasm.rs','evaluate_jxl_resources_wasm.py']}
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
