#!/usr/bin/env python3
"""Build and run independent JPEG XL fixtures in actual wasm32 with Node."""
import argparse,hashlib,json,pathlib,subprocess
p=argparse.ArgumentParser();p.add_argument('--exif', action='store_true');p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args()
if not a.output.name.startswith('.test'):p.error('--output must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
target=out/'.test-target'
def run(cmd,log):
 r=subprocess.run([str(x) for x in cmd],cwd=root,capture_output=True,text=True);(out/log).write_text(r.stdout+r.stderr);r.check_returncode();return r.stdout
run(['cargo','build','-p','wml2','--locked','--release','--target','wasm32-unknown-unknown','--target-dir',target,'--no-default-features','--features','jpeg-xl,image-buffer-ex'+(',exif' if a.exif else '')],'build.log')
deps=target/'wasm32-unknown-unknown/release';wasm=out/'verify.wasm';harness=pathlib.Path(__file__).with_name('jpegxl_wasm.rs')
run(['rustc','--edition','2024','--target','wasm32-unknown-unknown','--crate-type','cdylib','-C','opt-level=3','-C','panic=abort',harness,*(['--cfg','feature="exif"'] if a.exif else []),'--extern',f'wml2={deps/"libwml2.rlib"}','-L',f'dependency={deps/"deps"}','-L',f'dependency={target/"release/deps"}','-o',wasm],'harness.log')
runner=out/'run.cjs';runner.write_text('''const fs=require('fs');
(async()=>{const module=await WebAssembly.compile(fs.readFileSync(process.argv[2]));
const imports=WebAssembly.Module.imports(module);if(imports.length)throw Error('unexpected host imports');
const instance=await WebAssembly.instantiate(module,{});const cases=instance.exports.verify_jpegxl();
if(cases!==73)throw Error('case count mismatch: '+cases);
process.stdout.write(JSON.stringify({runtime:process.version,cases,all_passed:true,host_imports:imports,memory_bytes:instance.exports.memory.buffer.byteLength})+'\\n');
})().catch(e=>{console.error(e);process.exit(1)});
''')
report=json.loads(run(['node',runner,wasm],'run.log'))
report.update(target='wasm32-unknown-unknown',features='jpeg-xl,image-buffer-ex'+(',exif' if a.exif else '')+'; no defaults',wasm_sha256=hashlib.sha256(wasm.read_bytes()).hexdigest(),scope='structured XMP and optional normalized EXIF (LE/BE/offset/brob/multiple), plus bounded brob raw/decoded metadata, strict rejection and input/metadata/workspace limits; additional normalized9/10-bit promotion with source-code recovery, native Gray/GrayAlpha/RGB/RGBA8/12/16 and RGBA8 rounding, pixel limits, RGB/Gray embedded ICC and metadata limits, lossy djxl comparison, jxll levels5/10 with jxlc/jxlp and opaque jbrd, official jbrd still images (25 spatial pixel probes each, full ICC, dimensions, limits, ordinary Gray CMS refusal)')
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
