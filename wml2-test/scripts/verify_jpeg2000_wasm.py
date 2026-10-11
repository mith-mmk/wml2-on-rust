"""Execute JPEG2000 public APIs in real wasm32-unknown-unknown via Node.js.

Creates a disposable .test* cdylib using this workspace's wml2 crate and embeds
the checked-in fixture bytes. No production API or buffer representation changes.
Usage: python3 .../verify_jpeg2000_wasm.py [--default-features]
Requires the installed Rust wasm32 target, Cargo, and Node.js with WebAssembly.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--default-features', action='store_true')
args = parser.parse_args()
repo = Path(__file__).resolve().parents[2]
fixtures = repo / 'wml2/tests/fixtures/review'
work = repo / '.test-jpeg2000-wasm'
work.mkdir(exist_ok=True)
cases = []
for path in sorted(fixtures.glob('jp2*.j2k')) + sorted(fixtures.glob('jp2*.jp2')):
    stem = path.stem.split('_mct')[0]
    sources = [fixtures / (stem + ext) for ext in ['.rgb', '.gray']]
    sources = [source for source in sources if source.exists()]
    if not sources:
        continue
    assert len(sources) == 1, stem
    source = sources[0]
    w, h = map(int, re.search(r'_(\d+)x(\d+)_', stem).groups())
    components = 3 if source.suffix == '.rgb' else 1
    assert source.stat().st_size == w * h * components
    cases.append((path, source, w, h, components))
assert cases, 'no fixtures found'
default = str(args.default_features).lower()
(work / 'Cargo.toml').write_text(f'''[package]
name = "jpeg2000-wasm-verification"
version = "0.0.0"
edition = "2024"
[workspace]
[lib]
path = "lib.rs"
crate-type = ["cdylib"]
[dependencies]
wml2 = {{ path = {json.dumps(str(repo / 'wml2'))}, default-features = {default}, features = ["jpeg2000"] }}
bin-rs = {{ version = "0.0.10", features = ["util"] }}
''')
entries = '\n'.join(
    f'Case {{ encoded: include_bytes!({json.dumps(str(p))}), source: include_bytes!({json.dumps(str(s))}), '
    f'w: {w}, h: {h}, components: {n} }},' for p, s, w, h, n in cases)
rust = r'''
use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;
type Error = Box<dyn std::error::Error>;
struct Case { encoded: &'static [u8], source: &'static [u8], w: usize, h: usize, components: usize }
static CASES: &[Case] = &[
ENTRIES
];
fn pixels_match(case: &Case, pixels: &[u8]) -> bool {
    pixels.len() == case.w * case.h * 4 && pixels.chunks_exact(4).enumerate().all(|(i,p)| {
        let source = &case.source[i * case.components..(i+1)*case.components];
        p[3] == 255 && if case.components == 1 { p[..3] == [source[0];3] } else { p[..3] == *source }
    })
}
struct Probe<'a> { case: &'a Case, abort: u8, events: Vec<u8> }
impl DrawCallback for Probe<'_> {
    fn init(&mut self, w: usize, h: usize, _: Option<InitOptions>) -> Result<Option<CallbackResponse>, Error> {
        assert_eq!((w,h), (self.case.w,self.case.h));
        self.events.push(1);
        Ok((self.abort == 1).then(CallbackResponse::abort))
    }
    fn draw(&mut self, x: usize, y: usize, w: usize, h: usize, pixels: &[u8], _: Option<DrawOptions>) -> Result<Option<CallbackResponse>, Error> {
        assert_eq!((x,y,w,h), (0,0,self.case.w,self.case.h));
        assert!(pixels_match(self.case,pixels));
        self.events.push(2);
        Ok((self.abort == 2).then(CallbackResponse::abort))
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Result<Option<CallbackResponse>, Error> {
        self.events.push(3); Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Result<Option<CallbackResponse>, Error> { panic!("unexpected next") }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Result<Option<CallbackResponse>, Error> { panic!("unexpected verbose") }
    fn set_metadata(&mut self, _: &str, _: DataMap) -> Result<Option<CallbackResponse>, Error> { panic!("unexpected metadata") }
}
#[unsafe(no_mangle)]
pub extern "C" fn case_count() -> u32 { CASES.len() as u32 }
#[unsafe(no_mangle)]
pub extern "C" fn run_case(index: u32) -> u32 {
    let Some(case) = CASES.get(index as usize) else { return 1; };
    let Ok(image) = image_load(case.encoded) else { return 2; };
    if (image.width, image.height) != (case.w,case.h) || !image.buffer.as_deref().is_some_and(|p| pixels_match(case,p)) { return 3; }
    let mut limit = DecodeLimits::unlimited();
    limit.input_bytes = case.encoded.len();
    limit.pixels = case.w * case.h;
    limit.expanded_bytes = case.w * case.h * case.components * 64 + 8192;
    limit.frames = 1;
    limit.metadata_bytes = 64;
    // Repeated finite scopes exercise WASM's thread-local limits and restoration.
    for field in 0..5 {
        let mut short = limit;
        match field { 0 => short.input_bytes -= 1, 1 => short.pixels -= 1,
            2 => short.expanded_bytes -= 1, 3 => short.frames = 0, _ => short.metadata_bytes = 0 }
        if image_from_with_limits(case.encoded, short).is_ok() { return 4 + field; }
    }
    let Ok(limited) = image_from_with_limits(case.encoded,limit) else { return 9; };
    if !limited.buffer.as_deref().is_some_and(|p| pixels_match(case,p)) { return 10; }
    // Exercise callback and abort contracts without a host-provided allocator/ABI.
    for abort in 0..=2 {
        let mut probe = Probe { case, abort, events: Vec::new() };
        let result = image_decoder(&mut BytesReader::new(case.encoded), &mut DecodeOptions { drawer: &mut probe, debug_flag: 0 });
        if result.is_err() || probe.events != if abort == 1 { vec![1,3] } else { vec![1,2,3] } { return 11; }
    }
    let mut probe = Probe { case, abort: 0, events: Vec::new() };
    let result = image_decoder(&mut BytesReader::new(&case.encoded[..case.encoded.len()-1]), &mut DecodeOptions { drawer: &mut probe, debug_flag: 0 });
    if result.is_ok() || !probe.events.is_empty() { return 12; }
    // A default decode after the finite scopes detects leaked limit state.
    if image_load(case.encoded).is_err() { return 13; }
    0
}
'''.replace('ENTRIES', entries)
(work / 'lib.rs').write_text(rust)
subprocess.run(['cargo', 'build', '--offline', '--release', '--manifest-path',
                str(work / 'Cargo.toml'), '--target', 'wasm32-unknown-unknown'], check=True)
target = Path(os.environ.get('CARGO_TARGET_DIR', str(work / 'target')))
wasm = target / 'wasm32-unknown-unknown/release/jpeg2000_wasm_verification.wasm'
runner = r'''
const fs = require('node:fs');
const bytes = fs.readFileSync(process.argv[2]);
const module_ = new WebAssembly.Module(bytes);
const imports = WebAssembly.Module.imports(module_);
if (imports.length) throw new Error('Unexpected host imports: ' + JSON.stringify(imports));
const instance = new WebAssembly.Instance(module_, {});
const count = instance.exports.case_count();
for (let index = 0; index < count; index++) {
  const status = instance.exports.run_case(index);
  if (status !== 0) throw new Error(`Fixture ${index}: failure status ${status}`);
}
console.log(JSON.stringify({runtime:process.version, fixtures:count, all_passed:true,
  host_imports:imports, memory_bytes:instance.exports.memory.buffer.byteLength}));
'''
(work / 'run.cjs').write_text(runner)
result = subprocess.check_output(['node', str(work / 'run.cjs'), str(wasm)], text=True)
report = json.loads(result)
assert report['fixtures'] == len(cases) and report['all_passed']
report.update({'target': 'wasm32-unknown-unknown', 'default_features': args.default_features,
               'git_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
               'wasm_sha256': hashlib.sha256(wasm.read_bytes()).hexdigest(),
               'checks': ['RGBA pixels', 'dimensions', 'exact limits', 'short limits',
                          'limit scope restoration', 'callbacks', 'init/draw abort',
                          'truncation rejected before callbacks']})
(work / ('default-report.json' if args.default_features else 'minimal-report.json')).write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
