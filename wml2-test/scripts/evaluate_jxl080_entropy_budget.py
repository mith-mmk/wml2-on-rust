#!/usr/bin/env python3
"""Hash-verified isolated jxl0.8 evaluation. Product manifests/lock are never changed."""
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

def sha(data):
    return hashlib.sha256(data).hexdigest()

def apply_patch(tree, patch):
    lines = patch.splitlines(keepends=True)
    i = 0
    while i < len(lines):
        assert lines[i].startswith('--- a/') and lines[i+1].startswith('+++ b/')
        name = lines[i][6:].rstrip('\n')
        assert name == lines[i+1][6:].rstrip('\n')
        assert '..' not in Path(name).parts and (name == 'Cargo.toml' or Path(name).parts[0] == 'src')
        target = tree / name
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
                flag, text = lines[i][0], lines[i][1:]
                if flag in ' -':
                    assert pos < len(source) and source[pos] == text, (name, pos)
                    pos += 1; removed += 1
                if flag in ' +':
                    dest.append(text); inserted += 1
                i += 1
            assert removed == int(match[2] or 1) and inserted == int(match[4] or 1)
        dest.extend(source[pos:])
        target.write_text(''.join(dest))

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--target-dir', type=Path)
    p.add_argument('--offline', action='store_true')
    p.add_argument('--wasm', action='store_true')
    p.add_argument('--wml-regression', action='store_true')
    a = p.parse_args()
    out = a.output.resolve()
    target = a.target_dir.resolve() if a.target_dir else out / '.test-target'
    if not out.name.startswith('.test') or not target.name.startswith('.test'):
        p.error('output and target directories must start .test')
    out.mkdir(exist_ok=False)
    env = dict(os.environ)
    def run(command, log, expected=0):
        result = subprocess.run([str(x) for x in command], cwd=ROOT, env=env,
                                capture_output=True, text=True)
        (out / log).write_text(result.stdout + result.stderr)
        if result.returncode != expected:
            raise RuntimeError(f'{command[0]} failed ({result.returncode}); see {out/log}')
        return result.stdout + result.stderr
    spec = json.loads((ROOT / 'docs/jpeg-xl080-entropy-budget-validation.json').read_text())
    cargo_home = Path(env.get('CARGO_HOME', str(Path.home() / '.cargo')))
    archives = list(cargo_home.glob('registry/cache/*/jxl-0.8.0.crate'))
    if not archives and not a.offline:
        run(['cargo', 'fetch', '--locked'], 'fetch.log')
        archives = list(cargo_home.glob('registry/cache/*/jxl-0.8.0.crate'))
    archive = next((v for v in archives if sha(v.read_bytes()) == spec['archive_sha256']), None)
    if archive is None:
        raise RuntimeError('hash-verified jxl0.8 registry archive unavailable')
    tree = out / '.test-codec'
    tree.mkdir()
    with tarfile.open(archive) as tar:
        for member in tar:
            if not member.isfile():
                continue
            parts = Path(member.name).parts
            assert parts[0] == 'jxl-0.8.0' and '..' not in parts
            dst = tree.joinpath(*parts[1:])
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.write_bytes(tar.extractfile(member).read())
    for name, expected in spec['files'].items():
        path = tree / name
        assert (sha(path.read_bytes()) if path.exists() else None) == expected['before'], name
    patch = (SCRIPTS / 'jxl080_entropy_budget.patch').read_bytes()
    assert sha(patch) == spec['patch_sha256']
    apply_patch(tree, patch.decode())
    for name, expected in spec['files'].items():
        assert sha((tree / name).read_bytes()) == expected['after'], name
    harness = out / '.test-harness'
    harness.mkdir()
    q = json.dumps
    (harness / 'Cargo.toml').write_text(f'''[package]
name="jxl080-entropy-eval"
version="0.0.0"
edition="2024"
rust-version="1.91"
[workspace]
[lib]
path={q(str(SCRIPTS/'jxl080_entropy_eval.rs'))}
crate-type=["rlib","cdylib"]
[dependencies]
jxl={{path={q(str(tree))},default-features=false,features=["entropy-budget-evaluation"]}}
''')
    shutil.copyfile(SCRIPTS/'jxl080_entropy_eval.lock', harness/'Cargo.lock')
    product = tomllib.loads((ROOT/'Cargo.lock').read_text())['package']
    allowed = {(v['name'], v['version'], v.get('checksum')) for v in product if 'source' in v}
    def check_lock(lock):
        packages = tomllib.loads(lock.read_text())['package']
        assert all((v['name'], v['version'], v.get('checksum')) in allowed for v in packages if 'source' in v)
    check_lock(harness/'Cargo.lock')
    common = ['--manifest-path', harness/'Cargo.toml', '--target-dir', target, '--locked']
    if a.offline:
        common += ['--offline']
    native = run(['cargo', 'test', *common, '--message-format=json-render-diagnostics'], 'native.log')
    assert 'test result: ok. 10 passed' in native
    artifacts = [json.loads(line) for line in native.splitlines() if line.startswith('{')]
    libs = [Path(f) for v in artifacts if v.get('reason') == 'compiler-artifact'
            and v['target']['name'] == 'jxl' for f in v['filenames'] if f.endswith('.rlib')]
    assert len(libs) == 1
    for name, text, code in [
        ('resize', 'fn no_escape(mut h:BudgetedHistograms){h.resize(3);}', 'E0596'),
        ('finish', "fn no_reuse(r:SymbolReader,h:&BudgetedHistograms,br:&mut BitReader<'_>,s:&mut EntropyScratch){r.finish_with_budgeted_scratch(h,br,s).unwrap();r.has_lz77();}", 'E0382')]:
        source = out / ('.test-'+name+'.rs')
        source.write_text('use jxl::entropy_budget_evaluation::*;\n'+text)
        result = subprocess.run(['rustc', '--edition', '2024', '--crate-name', 'ownership_probe',
                                 '--crate-type', 'lib', '--emit=metadata', str(source), '--extern',
                                 'jxl='+str(libs[0]), '-L', 'dependency='+str(libs[0].parent),
                                 '-o', str(out/('.test-'+name+'.rmeta'))], cwd=ROOT, env=env,
                                capture_output=True, text=True)
        (out/(name+'-compile-fail.log')).write_text(result.stdout+result.stderr)
        assert result.returncode != 0 and code in result.stderr, name
    report = dict(scope=spec['scope'], source_sha256=spec['archive_sha256'],
                  patch_sha256=spec['patch_sha256'], native_tests=10, compile_fail_checks=2)
    if a.wasm:
        run(['cargo', 'rustc', *common, '--release', '--target', 'wasm32-unknown-unknown', '--lib',
             '--', '-C', 'link-arg=--max-memory=67108864'], 'wasm-build.log')
        wasm = target/'wasm32-unknown-unknown/release/jxl080_entropy_eval.wasm'
        runner = out/'.test-run.cjs'
        runner.write_text('''const fs=require('fs');const b=fs.readFileSync(process.argv[2]);const m=new WebAssembly.Module(b);if(WebAssembly.Module.imports(m).length)throw Error('imports');const e=new WebAssembly.Instance(m,{}).exports;if(e.evaluate_entropy080()!==6)throw Error('groups');let denied=false;try{e.memory.grow(1024)}catch(err){if(!(err instanceof RangeError))throw err;denied=true}if(!denied)throw Error('ceiling');console.log(JSON.stringify({passed:true,groups:6,imports:[],ceiling_bytes:67108864,ceiling_enforced:denied,memory_bytes:e.memory.buffer.byteLength,node:process.version}));''')
        run(['node', runner, wasm], 'wasm.json')
        report['wasm'] = dict(json.loads((out/'wasm.json').read_text()), sha256=sha(wasm.read_bytes()))
    if a.wml_regression:
        consumer = out/'.test-wml'
        consumer.mkdir()
        stub = out/'.test-stub.rs'
        stub.write_text('// Evaluation consumer only.\n')
        manifest = f'''[package]
name="jxl080-wml-eval"
version="0.0.0"
edition="2024"
[workspace]
[lib]
path={q(str(stub))}
[features]
default=["jpeg-xl","image-buffer-ex","color-management"]
jpeg-xl=[]
image-buffer-ex=[]
color-management=[]
[dependencies]
wml2={{path={q(str(ROOT/'wml2'))},default-features=false,features=["jpeg-xl","image-buffer-ex","color-management"]}}
bin-rs={{version="0.0.10",features=["util"]}}
miniz_oxide="0.7.1"
[patch.crates-io]
jxl={{path={q(str(tree))}}}
'''
        for name in ['jpeg_xl', 'jpeg_xl_resources', 'jpeg_xl_buffered_animation', 'jpeg_xl_animation_callbacks']:
            manifest += '\n[[test]]\nname='+q(name)+'\npath='+q(str(ROOT/'wml2/tests'/(name+'.rs')))+'\n'
        (consumer/'Cargo.toml').write_text(manifest)
        shutil.copyfile(SCRIPTS/'jxl080_entropy_wml_eval.lock', consumer/'Cargo.lock')
        check_lock(consumer/'Cargo.lock')
        # Keep this optional consumer small; its pixels remain the unbudgeted codec path.
        env.update(CARGO_PROFILE_DEV_DEBUG='0', CARGO_INCREMENTAL='0')
        text = run(['cargo', 'test', '--manifest-path', consumer/'Cargo.toml', '--target-dir', target,
                    '--locked', *(['--offline'] if a.offline else [])], 'wml.log')
        count = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', text)))
        assert count == 65, count
        report['unbudgeted_wml_regressions'] = 65
    (out/'report.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))

if __name__ == '__main__':
    main()
