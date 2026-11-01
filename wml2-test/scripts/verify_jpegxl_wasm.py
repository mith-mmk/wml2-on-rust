#!/usr/bin/env python3
"""Build and run independent JPEG XL fixtures in actual wasm32 with Node."""
import argparse,hashlib,json,pathlib,subprocess
p=argparse.ArgumentParser();p.add_argument('--exif', action='store_true');p.add_argument('--f16',action='store_true');p.add_argument('--hdr-encoded',action='store_true');p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args()
if a.f16 and a.hdr_encoded:p.error('choose one specialized harness')
if not a.output.name.startswith('.test'):p.error('--output must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
target=out/'.test-target'
def run(cmd,log):
 r=subprocess.run([str(x) for x in cmd],cwd=root,capture_output=True,text=True);(out/log).write_text(r.stdout+r.stderr);r.check_returncode();return r.stdout
build=run(['cargo','build','-p','wml2','--locked','--release','--target','wasm32-unknown-unknown','--target-dir',target,'--no-default-features','--features','jpeg-xl,image-buffer-ex'+(',exif' if a.exif else ''),'--message-format=json-render-diagnostics'],'build.log')
deps=target/'wasm32-unknown-unknown/release';wasm=out/'verify.wasm';harness=pathlib.Path(__file__).with_name('jpegxl_encoded_hdr_wasm.rs' if a.hdr_encoded else ('jpegxl_f16_wasm.rs' if a.f16 else 'jpegxl_wasm.rs'))
# Use the actual Cargo artifact, never Rust std's private miniz crate.
miniz={pathlib.Path(f) for line in build.splitlines() if line.startswith('{')
       for artifact in [json.loads(line)] if artifact.get('reason')=='compiler-artifact' and artifact.get('target',{}).get('name')=='miniz_oxide'
       for f in artifact['filenames'] if f.endswith('.rlib') and pathlib.Path(f).parent==deps/'deps'}
if len(miniz)!=1:raise RuntimeError('expected one target miniz artifact: '+str(miniz))
run(['rustc','--edition','2024','--target','wasm32-unknown-unknown','--crate-type','cdylib','-C','opt-level=3','-C','panic=abort',*(['-C','link-arg=--max-memory=33554432'] if (a.f16 or a.hdr_encoded) else []),harness,*(['--cfg','feature="exif"'] if a.exif else []),'--extern',f'wml2={deps/"libwml2.rlib"}','--extern',f'miniz_oxide={next(iter(miniz))}','-L',f'dependency={deps/"deps"}','-L',f'dependency={target/"release/deps"}','-o',wasm],'harness.log')
runner=out/'run.cjs';runner.write_text('''const fs=require('fs');
(async()=>{const module=await WebAssembly.compile(fs.readFileSync(process.argv[2]));
const imports=WebAssembly.Module.imports(module);if(imports.length)throw Error('unexpected host imports');
const instance=await WebAssembly.instantiate(module,{});const cases=instance.exports.verify_jpegxl();
if(cases!==615)throw Error('case count mismatch: '+cases);
process.stdout.write(JSON.stringify({runtime:process.version,cases,all_passed:true,host_imports:imports,memory_bytes:instance.exports.memory.buffer.byteLength})+'\\n');
})().catch(e=>{console.error(e);process.exit(1)});
''')
if a.f16 or a.hdr_encoded:
 runner.write_text("""const fs=require('fs');
const m=new WebAssembly.Module(fs.readFileSync(process.argv[2]));const imports=WebAssembly.Module.imports(m);if(imports.length)throw Error('unexpected host imports');
const i=new WebAssembly.Instance(m,{});const result=i.exports.run(),probes=i.exports.probes(),cases=i.exports.checks();let denied=false;try{i.exports.memory.grow(512)}catch(e){denied=e instanceof RangeError}
if(result!==0||probes<=10||cases!==22+probes||!denied)throw Error(JSON.stringify({result,probes,cases,denied}));
console.log(JSON.stringify({runtime:process.version,cases,probes,all_passed:true,host_imports:imports,memory_bytes:i.exports.memory.buffer.byteLength,ceiling_bytes:33554432,ceiling_rejected:denied}));
""")
if a.hdr_encoded:
 runner.write_text(runner.read_text().replace('cases!==22+probes','cases!==51+probes'))
report=json.loads(run(['node',runner,wasm],'run.log'))
report.update(target='wasm32-unknown-unknown',features='jpeg-xl,image-buffer-ex'+(',exif' if a.exif else '')+'; no defaults',wasm_sha256=hashlib.sha256(wasm.read_bytes()).hexdigest(),scope='Depth separate-plane output:50 independent C vectors with all integer depth pairs and orientations,52 success groups, combined output/exact workspace boundaries, all cancellation checkpoints, unsupported extras refusal; COMPOSED mixed alpha, intrinsic, preview and structured metadata: structured XMP and optional normalized EXIF (LE/BE/offset/brob/multiple), plus bounded brob raw/decoded metadata, strict rejection and input/metadata/workspace limits; declared2/4/8x straight+associated subsampled alpha (98 native/legacy cases, coded-plane metadata, exact workspace boundary, unsupported layers; association, Exact/Round policy), plus associated8/9/10/12/16 integer/mixed alpha explicit straight normalization (66 independent vectors including colored zero-alpha, all depth pairs, source depths, native and legacy) plus multi-layer refusal; additional normalized9/10-bit promotion with source-code recovery, native Gray/GrayAlpha/RGB/RGBA8/12/16 and RGBA8 rounding, pixel limits, RGB/Gray embedded ICC and metadata limits, lossy djxl comparison, jxll levels5/10 with jxlc/jxlp and opaque jbrd, official jbrd still images (25 spatial pixel probes each, full ICC, dimensions, limits, ordinary Gray CMS refusal)')
if a.f16:
 report['scope']='F16-source/F32-output:7 exact independent vectors,7 immediate aborts,5 refusals, all cancellation checkpoints, recovery and both F32 acceptance/strict-source compatibility checks; enforced32MiB memory ceiling'
 report['harness_sha256']=hashlib.sha256(harness.read_bytes()).hexdigest()
if a.hdr_encoded:
 report['scope']='Structured integer16 PQ/HLG encoded F32 preservation and explicit PQ-nits / HLG-scene linearization against70-digit Decimal oracle; in-place pointer, exact budgets, bad samples, mismatch and conversion cancellation; independent vectors/source metadata, refusals, legacy API compatibility, all cancellation probes, recovery and budgets;32MiB ceiling'
 report['harness_sha256']=hashlib.sha256(harness.read_bytes()).hexdigest()
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
