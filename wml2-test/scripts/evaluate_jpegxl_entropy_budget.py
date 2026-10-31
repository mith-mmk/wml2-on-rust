#!/usr/bin/env python3
"""Apply a hash-pinned *evaluation* patch in a fresh .test directory, never ship it."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = Path(__file__).resolve().parent
SPEC = ROOT / 'docs/jpeg-xl-entropy-budget-evaluation.json'

def sha(data):
    return hashlib.sha256(data).hexdigest()

def run(args, output, env, separate_stderr=False):
    with output.open('w') as log, output.with_suffix('.stderr.log').open('w') as err:
        result = subprocess.run(args, cwd=ROOT, env=env, stdout=log, stderr=err if separate_stderr else subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f'{args[0]} failed ({result.returncode}); see {output}')

def apply_patch(tree, data):
    """Strict unified-patch application: only manifest-pinned relative src paths."""
    lines = data.splitlines(keepends=True)
    i = 0
    while i < len(lines):
        old = lines[i].removeprefix('--- a/').rstrip('\n')
        assert lines[i].startswith('--- a/')
        new = lines[i+1].removeprefix('+++ b/').rstrip('\n')
        assert old == new and Path(new).parts[0] == 'src' and '..' not in Path(new).parts
        target = tree / new
        source = target.read_text().splitlines(keepends=True) if target.exists() else []
        dest, pos = [], 0
        i += 2
        while i < len(lines) and not lines[i].startswith('--- a/'):
            match = re.match(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@', lines[i])
            assert match, lines[i]
            start = int(match[1]) - (int(match[2] or 1) != 0)
            assert start >= pos
            dest.extend(source[pos:start]); pos = start
            i += 1
            removed = inserted = 0
            while i < len(lines) and lines[i][0] in ' +-':
                if lines[i].startswith('--- a/'):
                    break
                flag, content = lines[i][0], lines[i][1:]
                if flag in ' -':
                    assert pos < len(source) and source[pos] == content, (new, pos)
                    pos += 1; removed += 1
                if flag in ' +':
                    dest.append(content); inserted += 1
                i += 1
            assert removed == int(match[2] or 1) and inserted == int(match[4] or 1)
        dest.extend(source[pos:])
        target.write_text(''.join(dest), newline='\n')

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--output', type=Path, default=Path('.test-jxl-entropy-budget'))
    ap.add_argument('--target-dir', type=Path)
    ap.add_argument('--offline', action='store_true')
    ap.add_argument('--wasm', action='store_true')
    ap.add_argument('--wml-regression', action='store_true')
    args = ap.parse_args()
    out = args.output.resolve()
    if not out.name.startswith('.test'):
        raise ValueError('output must be a fresh .test* directory')
    out.mkdir(exist_ok=False)
    target = args.target_dir.resolve() if args.target_dir else out / '.test-target'
    if not target.name.startswith('.test'):
        raise ValueError('target directory must start .test')
    spec = json.loads(SPEC.read_text())
    env = dict(os.environ, WML2_ROOT=str(ROOT))
    cargo_home = Path(env.get('CARGO_HOME', str(Path.home() / '.cargo')))
    archives = list(cargo_home.glob('registry/cache/*/jxl-0.7.4.crate'))
    if not archives and not args.offline:
        run(['cargo','fetch','--locked'], out / 'fetch.log', env)
        archives = list(cargo_home.glob('registry/cache/*/jxl-0.7.4.crate'))
    archive = next((p for p in archives if sha(p.read_bytes()) == spec['source']['sha256']), None)
    if archive is None:
        raise RuntimeError('verified registry archive missing; cargo fetch --locked first')
    tree = out / '.test-codec'
    tree.mkdir()
    with tarfile.open(archive) as tar:
        for member in tar:
            if not member.isfile():
                continue
            parts = Path(member.name).parts
            assert parts[0] == 'jxl-0.7.4' and '..' not in parts
            dst = tree.joinpath(*parts[1:])
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.write_bytes(tar.extractfile(member).read())
    for name, expected in spec['files'].items():
        path = tree / name
        assert (sha(path.read_bytes()) if path.exists() else None) == expected['before'], name
    patch = (SCRIPTS / 'jxl_entropy_budget_074.patch').read_bytes()
    assert sha(patch) == spec['patch_sha256']
    apply_patch(tree, patch.decode())
    for name, expected in spec['files'].items():
        assert sha((tree / name).read_bytes()) == expected['after'], name
    harness = out / '.test-harness'
    harness.mkdir()
    quote = json.dumps
    (harness / 'Cargo.toml').write_text(f'''[package]
name="jxl-entropy-budget-evaluation"
version="0.0.0"
edition="2024"
rust-version="1.91"
[workspace]
[lib]
path={quote(str(SCRIPTS / 'jxl_entropy_budget_eval.rs'))}
crate-type=["rlib","cdylib"]
[dependencies]
jxl={{path={quote(str(tree))},default-features=false}}
''')
    shutil.copyfile(SCRIPTS / 'jxl_entropy_budget_eval.lock', harness / 'Cargo.lock')
    common = ['--manifest-path',str(harness/'Cargo.toml'),'--target-dir',str(target),'--locked']
    if args.offline:
        common += ['--offline']
    run(['cargo','test',*common], out / 'native.log', env)
    assert 'test result: ok. 14 passed' in (out/'native.log').read_text()
    report = {'scope':spec['scope'],'source_sha256':spec['source']['sha256'],
              'patch_sha256':sha(patch),'native_tests':14,'native_passed':True}
    if args.wasm:
        run(['cargo','rustc',*common,'--release','--target','wasm32-unknown-unknown','--lib',
             '--','-C','link-arg=--max-memory=67108864'], out / 'wasm-build.log', env)
        wasm = target / 'wasm32-unknown-unknown/release/jxl_entropy_budget_evaluation.wasm'
        runner = out / '.test-run.cjs'
        runner.write_text('''const fs=require('fs');
const moduleBytes=fs.readFileSync(process.argv[2]);const mod=new WebAssembly.Module(moduleBytes);
if(WebAssembly.Module.imports(mod).length)throw Error('unexpected WASM imports');
const e=new WebAssembly.Instance(mod,{}).exports;
if(e.evaluate_entropy_budget()!==6)throw Error('case group count');
let denied=false;try{e.memory.grow(1024);}catch(err){if(!(err instanceof RangeError))throw err;denied=true;}
if(!denied)throw Error('memory ceiling not enforced');
console.log(JSON.stringify({passed:true,case_groups:6,imports:[],linear_ceiling_bytes:67108864,linear_ceiling_enforced:denied,memory_bytes:e.memory.buffer.byteLength,node:process.version}));
''')
        run(['node',str(runner),str(wasm)], out / 'wasm-runtime.json', env)
        report['wasm'] = dict(json.loads((out/'wasm-runtime.json').read_text()), sha256=sha(wasm.read_bytes()))
    if args.wml_regression:
        wml = out / '.test-wml'
        wml.mkdir()
        stub = out / '.test-stub.rs'
        stub.write_text('// Evaluation harness only.\n')
        manifest = f'''[package]
name="jxl-entropy-wml-evaluation"
version="0.0.0"
edition="2024"
[workspace]
[lib]
path={quote(str(stub))}
[features]
default=["jpeg-xl","image-buffer-ex","color-management","high-bit-depth","png","exif"]
jpeg-xl=[]
image-buffer-ex=[]
color-management=[]
high-bit-depth=[]
png=[]
exif=[]
[dependencies]
wml2={{path={quote(str(ROOT/'wml2'))},default-features=false,features=["jpeg-xl","image-buffer-ex","color-management","png","exif"]}}
bin-rs={{version="0.0.10",features=["util"]}}
miniz_oxide="0.7.1"
[patch.crates-io]
jxl={{path={quote(str(tree))}}}
'''
        for name in ['jpeg_xl','jpeg_xl_additional','jpeg_xl_layers','jpeg_xl_brob','jpeg_xl_metadata','jpeg_xl_slice_admission','jpeg_xl_resources']:
            manifest += f'\n[[test]]\nname="{name}"\npath={quote(str(ROOT/"wml2/tests"/(name+".rs")))}\n'
        manifest += f'\n[[example]]\nname="review_bench"\npath={quote(str(ROOT/"wml2/examples/review_bench.rs"))}\n'
        (wml/'Cargo.toml').write_text(manifest)
        shutil.copyfile(ROOT/'Cargo.lock',wml/'Cargo.lock')
        cmd=['--manifest-path',str(wml/'Cargo.toml'),'--target-dir',str(target)]
        if args.offline:
            cmd+=['--offline']
        run(['cargo','metadata',*cmd[:2],*(['--offline'] if args.offline else []),'--format-version','1'],out/'wml-metadata.json',env,separate_stderr=True)
        locked={(p['name'],p['version']) for p in tomllib.loads((ROOT/'Cargo.lock').read_text())['package']}
        packages=json.loads((out/'wml-metadata.json').read_text())['packages']
        assert all((p['name'],p['version']) in locked for p in packages if p['source']), 'registry version drift'
        assert any(p['name']=='jxl' and p['source'] is None and Path(p['manifest_path']).parent==tree for p in packages)
        run(['cargo','test',*cmd,'--locked','--tests'],out/'wml-regression.log',env)
        run(['cargo','build',*cmd,'--locked','--release','--example','review_bench'],out/'wml-release.log',env)
        count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed', (out/'wml-regression.log').read_text())))
        assert count == 84
        report['wml_regression']={'integration_tests':count,'passed':True,'registry_versions_match_product_lock':True,
                                  'note':'temporary consuming root patch only; production manifests and lock untouched; high-level decoder remains unbudgeted'}
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))

if __name__ == '__main__':
    main()
