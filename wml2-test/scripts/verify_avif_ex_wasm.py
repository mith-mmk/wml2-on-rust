#!/usr/bin/env python3
"""Build and execute AVIF EX checks with the EX wasm32/Node smoke procedure.

The encoder's v_frame dependency needs real wasm-bindgen processing. Install a
wasm-bindgen CLI matching Cargo.lock; no mock host imports are supplied here.
All generated Cargo packages and bindings stay in the selected .test directory.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--wasm-bindgen', default='wasm-bindgen')
    args = parser.parse_args()
    if not args.output.name.startswith('.test'):
        parser.error('--output directory must start with .test')
    root = Path(__file__).resolve().parents[2]
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env['CARGO_TARGET_DIR'] = str(Path(env.get('CARGO_TARGET_DIR', out / '.test-target')).resolve())
    fixture = out / '.test-rgba12.avif'
    env['WML2_AVIF_EX_FIXTURE'] = str(fixture)

    def run(command, log):
        result = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True)
        (out / log).write_text(result.stdout + result.stderr)
        if result.returncode:
            raise RuntimeError(f'{command[0]} failed; see {out / log}')
        return result.stdout

    offline = ['--offline'] if args.offline else []
    run(['cargo', 'run', '-p', 'wml2', '--release', '--no-default-features',
         '--features', 'avifenc,image-buffer-ex', '--example', 'ex_avif',
         *offline, '--', str(fixture)], 'fixture-build.log')
    lock = root / 'Cargo.lock'
    version = next(package['version'] for package in tomllib.loads(lock.read_text())['package']
                   if package['name'] == 'wasm-bindgen')
    cli = run([args.wasm_bindgen, '--version'], 'bindgen-version.log').strip()
    if cli != f'wasm-bindgen {version}':
        raise RuntimeError(f'wasm-bindgen CLI must match locked crate {version}; got {cli}')
    harness = Path(__file__).with_name('avif_ex_wasm.rs')
    scratch = out / '.test-harness'
    scratch.mkdir(exist_ok=True)
    manifest = scratch / 'Cargo.toml'
    manifest.write_text(
        '[package]\nname="avif-ex-wasm-verify"\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[lib]\ncrate-type=["cdylib"]\n'
        f'path={json.dumps(str(harness))}\n[dependencies]\n'
        f'wasm-bindgen="={version}"\n'
        f'wml2={{path={json.dumps(str(root / "wml2"))},default-features=false,'
        'features=["avifenc","image-buffer-ex"]}\n[profile.release]\npanic="abort"\n')
    shutil.copyfile(lock, scratch / 'Cargo.lock')
    run(['cargo', 'build', '--release', '--target', 'wasm32-unknown-unknown',
         '--manifest-path', str(manifest), *offline], 'build.log')
    raw = Path(env['CARGO_TARGET_DIR']) / 'wasm32-unknown-unknown/release/avif_ex_wasm_verify.wasm'
    bindings = out / '.test-bindings'
    run([args.wasm_bindgen, '--target', 'nodejs', '--out-dir', str(bindings),
         '--out-name', 'verify', str(raw)], 'bindgen.log')
    runner = out / 'run.cjs'
    runner.write_text('''const fs = require('fs');
const module_ = new WebAssembly.Module(fs.readFileSync(process.argv[3]));
const api = require(process.argv[2]);
const cases = api.verify_avif_ex();
if (cases !== 4) throw Error('case count mismatch');
process.stdout.write(JSON.stringify({runtime:process.version,cases,all_passed:true,
  host_imports:WebAssembly.Module.imports(module_),
  bindings:'wasm-bindgen nodejs; no mocked imports'})+'\\n');
''')
    wasm = bindings / 'verify_bg.wasm'
    report = json.loads(run(['node', str(runner), str(bindings / 'verify.js'), str(wasm)], 'run.log'))
    report.update({
        'target': 'wasm32-unknown-unknown',
        'features': 'avifenc,image-buffer-ex; no defaults',
        'wasm_bindgen': cli,
        'scope': ['RGBA12 fixture -> declared RGBA16 all samples',
                  'RGBA12 lossless encode/decode including transparent RGB and unchanged storage',
                  'RGBA16 Exact rejection and explicit Round -> RGBA12',
                  'legacy RGBA8 encode/decode including alpha'],
        'git_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        'working_tree_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root)),
        'wasm_sha256': digest(wasm), 'harness_sha256': digest(harness),
        'helper_sha256': digest(Path(__file__)), 'fixture_sha256': digest(fixture),
        'cargo_lock_sha256': digest(scratch / 'Cargo.lock'),
        'source_sha256': {str(path.relative_to(root)): digest(path) for path in [
            root / 'wml2/src/avif/ex.rs', root / 'wml2/src/avif/encoder.rs',
            root / 'wml2/src/draw.rs', root / 'wml2/src/image_buffer_ex.rs']},
    })
    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
