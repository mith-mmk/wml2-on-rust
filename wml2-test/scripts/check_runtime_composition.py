import subprocess,json,time,os,argparse
from pathlib import Path
parser=argparse.ArgumentParser(description='Run the Linux runtime composition matrix; does not mutate Git refs.')
parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
if not args.output.name.startswith('.test'):parser.error('--output must start with .test')
root=Path(__file__).resolve().parents[2];os.chdir(root)
out=args.output.resolve();out.mkdir(parents=True,exist_ok=True);summary=[]
os.environ.setdefault('PYTHONDONTWRITEBYTECODE','1')
os.environ.setdefault('CARGO_TARGET_DIR',str(out/'.test-target'))
jxl=['--test', 'jpeg_xl','--test','jpeg_xl_additional','--test','jpeg_xl_layers','--test','jpeg_xl_brob','--test','jpeg_xl_metadata','--test','jpeg_xl_slice_admission','--test','image_info']
cases=[('disabled',['cargo','test','-p','wml2','--locked','--no-default-features',*jxl]),('ordinary',['cargo','test','-p','wml2','--locked','--no-default-features','--features','jpeg-xl','--lib',*jxl]),('ex',['cargo','test','-p','wml2','--locked','--no-default-features','--features','jpeg-xl,image-buffer-ex','--lib',*jxl]),('managed',['cargo','test','-p','wml2','--locked','--no-default-features','--features','jpeg-xl,exif,image-buffer-ex,color-management','--lib',*jxl]),('codecs',['cargo','test','-p','wml2','--locked','--no-default-features','--features','jpeg-xl,exif,image-buffer-ex,color-management,jpeg,png,tiff,gif,psd,idct_llm','--lib',*sum((['--test',n]for n in ['image_info','jpeg_scaled','png_encode_ex','png_encode','tiff_encode_ex','tiff_encode','tiff_interop','image_buffer_ex_icc','jpeg_png_icc_metadata']),[])]),*[(n,['cargo','test','-p','wml2','--locked','--no-default-features','--features','jpeg,'+n,'--test','jpeg_scaled'])for n in ['idct_aan','idct_slower']],('compatible',['python3','.github/scripts/check_feature_contracts.py','--output',str(out/'.test-compatible')]),('ci-selection',['python3','.github/scripts/test_ci_selection.py'])]
cases += [('avif-capability',['cargo','test','-p','wml2','--locked','--no-default-features','--features','avifenc,image-buffer-ex,high-bit-depth','--test','avif_feature_off','--test','avif_encode']),('encode-disabled',['cargo','test','-p','wml2','--locked','--no-default-features','--test','avif_feature_off','--test','png_encode_ex','--test','tiff_encode_ex']),('legacy-noex-check',['cargo','check','-p','wml2','--locked','--no-default-features','--features','png,tiff','--lib'])]
for name,cmd in cases:
 start=time.monotonic()
 with (out/(name+'.log')).open('w')as log:r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
 summary.append(dict(name=name,cmd=cmd,status=r.returncode,seconds=round(time.monotonic()-start,3)));(out/'matrix.json').write_text(json.dumps(summary,indent=2)+'\n');print(name,r.returncode,flush=True)
if any(row['status'] for row in summary):raise SystemExit(1)
