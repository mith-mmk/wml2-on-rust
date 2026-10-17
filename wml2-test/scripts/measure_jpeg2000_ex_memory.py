#!/usr/bin/env python3
"""Paired fresh-process live-allocation/RSS measurements, with resumable rows."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
    return h.hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base',type=Path,required=True);parser.add_argument('--current',type=Path,required=True)
    parser.add_argument('--manifest',type=Path,required=True);parser.add_argument('--work-dir',type=Path,required=True)
    parser.add_argument('--stage',type=int,required=True);parser.add_argument('--pairs',type=int,default=2)
    args=parser.parse_args();root=args.work_dir.resolve();assert root.name.startswith('.test');root.mkdir(exist_ok=True)
    binaries={'base':args.base.resolve(),'current':args.current.resolve()};identity={k:sha(p)for k,p in binaries.items()};cases=[c for c in json.loads(args.manifest.read_text())if c['stage']==args.stage]
    record=root/'measurements.json';report=json.loads(record.read_text())if record.exists()else dict(base_commit='fd605d82347c80ba7ac7cd05ac56b5b6a917ae13',binaries=identity,rows=[])
    assert report['binaries']==identity,'measurement binary changed: use a new work directory'
    rows=report['rows'];done={tuple(r[k]for k in ['case','budget','mode','format','pair','variant'])for r in rows};available=sorted(os.sched_getaffinity(0));cpu=available[0]
    if len(available)>1:os.sched_setaffinity(0,{available[1]})
    cg=Path('/sys/fs/cgroup')
    for c in cases:
        modes=[('owned',c['native_format'],'exact'),('callback',c['native_format'],'exact'),('discard',c['native_format'],'exact')]
        if c['stage'] in [1,2] and c['components']==4 and c['bits']==16 and c['tile']>0 and c['pattern']=='ramp':modes.append(('owned','rgba8','round'))
        if c['stage'] in [1,2] and c['components']==4 and c['bits']==16 and c['tile']>0 and c['pattern']=='ramp':modes.append(('highres',c['native_format'],'exact'))
        for budget in c['budgets']:
            for mode,fmt,policy in modes:
                for pair in range(args.pairs):
                    for variant in (['base','current']if pair%2==0 else ['current','base']):
                        key=(c['name'],budget,mode,fmt,pair,variant)
                        if key in done:continue
                        assert int((cg/'memory.current').read_text())<12*1024**3,'cgroup already over conservative12GiB stop'
                        output=root/'pixels.raw';log=root/'process.log';binary=binaries[variant];cmd=['taskset','-c',str(cpu),str(binary),c['input'],str(output),mode,fmt,policy,budget]
                        peak_rss=0;peak_cgroup=0;start=time.monotonic()
                        with log.open('w')as stdout:
                            process=subprocess.Popen(cmd,stdout=stdout,stderr=subprocess.STDOUT)
                            while process.poll()is None:
                                current=int((cg/'memory.current').read_text());peak_cgroup=max(peak_cgroup,current)
                                if current>12*1024**3:process.kill();raise RuntimeError('stopped own probe above12GiB cgroup')
                                try:
                                    if Path(f'/proc/{process.pid}/exe').resolve()==binary:
                                        m=re.search(r'VmHWM:\s+(\d+)',Path(f'/proc/{process.pid}/status').read_text())
                                        if m:peak_rss=max(peak_rss,int(m[1]))
                                except(FileNotFoundError,ProcessLookupError,PermissionError):pass
                                time.sleep(.002)
                        assert process.returncode==0,log.read_text()
                        measurement=json.loads(log.read_text());measurement.update(case=c['name'],budget=budget,mode=mode,format=fmt,policy=policy,pair=pair,variant=variant,elapsed_process_seconds=time.monotonic()-start,sampled_exec_vmhwm_kib=peak_rss,peak_cgroup_bytes=peak_cgroup,output_sha256=sha(output)if measurement['accepted']else None,command=cmd)
                        expected=not(c['stage']==3 and c['tile']==0 and budget=='default')
                        assert measurement['accepted']==expected,(c['name'],budget,measurement['error'])
                        if measurement['accepted']and c['wavelet']=='53'and fmt==c['native_format']:assert measurement['output_sha256']==c['source_packed_sha256'],c['name']
                        rows.append(measurement);done.add(key);record.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:measurement[k]for k in ['case','budget','mode','format','variant','pair','accepted','peak_extra_bytes','self_vmhwm_kib']}),flush=True)
    groups={}
    for r in rows:groups.setdefault(tuple(r[k]for k in ['case','budget','mode','format','pair']),[]).append(r)
    for key,group in groups.items():
        if len(group)==2:
            a,b=group;assert a['accepted']==b['accepted']and a['output_sha256']==b['output_sha256'],key
            if not a['accepted']:assert a['error']==b['error'],key
    report.update(cpu=cpu,peak_measure='Rust requested live allocation bytes; separately self/sampled post-exec VmHWM. Callback/discard operation_us includes sink file I/O; compare timings only within same mode. Cgroup includes cache/other processes; not decoder RSS.',memory_events=(cg/'memory.events').read_text());record.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':main()
