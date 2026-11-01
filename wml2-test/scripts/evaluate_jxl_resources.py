#!/usr/bin/env python3
"""Bounded, trusted-fixture resource evaluation; shipping runtime/caps unchanged.
Requires Rust 1.91, libjxl0.11.2 headers/runtime and an independent C encoder.
--prepare-only creates the isolated probe/fixtures for manual measurements.
"""
import argparse, hashlib, json, os, subprocess, struct
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--encoder',type=Path,required=True)
p.add_argument('--prepare-only',action='store_true')
a=p.parse_args()
if not a.output.name.startswith('.test'):p.error('--output must start with .test')
root=Path(__file__).resolve().parents[2];out=a.output.resolve()
# Never overwrite a checkout, another task's directory or unique user inputs.
owner=out/'.test-jxl-resource-owner';identity=str(root)+'/native-resource-v1'
if out==root or root.is_relative_to(out) or (out/'.git').exists():p.error('--output must not contain a checkout')
if out.exists() and any(out.iterdir()) and not owner.is_file():p.error('--output must be empty or owned by this probe')
if owner.exists() and owner.read_text()!=identity:p.error('--output belongs to another probe/check-out')
out.mkdir(parents=True,exist_ok=True);owner.write_text(identity)
fixtures=root/'wml2/tests/fixtures/review';encoder=a.encoder.resolve()
def run(cmd,log=None,**kwargs):
 kwargs.setdefault('cwd',root)
 r=subprocess.run(list(map(str,cmd)),capture_output=True,text=True,**kwargs)
 if log:(out/log).write_text(r.stdout+r.stderr)
 r.check_returncode();return r.stdout
source=(root/'wml2/src/jpegxl.rs').read_text()
marker='// Validate the profile envelope instead of trusting channels()'
assert source.count(marker)==1
prefix=source.split(marker)[0]
assert 'fn preflight(' in prefix and 'const MAX_PIXELS: usize = 4 * 1024 * 1024;' in prefix
wrapper='''
pub fn describe(bytes:&[u8],budget:DecodeLimits)->Result<[usize;10],Error> {
 limits::check(bytes.len(),budget.input_bytes.min(MAX_INPUT),"JPEG XL input")?;
 limits::check(mul(bytes.len(),3)?,budget.expanded_bytes,"JPEG XL input assembly workspace")?;
 let input=unpack(bytes,budget)?;let h=preflight(&input,budget)?;
 let mut br=BitReader::new(&input.codestream[..input.codestream.len().min(MAX_HEADER)]);
 let header=FileHeader::read(&mut br).map_err(codec_error)?;
 Ok([h.width,h.height,h.bits as usize,usize::from(h.gray),input.metadata_bytes,h.workspace,h.sample_limit,input.codestream.len(),h.source_header_bytes,3+header.image_metadata.extra_channel_info.len()])
}
'''
# Only leading inner doc comments become ordinary comments for include!.
(out/'admission.rs').write_text(prefix.replace('//!','//')+wrapper)
(out/'Cargo.toml').write_text('''[package]
name="jxl-resource-evaluation"
version="0.0.0"
edition="2024"
[workspace]
[dependencies]
wml2={path='''+json.dumps(str(root/'wml2'))+''',default-features=false,features=["jpeg-xl","image-buffer-ex","color-management"]}
jxl_codec={package="jxl",version="=0.7.4",default-features=false}
bin-rs={version="0.0.10",features=["util"]}
serde_json="1"
[[bin]]
name="resource-probe"
path='''+json.dumps(str(root/'wml2-test/scripts/jxl_resource_probe.rs'))+'\n')
# Inputs are either existing pinned fixtures or deterministic libjxl-encoded
# inputs. No patched WML2 or modified codec is used to encode them.
generated=[]
for name,w,h,c in [('boundary',2048,1536,3),('beyond',2049,1536,1),('qhd',2560,1440,3),('thin',16384,1,1),('wide',16385,1,1)]:
 raw=out/(name+'.raw');raw.write_bytes((bytes([64,128,192]) if c==3 else bytes([133]))*(w*h))
 encoded=out/(name+'.jxl');run([encoder,raw,encoded,w,h,c,8,'lossless','-'],name+'-encode.log');generated.append(encoded)
# A valid matrix/TRC profile with a private opaque tag; original public tag
# offsets shift by twelve bytes, so all existing colorimetry remains intact.
profile=bytearray((fixtures/'jxl-gamma2.icc').read_bytes());n=struct.unpack_from('>I',profile,128)[0];end=132+n*12
for k in range(n):struct.pack_into('>I',profile,132+k*12+4,struct.unpack_from('>I',profile,132+k*12+4)[0]+12)
profile[end:end]=b'\0'*12;struct.pack_into('>I',profile,128,n+1)
payload=b'text'+b'\0'*4+b'x'*(1024*1024-9)+b'\0';at=len(profile)
struct.pack_into('>4sII',profile,end,b'zzzz',at,len(payload));profile.extend(payload);struct.pack_into('>I',profile,0,len(profile))
icc=out/'large.icc';icc.write_bytes(profile);raw=out/'small.raw';raw.write_bytes(bytes([64,128,192])*35)
encoded=out/'large-icc.jxl';run([encoder,raw,encoded,7,5,3,8,'lossless',icc],'icc-encode.log');generated.append(encoded)
signature=b'\0\0\0\x0cJXL \r\n\x87\n'
def box(kind,data):return struct.pack('>I4s',len(data)+8,kind)+data
small=(fixtures/'jxl-rgb8.jxl').read_bytes()
ftyp=box(b'ftyp',b'jxl \0\0\0\0jxl ')
for mb in [1,8]:
 encoded=out/f'metadata-{mb}.jxl';encoded.write_bytes(signature+ftyp+box(b'zzzz',b'x'*(mb<<20))+box(b'jxlc',small));generated.append(encoded)
for suffix,size in [('at',64<<20),('over',(64<<20)+1)]:
 large=out/f'input-cap-{suffix}.jxl';payload=size-len(signature)-len(ftyp)-8-8-len(small)
 large.write_bytes(signature+ftyp+box(b'zzzz',b'x'*payload)+box(b'jxlc',small));generated.append(large)
layer=(fixtures/'jxl-sunset-logo.jxl').read_bytes()
for count in [64,65]:
 encoded=out/f'layers-{count}.jxl';encoded.write_bytes(layer[:9]+layer[9:125]*(count-1)+layer[125:]);generated.append(encoded)
identities={str(path.relative_to(root)) if path.is_relative_to(root) else str(path):{'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()} for path in generated}
(out/'inputs.json').write_text(json.dumps(identities,indent=2)+'\n')
env=os.environ.copy();env['JXL_RESOURCE_OUT']=str(out)
# Freeze the exact development graph used for the recorded native probe;
# the repository runtime lockfile remains untouched.
(out/'Cargo.lock').write_bytes((root/'wml2-test/scripts/jxl_resource_probe.lock').read_bytes())
run(['cargo','build','--locked','--release','--manifest-path',out/'Cargo.toml','--target-dir',out/'.test-target'],'build.log',env=env)
exe=out/'.test-target/release/resource-probe'
if a.prepare_only:print(exe);raise SystemExit
report={'runtime_sha':run(['git','rev-parse','HEAD']).strip(),'runtime_source_sha256':hashlib.sha256(source.encode()).hexdigest(),'admission_prefix_sha256':hashlib.sha256(prefix.encode()).hexdigest(),'probe_sha256':hashlib.sha256((root/'wml2-test/scripts/jxl_resource_probe.rs').read_bytes()).hexdigest(),'manifest_sha256':hashlib.sha256((out/'Cargo.toml').read_bytes()).hexdigest(),'lock_sha256':hashlib.sha256((out/'Cargo.lock').read_bytes()).hexdigest(),'scope':'trusted fixtures; preflight source extraction unchanged; no shipping runtime/cap changes; requested heap and process RSS are different quantities','rustc':run(['rustc','-vV']),'inputs':identities,'records':[]}
cases=[('jxl-rgb8.jxl','rgba8'),('jxl-rgba16.jxl','rgba16'),('jxl-icc-gray16.jxl','gray16'),('jxl-icc-lossless.jxl','rgba8'),('jxl-jbrd-bench.jxl','rgba8'),('jxl-sunset-logo.jxl','rgba16')]
for name,ex in cases:
 for route in ['inspect','ordinary',ex,'cms']:
  for trial in range(3):
   item=json.loads(run([exe,fixtures/name,route,'default',3]));item.update(case=name,trial=trial);report['records'].append(item);print(name,route,trial,item['ok'],flush=True)
   assert item['ok']==(name!='jxl-icc-gray16.jxl' or route!='ordinary'),item
   if not item['ok']:assert 'Gray ICC' in item['error'],item
for path in generated:
 routes=['inspect'] if path.name in ['layers-64.jxl','layers-65.jxl'] else ['inspect','ordinary'] if path.name.startswith('input-cap-') else ['inspect','ordinary','gray8' if path.name in ['beyond.jxl','thin.jxl','wide.jxl'] else 'rgba8','cms']
 for budget in ['default','unlimited']:
  for route in routes:
   item=json.loads(run([exe,path,route,budget,1]));item.update(case=path.name,trial=0);report['records'].append(item);print(path.name,route,budget,item['ok'],flush=True)
   expected=(path.name in ['thin.jxl','large-icc.jxl','metadata-1.jxl','metadata-8.jxl'] or (budget=='unlimited' and path.name in ['boundary.jxl','layers-64.jxl','input-cap-at.jxl']))
   assert item['ok']==expected,(path.name,route,budget,item)
 # Core-only comparisons explicitly bypass WML2 admission on trusted flat inputs
 # after the unchanged public WML2 route has recorded its refusal.
 if path.name in ['boundary.jxl','beyond.jxl','qhd.jxl','thin.jxl','wide.jxl']:
  item=json.loads(run([exe,path,'core-only','default',3]));item.update(case=path.name,trial=0,budget_scope='trusted core-only finite sample guard; WML2 DecodeLimits not applied');report['records'].append(item)
def require_budget_refusal(item,stage):
 # Do not silently turn an admission regression or an unrelated codec error
 # into a successful measurement report.
 assert item.get('ok') is False,item
 assert item.get('error')==f'Out of memory: JPEG XL {stage} exceeds decode limit',item
for name,budget,stage in [('jxl-sunset-logo.jxl','expanded=268435456','internal layer workspace'),('jxl-rgb8.jxl','metadata=1','source metadata'),('jxl-rgb8.jxl','frames=0','still frames'),('jxl-rgb8.jxl','pixels=34','pixels')]:
 for route in ['inspect','ordinary']:
  item=json.loads(run([exe,fixtures/name,route,budget,1]));item.update(case=name,trial=0)
  require_budget_refusal(item,stage);report['records'].append(item)
# Exact metadata admission allowance around the current 16KiB generated-profile
# reservation; this remains a policy screen, not actual metadata allocation.
for limit in [1064994,1064995]:
 for route in ['inspect','ordinary']:
  item=json.loads(run([exe,out/'metadata-1.jxl',route,f'metadata={limit}',1]));item.update(case='metadata-1.jxl',trial=0);assert item['ok']==(limit==1064995),item;report['records'].append(item)
# Independently prescribed constant source codes, normalized to RGBA16, check
# hashes of the trusted-core decodes. Hash comparison is not a second decoder.
def fnv_repeat(pixel,count):
 h=0xcbf29ce484222325
 for _ in range(count):
  for b in pixel:h=((h^b)*0x100000001b3)&0xffffffffffffffff
 return f'{h:016x}'
report['constant_source_checks']=[]
for name,w,h,c in [('boundary.jxl',2048,1536,3),('beyond.jxl',2049,1536,1),('qhd.jxl',2560,1440,3),('thin.jxl',16384,1,1),('wide.jxl',16385,1,1)]:
 pixel=[64,128,192,255] if c==3 else [133,133,133,255]
 expected=fnv_repeat(bytes(v for code in pixel for v in [code,code]),w*h)
 actual=next(r for r in report['records'] if r['case']==name and r['route']=='core-only')
 assert actual['ok'] and (actual['width'],actual['height'])==(w,h) and actual['fnv64']==expected,(name,actual,expected)
 report['constant_source_checks'].append({'case':name,'normalized_rgba16_fnv64':expected,'passed':True})
for item in report['records']:
 if item['case'].startswith('metadata-') and item['ok'] and item['route']!='inspect':
  expected=next(r for r in report['records'] if r['case']=='jxl-rgb8.jxl' and r['route']==item['route'])
  assert item['fnv64']==expected['fnv64'],item
report['encoder_sha256']=hashlib.sha256(encoder.read_bytes()).hexdigest()
report['encoder_source_sha256']=hashlib.sha256((root/'wml2-test/scripts/jpegxl_fixture_encoder.c').read_bytes()).hexdigest()
report['base_fixture_sha256']={name:hashlib.sha256((fixtures/name).read_bytes()).hexdigest() for name in ['jxl-rgb8.jxl','jxl-rgba16.jxl','jxl-icc-gray16.jxl','jxl-icc-lossless.jxl','jxl-jbrd-bench.jxl','jxl-sunset-logo.jxl','jxl-fixtures.json','jxl-additional-fixtures.json','jxl-sunset-logo.json']}
report['probe_binary_sha256']=hashlib.sha256(exe.read_bytes()).hexdigest()
report['preflight_array_fields']=['oriented_width','oriented_height','coded_bits','native_gray_no_alpha','auxiliary_metadata_accounted_bytes','header_layer_workspace_estimate','codec_sample_limit','codestream_bytes','source_header_bytes','codec_sample_channels']
# Keep the recorded row matrix compact while retaining ordinary JSON.
records=report.pop('records')
doc=json.dumps(report,indent=2);assert doc.endswith('\n}')
rows=[]
for row in records:
 ordered={k:row[k] for k in ['case','trial','route','budget','ok'] if k in row};ordered.update(row);rows.append('    '+json.dumps(ordered))
(out/'report.json').write_text(doc[:-2]+',\n  "records": [\n'+',\n'.join(rows)+'\n  ]\n}\n')
print('saved',out/'report.json')
