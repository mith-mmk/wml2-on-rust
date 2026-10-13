#!/usr/bin/env python3
"""Build and execute a bounded EX foundation smoke suite in actual wasm32."""
import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    if not args.output.name.startswith('.test'):
        parser.error('--output directory must start with .test')
    root = Path(__file__).resolve().parents[2]
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    target = Path(env.get('CARGO_TARGET_DIR', out / '.test-target')).resolve()
    env['CARGO_TARGET_DIR'] = str(target)
    env['WML2_EX_FIXTURES'] = str(root / 'wml2/tests/fixtures')
    command = ['cargo', 'build', '-p', 'wml2', '--release', '--target',
               'wasm32-unknown-unknown', '--no-default-features', '--features',
               'image-buffer-ex,high-bit-depth,png,tiff']
    if args.offline:
        command.append('--offline')
    build = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True)
    (out / 'build.log').write_text(build.stdout + build.stderr)
    build.check_returncode()
    deps = target / 'wasm32-unknown-unknown/release'
    wasm = out / 'verify.wasm'
    harness = Path(__file__).with_name('image_buffer_ex_wasm.rs')
    subprocess.run(['rustc', '--edition', '2024', '--target', 'wasm32-unknown-unknown',
                    '--crate-type', 'cdylib', '-C', 'opt-level=3', '-C', 'panic=abort',
                    str(harness), '--extern', f'wml2={deps / "libwml2.rlib"}',
                    '-L', f'dependency={deps / "deps"}', '-o', str(wasm)],
                   cwd=root, env=env, check=True)
    runner = out / 'run.cjs'
    runner.write_text('''const fs = require('fs');
(async () => {
  const module = await WebAssembly.compile(fs.readFileSync(process.argv[2]));
  const imports = WebAssembly.Module.imports(module);
  if (imports.length) throw Error('unexpected host imports');
  const instance = await WebAssembly.instantiate(module, {});
  const cases = instance.exports.verify_ex();
  if (cases !== 82) throw Error('case count mismatch');
  process.stdout.write(JSON.stringify({runtime:process.version,cases,
    all_passed:true,host_imports:imports,
    memory_bytes:instance.exports.memory.buffer.byteLength}) + '\\n');
})().catch(error => { process.stderr.write(String(error)+'\\n'); process.exit(1); });
''')
    run = subprocess.run(['node', str(runner), str(wasm)], cwd=root,
                         capture_output=True, text=True)
    (out / 'run.log').write_text(run.stdout + run.stderr)
    run.check_returncode()
    report = json.loads(run.stdout)
    report.update({
        'target': 'wasm32-unknown-unknown',
        'features': 'image-buffer-ex,high-bit-depth,png,tiff; no defaults',
        'git_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'],
                                            cwd=root, text=True).strip(),
        'working_tree_dirty': bool(subprocess.check_output(
            ['git', 'status', '--porcelain'], cwd=root, text=True)),
        'scope': 'six native/highres layouts, static legacy bridge, explicit encoder '
                 'precision/PNG handoff, low-bit highres handoff, four independent '
                 'LZW/PackBits TIFF fixtures with all pixels and short pixel limit; '
                 '54 FillOrder oracle cases,6 SampleFormat cases,8 LSB compatibility '
                 'cases and candidate-budget scope restoration',
        'wasm_sha256': digest(wasm), 'harness_sha256': digest(harness),
        'helper_sha256': digest(Path(__file__)),
        'source_sha256': {str(path.relative_to(root)): digest(path) for path in [
            root / 'wml2/src/image_buffer_ex.rs', root / 'wml2/src/draw.rs',
            root / 'wml2/src/decode_guard.rs', root / 'wml2/src/png/decoder.rs',
            root / 'wml2/src/tiff/decoder/ex.rs']},
        'fixture_sha256': {path.name: digest(path) for path in
                           sorted((root / 'wml2/tests/fixtures').glob('ex-*.tif'))},
    })
    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
