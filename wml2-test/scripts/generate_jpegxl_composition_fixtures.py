from pathlib import Path
import json,subprocess,hashlib,argparse
parser=argparse.ArgumentParser();parser.add_argument('--include',type=Path,required=True);parser.add_argument('--work',type=Path,required=True);args=parser.parse_args()
if not args.work.name.startswith('.test'):parser.error('--work must start with .test')
root=Path(__file__).resolve().parents[2]; fixtures=root/'wml2/tests/fixtures/review'; work=args.work.resolve();work.mkdir(parents=True,exist_ok=True);headers=args.include.resolve()
def bits(d):return [(v>>j)&1 for v in d for j in range(8)]
def little(v,n):return [(v>>i)&1 for i in range(n)]
def pack(b):
 b=b+[0]*((-len(b))%8);return bytes(sum(v<<j for j,v in enumerate(b[i:i+8])) for i in range(0,len(b),8))
def size(w,h):return [0]+[0,0]+little(h-1,9)+[0,0,0]+[0,0]+little(w-1,9)
source=(fixtures/'jxl-preview-oriented-icc16.jxl').read_bytes();manifest=json.loads((fixtures/'jxl-preview-fixtures.json').read_text());case=next(c for c in manifest['cases'] if c['file']=='jxl-preview-oriented-icc16.jxl');assert case['preview_start']==246
b=bits(source); assert b[36]==0
# C fixture layout: original 1940 header/ICC bits +27 preview/extra-field bits.
header=b[:1967]; assert all(v==0 for v in b[1967:1968]);header[36]=1
result=pack(header[:37]+size(19,11)+header[37:])+source[246:]
out=fixtures/'jxl-composition-preview-intrinsic.jxl';out.write_bytes(result)
for src,name in [('jpegxl_preview_reference.c','preview-reference'),('jpegxl_intrinsic_reference.c','dimensions')]:
 subprocess.run(['cc','-O2','-Wall','-Wextra','-Werror','-I'+str(headers),str(root/'wml2-test/scripts'/src),'-Wl,-l:libjxl.so.0.11','-o',str(work/name)],check=True)
r=subprocess.run([str(work/'preview-reference'),str(out),str(work/'main.rgba16le'),str(work/'main.icc'),str(work/'preview.rgba16le')],check=True,capture_output=True,text=True)
d=subprocess.run([str(work/'dimensions'),str(out),'0'],check=True,capture_output=True,text=True)
import zlib
expected=zlib.decompress((fixtures/'jxl-preview-oriented-icc16.rgba16le.zlib').read_bytes()); assert (work/'main.rgba16le').read_bytes()==expected
report={'input_sha256':hashlib.sha256(result).hexdigest(),'preview_reference':r.stdout.strip(),'dimensions':d.stdout.strip(),'main_native_matches_frozen_independent_oracle':True}
(work/'report.json').write_text(json.dumps(report,indent=2)+'\n'); print(report)
