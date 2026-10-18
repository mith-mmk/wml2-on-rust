#!/usr/bin/env python3
"""Large reduced outputs versus independent OpenJPEG and FFmpeg, bounded rows."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import numpy as np


def digest(path):
    h=hashlib.sha256()
    with Path(path).open('rb')as f:
        for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
    return h.hexdigest()


def pgx(path,bits,width,height):
    with Path(path).open('rb')as f:
        header=f.readline();offset=f.tell()
    magic,endian,sign,depth,w,h=header.split()
    assert (magic,sign,int(depth),int(w),int(h))==(b'PG',b'+',bits,width,height)
    dtype='u1'if bits==8 else('>u2'if endian==b'ML'else'<u2')
    return np.memmap(path,dtype=dtype,offset=offset,mode='r',shape=(height,width))


def difference(actual,planes,bits,nc,width,height,ffmpeg=False):
    maximum=(1<<bits)-1;total=0;squared=0;peak=0;per_channel=[0]*actual.shape[2]
    reference_hash=hashlib.sha256()
    for first in range(0,height,64):
        count=min(64,height-first)
        values=np.empty((count,width,actual.shape[2]),dtype=np.int32)
        if ffmpeg:
            values[:]=planes[first:first+count]
            if bits!=8:values>>=16-bits
        elif nc==1:
            values[:]=planes[0][first:first+count,:,None]
            if bits==8:values[:,:,3]=maximum
        else:
            for c in range(nc):values[:,:,c]=planes[c][first:first+count]
            if nc==3:values[:,:,3]=maximum
        reference_hash.update(values.astype('u1'if bits==8 else'<u2').tobytes())
        diff=np.abs(actual[first:first+count].astype(np.int32)-values)
        peak=max(peak,int(diff.max()));total+=int(diff.sum(dtype=np.int64));squared+=int((diff.astype(np.int64)**2).sum())
        per_channel=[max(old,int(diff[:,:,c].max()))for c,old in enumerate(per_channel)]
    samples=width*height*actual.shape[2]
    return dict(max_abs=peak,mean_abs=total/samples,rmse=(squared/samples)**.5,per_channel_max=per_channel,normalized_reference_sha256=reference_hash.hexdigest())


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--probe',type=Path,required=True);p.add_argument('--manifest',type=Path,required=True)
    p.add_argument('--decompress',required=True);p.add_argument('--ffmpeg',default='ffmpeg')
    p.add_argument('--work-dir',type=Path,required=True);p.add_argument('--stage',type=int)
    args=p.parse_args();root=args.work_dir.resolve();assert root.name.startswith('.test');root.mkdir(exist_ok=True)
    binary=args.probe.resolve();identity=digest(binary);record=root/'results.json'
    report=json.loads(record.read_text())if record.exists()else dict(probe_sha256=identity,rows=[])
    assert report['probe_sha256']==identity,'probe changed: use another verification directory'
    rows=report['rows'];done={(x['case'],x['reduce'])for x in rows}
    cases=[x for x in json.loads(args.manifest.read_text())if args.stage is None or x['stage']==args.stage]
    for case in cases:
        assert digest(case['input'])==case['sha256']
        bits,nc=case['bits'],case['components'];dtype='u1'if bits==8 else'<u2';channels=1 if bits!=8 and nc==1 else 4
        for reduce in [1,2,3]:
            if (case['name'],reduce)in done:continue
            actual_path=root/'actual.raw';command=[str(binary),case['input'],str(actual_path),str(reduce),str(bits),str(nc),'large']
            process=subprocess.run(command,capture_output=True,text=True);assert process.returncode==0,process.stderr
            measurement=json.loads(process.stdout);assert measurement['accepted'],measurement
            width,height=measurement['width'],measurement['height']
            actual=np.memmap(actual_path,dtype=dtype,mode='r',shape=(height,width,channels))
            for c in range(4):(root/f'reference_{c}.pgx').unlink(missing_ok=True)
            opj_command=[args.decompress,'-i',case['input'],'-o',str(root/'reference.pgx'),'-r',str(reduce),'-threads','1']
            process=subprocess.run(opj_command,capture_output=True,text=True);assert process.returncode==0,(process.stdout,process.stderr)
            refs=[pgx(root/f'reference_{c}.pgx',bits,width,height)for c in range(nc)]
            opj=difference(actual,refs,bits,nc,width,height)
            assert opj['max_abs']<=(0 if case['wavelet']=='53'else 4 if bits==16 else 1),(case['name'],reduce,opj)
            result=dict(case=case['name'],stage=case['stage'],reduce=reduce,bits=bits,components=nc,width=width,height=height,wavelet=case['wavelet'],output_sha256=digest(actual_path),openjpeg=opj,probe_command=command,openjpeg_command=opj_command)
            ff=root/'ffmpeg.raw';ff.unlink(missing_ok=True)
            pix='rgba'if bits==8 else'gray16le'if nc==1 else'rgba64le'
            ff_command=[args.ffmpeg,'-v','error','-threads','1','-lowres',str(reduce),'-c:v','jpeg2000','-i',case['input'],'-frames:v','1','-pix_fmt',pix,'-f','rawvideo','-y',str(ff)]
            process=subprocess.run(ff_command,capture_output=True,text=True)
            if process.returncode:result['ffmpeg_error']=dict(exit=process.returncode,stderr=process.stderr)
            elif ff.stat().st_size!=width*height*channels*(1 if bits==8 else 2):result['ffmpeg_error']=dict(exit=0,reason='unexpected output extent',bytes=ff.stat().st_size)
            else:
                decoded=np.memmap(ff,dtype=dtype,mode='r',shape=(height,width,channels))
                result['ffmpeg']=difference(actual,decoded,bits,nc,width,height,True)
                assert result['ffmpeg']['max_abs']<=(0 if case['wavelet']=='53'else 1),(case['name'],reduce,result['ffmpeg'])
                del decoded
            result['ffmpeg_command']=ff_command
            rows.append(result);done.add((case['name'],reduce));record.write_text(json.dumps(report,indent=2)+'\n')
            print(json.dumps({k:result[k]for k in ['case','reduce','width','height','openjpeg']}),flush=True)
            del actual,refs
    print(json.dumps(dict(cases=len(rows),openjpeg_max=max(x['openjpeg']['max_abs']for x in rows),ffmpeg_passed=sum('ffmpeg'in x for x in rows),ffmpeg_failures=[dict(case=x['case'],reduce=x['reduce'],error=x['ffmpeg_error'])for x in rows if'ffmpeg_error'in x])),flush=True)


if __name__=='__main__':main()
