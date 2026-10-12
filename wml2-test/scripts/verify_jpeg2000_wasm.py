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
mapped_sources = {}
for manifest_name, field_count in [('jpeg2000-blocks-manifest.txt', 11),
                                   ('jpeg2000-tiles-manifest.txt', 14),
                                   ('jpeg2000-origins-manifest.txt', 14),
                                   ('jpeg2000-global-origins-manifest.txt', 18),
                                   ('jpeg2000-parts-manifest.txt', 20)]:
    manifest = fixtures / manifest_name
    if manifest.exists():
        for line in manifest.read_text().splitlines()[1:]:
            fields = line.split()
            assert len(fields) == field_count
            mapped_sources[fields[1]] = fields
promoted = []
unsupported_manifest = fixtures / 'jpeg2000-tiles-unsupported-manifest.txt'
rejected = []
if unsupported_manifest.exists():
    for line in unsupported_manifest.read_text().splitlines()[1:]:
        fields = line.split()
        assert len(fields) == 15
        if fields[14] == 'count':
            rejected.append(fixtures / fields[1])
        else:
            promoted.append(fixtures / fields[1])
            mapped_sources[fields[1]] = fields
origins_unsupported = fixtures / 'jpeg2000-origins-unsupported-manifest.txt'
if origins_unsupported.exists():
    for line in origins_unsupported.read_text().splitlines()[1:]:
        fields = line.split()
        assert len(fields) == 15 and fields[14] == 'count'
        rejected.append(fixtures / fields[1])
global_unsupported = fixtures / 'jpeg2000-global-origins-unsupported-manifest.txt'
if global_unsupported.exists():
    for line in global_unsupported.read_text().splitlines()[1:]:
        fields = line.split()
        assert len(fields) == 19
        rejected.append(fixtures / fields[1])
parts_unsupported = fixtures / 'jpeg2000-parts-unsupported-manifest.txt'
if parts_unsupported.exists():
    for line in parts_unsupported.read_text().splitlines()[1:]:
        fields = line.split()
        assert len(fields) == 21
        rejected.append(fixtures / fields[1])
paths = sorted(fixtures.glob('jp2*.j2k')) + sorted(fixtures.glob('jp2*.jp2')) + promoted
for path in paths:
    stem = path.stem.split('_mct')[0]
    if path.name in mapped_sources:
        sources = [fixtures / mapped_sources[path.name][3]]
    else:
        sources = [fixtures / (stem + ext) for ext in ['.rgb', '.gray']]
    sources = [source for source in sources if source.exists()]
    if not sources:
        continue
    assert len(sources) == 1, stem
    source = sources[0]
    w, h = map(int, re.search(r'_(\d+)x(\d+)_', stem).groups())
    components = 3 if source.suffix == '.rgb' else 1
    assert source.stat().st_size == w * h * components
    extra_workspace = int(mapped_sources[path.name][19]) if path.name in mapped_sources and len(mapped_sources[path.name]) == 20 else 0
    cases.append((path, source, w, h, components, extra_workspace))
compatibility_name = 'oracle_origins_22x11_d4_gray_b8x4_t20x22_singleton'
compatibility_path = fixtures / (compatibility_name + '.j2k')
if compatibility_path.exists():
    cases.append((compatibility_path, fixtures / (compatibility_name + '.expected.gray'), 22, 11, 1, 0))
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
wml2 = {{ path = {json.dumps(str(repo / 'wml2'), ensure_ascii=False)}, default-features = {default}, features = ["jpeg2000"] }}
bin-rs = {{ version = "0.0.10", features = ["util"] }}
''')
entries = '\n'.join(
    f'Case {{ encoded: include_bytes!({json.dumps(str(p), ensure_ascii=False)}), source: include_bytes!({json.dumps(str(s), ensure_ascii=False)}), '
    f'w: {w}, h: {h}, components: {n}, extra_workspace: {extra} }},' for p, s, w, h, n, extra in cases)
rejected_entries = '\n'.join(
    f'include_bytes!({json.dumps(str(p), ensure_ascii=False)}),' for p in rejected)
rust = r'''
use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;
type Error = Box<dyn std::error::Error>;
struct Case { encoded: &'static [u8], source: &'static [u8], w: usize, h: usize, components: usize, extra_workspace: usize }
static CASES: &[Case] = &[
ENTRIES
];
static REJECTED: &[&[u8]] = &[
REJECTED_ENTRIES
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
    limit.expanded_bytes = case.w * case.h * case.components * 64 + 8192 + case.extra_workspace;
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
fn rejected_before_callbacks(data: &[u8]) -> bool {
    let mut probe = Probe { case: &CASES[0], abort: 0, events: Vec::new() };
    image_decoder(&mut BytesReader::new(data), &mut DecodeOptions {
        drawer: &mut probe, debug_flag: 0,
    }).is_err() && probe.events.is_empty()
}
#[unsafe(no_mangle)]
pub extern "C" fn rejected_count() -> u32 { REJECTED.len() as u32 }
#[unsafe(no_mangle)]
pub extern "C" fn run_rejections() -> u32 {
    for data in REJECTED {
        if !rejected_before_callbacks(data) { return 1; }
    }
    // Validate late framing after earlier tiles, and repaired short packets.
    for case in CASES {
        let data = case.encoded;
        if !data.starts_with(&[255,79]) { continue; }
        let mut p = 2;
        while data[p..p+2] != [255,0x90] {
            p += usize::from(u16::from_be_bytes([data[p+2],data[p+3]])) + 2;
        }
        let first = p;
        let mut count = 0;
        let mut last = p;
        while data[p..p+2] == [255,0x90] {
            last = p;
            p += u32::from_be_bytes(data[p+6..p+10].try_into().unwrap()) as usize;
            count += 1;
        }
        if count < 2 { continue; }
        for (offset,value) in [(4,255),(10,255),(11,0),(12,0)] {
            let mut bad = data.to_vec();
            bad[last+offset] = value;
            if !rejected_before_callbacks(&bad) { return 2; }
        }
        let mut missing = data[..last].to_vec();
        missing.extend_from_slice(&[255,217]);
        if !rejected_before_callbacks(&missing) { return 3; }
        let mut short = data[..first+14].to_vec();
        short[first+6..first+10].copy_from_slice(&14u32.to_be_bytes());
        let first_end = first + u32::from_be_bytes(data[first+6..first+10].try_into().unwrap()) as usize;
        short.extend_from_slice(&data[first_end..]);
        if !rejected_before_callbacks(&short) { return 4; }
    }
    0
}
'''.replace('REJECTED_ENTRIES', rejected_entries).replace('ENTRIES', entries)
(work / 'lib.rs').write_text(rust)
# Snapshot the tested inputs before building; edits/commits during the runtime
# must not change the provenance attached to an already-built module.
build_context = {
    'git_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
    'working_tree_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=repo, text=True).strip()),
    'rust_source_sha256': hashlib.sha256((repo / 'wml2/src/jpeg2000.rs').read_bytes()).hexdigest(),
    'tier1_source_sha256': hashlib.sha256((repo / 'wml2/src/jpeg2000_tier1.rs').read_bytes()).hexdigest(),
    'fixture_manifest_sha256': {
        p.name: hashlib.sha256(p.read_bytes()).hexdigest()
        for p in fixtures.glob('jpeg2000-*-manifest.txt')},
}
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
const rejected = instance.exports.rejected_count();
const rejectionStatus = instance.exports.run_rejections();
if (rejectionStatus !== 0) throw new Error(`Rejections: failure status ${rejectionStatus}`);
console.log(JSON.stringify({runtime:process.version, fixtures:count, rejected_fixtures:rejected, all_passed:true,
  host_imports:imports, memory_bytes:instance.exports.memory.buffer.byteLength}));
'''
(work / 'run.cjs').write_text(runner)
result = subprocess.check_output(['node', str(work / 'run.cjs'), str(wasm)], text=True)
report = json.loads(result)
assert report['fixtures'] == len(cases) and report['all_passed']
report.update({**build_context, 'target': 'wasm32-unknown-unknown', 'default_features': args.default_features,
               'source_exact_fixtures': len(cases) - int(compatibility_path.exists()),
               'encoder_compatibility_fixtures': int(compatibility_path.exists()),
               'wasm_sha256': hashlib.sha256(wasm.read_bytes()).hexdigest(),
               'checks': ['RGBA pixels', 'dimensions', 'exact limits', 'short limits',
                          'limit scope restoration', 'callbacks', 'init/draw abort',
                          'truncation rejected before callbacks', 'unsupported and malformed tile/part profiles rejected before callbacks',
                          'late tile framing, missing tile and repaired short packet rejection']})
(work / ('default-report.json' if args.default_features else 'minimal-report.json')).write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
