#!/usr/bin/env python3
"""Paired old/current full decode and current native reductions, fresh processes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


def digest(path):
    h=hashlib.sha256()
    with Path(path).open('rb')as f:
        for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
    return h.hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base',type=Path,required=True)
    parser.add_argument('--current',type=Path,required=True)
    parser.add_argument('--manifest',type=Path,required=True)
    parser.add_argument('--work-dir',type=Path,required=True)
    parser.add_argument('--stage',type=int,choices=[1,2,3,4],required=True)
    parser.add_argument('--pairs',type=int,default=2)
    args=parser.parse_args()
    root=args.work_dir.resolve()
    assert root.name.startswith('.test')
    root.mkdir(exist_ok=True)
    binaries=dict(base=args.base.resolve(),current=args.current.resolve())
    identity={key:digest(path)for key,path in binaries.items()}
    path=root/'measurements.json'
    report=json.loads(path.read_text())if path.exists()else dict(base_commit='bc6c1182c7478c385b10296af71c996c4f4f3ab7',binaries=identity,rows=[])
    assert report['binaries']==identity,'binary changed: use a separate measurement directory'
    rows=report['rows']
    fields=['case','budget','pair','variant','reduce']
    done={tuple(r[k]for k in fields)for r in rows}
    cpus=sorted(os.sched_getaffinity(0));cpu=cpus[0]
    if len(cpus)>1:os.sched_setaffinity(0,{cpus[1]})
    cgroup=Path('/sys/fs/cgroup')
    cases=[c for c in json.loads(args.manifest.read_text())if c['stage']==args.stage]
    for case in cases:
        assert digest(case['input'])==case['sha256']
        for budget in case['budgets']:
            for pair in range(args.pairs):
                specs=[('base',0),('current',0),('current',1),('current',2),('current',3)]
                if pair%2:specs.reverse()
                for variant,reduce in specs:
                    key=(case['name'],budget,pair,variant,reduce)
                    if key in done:continue
                    assert int((cgroup/'memory.current').read_text())<12*1024**3,'cgroup already above12GiB'
                    binary=binaries[variant];output=root/'pixels.raw';log=root/'process.log'
                    command=['taskset','-c',str(cpu),str(binary),case['input'],str(output),str(reduce),str(case['bits']),str(case['components']),budget]
                    peak_rss=0;peak_cgroup=0;start=time.monotonic()
                    with log.open('w')as f:
                        process=subprocess.Popen(command,stdout=f,stderr=subprocess.STDOUT)
                        while process.poll()is None:
                            memory=int((cgroup/'memory.current').read_text());peak_cgroup=max(peak_cgroup,memory)
                            if memory>12*1024**3:
                                process.kill();process.wait();raise RuntimeError('stopped own probe over12GiB')
                            try:
                                if Path(f'/proc/{process.pid}/exe').resolve()==binary:
                                    m=re.search(r'VmHWM:\s+(\d+)',Path(f'/proc/{process.pid}/status').read_text())
                                    if m:peak_rss=max(peak_rss,int(m[1]))
                            except(FileNotFoundError,ProcessLookupError,PermissionError):pass
                            time.sleep(.002)
                    assert process.returncode==0,log.read_text()
                    result=json.loads(log.read_text())
                    result.update(case=case['name'],stage=case['stage'],budget=budget,pair=pair,variant=variant,route='legacy RGBA8 callback'if case['bits']==8 else'native EX owned',elapsed_process_seconds=time.monotonic()-start,sampled_exec_vmhwm_kib=peak_rss,peak_cgroup_bytes=peak_cgroup,output_sha256=digest(output)if result['accepted']else None,command=command)
                    if result['accepted']:
                        expected=((case['width']+(1<<reduce)-1)//(1<<reduce),(case['height']+(1<<reduce)-1)//(1<<reduce))
                        assert (result['width'],result['height'])==expected
                        if reduce==0 and case['wavelet']=='53':assert result['output_sha256']==case['source_packed_sha256']
                    rows.append(result);done.add(key);path.write_text(json.dumps(report,indent=2)+'\n')
                    print(json.dumps({k:result[k]for k in ['case','budget','variant','reduce','accepted','peak_extra_bytes','self_vmhwm_kib','operation_us']}),flush=True)
    for case in cases:
        for budget in case['budgets']:
            for pair in range(args.pairs):
                group=[r for r in rows if r['case']==case['name']and r['budget']==budget and r['pair']==pair and r['reduce']==0]
                assert len(group)==2
                a,b=group;assert a['accepted']==b['accepted']and a['output_sha256']==b['output_sha256'],(case['name'],budget)
                if not a['accepted']:assert a['error']==b['error']
    for key in {(r['case'],r['budget'],r['variant'],r['reduce'])for r in rows}:
        group=[r for r in rows if tuple(r[k]for k in ['case','budget','variant','reduce'])==key]
        assert len({(r['accepted'],r['output_sha256'])for r in group})==1,key
    report.update(cpu=cpu,method='Requested live allocation bytes above already-loaded input baseline; separate self/sampled post-exec VmHWM and cgroup usage. Atomic allocator instrumentation included in operation_us. Fresh processes; alternating order; original APIs for both zero-mode builds. Generic/codestream dispatch may differ for nonzero API. No universal RSS/time guarantee.',memory_events=(cgroup/'memory.events').read_text())
    path.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':main()
