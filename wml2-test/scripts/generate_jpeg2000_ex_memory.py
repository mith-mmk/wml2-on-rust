#!/usr/bin/env python3
"""Generate staged, untracked large JPEG2000 fixtures for native memory probes."""
import argparse
import hashlib
import json
from pathlib import Path
import resource
import runpy
import subprocess
import numpy as np


def digest(path):
    result=hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda:source.read(1024*1024),b''):result.update(block)
    return result.hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compress',required=True)
    parser.add_argument('--work-dir',type=Path,required=True)
    parser.add_argument('--stage',type=int,choices=[1,2,3,4],required=True)
    args=parser.parse_args();resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    work=args.work_dir.resolve();assert work.name.startswith('.test');work.mkdir(exist_ok=True)
    helper=Path(__file__).resolve().parents[2]/'wml2/tests/fixtures/review/generate_jpeg2000_ex_color.py';declare=runpy.run_path(str(helper))['declare_rgb']
    cases={1:[(1024,1024,4,16,'ramp',0),(1024,1024,4,16,'ramp',256),(1024,1024,4,16,'noise',256),(1024,1024,4,12,'ramp',256)],2:[(2048,1536,1,16,'ramp',0),(2048,1536,4,16,'ramp',0),(2048,1536,4,16,'ramp',256),(2048,1536,3,16,'ramp',256)],3:[(4096,4096,4,16,'ramp',0),(4096,4096,4,16,'ramp',512)],4:[(8192,4096,4,16,'ramp',512)]}[args.stage]
    manifest=work/'manifest.json';rows=json.loads(manifest.read_text()) if manifest.exists() else []
    for w,h,nc,bits,pattern,tile in cases:
        name=f'{w}x{h}-{nc}c-{bits}b-{pattern}-t{tile}';raw=work/'source.rawl';maximum=(1<<bits)-1;rng=np.random.default_rng(170000+w+h+bits+nc)
        x=np.arange(w,dtype=np.uint32)[None,:];y=np.arange(h,dtype=np.uint32)[:,None]
        with raw.open('wb') as out:
            for c in range(nc):
                if pattern=='noise':plane=rng.integers(0,maximum+1,size=(h,w),dtype=np.uint16)
                elif c==3:
                    palette=np.array([0,1,255,256,257,maximum-1,maximum],dtype=np.uint16);plane=palette[((x//16+y//16)%7).astype(np.uint8)]
                else:plane=((x*[17,37,13][c]+y*[31,11,67][c])&maximum).astype('<u2')
                plane.tofile(out);del plane
        source_sha=digest(raw);planes=np.memmap(raw,dtype='<u2',mode='r',shape=(nc,h,w));expected=hashlib.sha256()
        for row in range(0,h,32):
            if nc==1:packed=np.asarray(planes[0,row:row+32,:],dtype='<u2')
            else:
                packed=np.empty((min(32,h-row),w,4),dtype='<u2');packed[:,:,:nc]=planes[:,row:row+32,:].transpose(1,2,0)
                if nc==3:packed[:,:,3]=maximum
            expected.update(packed.tobytes())
        del packed,planes
        for wavelet in ['53','97']:
            path=work/f'{name}-{wavelet}.jp2';command=[args.compress,'-i',str(raw),'-o',str(path),'-n','6','-F',f'{w},{h},{nc},{bits},u','-mct',str(int(nc>1)),'-b','64,64','-threads','1','-C','WML2 native EX memory probe']
            if tile:command+=['-t',f'{tile},{tile}']
            if wavelet=='97':command+=['-I']
            subprocess.run(command,check=True,capture_output=True)
            if nc>1:path.write_bytes(declare(path.read_bytes(),[(0,0,1),(1,0,2),(2,0,3),(3,1,0)] if nc==4 else None))
            row=dict(name=path.stem,input=str(path),stage=args.stage,width=w,height=h,components=nc,bits=bits,pattern=pattern,tile=tile,wavelet=wavelet,native_format=('gray' if nc==1 else 'rgba')+str(bits),source_sha256=source_sha,source_packed_sha256=expected.hexdigest(),sha256=digest(path),input_bytes=path.stat().st_size,compress_command=command,budgets=['default','large'] if args.stage==3 and tile==0 else ['default'])
            rows=[r for r in rows if r['name']!=row['name']];rows.append(row);manifest.write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps({k:row[k]for k in ['name','stage','input_bytes']}),flush=True)


if __name__=='__main__':main()
