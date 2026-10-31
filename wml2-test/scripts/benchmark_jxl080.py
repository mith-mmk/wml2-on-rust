#!/usr/bin/env python3
"""Matched Linux end-to-end JXL migration benchmark; creates a retained .test worktree."""
import argparse,os,subprocess,json,statistics,csv,io,hashlib,random,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',type=Path,required=True)
args=p.parse_args()
rustc=subprocess.check_output(['rustc','-V'],text=True).strip()
if not rustc.startswith('rustc 1.91.0 '): raise RuntimeError('Use the recorded Rust 1.91.0 toolchain')
root=ROOT;out=args.output.resolve();
if not out.name.startswith('.test'): raise ValueError('output must start .test')
out.mkdir(exist_ok=False);stage=out/'.test-stage';
subprocess.run(['git','worktree','add','--detach',str(stage),'8fa0b978c4f68e998439e1bffe0b567f46024074'],cwd=root,check=True);
names=['Cargo.lock','wml2/Cargo.toml','wml2/src/jpegxl.rs','wml2/src/jpegxl_dimensions.rs','wml2/src/image_info/jpegxl.rs','wml2/src/jpegxl_resources.rs'];after={f:(root/f).read_bytes() for f in names};before={f:subprocess.check_output(['git','show','8fa0b97:'+f],cwd=root) for f in names};env=dict(os.environ,RUSTFLAGS='',CARGO_INCREMENTAL='0');env.pop('CARGO_ENCODED_RUSTFLAGS',None)
for key in list(env):
 if key.startswith('CARGO_PROFILE_RELEASE_'): del env[key]
record={}
for label,contents in [('base',before),('after',after),('base-repeat',before)]:
 for f,data in contents.items():(stage/f).write_bytes(data)
 target=out/('.test-target-'+label)
 with (out/(label+'-build.log')).open('w') as log:subprocess.run(['cargo','build','-p','wml2','--locked','--offline','--no-default-features','--features','jpeg-xl,color-management,image-buffer-ex,png','--release','--example','review_bench','--lib','--target-dir',str(target)],cwd=stage,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
 binary=target/'release/examples/review_bench';record[label]={'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'bytes':binary.stat().st_size,'source_sha256':{f:hashlib.sha256(data).hexdigest() for f,data in contents.items()}}
 (out/'builds.json').write_text(json.dumps(record,indent=2)+'\n');print(label,record[label]['bytes'],flush=True)
assert record['base']['binary_sha256']==record['base-repeat']['binary_sha256']

variants=['base','after','base-repeat'];cpus=sorted(os.sched_getaffinity(0))[:2];records=[];rng=random.Random(800)
for cpu in cpus:
 os.sched_setaffinity(0,{cpu})
 for name,repeat,warm in [('jxl-rgb8.jxl',23,3),('jxl-icc-lossless.jxl',23,3),('jxl-jbrd-bench.jxl',13,3),('jxl-sunset-logo.jxl',5,1)]:
  inp=root/'wml2/tests/fixtures/review'/name;trials={v:[] for v in variants};metrics={v:set() for v in variants};digests=set()
  for block in range(6):
   k=block%3;order=variants[k:]+variants[:k]
   if (block//3)%2:order=list(reversed(order))
   for v in order:
    text=subprocess.check_output([str(out/('.test-target-'+v)/'release/examples/review_bench'),'decode-file',str(repeat),str(inp)],text=True)
    (out/f'{cpu}-{name}-{block}-{v}.csv').write_text(text);rows=list(csv.DictReader(io.StringIO(text)))[warm:];assert len(rows)==repeat-warm
    trials[v].append(statistics.median(int(r['ns']) for r in rows));metrics[v].update((int(r['allocations']),int(r['allocated_bytes']),int(r['peak_live_bytes'])) for r in rows);digests.update(r['hash'] for r in rows)
  assert len(digests)==1
  ratios={};variability={}
  for v,values in trials.items():
   pairs=[x/y for x,y in zip(values,trials['base'])];boot=sorted(statistics.median(rng.choices(pairs,k=len(pairs))) for _ in range(10000));q=statistics.quantiles(values,n=4)
   ratios[v]={'paired_median':statistics.median(pairs),'bootstrap95':[boot[250],boot[9749]]};variability[v]={'median_ns':statistics.median(values),'iqr_ns':q[2]-q[0],'minmax_ns':[min(values),max(values)]}
  records.append({'input':name,'input_sha256':hashlib.sha256(inp.read_bytes()).hexdigest(),'cpu':cpu,'blocks':6,'warmups':warm,'timed_per_process':repeat-warm,'trial_medians_ns':trials,'ratios':ratios,'variability':variability,'allocation_count_traffic_peak':{v:sorted(s) for v,s in metrics.items()},'hash':next(iter(digests))})
  print(cpu,name,ratios['after'],flush=True)
report={'method':'Matched source path in detached worktree, Rust1.91.0 release opt3, default features off jpeg-xl/color-management/image-buffer-ex/png, empty RUSTFLAGS, original rebuilt byte-identically; two pinned logical CPUs sequentially, six balanced rotated/reversed process blocks; old allocation-meter and pixel hash included in timing; process-paired bootstrap seed800; no concurrent builds','builds':json.loads((out/'builds.json').read_text()),'results':records}
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
