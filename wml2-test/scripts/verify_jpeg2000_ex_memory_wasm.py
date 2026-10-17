#!/usr/bin/env python3
"""Compare all 400 native-owned JPEG2000 EX fixtures with callbacks in actual WASM."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work-dir',type=Path,required=True)
    parser.add_argument('--default-features',action='store_true')
    args=parser.parse_args();work=args.work_dir.resolve();assert work.name.startswith('.test');work.mkdir(exist_ok=True)
    repo=Path(__file__).resolve().parents[2];fixtures=repo/'wml2/tests/fixtures/review';source=Path(__file__).with_name('jpeg2000_ex_memory_checks.rs')
    entries=[]
    for name in ['gray','color','97']:
        for line in (fixtures/f'jpeg2000-ex-{name}-manifest.txt').read_text().splitlines():
            f=line.split();nc=1 if name=='gray' else int(f[5]);extra=name=='97' and f[0].endswith('_edges_mct0.jp2')
            entries.append('Case{data:include_bytes!('+json.dumps(str(fixtures/f[0]))+f'),bits:{f[2]},w:{f[3]},h:{f[4]},nc:{nc},extra:{str(extra).lower()}}},')
    rejected=[]
    for name in ['gray','color']:
        rejected += ['include_bytes!('+json.dumps(str(fixtures/f['input']))+'),'for f in json.loads((fixtures/f'jpeg2000-ex-{name}-rejected.json').read_text())]
    (work/'Cargo.toml').write_text('[package]\nname="jpeg2000-ex-memory-wasm-check"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\ncrate-type=["cdylib"]\n[dependencies]\n'+f'wml2={{path={json.dumps(str(repo/"wml2"))},default-features={str(args.default_features).lower()},features=["jpeg2000","image-buffer-ex"]}}\n'+'bin-rs={version="0.0.10",features=["util"]}\n')
    (work/'lib.rs').write_text('#[path='+json.dumps(str(source))+'] mod checks;\n'+r'''
struct Case{data:&'static[u8],bits:u8,w:usize,h:usize,nc:usize,extra:bool}
static CASES:&[Case]=&[ENTRIES];
static REJECTED:&[&[u8]]=&[REJECTS];
#[unsafe(no_mangle)] pub extern "C" fn count()->u32{CASES.len() as u32}
#[unsafe(no_mangle)] pub extern "C" fn run(i:u32){
 let c=&CASES[i as usize];checks::native(c.data,c.bits,c.nc);
 if c.extra{checks::conversions_and_limits(c.data,c.bits,c.nc,c.w,c.h);checks::origins(c.data,c.bits,c.nc);}
}
#[unsafe(no_mangle)] pub extern "C" fn reject()->u32{
 use wml2::draw::*;
 for data in REJECTED{
  assert!(image_from_ex(data,PixelFormatEx::Rgba16,PrecisionConversion::Round).is_err());
  checks::compare(data,PixelFormatEx::Rgba16,PrecisionConversion::Round,wml2::limits::DecodeLimits::default());
 }REJECTED.len() as u32
}
'''.replace('ENTRIES','\n'.join(entries)).replace('REJECTS','\n'.join(rejected)))
    shutil.copyfile(repo/'Cargo.lock',work/'Cargo.lock');env=dict(os.environ);env['CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS']='true';env.setdefault('CARGO_TARGET_DIR',str(work/'.test-target'))
    subprocess.run(['cargo','build','--offline','--release','--target','wasm32-unknown-unknown'],cwd=work,env=env,check=True)
    wasm=Path(env['CARGO_TARGET_DIR'])/'wasm32-unknown-unknown/release/jpeg2000_ex_memory_wasm_check.wasm'
    (work/'run.cjs').write_text('''const fs=require('fs');(async()=>{const bytes=fs.readFileSync(process.argv[2]);const module_=new WebAssembly.Module(bytes);const imports=WebAssembly.Module.imports(module_);if(imports.length)throw Error('unexpected imports');const {instance}=await WebAssembly.instantiate(bytes,{});const e=instance.exports;const n=e.count();for(let i=0;i<n;i++){try{e.run(i)}catch(err){throw Error(`case ${i}: ${err}`)}}console.log(JSON.stringify({runtime:process.version,passed:n,rejected:e.reject(),host_imports:imports}));})().catch(e=>{console.error(e);process.exit(1)});''')
    result=subprocess.run(['node',str(work/'run.cjs'),str(wasm)],check=True,capture_output=True,text=True);report=json.loads(result.stdout);assert report['passed']==400 and report['rejected']==12
    report.update(default_features=args.default_features,release_overflow_checks=True,wasm_sha256=hashlib.sha256(wasm.read_bytes()).hexdigest(),source_sha256={str(p.relative_to(repo)):hashlib.sha256(p.read_bytes()).hexdigest()for p in [source,repo/'wml2/src/jpeg2000.rs',repo/'wml2/src/jpeg2000_general.rs',repo/'wml2/src/draw.rs',repo/'wml2/src/decode_guard.rs',repo/'wml2/src/image_buffer_ex.rs']})
    (work/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))


if __name__=='__main__':main()
