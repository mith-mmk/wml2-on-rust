"""Run expanded-profile pixel, limit and callback checks in actual Node WASM."""
import argparse,hashlib,json,os,subprocess,sys
from pathlib import Path

parser=argparse.ArgumentParser()
parser.add_argument('--work-dir',type=Path,required=True)
parser.add_argument('--default-features',action='store_true')
args=parser.parse_args()
repo=Path(__file__).resolve().parents[2]
fixtures=repo/'wml2/tests/fixtures/review'
work=args.work_dir.resolve()
assert work.name.startswith('.test')
work.mkdir(exist_ok=True)
cases=[]
for manifest in ['jpeg2000-general-manifest.txt','jpeg2000-general-reactivated-manifest.txt']:
    for line in (fixtures/manifest).read_text().splitlines():
        input,reference,width,height,tolerance=line.split()
        cases.append((fixtures/input,fixtures/reference,int(width),int(height),4,int(tolerance)))
for name,value in [('zero',128),('one',129),('u32_boundary',129)]:
    reference=work/('ht_'+name+'.rgba');reference.write_bytes(bytes([value,value,value,255]))
    cases.append((fixtures/('jp2_general_ht_'+name+'.j2k'),reference,1,1,4,0))
for line in (fixtures/'jpeg2000-global-origins-unsupported-manifest.txt').read_text().splitlines()[1:]:
    f=line.split();cases.append((fixtures/f[1],fixtures/f[3],int(f[4]),int(f[5]),int(f[9]),0))

entries='\n'.join(f'Case{{input:include_bytes!({json.dumps(str(p))}),pixels:include_bytes!({json.dumps(str(s))}),w:{w},h:{h},channels:{c},tolerance:{t}}},' for p,s,w,h,c,t in cases)
(work/'Cargo.toml').write_text(f'''[package]
name="jpeg2000-general-wasm-check"
version="0.0.0"
edition="2024"
[workspace]
[lib]
path="lib.rs"
crate-type=["cdylib"]
[dependencies]
wml2={{path={json.dumps(str(repo/'wml2'))},default-features={str(args.default_features).lower()},features=["jpeg2000"]}}
bin-rs={{version="0.0.10",features=["util"]}}
''')
rust=r'''
use bin_rs::reader::BytesReader;
use wml2::{draw::*,limits::DecodeLimits,metadata::DataMap};
type Error=Box<dyn std::error::Error>;
struct Case{input:&'static[u8],pixels:&'static[u8],w:usize,h:usize,channels:usize,tolerance:u8}
static CASES:&[Case]=&[ENTRIES];
fn matches(case:&Case,p:&[u8])->bool{
    p.len()==case.w*case.h*4&&p.chunks_exact(4).zip(case.pixels.chunks_exact(case.channels)).all(|(a,b)|{
        a.iter().enumerate().all(|(c,&v)|v.abs_diff(if case.channels==4{b[c]}else if c==3{255}else{b[if case.channels==1{0}else{c}]})<=case.tolerance)
    })
}
struct Probe<'a>{case:&'a Case,abort:bool,events:Vec<u8>}
impl DrawCallback for Probe<'_>{
fn init(&mut self,w:usize,h:usize,_:Option<InitOptions>)->Result<Option<CallbackResponse>,Error>{assert_eq!((w,h),(self.case.w,self.case.h));self.events.push(1);Ok(self.abort.then(CallbackResponse::abort))}
fn draw(&mut self,x:usize,y:usize,w:usize,h:usize,p:&[u8],_:Option<DrawOptions>)->Result<Option<CallbackResponse>,Error>{assert_eq!((x,y,w,h),(0,0,self.case.w,self.case.h));assert!(matches(self.case,p));self.events.push(2);Ok(None)}
fn terminate(&mut self,_:Option<TerminateOptions>)->Result<Option<CallbackResponse>,Error>{self.events.push(3);Ok(None)}
fn next(&mut self,_:Option<NextOptions>)->Result<Option<CallbackResponse>,Error>{panic!("unexpected frame")}
fn verbose(&mut self,_:&str,_:Option<VerboseOptions>)->Result<Option<CallbackResponse>,Error>{panic!("unexpected verbose")}
fn set_metadata(&mut self,_:&str,_:DataMap)->Result<Option<CallbackResponse>,Error>{panic!("unexpected metadata")}
}
#[unsafe(no_mangle)] pub extern "C" fn count()->u32{CASES.len() as u32}
#[unsafe(no_mangle)] pub extern "C" fn run(index:u32)->u32{
let c=&CASES[index as usize];let image=match image_load(c.input){Ok(i)=>i,Err(_)=>return 1};
if(image.width,image.height)!=(c.w,c.h)||!image.buffer.as_deref().is_some_and(|p|matches(c,p)){return 2;}
for field in 0..4{
let mut limits=DecodeLimits::default();match field{0=>limits.input_bytes=c.input.len()-1,1=>limits.pixels=c.w*c.h-1,2=>limits.expanded_bytes=c.w*c.h*4-1,_=>limits.frames=0}
let mut image=ImageBuffer::new();if image_decoder_with_limits(&mut BytesReader::new(c.input),&mut DecodeOptions{drawer:&mut image,debug_flag:0},limits).is_ok()||image.buffer.is_some(){return 3+field;}
}
for abort in [false,true]{let mut probe=Probe{case:c,abort,events:Vec::new()};if image_decoder(&mut BytesReader::new(c.input),&mut DecodeOptions{drawer:&mut probe,debug_flag:0}).is_err(){return 7;}if probe.events!=if abort{vec![1,3]}else{vec![1,2,3]}{return 8;}}
let mut image=ImageBuffer::new();if image_decoder(&mut BytesReader::new(&c.input[..c.input.len()-1]),&mut DecodeOptions{drawer:&mut image,debug_flag:0}).is_ok()||image.buffer.is_some(){return 9;}
// Bounded mutations exercise checked geometry/header parsing on a 32-bit target.
for offset in (0..c.input.len()).step_by((c.input.len()/24).max(1)){
for mask in [1u8,128,255]{let mut data=c.input.to_vec();data[offset]^=mask;
// Keep this codec test inside JPEG2000; changing the magic otherwise dispatches
// to unrelated optional codecs in a default-feature build.
if !matches!(wml2::util::format_check(&data),wml2::util::ImageFormat::Jpeg2000){continue;}
let mut image=ImageBuffer::new();let limits=DecodeLimits{pixels:65536,expanded_bytes:1024*1024,metadata_bytes:65536,..Default::default()};let _=image_decoder_with_limits(&mut BytesReader::new(&data),&mut DecodeOptions{drawer:&mut image,debug_flag:0},limits);}
}
if image_load(c.input).is_err(){return 10;}0
}
'''.replace('ENTRIES',entries)
(work/'lib.rs').write_text(rust)
environment=dict(os.environ)
environment['CARGO_TARGET_DIR']=str(work/'.test-target')
source_hashes={str(p.relative_to(repo)).replace('\\','/'):hashlib.sha256(p.read_bytes()).hexdigest()
               for p in [*sorted((repo/'wml2/src').glob('jpeg2000*.rs')),repo/'wml2/src/limits.rs',repo/'wml2/src/error.rs']}
subprocess.run(['cargo','build','--release','--target','wasm32-unknown-unknown'],cwd=work,env=environment,check=True)
wasm=work/'.test-target/wasm32-unknown-unknown/release/jpeg2000_general_wasm_check.wasm'
javascript='''const fs=require('fs');(async()=>{let {instance}=await WebAssembly.instantiate(fs.readFileSync(process.argv[2]),{});let e=instance.exports;let n=e.count();for(let i=0;i<n;i++){let r=e.run(i);if(r)throw Error(`case ${i}: code ${r}`);}console.log(JSON.stringify({passed:n,failed:0}));})().catch(e=>{console.error(e);process.exit(1)});'''
(work/'run.cjs').write_text(javascript)
try:
    result=subprocess.run(['node',str(work/'run.cjs'),str(wasm)],check=True,text=True,capture_output=True)
except subprocess.CalledProcessError as error:
    print(error.stderr,file=sys.stderr)
    raise
report=dict(json.loads(result.stdout),default_features=args.default_features,wasm_sha256=hashlib.sha256(wasm.read_bytes()).hexdigest(),source_sha256=source_hashes,cases=[p.name for p,*_ in cases])
(work/'results.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report),flush=True)
