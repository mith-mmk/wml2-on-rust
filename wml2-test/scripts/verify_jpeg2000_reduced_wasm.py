#!/usr/bin/env python3
"""Run independent native-resolution JPEG2000 references in actual32-bit WASM."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work-dir', type=Path, required=True)
    parser.add_argument('--default-features', action='store_true')
    parser.add_argument('--legacy-only', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    fixtures = root / 'wml2/tests/fixtures/review'
    work = args.work_dir.resolve()
    assert work.name.startswith('.test')
    work.mkdir(exist_ok=True)
    checks = Path(__file__).with_name('jpeg2000_reduced_checks.rs')
    entries = []
    for line in (fixtures/'jpeg2000-reduced-manifest.txt').read_text().splitlines():
        name, offset, length, bits, nc, reduce, w, h, reversible = line.split()
        if args.legacy_only and bits != '8':
            continue
        entries.append('Case{data:include_bytes!('+json.dumps(str(fixtures/name))+f'),offset:{offset},length:{length},bits:{bits},nc:{nc},reduce:{reduce},w:{w},h:{h},reversible:{reversible}'+'},')
    zero = []
    for line in (fixtures/'jpeg2000-general-manifest.txt').read_text().splitlines():
        name = line.split()[0]
        zero.append('Zero{data:include_bytes!('+json.dumps(str(fixtures/name))+'),bits:8,nc:0},')
    if not args.legacy_only:
        for group in ['gray','color','97']:
            for line in (fixtures/f'jpeg2000-ex-{group}-manifest.txt').read_text().splitlines():
                fields=line.split(); nc='1' if group=='gray' else fields[5]
                zero.append('Zero{data:include_bytes!('+json.dumps(str(fixtures/fields[0]))+f'),bits:{fields[2]},nc:{nc}'+'},')
    extra = []
    for bits in ([8] if args.legacy_only else [8,12,16]):
        for nc in [1,4]:
            for wavelet in ['53','97']:
                extra.append('Zero{data:include_bytes!('+json.dumps(str(fixtures/f'jp2_reduced_odd_{bits}b_{nc}c_{wavelet}.jp2'))+f'),bits:{bits},nc:{nc}'+'},')
    cargo = '[package]\nname="jpeg2000-reduced-wasm-check"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[features]\ndefault=["image-buffer-ex"]\nimage-buffer-ex=["wml2/image-buffer-ex"]\n[lib]\npath="lib.rs"\ncrate-type=["cdylib"]\n[dependencies]\n'
    cargo += 'wml2={path='+json.dumps(str(root/'wml2'))+',default-features='+str(args.default_features).lower()+',features=["jpeg2000"]}\nbin-rs={version="0.0.10",features=["util"]}\n'
    position_fields=(fixtures/'jpeg2000-reduced-packet-positions.txt').read_text().split()
    position_values=dict(zip(['eph','payload_start','payload_end','plt','tlm_length','sot_length'],position_fields[1:]))
    packet_constants='static PACKET:&[u8]=include_bytes!('+json.dumps(str(fixtures/position_fields[0]))+');\n'
    packet_constants+='const POS:checks::PacketPositions=checks::PacketPositions{'+','.join(k+':'+v for k,v in position_values.items())+'};\n'
    for symbol,name in [('PPT','jp2_reduced_ppt_multi_part_deferred.j2k'),('MCT','jp2_reduced_mixed_coc_mct_unsupported.j2k'),('EMPTY','jp2_reduced_empty_global.j2k')]:
        packet_constants+='static '+symbol+':&[u8]=include_bytes!('+json.dumps(str(fixtures/name))+');\n'
    (work/'Cargo.toml').write_text(cargo)
    (work/'lib.rs').write_text('#[path='+json.dumps(str(checks))+']mod checks;\n'+packet_constants+r'''
struct Case{data:&'static[u8],offset:usize,length:usize,bits:u8,nc:usize,reduce:u8,w:usize,h:usize,reversible:u8}
struct Zero{data:&'static[u8],bits:u8,nc:usize}
static CASES:&[Case]=&[ENTRIES];
static ZERO:&[Zero]=&[ZEROS];
static EXTRA:&[Zero]=&[EXTRAS];
static REFS:&[u8]=include_bytes!(REFPATH);
#[unsafe(no_mangle)]pub extern "C" fn count()->u32{CASES.len()as u32}
#[unsafe(no_mangle)]pub extern "C" fn bits(i:u32)->u32{u32::from(CASES[i as usize].bits)}
#[unsafe(no_mangle)]pub extern "C" fn reversible(i:u32)->u32{u32::from(CASES[i as usize].reversible)}
#[unsafe(no_mangle)]pub extern "C" fn run(i:u32)->u32{
 let c=&CASES[i as usize];let _=c.nc;let reference=&REFS[c.offset..c.offset+c.length];
 if c.bits==8{return u32::from(checks::legacy_pixels(c.data,reference,c.reduce,c.w,c.h).maximum);}
 #[cfg(feature="image-buffer-ex")]{return u32::from(checks::ex_pixels(c.data,reference,c.reduce,c.w,c.h,c.bits,c.nc).maximum);}
 #[cfg(not(feature="image-buffer-ex"))]panic!("unexpected high-depth case");
}
#[unsafe(no_mangle)]pub extern "C" fn zero_count()->u32{ZERO.len()as u32}
#[unsafe(no_mangle)]pub extern "C" fn zero(i:u32){
 let c=&ZERO[i as usize];if c.bits==8{checks::legacy_zero(c.data);}
 #[cfg(feature="image-buffer-ex")]if c.bits!=8{checks::ex_zero(c.data,c.bits,c.nc);}
}
#[unsafe(no_mangle)]pub extern "C" fn extra()->u32{
 for c in EXTRA{
  if c.bits==8{checks::legacy_contracts(c.data,1);}
  #[cfg(feature="image-buffer-ex")]if c.bits!=8{checks::ex_contracts(c.data,1,c.bits,c.nc);}
  checks::shifted_origins(c.data,1,c.bits,c.nc);
  checks::unlimited_geometry_overflow(c.data,c.bits,c.nc);
  for reduce in [4,32,33,255]{checks::rejects_before_callbacks(c.data,reduce,c.bits);}
  for end in [0,1,2,16,c.data.len()/2,c.data.len()-2,c.data.len()-1]{checks::rejects_before_callbacks(&c.data[..end],1,c.bits);}
 }EXTRA.len()as u32
}
#[unsafe(no_mangle)]pub extern "C" fn packet()->u32{checks::packet_structure(PACKET,&POS);checks::deferred_profiles(PPT,MCT,EMPTY);1}
'''.replace('ENTRIES','\n'.join(entries)).replace('ZEROS','\n'.join(zero)).replace('EXTRAS','\n'.join(extra)).replace('REFPATH',json.dumps(str(fixtures/'jpeg2000-reduced-refs.bin'))))
    shutil.copyfile(root/'Cargo.lock',work/'Cargo.lock')
    env=dict(os.environ)
    env['CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS']='true'
    env.setdefault('CARGO_TARGET_DIR',str(work/'.test-target'))
    command=['cargo','build','--offline','--release','--target','wasm32-unknown-unknown']
    if args.legacy_only:command.append('--no-default-features')
    subprocess.run(command,cwd=work,env=env,check=True)
    wasm=Path(env['CARGO_TARGET_DIR'])/'wasm32-unknown-unknown/release/jpeg2000_reduced_wasm_check.wasm'
    (work/'run.cjs').write_text('''const fs=require('fs');(async()=>{const bytes=fs.readFileSync(process.argv[2]);const module_=new WebAssembly.Module(bytes);const imports=WebAssembly.Module.imports(module_);if(imports.length)throw Error('unexpected imports');const {instance}=await WebAssembly.instantiate(bytes,{});const e=instance.exports;const n=e.count(),maxima={};for(let i=0;i<n;i++){try{const m=e.run(i),b=e.bits(i);if(e.reversible(i)&&m)throw Error(`reversible error ${m}`);maxima[b]=Math.max(maxima[b]||0,m)}catch(err){throw Error(`reference ${i}: ${err}`)}}const zero=e.zero_count();for(let i=0;i<zero;i++){try{e.zero(i)}catch(err){throw Error(`zero ${i}: ${err}`)}}console.log(JSON.stringify({runtime:process.version,passed:n,zero_comparisons:zero,extra_contract_cases:e.extra(),packet_contracts:e.packet(),maxima,host_imports:imports}));})().catch(e=>{console.error(e);process.exit(1)});''')
    result=subprocess.run(['node',str(work/'run.cjs'),str(wasm)],check=True,capture_output=True,text=True)
    report=json.loads(result.stdout)
    assert report['passed']==len(entries) and report['zero_comparisons']==len(zero) and report['extra_contract_cases']==len(extra) and report['packet_contracts']==1
    files=[checks,root/'wml2/src/jpeg2000.rs',root/'wml2/src/jpeg2000_general.rs',fixtures/'jpeg2000-reduced-manifest.txt',fixtures/'jpeg2000-reduced-refs.bin']
    shutil.copyfile(wasm,work/'verified.wasm')
    report.update(default_features=args.default_features,legacy_only=args.legacy_only,release_overflow_checks=True,wasm_sha256=hashlib.sha256(wasm.read_bytes()).hexdigest(),source_sha256={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest()for p in files})
    (work/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':main()
