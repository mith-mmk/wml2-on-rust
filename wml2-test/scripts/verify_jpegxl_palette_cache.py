#!/usr/bin/env python3
"""Linux-only first-use allocation attribution. Fresh processes; no product dependency changes."""
import argparse,hashlib,json,pathlib,re,subprocess,tarfile,os,platform
p=argparse.ArgumentParser();p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--phase',choices=['all','build','matrix','symbols'],default='all');a=p.parse_args()
if not a.output.name.startswith('.test'):p.error('output must start with .test')
if platform.system()!='Linux' or platform.machine()!='x86_64':p.error('attribution harness requires Linux x86_64')
root=pathlib.Path(__file__).resolve().parents[2];out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
def run(args):return subprocess.run([str(x)for x in args],cwd=root,capture_output=True,text=True,check=True).stdout
if a.phase in ['all','build']:
 build=run(['cargo','build','-p','wml2','--locked','--target-dir',out/'.test-target','--features','jpeg-xl,image-buffer-ex,color-management','--message-format=json']);(out/'build.json').write_text(build)
 artifacts=[json.loads(l)for l in build.splitlines()if l.startswith('{')]
 def artifact(name):
  values=[f for x in artifacts if x.get('reason')=='compiler-artifact'and x['target']['name']==name for f in x['filenames']if f.endswith('.rlib')];assert len(values)==1;return values[0]
 exe=out/'probe';source=root/'wml2-test/scripts/jpegxl_palette_cache_probe.rs'
 run(['rustc','--edition','2024','-g','-C','relocation-model=static','-C','link-arg=-no-pie','--crate-name','palette_cache_probe',source,'--extern','wml2='+artifact('wml2'),'--extern','bin_rs='+artifact('bin_rs'),'-L','dependency='+str(out/'.test-target/debug/deps'),'-o',exe])
 if a.phase=='build':raise SystemExit(0)
exe=out/'probe'
source=root/'wml2-test/scripts/jpegxl_palette_cache_probe.rs'
cases={};stacks={}
for mode,expected in [('success',[4008,0,0]),('native',[4008,0,0]),('begin',[0,0,0]),('init',[4008,0,0]),('draw',[4008,0,0]),('early_error',[0,0,0]),('late_error',[4008,0,0]),('budget',[0,0,0]),('depths',[4008,4008,0,0]),('threads',[4008,0])]:
 text=run([exe,mode]) if a.phase in ['all','matrix'] else (out/(mode+'.log')).read_text();(out/(mode+'.log')).write_text(text);rows=[]
 for line in text.splitlines():
  if line.startswith('mode='):
   assert 'external_frees=0,overflow=false'in line
   sizes=json.loads(re.search(r'live=(\[.*?\])',line).group(1));value=int(re.search(r'sum=(\d+)',line).group(1));assert sizes==([24,768,1500,1716]if value else[]);rows.append(value)
  if a.phase in ['all','symbols'] and mode=='success'and line.startswith('trace'):
   size=re.search(r'bytes=(\d+)',line).group(1);addresses=re.search(r'ip=\[(.*?)\]',line).group(1).split(', ')
   symbols=run(['addr2line','-f','-C','-e',exe]+['0x'+v for v in addresses]);(out/('stack-'+size+'.txt')).write_text(symbols)
   functions=[s for s in symbols.splitlines()if 'jxl::'in s or 'once_lock' in s or '/palette.rs:' in s];assert 'ImplicitPalette::new'in symbols or (size=='24'and 'Palette::new::'in symbols);assert 'once_lock::OnceLock'in symbols;stacks[size]=functions
 assert rows==expected,(mode,rows);cases[mode]=rows
if a.phase=='matrix':
 (out/'matrix.json').write_text(json.dumps(cases,indent=2)+'\n');raise SystemExit(0)
registry=pathlib.Path(os.environ.get('CARGO_HOME',str(pathlib.Path.home()/'.cargo')))/'registry';folder=next((registry/'src').glob('*/jxl-0.8.0'));path='src/frame/modular/transforms/palette.rs';data=(folder/path).read_bytes();archive=next((registry/'cache').glob('*/jxl-0.8.0.crate'))
with tarfile.open(archive)as t:assert t.extractfile('jxl-0.8.0/'+path).read()==data
report=dict(platform='Linux x86_64; '+run(['rustc','--version']).strip()+'; debug; jpeg-xl,image-buffer-ex,color-management',source=dict(crate='jxl',version='0.8.0',file=path,sha256=hashlib.sha256(data).hexdigest(),matches_locked_archive=True),probe_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),cases=cases,allocation_stacks=stacks,attribution='Process-global static OnceLock<Box<ImplicitPalette>> by min(source bit depth,24),25 slots; three fixed RGB i32 arrays plus3-pointer boxed object. Retained until process exit, not image ownership or TLS. No general leak-absence inference.',requested_bytes=dict(arrays=[1716,768,1500],descriptor64=24,per_slot64=4008,all_slots64=100200,descriptor32_static=12,per_slot32_static=3996,all_slots32_static=99900),method='Fixed TLS pointer table; address-matched alloc/free/realloc, empty baseline per measured call, no allocation by table. GCC unwinder captures allocation sites; addr2line symbolizes outside measured interval. Separate fresh process for every mode. Thread creation outside measurement. No RSS/allocator-overhead/all-thread guarantee.')
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(cases=cases,source=report['source']),indent=2))
