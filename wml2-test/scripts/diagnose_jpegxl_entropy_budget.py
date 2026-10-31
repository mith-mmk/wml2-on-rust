#!/usr/bin/env python3
"""Linux-only controlled performance experiment; never changes product dependencies."""
import argparse
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import random
import runpy
import shutil
import statistics
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
PREVIOUS = "af18090a8766ca46cc79b702f2e56c2ee420c498"
BENCH = r'''use std::{hint::black_box,time::Instant};
use wml2::{draw::image_from_with_limits, limits::DecodeLimits};
fn main(){
 let args:Vec<_>=std::env::args().collect();let input=std::fs::read(&args[1]).unwrap();let repeat:usize=args[2].parse().unwrap();
 println!("iteration,decode_ns,hash");
 for i in 0..repeat {let start=Instant::now();let image=image_from_with_limits(black_box(&input),DecodeLimits::default()).unwrap();let ns=start.elapsed().as_nanos();
 let hash=image.buffer.as_ref().unwrap().iter().fold(0xcbf29ce484222325u64,|h,&v|(h^u64::from(v)).wrapping_mul(0x100000001b3));
 println!("{i},{ns},{hash}");black_box(image);}
}
'''
DIAGNOSTIC = r'''use jxl::{bit_reader::BitReader,entropy_coding::{context_map::decode_context_map_with_budget as map,decode::{Histograms,SymbolReader}},util::entropy_budget::{EntropyBudget,diagnostic_counts}};
fn bits(parts:&[(u64,usize)])->Vec<u8>{let mut out=vec![0;parts.iter().map(|x|x.1).sum::<usize>().div_ceil(8)];let mut p=0;for &(v,n) in parts{for b in 0..n{out[p/8]|=(((v>>b)&1)as u8)<<(p%8);p+=1;}}out}
fn main(){let args:Vec<_>=std::env::args().collect();if args[1]=="file"{let input=std::fs::read(&args[2]).unwrap();drop(wml2::draw::image_from_with_limits(&input,wml2::limits::DecodeLimits::default()).unwrap());}else if args[1]=="map"{let b=EntropyBudget::new(65536);for _ in 0..100{drop(map(65536,&mut BitReader::new(&[1]),Some(&b)).unwrap());}assert_eq!(b.live_bytes(),0);}else{let b=EntropyBudget::new(4*1024*1024+2);let raw=bits(&[(1,1),(0,2),(0,2),(0,4),(1,1),(0,2),(1,1),(0,4),(0,1)]);let mut br=BitReader::new(&raw);let hist=Histograms::decode_with_budget(1,&mut br,true,Some(&b)).unwrap();for _ in 0..100{drop(SymbolReader::new(&hist,&mut br,None).unwrap());}drop(hist);assert_eq!(b.live_bytes(),0);}
 let (attempts,bytes,drops)=diagnostic_counts();println!("{{\"reserve_calls\":{attempts},\"reserved_bytes\":{bytes},\"drops\":{drops},\"histogram_size\":{},\"reader_size\":{},\"map_wrapper_size\":{}}}",std::mem::size_of::<Histograms>(),std::mem::size_of::<SymbolReader>(),std::mem::size_of::<jxl::util::entropy_budget::EntropyVec<u8>>());}
'''

def sha(data):
    return hashlib.sha256(data).hexdigest()

def previous(path):
    try:
        return subprocess.check_output(['git','show',PREVIOUS+':'+path],cwd=ROOT)
    except subprocess.CalledProcessError as error:
        raise RuntimeError('The local Git history must contain '+PREVIOUS) from error

def process_order(names, block):
    shift=block%len(names)
    order=names[shift:]+names[:shift]
    return list(reversed(order)) if (block//len(names))%2 else order

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--check-order',action='store_true',help='Verify balanced positions without builds or timing')
    ap.add_argument('--output',type=Path,default=Path('.test-entropy-performance'))
    ap.add_argument('--offline',action='store_true')
    ap.add_argument('--ablations',action='store_true',help='Rebuild the archived, non-adopted diagnostic variants too')
    ap.add_argument('--diagnostics',action='store_true',help='Untimed event counts after ALL latency measurements')
    ap.add_argument('--blocks',type=int,default=12)
    ap.add_argument('--cpus',type=int,default=2)
    ap.add_argument('--extra-input',type=Path,help='Optional independent large-ICC scratch codestream')
    args=ap.parse_args()
    if args.check_order:
        for n,blocks in [(4,12),(5,10),(8,24),(3,12)]:
            names=list(range(n));positions={v:[0]*n for v in names}
            for block in range(blocks):
                order=process_order(names,block)
                assert sorted(order)==names
                for i,v in enumerate(order):positions[v][i]+=1
            assert all(v==[blocks//n]*n for v in positions.values())
        print('balanced process positions verified')
        return
    if args.blocks<4 or args.cpus<1:
        ap.error('Use at least four blocks and one CPU')
    if not hasattr(os,'sched_getaffinity'):
        raise RuntimeError('This controlled recipe requires Linux CPU affinity')
    available_cpus=set(os.sched_getaffinity(0))
    cpus=sorted(available_cpus)[:args.cpus]
    out=args.output.resolve()
    if not out.name.startswith('.test'):
        raise ValueError('output must be a fresh .test* directory')
    out.mkdir(exist_ok=False)
    env=dict(os.environ,RUSTFLAGS='',CARGO_INCREMENTAL='0')
    env.pop('CARGO_ENCODED_RUSTFLAGS',None)
    for key in list(env):
        if key.startswith('CARGO_PROFILE_RELEASE_'):
            del env[key]
    rustc=subprocess.check_output(['rustc','-vV'],env=env,text=True)
    if not rustc.startswith('rustc 1.91.0 '):
        raise RuntimeError('Use the recorded Rust 1.91.0 toolchain')
    old=json.loads(previous('docs/jpeg-xl-entropy-budget-evaluation.json'))
    new=json.loads((ROOT/'docs/jpeg-xl-entropy-budget-evaluation.json').read_text())
    assert old['source']['sha256']==new['source']['sha256']
    for path,expected in new['product_files_unchanged'].items():
        assert sha((ROOT/path).read_bytes())==expected, 'Product baseline changed: '+path
    home=Path(env.get('CARGO_HOME',str(Path.home()/'.cargo')))
    archives=list(home.glob('registry/cache/*/jxl-0.7.4.crate'))
    if not archives and not args.offline:
        subprocess.run(['cargo','fetch','--locked'],cwd=ROOT,env=env,check=True)
        archives=list(home.glob('registry/cache/*/jxl-0.7.4.crate'))
    archive=next((p for p in archives if sha(p.read_bytes())==old['source']['sha256']),None)
    if archive is None:
        raise RuntimeError('Verified registry archive missing; cargo fetch --locked first')
    tree=out/'.test-codec'
    tree.mkdir()
    with tarfile.open(archive) as tar:
        for entry in tar:
            if entry.isfile():
                parts=Path(entry.name).parts
                assert parts[0]=='jxl-0.7.4' and '..' not in parts
                target=tree.joinpath(*parts[1:]);target.parent.mkdir(parents=True,exist_ok=True)
                target.write_bytes(tar.extractfile(entry).read())
    originals={name:(tree/name).read_bytes() if (tree/name).exists() else None for name in new['files']}
    apply=runpy.run_path(str(ROOT/'wml2-test/scripts/evaluate_jpegxl_entropy_budget.py'))['apply_patch']
    old_patch=previous('wml2-test/scripts/jxl_entropy_budget_074.patch')
    new_patch=(ROOT/'wml2-test/scripts/jxl_entropy_budget_074.patch').read_bytes()
    assert sha(old_patch)==old['patch_sha256'] and sha(new_patch)==new['patch_sha256']
    variants=['base','before','after','base-repeat']
    recipes=json.loads((ROOT/'docs/jpeg-xl-entropy-budget-performance.json').read_text())
    if args.ablations:
        variants[3:3]=list(recipes['ablation_deltas'])
    if args.blocks%len(variants) or args.blocks%3:
        raise ValueError('blocks must be a multiple of both the condition count and three: '+str(len(variants)))
    def stage(variant):
        for name,content in originals.items():
            path=tree/name
            if content is None:
                if path.exists(): path.unlink()
            else: path.write_bytes(content)
        if variant not in ['base','base-repeat']:
            apply(tree,(new_patch if variant=='after' else old_patch).decode())
            expected=new['files'] if variant=='after' else old['files']
            for name,entry in expected.items():
                assert sha((tree/name).read_bytes())==entry['after']
            if variant in recipes['ablation_deltas']:
                apply(tree,recipes['ablation_deltas'][variant]['patch'])
                for name,digest in recipes['ablation_deltas'][variant]['source_sha256'].items():
                    assert sha((tree/name).read_bytes())==digest
    (out/'.test-bench.rs').write_text(BENCH,newline='\n')
    (out/'.test-stub.rs').write_text('// fixed-path evaluation root\n')
    consumer=out/'.test-consumer';consumer.mkdir()
    manifest=f'''[package]
name="jxl-entropy-wml-evaluation"
version="0.0.0"
edition="2024"
[workspace]
[lib]
path={json.dumps(str(out/'.test-stub.rs'))}
[features]
default=["jpeg-xl","image-buffer-ex","color-management","high-bit-depth","png","exif"]
jpeg-xl=[]
image-buffer-ex=[]
color-management=[]
high-bit-depth=[]
png=[]
exif=[]
[dependencies]
wml2={{path={json.dumps(str(ROOT/'wml2'))},default-features=false,features=["jpeg-xl","image-buffer-ex","color-management","png","exif"]}}
bin-rs={{version="0.0.10",features=["util"]}}
miniz_oxide="0.7.1"
[patch.crates-io]
jxl={{path={json.dumps(str(tree))}}}
[[example]]
name="entropy_decode_bench"
path={json.dumps(str(out/'.test-bench.rs'))}
[[example]]
name="review_bench"
path={json.dumps(str(ROOT/'wml2/examples/review_bench.rs'))}
[profile.release]
opt-level=3
codegen-units=16
lto=false
incremental=false
debug=false
panic="unwind"
'''
    manifest_path=consumer/'Cargo.toml';manifest_path.write_text(manifest)
    shutil.copyfile(ROOT/'Cargo.lock',consumer/'Cargo.lock')
    common=['--manifest-path',str(manifest_path)]+(['--offline'] if args.offline else [])
    with (out/'metadata.json').open('w') as output:
        subprocess.run(['cargo','metadata',*common,'--format-version','1'],cwd=ROOT,env=env,stdout=output,check=True)
    locked={(p['name'],p['version']) for p in tomllib.loads((ROOT/'Cargo.lock').read_text())['package']}
    assert all((p['name'],p['version']) in locked for p in json.loads((out/'metadata.json').read_text())['packages'] if p['source'])
    binaries=out/'.test-binaries';binaries.mkdir()
    report={'rustc':rustc,'manifest_sha256':sha(manifest.encode()),'lock_sha256':sha((consumer/'Cargo.lock').read_bytes()),'bench_source_sha256':sha(BENCH.encode()),'rustflags':[],'profile':'release opt3 codegen16 lto=false incremental=false debug=false panic=unwind','builds':{},'records':[]}
    # Build ALL variants sequentially at the SAME source/manifest paths before timing.
    for variant in variants:
        stage(variant)
        with (out/(variant+'-build.log')).open('w') as log:
            subprocess.run(['cargo','build',*common,'--locked','--release','--target-dir',str(out/('.test-target-'+variant)),'--example','entropy_decode_bench','--example','review_bench'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
        hashes={}
        for name in ['entropy_decode_bench','review_bench']:
            dest=binaries/(variant+'-'+name)
            shutil.copyfile(out/('.test-target-'+variant)/'release/examples'/name,dest);dest.chmod(0o755);hashes[name]=sha(dest.read_bytes())
        report['builds'][variant]=hashes
    assert report['builds']['base']==report['builds']['base-repeat'], 'A/A rebuild changed bytes'
    inputs={'simple':ROOT/'wml2/tests/fixtures/review/jxl-rgb8.jxl','icc':ROOT/'wml2/tests/fixtures/review/jxl-icc-lossless.jxl','photo':ROOT/'wml2/tests/fixtures/review/jxl-jbrd-bench.jxl'}
    if args.extra_input: inputs['large-icc']=args.extra_input.resolve()
    report['inputs_sha256']={name:sha(path.read_bytes()) for name,path in inputs.items()}
    rng=random.Random(7401)
    def measure(cpu,case,path,names,meter=False):
        trials={name:[] for name in names};hashes=set();repeat=25 if meter else (205 if case=='simple' else (105 if case=='icc' else 25))
        for block in range(args.blocks):
            order=process_order(names,block)
            for name in order:
                cmd=[str(binaries/(name+('-review_bench' if meter else '-entropy_decode_bench')))]
                cmd+=['decode-file',str(repeat),str(path)] if meter else [str(path),str(repeat)]
                text=subprocess.check_output(cmd,text=True)
                (out/f'raw-{cpu}-{case}-{block}-{name}.csv').write_text(text)
                rows=list(csv.DictReader(io.StringIO(text)))[5:];assert len(rows)==repeat-5
                trials[name].append(statistics.median(int(row['ns' if meter else 'decode_ns']) for row in rows));hashes.update(row['hash'] for row in rows)
        assert len(hashes)==1
        record={'cpu':cpu,'case':case,'blocks':args.blocks,'warmups':5,'measured_per_process':repeat-5,'trial_medians_ns':trials,'hash':next(iter(hashes)),'ratios':{},'variability':{}}
        for name,values in trials.items():
            pairs=[x/y for x,y in zip(values,trials['base'])]
            boot=sorted(statistics.median(rng.choices(pairs,k=len(pairs))) for _ in range(10000))
            record['ratios'][name]={'paired_median':statistics.median(pairs),'bootstrap_95':[boot[250],boot[9749]],'paired_min_max':[min(pairs),max(pairs)]}
            q=statistics.quantiles(values,n=4)
            record['variability'][name]={'median_ns':statistics.median(values),'iqr_ns':q[2]-q[0],'min_max_ns':[min(values),max(values)]}
        report['records'].append(record)
        print(cpu,case,{k:round(v['paired_median'],4) for k,v in record['ratios'].items()},flush=True)
    for cpu in cpus:
        os.sched_setaffinity(0,{cpu})
        for case,path in inputs.items(): measure(cpu,case,path,variants)
    os.sched_setaffinity(0,{cpus[0]})
    measure(cpus[0],'photo-meter-and-timed-hash',inputs['photo'],['base','before','after'],True)
    if args.diagnostics:
        os.sched_setaffinity(0,available_cpus)
        # These added counters are NEVER timed. They are not part of the shipped patch.
        (out/'.test-diagnostic.rs').write_text(DIAGNOSTIC,newline='\n')
        manifest=manifest.replace('[dependencies]','[dependencies]\njxl={path='+json.dumps(str(tree))+',default-features=false}')
        manifest+='\n[[example]]\nname="diagnostic"\npath='+json.dumps(str(out/'.test-diagnostic.rs'))+'\n'
        manifest_path.write_text(manifest)
        report['untimed_diagnostics']={}
        for variant in ['before','after']:
            stage(variant)
            path=tree/'src/util/entropy_budget.rs';s=path.read_text()
            s=s.replace('    fn reserve(&self, bytes: usize) -> Result<Lease> {','    fn reserve(&self, bytes: usize) -> Result<Lease> {\n        DIAG_CALLS.fetch_add(1, Ordering::Relaxed);\n        DIAG_BYTES.fetch_add(bytes, Ordering::Relaxed);').replace('    fn drop(&mut self) {\n        self.budget','    fn drop(&mut self) {\n        DIAG_DROPS.fetch_add(1, Ordering::Relaxed);\n        self.budget')
            s+='\nstatic DIAG_CALLS: AtomicUsize=AtomicUsize::new(0);\nstatic DIAG_BYTES: AtomicUsize=AtomicUsize::new(0);\nstatic DIAG_DROPS: AtomicUsize=AtomicUsize::new(0);\npub fn diagnostic_counts()->(usize,usize,usize){(DIAG_CALLS.load(Ordering::Relaxed),DIAG_BYTES.load(Ordering::Relaxed),DIAG_DROPS.load(Ordering::Relaxed))}\n';path.write_text(s)
            target=out/('.test-diagnostic-target-'+variant)
            with (out/(variant+'-diagnostic-build.log')).open('w') as log:
                subprocess.run(['cargo','build',*common,'--release','--target-dir',str(target),'--example','diagnostic'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
            binary=target/'release/examples/diagnostic';counts={}
            for case in ['file','map','window']:
                cmd=[str(binary),case]+([str(inputs['photo'])] if case=='file' else [])
                counts[case]=json.loads(subprocess.check_output(cmd,text=True))
            assert counts['file']['reserve_calls']==0 and counts['file']['drops']==0
            assert counts['map']['reserve_calls']==counts['map']['drops']==100
            assert counts['window']['reserve_calls']==counts['window']['drops']==101
            report['untimed_diagnostics'][variant]=counts
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(out/'report.json')

if __name__=='__main__': main()
