"""Paired multilevel milestone benchmark; run from repository root.
Pass absolute base/head jpeg2000_verify release binaries as the two arguments.
"""
import subprocess,resource,os,json,hashlib,argparse
from pathlib import Path
os.sched_setaffinity(0,{min(os.sched_getaffinity(0))})
parser=argparse.ArgumentParser()
parser.add_argument('base_binary');parser.add_argument('head_binary');args=parser.parse_args()
exe={'base':str(Path(args.base_binary).resolve()),'head':str(Path(args.head_binary).resolve())}
work=Path('.test-jpeg2000-deep-benchmark');work.mkdir(exist_ok=True)
rows=[]
for name in ['jp2_16x16_d1_noise','jp2_16x16_d1_ramp','jp2_multi_65x49_d1_noise','jp2_multi_257x129_d1_mixed']:
 for version in ['base','head','head','base']:
  path=f'wml2/tests/fixtures/review/{name}.jp2';out=str(work/f'{version}.ppm');count=10000 if name.startswith('jp2_16') else 100
  before=resource.getrusage(resource.RUSAGE_CHILDREN)
  result=subprocess.check_output([exe[version],path,out,str(count)],text=True).strip()
  after=resource.getrusage(resource.RUSAGE_CHILDREN)
  cpu=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime
  rows.append({'input':name,'version':version,'iterations':count,'result':result,'whole_process_cpu_us_per_call':cpu*1e6/(count+101),'sha256':hashlib.sha256(Path(out).read_bytes()).hexdigest()})
 assert len({r['sha256'] for r in rows if r['input']==name})==1
for name in ['jp2_deep_33x35_d5_noise','jp2_deep_257x129_d5_mixed','jp2_deep_1024x1024_d5_ramp']:
 result=subprocess.check_output([exe['head'],f'wml2/tests/fixtures/review/{name}.jp2',str(work/'large.ppm'),'20'],text=True).strip()
 rows.append({'input':name,'version':'head','iterations':20,'result':result})
(work/'report.json').write_text(json.dumps(rows,indent=2)+'\n')
for r in rows:print(r['version'],r['input'],r['result'],r.get('whole_process_cpu_us_per_call',''))
