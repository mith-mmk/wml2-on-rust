#!/usr/bin/env python3
"""Independent high-depth 9/7 fixtures; original and decoded reference stay separate."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import random
try:
    import resource
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
except ImportError:
    pass
import runpy
import struct
import subprocess

ROOT = Path(__file__).resolve().parent
TOOLS = runpy.run_path(str(ROOT/'generate_jpeg2000_ex_color.py'))
box, children, declare_rgb, pgx = [TOOLS[k] for k in ['box','children','declare_rgb','pgx']]


def run(*args):
    return subprocess.run(args, check=True, capture_output=True).stdout


def derived(data):
    """Scalar-derived control: increase band precision, retain every packet body.

    Choose epsilon0 so epsilon0-floor((band-1)/3) >= each original epsilon.
    Extra declared bitplanes shift the coded midpoint and its inverse step
    consistently. Changed mantissas intentionally change the reconstruction;
    independent decoded PGX, not the original image, is the reference.
    """
    raw = data if data.startswith(b'\xffO') else next(v for k,v in children(data) if k==b'jp2c')
    out = raw[:2];pos=2
    while raw[pos:pos+2] != b'\xff\x90':
        marker,length=struct.unpack_from('>HH',raw,pos)
        payload=raw[pos+4:pos+2+length]
        if marker in [0xff5c,0xff5d]:
            prefix=payload[:1] if marker==0xff5d else b''
            q=payload[len(prefix):];assert q[0]&31==2
            words=[int.from_bytes(q[i:i+2],'big') for i in range(1,len(q),2)]
            exponent=max((v>>11)+(max(i-1,0)//3) for i,v in enumerate(words))
            assert (q[0]>>5)+exponent<=25
            payload=prefix+bytes([(q[0]&0xe0)|1])+struct.pack('>H',(exponent<<11)|(words[0]&2047))
            out+=struct.pack('>HH',marker,len(payload)+2)+payload
        else:out+=raw[pos:pos+length+2]
        pos+=length+2
    out+=raw[pos:]
    if data.startswith(b'\xffO'):return out
    return b''.join(box(k,out if k==b'jp2c' else v) for k,v in children(data))


def quality(a,b):
    errors=[abs(x-y) for x,y in zip(a,b)]
    return dict(max_abs=max(errors),mean_abs=sum(errors)/len(errors),rmse=math.sqrt(sum(e*e for e in errors)/len(errors)))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--compress',required=True);p.add_argument('--decompress',required=True)
    p.add_argument('--work-dir',type=Path,required=True)
    args=p.parse_args();work=args.work_dir.resolve();assert work.name.startswith('.test');work.mkdir(exist_ok=True)
    cases=[('zero',1,1,0,[]),('max',1,1,0,[]),('edges',9,7,2,[]),
           ('noise',17,13,3,[]),('origin',33,29,3,['-d','3,5']),('tiles',33,29,2,['-t','16,16','-d','3,5','-T','1,2']),
           ('layers',33,29,3,['-p','RPCL','-r','8,4,1','-TP','L']),
           ('singleton',7,1,0,['-t','3,2','-d','1,1']),('deep',65,65,6,[]),
           ('lossy',33,29,3,['-r','8']),('bench',128,96,4,['-t','64,48'])]
    rows=[]
    for bits in [12,16]:
        maximum=(1<<bits)-1;edges=[0,1,255,256,257,maximum//2,maximum-1,maximum]
        for nc,kind in [(1,'gray'),(3,'rgb'),(4,'rgba')]:
            for name,w,h,levels,extra in cases+([('order',9,7,2,[])] if nc!=1 else []):
                stem=f'jp2_ex_97_{kind}{bits}_{name}';rng=random.Random(970000+bits*nc+w+h)
                planes=[[0 if name=='zero' else maximum if name=='max' else edges[(i*(2*c+1)+c+2)%len(edges)] if name=='edges' else rng.randrange(maximum+1) for i in range(w*h)] for c in range(nc)]
                if nc==4:planes[3]=[0 if name=='zero' else maximum if name=='max' else edges[i%len(edges)] for i in range(w*h)]
                source=planes[0] if nc==1 else [v for i in range(w*h) for v in [planes[0][i],planes[1][i],planes[2][i],planes[3][i] if nc==4 else maximum]]
                source_path=ROOT/f'{stem}.source.u16le';source_path.write_bytes(struct.pack(f'<{len(source)}H',*source))
                permutation=[2,0,1] if name=='order' else list(range(min(nc,3)))
                coded=[planes[c] for c in permutation]+([planes[3]] if nc==4 else [])
                original=work/'source.rawl';original.write_bytes(b''.join(struct.pack(f'<{w*h}H',*v) for v in coded))
                definitions=[(i,0,c+1) for i,c in enumerate(permutation)]
                if nc==4:definitions.append((3,1,0))
                for mct in ([0] if nc==1 else [0,1]):
                    for ext in (['j2k','jp2'] if nc!=4 and name!='order' else ['jp2']):
                        path=ROOT/f'{stem}_mct{mct}.{ext}'
                        run(args.compress,'-i',str(original),'-o',str(path),'-n',str(levels+1),'-F',f'{w},{h},{nc},{bits},u','-I','-mct',str(mct),'-b','16,16','-threads','1','-C','WML2 independent EX9/7 reference',*extra)
                        if ext=='jp2' and nc!=1:path.write_bytes(declare_rgb(path.read_bytes(),definitions if nc==4 or name=='order' else None))
                        variants=[(path,'expounded')]
                        if name in ['noise','deep']:
                            alternate=path.with_name(path.stem+'_derived.'+ext);alternate.write_bytes(derived(path.read_bytes()));variants.append((alternate,'derived'))
                        for encoded,quant in variants:
                            target=work/'decoded.pgx';run(args.decompress,'-i',str(encoded),'-o',str(target),'-threads','1')
                            decoded=[];refs=[]
                            for c in range(nc):
                                pg=work/f'decoded_{c}.pgx';depth,pw,ph,values=pgx(pg);assert (depth,pw,ph)==(bits,w,h)
                                ref=ROOT/f'{encoded.stem}_{ext}_{c}.pgx';ref.write_bytes(pg.read_bytes());refs.append(ref.name);decoded.append(values)
                            reference=list(decoded[0]) if nc==1 else [v for i in range(w*h) for v in [decoded[0][i],decoded[1][i],decoded[2][i],decoded[3][i] if nc==4 else maximum]]
                            refpath=ROOT/f'{encoded.stem}_{ext}.ref.u16le';refbytes=struct.pack(f'<{len(reference)}H',*reference)
                            if refpath.exists():assert refpath.read_bytes()==refbytes, (str(encoded), str(refpath))
                            refpath.write_bytes(refbytes)
                            rows.append(dict(input=encoded.name,source=source_path.name,reference=refpath.name,pgx=refs,bits=bits,width=w,height=h,components=nc,levels=levels,mct=mct,quantization=quant,source_quality=quality(source,reference),sha256=hashlib.sha256(encoded.read_bytes()).hexdigest()))
    (ROOT/'jpeg2000-ex-97-manifest.json').write_text(json.dumps(dict(provenance='Original seeded unsigned U16; OpenJPEG CLI2.5.3 encode and independent PGX decode. Reference reconstruction is separate from original source. Derived controls retain packet bodies and intentionally alter quantization.',files=rows),indent=2)+'\n')
    (ROOT/'jpeg2000-ex-97-manifest.txt').write_text(''.join(f"{r['input']} {r['reference']} {r['bits']} {r['width']} {r['height']} {r['components']}\n" for r in rows))
    print(len(rows),'independent lossy references')


if __name__=='__main__':main()
