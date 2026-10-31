#!/usr/bin/env python3
"""Execute finite JXL resource APIs with real 32/64 MiB WASM linear ceilings."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if not args.output.name.startswith('.test'):parser.error('--output must start with .test')
    root=Path(__file__).resolve().parents[2];out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    target=out/'.test-target'
    def run(command,log):
        result=subprocess.run(list(map(str,command)),cwd=root,capture_output=True,text=True)
        (out/log).write_text(result.stdout+result.stderr);result.check_returncode();return result.stdout
    run(['cargo','build','-p','wml2','--locked','--release','--target','wasm32-unknown-unknown','--target-dir',target,'--no-default-features','--features','jpeg-xl,image-buffer-ex,color-management'],'build.log')
    deps=target/'wasm32-unknown-unknown/release';harness=Path(__file__).with_name('jpegxl_resources_wasm.rs')
    runner=out/'run.cjs'
    runner.write_text('''const fs=require('fs');(async()=>{const m=await WebAssembly.compile(fs.readFileSync(process.argv[2]));const imports=WebAssembly.Module.imports(m);if(imports.length)throw Error('unexpected imports');const i=await WebAssembly.instantiate(m,{});const cases=i.exports.verify_jxl_resources();if(cases!==12)throw Error('case count '+cases);const memory=i.exports.memory;if(memory.buffer.byteLength>Number(process.argv[3]))throw Error('already above ceiling');let enforced=false;try{memory.grow(Number(process.argv[3])/65536-memory.buffer.byteLength/65536+1);}catch(e){enforced=e instanceof RangeError;}if(!enforced)throw Error('linear ceiling not enforced');console.log(JSON.stringify({cases,passed:true,runtime:process.version,memory_bytes:memory.buffer.byteLength,linear_ceiling_enforced:enforced,imports}));})().catch(e=>{console.error(e);process.exit(1)});''')
    results=[]
    for mb in [32,64]:
        wasm=out/f'verify-{mb}.wasm'
        run(['rustc','--edition','2024','--target','wasm32-unknown-unknown','--crate-type','cdylib','-C','opt-level=3','-C','panic=abort','-C',f'link-arg=--max-memory={mb<<20}',harness,'--extern',f'wml2={deps/"libwml2.rlib"}','-L',f'dependency={deps/"deps"}','-L',f'dependency={target/"release/deps"}','-o',wasm],f'link-{mb}.log')
        result=json.loads(run(['node',runner,wasm,mb<<20],f'run-{mb}.log'))
        result.update(linear_ceiling_bytes=mb<<20,sha256=hashlib.sha256(wasm.read_bytes()).hexdigest());results.append(result)
    report={'scope':'finite ordinary/CMS resource APIs on small independent fixtures, every cancellation checkpoint, fixed-cap large-input refusal; no accepted-large-image WASM or global allocator bound claim','rustc':subprocess.check_output(['rustc','-V'],text=True).strip(),'results':results}
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))


if __name__=='__main__':main()
