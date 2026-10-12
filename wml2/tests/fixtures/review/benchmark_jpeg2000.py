"""Paired native benchmark; build immutable baseline/current release verifiers first.
Uses no oracle code and compares full PPM output hashes before reporting timings.
Run without other CPU test jobs. Requires Linux sched affinity.
"""
from pathlib import Path
import hashlib,json,os,resource,statistics,subprocess,time
import argparse
parser=argparse.ArgumentParser(description='Paired fixed-affinity JPEG2000 public-API release performance')
parser.add_argument('baseline', type=Path)
parser.add_argument('candidate', type=Path)
parser.add_argument('--baseline-git', required=True)
parser.add_argument('--stable-gray', action='store_true')
parser.add_argument('--output', type=Path, required=True)
args=parser.parse_args()
repo=Path(__file__).resolve().parents[4]
work=repo/'.test-jpeg2000-benchmark';work.mkdir(exist_ok=True)
binaries={'baseline':args.baseline.resolve(),'current':args.candidate.resolve()}
assert all(p.is_file() for p in binaries.values())
cpus=os.sched_getaffinity(0);cpu=min(cpus)
os.sched_setaffinity(0,{cpu})
cases=[('Gray16','jp2_16x16_d1_noise.j2k',30000),
       ('Gray65','jp2_multi_65x49_d1_noise.j2k',500)]
if args.stable_gray:
    cases += [('GrayDeep65','jp2_deep_65x49_d5_noise.j2k',500),
              ('GrayDeep257','jp2_deep_257x129_d5_noise.j2k',30)]
else:
    cases += [('RGB16','jp2_rgb_16x16_d1_noise_mct1.j2k',5000),
              ('RGB65','jp2_rgb_65x49_d2_noise_mct1.j2k',200),
              ('Gray256tiles','jp2_tiles_64x64_d2_gray_b16x16_t4x4_mixed_mct0.j2k',500),
              ('RGB9tiles','jp2_tiles_355x485_d5_rgb_b64x64_t160x224_ramp_mct1.j2k',20),
              ('RGB2oddtiles','jp2_origins_34x35_d5_rgb_b4x4_t33x35_noise_canonical_mct1.j2k',1000)]
rows=[]
for label,name,iterations in cases:
    path=repo/'wml2/tests/fixtures/review'/name
    if not path.exists():
        # Locate the existing 65x49 multiblock Gray source used by prior runs.
        path=next((repo/'wml2/tests/fixtures/review').glob('*65x49*d1*noise.j2k'))
    for version in ['baseline','current','current','baseline','current','baseline']:
        out=work/'benchmark.ppm'
        before=resource.getrusage(resource.RUSAGE_CHILDREN)
        start=time.perf_counter()
        result=subprocess.check_output([str(binaries[version]),str(path),str(out),str(iterations)],text=True)
        wall=time.perf_counter()-start
        after=resource.getrusage(resource.RUSAGE_CHILDREN)
        cpu_seconds=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime
        rows.append({'case':label,'fixture':path.name,'version':version,'iterations':iterations,
                     'total_calls':iterations+101,'wall_us_per_call':wall*1e6/(iterations+101),
                     'cpu_us_per_call':cpu_seconds*1e6/(iterations+101),
                     'ppm_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),
                     'encoded_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
                     'timed_output':result.strip()})
    selected=[r for r in rows if r['case']==label]
    assert len({r['ppm_sha256'] for r in selected})==1
    means={v:{m:statistics.mean(r[m] for r in selected if r['version']==v)
              for m in ['wall_us_per_call','cpu_us_per_call']} for v in ['baseline','current']}
    print(json.dumps({'case':label,'means':means,'cpu_change_percent':
                     (means['current']['cpu_us_per_call']/means['baseline']['cpu_us_per_call']-1)*100}),flush=True)
summary=[]
for label,_,_ in cases:
    group=[r for r in rows if r['case']==label]
    item={'case':label}
    for metric in ['wall_us_per_call','cpu_us_per_call']:
        b=statistics.mean(r[metric] for r in group if r['version']=='baseline')
        h=statistics.mean(r[metric] for r in group if r['version']=='current')
        item[metric]={'baseline':b,'current':h,'change_percent':(h/b-1)*100}
    summary.append(item)
report={'baseline_git':args.baseline_git,
        'working_tree_dirty':bool(subprocess.check_output(['git','status','--porcelain'],cwd=repo,text=True).strip()),
        'candidate_git':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),
        'cpu_affinity':cpu,'order':'B/H/H/B/H/B','rust':'1.91.0 release',
        'warmup_calls':100,'measurement':'whole-process wall and child CPU / total calls, including export/startup',
        'source_sha256':hashlib.sha256((repo/'wml2/src/jpeg2000.rs').read_bytes()).hexdigest(),
        'baseline_binary_sha256':hashlib.sha256(binaries['baseline'].read_bytes()).hexdigest(),
        'candidate_binary_sha256':hashlib.sha256(binaries['current'].read_bytes()).hexdigest(),
        'stable_gray':args.stable_gray,
        'limitations':'shared-host sample, not a general performance guarantee; paired inputs must be supported by both binaries; only Gray is supported by the original stable baseline',
        'summary':summary,'rows':rows}
args.output.write_text(json.dumps(report,indent=2)+'\n')
