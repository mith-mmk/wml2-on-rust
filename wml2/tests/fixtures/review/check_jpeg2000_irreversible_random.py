"""Seeded independent irreversible8bit oracle comparisons through public Rust API.

Every requested tuple parameter is checked against actual SIZ/COD by the isolated
encoder worker. Tiles retain at least2^levels samples per axis to avoid the known
OpenJPEG2.5.4 deep clipped/singleton encoder assertion. That fixture-generator
restriction is not a decoder acceptance rule. Failures are recorded, never silently
counted or replaced. Quantized reconstruction compares to decoded oracle(max1);
lossy source quality is separately recorded. Fixed controls cover small boundaries.
"""
import argparse,hashlib,json,random,subprocess,sys
from pathlib import Path
from PIL import Image
sys.dont_write_bytecode=True
import generate_jpeg2000_irreversible as G
parser=argparse.ArgumentParser();parser.add_argument('verifier',type=Path);parser.add_argument('--cases',type=int,default=24)
args=parser.parse_args();assert args.cases>0
repo=Path(__file__).resolve().parents[4];work=repo/'.test-jpeg2000-irreversible-random';work.mkdir(exist_ok=True)
rng=random.Random(0x4952524556);rows=[];failures=[]
shapes=[(1<<x,1<<y) for x in range(2,11) for y in range(2,11) if x+y<=12]
for index in range(args.cases):
 levels=index%6;factor=1<<levels;tw,th=2*factor+rng.randrange(factor),2*factor+rng.randrange(factor)
 nx,ny=rng.randint(2,3),rng.randint(2,3);sx,sy=rng.randrange(factor),rng.randrange(factor)
 w=(nx-1)*tw-sx+factor+rng.randrange(factor);h=(ny-1)*th-sy+factor+rng.randrange(factor)
 x0=32768-w if index%6==5 else rng.randint(100,32768-w)
 y0=32768-h if index%6==5 else rng.randint(100,32768-h)
 block=rng.choice(shapes);strategy=['single','packet','pair','alternate'][index%4]
 for components in [1,3]:
  source=rng.randbytes(w*h*components);source_path=work/f'{index}-{components}.source';source_path.write_bytes(source)
  for mct in ([0] if components==1 else [0,1]):
   rate=[None,2,4,10][(index+components+mct)%4]
   for ext in ['j2k','jp2']:
    stem=f'{index}-{components}-{mct}.{ext}';original=work/(stem+'.plt');expected=work/(stem+'.oracle');request_file=work/'request.json';response_file=work/'response.json'
    params=dict(no_jp2=ext=='j2k',offset=[x0,y0],tile_offset=[x0-sx,y0-sy],tile_size=[tw,th],num_resolutions=levels+1,codeblock_size=list(block),irreversible=True,mct=mct,plt=True,quality_mode='rates',progression='LRCP')
    if rate is not None:params['quality_layers']=[rate]
    request=dict(encoder_params=params,source_path=str(source_path),components=components,size=[w,h],encoded_path=str(original),oracle_path=str(expected))
    request_file.write_text(json.dumps(request));run=subprocess.run([sys.executable,str(G.ROOT/'generate_jpeg2000_irreversible.py'),'--worker',str(request_file),str(response_file)],cwd=work,capture_output=True,text=True)
    if run.returncode:
     failures.append(dict(case=stem,params=params,status=run.returncode,stderr=run.stderr[-2000:]));continue
    response=json.loads(response_file.read_text());assert response['profile']['tilecount']==nx*ny
    encoded,plt=G.PARTS.split_plt(original.read_bytes(),strategy)
    if index%3!=0:encoded,counts=G.UNKNOWN.rewrite_tnsot(encoded,['all_zero','late_known','alternating'][index%3])
    else:counts=None
    if index%2==0:encoded,terminal=G.TERMINAL.terminal_psot(encoded)
    else:terminal=None
    path=work/stem;path.write_bytes(encoded);ppm=work/'decoded.ppm'
    result=subprocess.run([str(args.verifier.resolve()),str(path),str(ppm),'--export-only'],capture_output=True,text=True)
    assert result.returncode==0,(stem,result.stderr)
    with Image.open(ppm) as public:
     assert public.size==(w,h);actual=public.tobytes()
    oracle=expected.read_bytes();rgb=bytes(v for p in oracle for v in ([p]*3 if components==1 else [p]));assert len(actual)==len(rgb)
    differences=[int(a)-int(b) for a,b in zip(actual,rgb)];maximum=max(map(abs,differences));assert maximum<=1,(stem,maximum)
    rows.append(dict(case=stem,encoder_params=params,width=w,height=h,components=components,levels=levels,mct=mct,rate=rate,block=block,tile_count=nx*ny,
                     source_sha256=G.sha(source),encoded_sha256=G.sha(encoded),oracle_sha256=G.sha(oracle),public_ppm_sha256=G.sha(ppm.read_bytes()),
                     max_abs_error=maximum,mean_abs_error=sum(map(abs,differences))/len(differences),mean_signed_error=sum(differences)/len(differences),different_channels=sum(d!=0 for d in differences),channels=len(differences),
                     source_quality=response['source_quality'],max_parts_per_tile=plt['max_parts_per_tile'],peak_join_bytes=plt['peak_join_bytes'],plt_proof=plt,counts_proof=counts,terminal_proof=terminal))
 report=dict(seed=0x4952524556,requested_geometries=args.cases,comparisons=len(rows),encoder_failures=failures,all_passed=not failures and len(rows)==args.cases*6,
             max_abs_error_policy=1,comparison_target='OpenJPEG decoded oracle',source_quality_is_decoder_error=False,versions=G.VERSIONS,
             production_source_sha256={p.name:G.sha(p.read_bytes()) for p in (repo/'wml2/src').glob('jpeg2000*.rs')},rows=rows)
 (work/'report.json').write_text(json.dumps(report,indent=2)+'\n')
assert report['all_passed'],failures
print(json.dumps({k:v for k,v in report.items() if k not in ['rows','production_source_sha256']}))
