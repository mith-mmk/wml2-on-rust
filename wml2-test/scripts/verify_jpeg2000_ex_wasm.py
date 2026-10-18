#!/usr/bin/env python3
"""Execute all JPEG2000 EX source/PGX fixture checks in actual wasm32/Node."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--work-dir', type=Path, required=True)
    p.add_argument('--default-features', action='store_true')
    p.add_argument('--overflow-checks', action='store_true')
    args = p.parse_args()
    work = args.work_dir.resolve()
    assert work.name.startswith('.test')
    work.mkdir(exist_ok=True)
    repo = Path(__file__).resolve().parents[2]
    fixtures = repo / 'wml2/tests/fixtures/review'
    source = Path(__file__).with_name('jpeg2000_ex_checks.rs')
    cases = []
    for line in (fixtures / 'jpeg2000-ex-gray-manifest.txt').read_text().splitlines():
        file, reference, bits, w, h = line.split()
        cases.append((file, reference, int(bits), int(w), int(h)))
    entries = '\n'.join('Case{input:include_bytes!(' + json.dumps(str(fixtures / f)) + '),expected:include_bytes!('
                        + json.dumps(str(fixtures / r)) + f'),bits:{b},w:{w},h:{h}}},' for f, r, b, w, h in cases)
    rejected = '\n'.join('include_bytes!(' + json.dumps(str(fixtures / r['input'])) + '),' for r in
                         json.loads((fixtures / 'jpeg2000-ex-gray-rejected.json').read_text()))
    (work / 'Cargo.toml').write_text(
        '[package]\nname="jpeg2000-ex-wasm-check"\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[lib]\npath="lib.rs"\ncrate-type=["cdylib"]\n[dependencies]\n'
        f'wml2={{path={json.dumps(str(repo / "wml2"))},default-features={str(args.default_features).lower()},'
        'features=["jpeg2000","image-buffer-ex"]}\n'
        'bin-rs={version="0.0.10",features=["util"]}\n')
    (work / 'lib.rs').write_text('#[path=' + json.dumps(str(source)) + '] mod checks;\n' + r'''
struct Case { input: &'static [u8], expected: &'static [u8], bits: u8, w: usize, h: usize }
static CASES: &[Case] = &[ENTRIES];
static REJECTED: &[&[u8]] = &[REJECTS];
#[unsafe(no_mangle)] pub extern "C" fn count() -> u32 { CASES.len() as u32 }
#[unsafe(no_mangle)] pub extern "C" fn run(i: u32) {
    let c = &CASES[i as usize];
    checks::pixels(c.input,c.expected,c.w,c.h,c.bits);
    if i == 8 || i == 48 {
        checks::callbacks_and_limits(c.input,c.expected,c.w,c.h,c.bits);
        checks::geometry_boundaries(c.input,c.expected,c.w,c.h,c.bits);
        checks::large_x_origin_pixels(c.input,c.expected,c.w,c.h,c.bits);
    }
}
#[unsafe(no_mangle)] pub extern "C" fn reject() -> u32 {
    for data in REJECTED {
        let mut p = checks::Probe::new(16);
        assert!(checks::decode(data,&mut p,wml2::limits::DecodeLimits::default()).is_err());
        assert!(p.events.is_empty());
    }
    REJECTED.len() as u32
}
'''.replace('ENTRIES', entries).replace('REJECTS', rejected))
    shutil.copyfile(repo / 'Cargo.lock', work / 'Cargo.lock')
    env = dict(os.environ)
    if args.overflow_checks:
        env['CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS'] = 'true'
    env.setdefault('CARGO_TARGET_DIR', str(work / '.test-target'))
    subprocess.run(['cargo', 'build', '--offline', '--release', '--target', 'wasm32-unknown-unknown'], cwd=work, env=env, check=True)
    wasm = Path(env['CARGO_TARGET_DIR']) / 'wasm32-unknown-unknown/release/jpeg2000_ex_wasm_check.wasm'
    (work / 'run.cjs').write_text('''const fs=require('fs'); (async()=>{
const bytes=fs.readFileSync(process.argv[2]); const module_=new WebAssembly.Module(bytes);
const imports=WebAssembly.Module.imports(module_); if(imports.length)throw Error('unexpected imports');
const {instance}=await WebAssembly.instantiate(bytes,{});const e=instance.exports;
const n=e.count(); for(let i=0;i<n;i++){try{e.run(i)}catch(error){throw Error(`case ${i}: ${error}`)}}
const rejected=e.reject();console.log(JSON.stringify({runtime:process.version,passed:n,rejected,host_imports:imports}));
})().catch(e=>{console.error(e);process.exit(1)});''')
    run = subprocess.run(['node', str(work / 'run.cjs'), str(wasm)], check=True, capture_output=True, text=True)
    report = json.loads(run.stdout)
    report['default_features'] = args.default_features
    report['release_overflow_checks'] = env.get('CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS') == 'true'
    report['source_sha256'] = {str(path.relative_to(repo)): hashlib.sha256(path.read_bytes()).hexdigest() for path in
                               [source, repo / 'wml2/src/jpeg2000.rs', repo / 'wml2/src/jpeg2000_general.rs', repo / 'wml2/src/draw.rs']}
    report['wasm_sha256'] = hashlib.sha256(wasm.read_bytes()).hexdigest()
    (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
