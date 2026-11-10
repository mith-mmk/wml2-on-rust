#!/usr/bin/env python3
"""Execute the SDR animation integration-test bodies in real WebAssembly.

Requires Rust's wasm32-unknown-unknown target and Node.js. This is execution,
not cargo check. The generated standalone crate remains under a .test directory.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--work-dir", type=Path)
    parser.add_argument("--node", default="node")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    work = (args.work_dir or root / ".test-jxl-sdr-wasm").resolve()
    if not work.name.startswith(".test"):
        parser.error("--work-dir basename must start with .test")
    work.mkdir(parents=True, exist_ok=True)
    crate_path = json.dumps((root / "wml2").as_posix())
    (work / "Cargo.toml").write_text(f'''[package]
name = "wml2-sdr-wasm-validation"
version = "0.1.0"
edition = "2024"
[workspace]
[lib]
crate-type = ["cdylib"]
path = "lib.rs"
[dependencies]
wml2 = {{path={crate_path},default-features=false,features=["jpeg-xl","color-management","image-buffer-ex"]}}
jxl_preflight = {{package="jxl",version="=0.7.4",default-features=false}}
bin-rs = {{version="0.0.11",features=["util"]}}
miniz_oxide = "0.7.1"
[features]
default=["jpeg-xl","color-management","image-buffer-ex"]
jpeg-xl=[]
color-management=[]
image-buffer-ex=[]
''', encoding="utf-8")
    modules = ["jpeg_xl_xyb_animation", "jpeg_xl_gray_animation", "jpeg_xl_xyb_icc_animation"]
    tests = []
    source_hashes = {}
    for module in modules:
        path = root / "wml2/tests" / (module + ".rs")
        source = path.read_text(encoding="utf-8")
        names = re.findall(r"#\[test\]\s*fn (\w+)\(", source)
        if not names:
            raise RuntimeError(f"No test bodies found in {path}")
        source_hashes[path.relative_to(root).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
        source = source.replace("#[test]", "")
        for name in names:
            source = source.replace("fn " + name + "(", "pub fn " + name + "(")
            tests.append((module, name))
        source = source.replace("fixtures/review/", (root / "wml2/tests/fixtures/review").as_posix() + "/")
        source = source.replace("../src/srgb.icc", (root / "wml2/src/srgb.icc").as_posix())
        (work / (module + ".rs")).write_text(source, encoding="utf-8")
    source = "\n".join("mod " + module + ";" for module in modules)
    source += '\n#[unsafe(no_mangle)]\npub extern "C" fn run_test(index: u32) -> u32 {\nmatch index {\n'
    source += "\n".join(f"{i} => {module}::{name}()," for i, (module, name) in enumerate(tests))
    source += "\n_ => return 0,\n}\n1\n}\n"
    (work / "lib.rs").write_text(source, encoding="utf-8")
    for path in sorted((root / "wml2/src").glob("jpegxl*.rs")):
        source_hashes[path.relative_to(root).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
    metadata = {"tests": [name for _, name in tests], "source_sha256": source_hashes}
    (work / "inputs.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    (work / "run.cjs").write_text('''const fs = require('fs');
const crypto = require('crypto');
(async () => {
  const bytes = fs.readFileSync(process.argv[2]);
  const input = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
  const module = await WebAssembly.compile(bytes);
  const report = { runtime: process.version,
    wasm_sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
    imports: WebAssembly.Module.imports(module), source_sha256: input.source_sha256,
    results: [] };
  for (let i = 0; i < input.tests.length; i++) {
    const instance = await WebAssembly.instantiate(module, {});
    const start = performance.now();
    try {
      if (instance.exports.run_test(i) !== 1) throw Error('Unexpected result');
      report.results.push({test: input.tests[i], passed: true, milliseconds: performance.now() - start});
    } catch (e) {
      report.results.push({test: input.tests[i], passed: false, error: String(e), milliseconds: performance.now() - start});
    }
  }
  report.passed = report.results.every(x => x.passed);
  fs.writeFileSync(process.argv[4], JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  process.exitCode = report.passed ? 0 : 1;
})().catch(e => { console.error(e); process.exitCode = 1; });
''', encoding="utf-8")
    target = work / ".test-target"
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    command = ["cargo", "build", "--manifest-path", str(work / "Cargo.toml"),
               "--release", "--target", "wasm32-unknown-unknown"]
    if args.offline:
        command.append("--offline")
    subprocess.run(command, env=env, check=True)
    module = target / "wasm32-unknown-unknown/release/wml2_sdr_wasm_validation.wasm"
    subprocess.run([args.node, str(work / "run.cjs"), str(module),
                    str(work / "inputs.json"), str(work / "report.json")], check=True)
    print(f"WASM execution report: {work / 'report.json'}")


if __name__ == "__main__":
    main()
