#!/usr/bin/env python3
"""Staged reduction measurements: full depth/component matrix and large controls."""
import argparse
import hashlib
import json
from pathlib import Path
import resource
import runpy
import subprocess
import numpy as np


def digest(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
    return h.hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compress',required=True)
    parser.add_argument('--work-dir',type=Path,required=True)
    parser.add_argument('--large-manifest',type=Path,required=True)
    args=parser.parse_args()
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    root=Path(__file__).resolve().parents[2]
    work=args.work_dir.resolve()
    assert work.name.startswith('.test')
    work.mkdir(exist_ok=True)
    declare=runpy.run_path(str(root/'wml2/tests/fixtures/review/generate_jpeg2000_ex_color.py'))['declare_rgb']
    rows=[]
    w=h=1024
    x=np.arange(w,dtype=np.uint32)[None,:]
    y=np.arange(h,dtype=np.uint32)[:,None]
    for bits in [8,12,16]:
        maximum=(1<<bits)-1
        dtype='u1'if bits==8 else'<u2'
        for nc in [1,3,4]:
            raw=work/'source.rawl'
            with raw.open('wb')as f:
                for c in range(nc):
                    if c==3:
                        palette=np.array([0,1,maximum//4,maximum//2,maximum-1,maximum],dtype=dtype)
                        plane=palette[((x//16+y//16)%len(palette)).astype(np.uint8)]
                    else:plane=((x*[17,37,13][c]+y*[31,11,67][c])&maximum).astype(dtype)
                    plane.tofile(f)
                    del plane
            source_sha=digest(raw)
            planes=np.memmap(raw,dtype=dtype,mode='r',shape=(nc,h,w))
            packed=hashlib.sha256()
            for first in range(0,h,32):
                if bits!=8 and nc==1:values=np.asarray(planes[0,first:first+32],dtype=dtype)
                else:
                    values=np.empty((32,w,4),dtype=dtype)
                    if nc==1:values[:,:,:3]=planes[0,first:first+32,:,None]
                    else:values[:,:,:nc]=planes[:,first:first+32,:].transpose(1,2,0)
                    if nc!=4:values[:,:,3]=maximum
                packed.update(values.tobytes())
            del values,planes
            for wavelet in ['53','97']:
                name=f'cross1024-{bits}b-{nc}c-{wavelet}'
                path=work/(name+'.jp2')
                command=[args.compress,'-i',str(raw),'-o',str(path),'-F',f'{w},{h},{nc},{bits},u','-n','6','-t','256,256','-mct',str(int(nc>1)),'-b','64,64','-threads','1']
                if wavelet=='97':command+=['-I']
                subprocess.run(command,check=True,capture_output=True)
                if nc>1:path.write_bytes(declare(path.read_bytes(),[(0,0,1),(1,0,2),(2,0,3),(3,1,0)]if nc==4 else None))
                row=dict(name=name,input=str(path),stage=1,width=w,height=h,components=nc,bits=bits,wavelet=wavelet,tile=256,budgets=['default'],input_bytes=path.stat().st_size,sha256=digest(path),source_sha256=source_sha,source_packed_sha256=packed.hexdigest(),compress_command=command)
                rows.append(row)
                print(json.dumps({k:row[k]for k in ['name','input_bytes']}),flush=True)
    for item in json.loads(args.large_manifest.read_text()):
        if item['stage']<2:continue
        assert digest(item['input'])==item['sha256']
        rows.append(item)
    (work/'manifest.json').write_text(json.dumps(rows,indent=2)+'\n')
    print(json.dumps(dict(streams=len(rows),stages={str(n):sum(c['stage']==n for c in rows)for n in [1,2,3,4]})),flush=True)


if __name__=='__main__':main()
