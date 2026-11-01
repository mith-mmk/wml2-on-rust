#!/usr/bin/env python3
"""Independent C decoder/LittleCMS2.16 animation display references."""
from pathlib import Path
import ctypes as C,json,hashlib,argparse,subprocess
p=argparse.ArgumentParser();p.add_argument("--work",type=Path,required=True);a=p.parse_args()
if not a.work.name.startswith(".test"):p.error("work directory must start with .test")
work=a.work.resolve();root=Path(__file__).resolve().parents[2]
import os;os.chdir(root)
cms=C.CDLL('liblcms2.so.2');cms.cmsGetEncodedCMMversion.restype=C.c_uint32;assert cms.cmsGetEncodedCMMversion()==2160
cms.cmsOpenProfileFromMem.argtypes=[C.c_void_p,C.c_uint32];cms.cmsOpenProfileFromMem.restype=C.c_void_p
cms.cmsCreate_sRGBProfile.restype=C.c_void_p
cms.cmsCreateTransform.argtypes=[C.c_void_p,C.c_uint32,C.c_void_p,C.c_uint32,C.c_uint32,C.c_uint32];cms.cmsCreateTransform.restype=C.c_void_p
cms.cmsDoTransform.argtypes=[C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32];cms.cmsDeleteTransform.argtypes=[C.c_void_p];cms.cmsCloseProfile.argtypes=[C.c_void_p]
rows=[];fixtures=Path('wml2/tests/fixtures/review');manifest=json.load(open('docs/jpeg-xl-buffered-animation-oracle.json'))
for c in manifest['oracle']['cases']:
 if not c['buffered_main5_supported'] or any(f['ticks']*c['tps_denominator']*1000%c['tps_numerator']for f in c['frames']):continue
 stem=c['file'][:-4];subprocess.run([str(work/'reference'),str(fixtures/c['file']),str(work/stem)],check=True,capture_output=True);icc=(work/(stem+'.icc')).read_bytes();buf=C.create_string_buffer(icc);source=cms.cmsOpenProfileFromMem(buf,len(icc));dest=cms.cmsCreate_sRGBProfile();fmt=(4<<16)|(1<<7)|(3<<3)|2
 t=cms.cmsCreateTransform(source,fmt,dest,fmt,1,0x04000100);assert t
 files=[]
 for i,f in enumerate(c['frames']):
  raw=(fixtures/(stem+f'.{i}.rgba16le')).read_bytes();inp=C.create_string_buffer(raw);out=(C.c_uint16*(len(raw)//2))();cms.cmsDoTransform(t,inp,out,len(raw)//8)
  data=bytes((v*255+32767)//65535 for v in out);name=stem+f'.{i}.srgb8';(fixtures/name).write_bytes(data);files.append(dict(file=name,sha256=hashlib.sha256(data).hexdigest()))
 rows.append(dict(input=c['file'],source_icc_sha256=hashlib.sha256(icc).hexdigest(),frames=files));cms.cmsDeleteTransform(t);cms.cmsCloseProfile(source);cms.cmsCloseProfile(dest)
maximum_file=fixtures/'jxl-animation-timebase-max.jxl'
result=subprocess.run([str(work/'reference'),str(maximum_file),str(work/'timebase-max')],check=True,capture_output=True,text=True)
assert ' 1 1024 4294967295 0 0' in result.stdout
for i in range(3):assert (work/f'timebase-max.{i}.rgba16le').read_bytes()==(fixtures/f'jxl-animation-long.{i}.rgba16le').read_bytes()
maximum=dict(file=maximum_file.name,sha256=hashlib.sha256(maximum_file.read_bytes()).hexdigest(),construction='C encoder long8 orientation1, numerator1 denominator1024 loops4294967295; C validates all3 frames and byte-compares to committed jxl-animation-long goldens',reference_output=result.stdout)
Path('docs/jpeg-xl-animation-callback-oracle.json').write_text(json.dumps(dict(maximum_timebase_fixture=maximum,generator_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),source='C libjxl0.11.2 coalesced RGBA16 and C output ICC from PR93 manifest; independent LittleCMS2.16 relative intent, NOOPTIMIZE, COPY_ALPHA; quantize16-to8 last',cases=rows),indent=2)+'\n')
Path('wml2/tests/jpeg_xl/animation_display_cases.rs').write_text('[\n'+ '\n'.join('(include_bytes!("../fixtures/review/'+r['input']+'").as_slice(),&['+','.join('include_bytes!("../fixtures/review/'+f['file']+'").as_slice()'for f in r['frames'])+']as &[&[u8]]),'for r in rows)+'\n]\n')
print(len(rows),sum(len(r['frames'])for r in rows))
