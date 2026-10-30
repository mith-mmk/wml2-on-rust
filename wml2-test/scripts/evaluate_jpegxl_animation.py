#!/usr/bin/env python3
"""Trusted-fixture animation feasibility, not production JPEG XL admission.
Requires matching C libjxl0.11.2 headers/runtime and LittleCMS2.16.
"""
import argparse, ctypes as C, ctypes.util, hashlib, json, math, pathlib, struct, subprocess
from fractions import Fraction
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--include',type=pathlib.Path,required=True);p.add_argument('--target-dir',type=pathlib.Path);p.add_argument('--write-fixtures',action='store_true');a=p.parse_args()
if not a.output.name.startswith('.test'):p.error('--output must start with .test')
root=pathlib.Path(__file__).resolve().parents[2];out=a.output.resolve();out.mkdir(parents=True,exist_ok=True);dest=root/'wml2/tests/fixtures/review';target=(a.target_dir or out/'.test-target').resolve()
if not target.name.startswith('.test'):p.error('--target-dir must start with .test')
def sha(data):return hashlib.sha256(data).hexdigest()
def run(cmd,label,check=True):
 r=subprocess.run([str(v) for v in cmd],cwd=root,capture_output=True,text=True);(out/(label+'.log')).write_text(r.stdout+r.stderr)
 if check:r.check_returncode()
 return r
run(['cargo','build','-p','wml2','--locked','--release','--no-default-features','--features','jpeg-xl,image-buffer-ex,color-management','--target-dir',target],'build')
def artifact(pattern):
 found=list((target/'release/deps').glob(pattern));assert len(found)==1,(pattern,found);return found[0]
for source,name,external in [('jpegxl_animation_probe.rs','probe',[('jxl_codec',artifact('libjxl-*.rlib')),('icc_profile',artifact('libicc_profile-*.rlib'))]),('jpegxl_animation_contract.rs','contract',[('wml2',target/'release/libwml2.rlib')])]:
 cmd=['rustc','--edition','2024','-C','opt-level=3',root/'wml2-test/scripts'/source,'-L',f'dependency={target/"release/deps"}','-o',out/name]
 for name_,path in external:cmd+=['--extern',f'{name_}={path}']
 run(cmd,'build-'+name)
for source,name in [('jpegxl_animation_encoder.c','encoder'),('jpegxl_animation_reference.c','reference')]:run(['cc','-Wall','-Wextra','-Werror','-O2','-I'+str(a.include),root/'wml2-test/scripts'/source,'-Wl,-l:libjxl.so.0.11','-o',out/name],'build-'+name)
assert run([out/'reference','--version'],'C-version').stdout.strip()=='runtime=11002 header=11002'
cases=[('variable','replace',8,1,100,1,7,'-'),('fractional','replace',8,1,30000,1001,0,'-'),('reference','reference',8,6,100,1,1,'-'),('internal','internal',8,8,1000,1,0,'-'),('timecodes','timecodes',8,1,100,1,1,'-'),('long','long',8,1,1,1024,4294967295,'-'),('icc16','reference',16,6,1000,1,3,dest/'jxl-gamma2.icc'),('icc16-variable','replace',16,8,1000,1,3,dest/'jxl-gamma2.icc')]
if a.write_fixtures:
 for name,mode,bits_,ori,num,den,loops,profile in cases:run([out/'encoder',dest/f'jxl-animation-{name}.jxl',mode,bits_,ori,num,den,loops,profile],'encode-'+name)
 # Preview frames inherit the global animation syntax too; a static preview
 # payload without a duration field is NOT valid in this animation header.
 run([out/'encoder',out/'preview-one.jxl','preview-one',8,1,100,1,7,'-'],'encode-preview')
 bits=lambda data:[(v>>j)&1 for v in data for j in range(8)]
 little=lambda v,n:[(v>>j)&1 for j in range(n)]
 def pack(v):
  v=v+[0]*((-len(v))%8);return bytes(sum(b<<j for j,b in enumerate(v[i:i+8])) for i in range(0,len(v),8))
 main=(dest/'jxl-animation-variable.jxl').read_bytes();preview=(out/'preview-one.jxl').read_bytes();raw=bits(main);assert raw[31:39]==[0,1,0,0,0,0,0,1]
 inspected=run([out/'probe',out/'preview-one.jxl',out/'preview-one'],'inspect-preview').stdout;assert int(inspected.splitlines()[0].split()[1])==73
 size=[0]+little(0,2)+little(2,6)+[0,0,0]+little(0,2)+little(1,6);header=pack(raw[:37]+[1]+size+raw[38:62]);(dest/'jxl-animation-preview.jxl').write_bytes(header+preview[10:]+main[8:])
 # Known-invalid splice: static frame syntax omits animation duration bits.
 (out/'malformed-preview.jxl').write_bytes(header+(dest/'jxl-orientation1.jxl').read_bytes()[7:]+main[8:])
cases.append(('preview','preview',8,1,100,1,7,'-'))
manifest_path=dest/'jxl-animation-fixtures.json';old=json.loads(manifest_path.read_text()) if manifest_path.exists() else None
cms=C.CDLL(ctypes.util.find_library('lcms2'));cms.cmsOpenProfileFromMem.argtypes=[C.c_void_p,C.c_uint32];cms.cmsOpenProfileFromMem.restype=C.c_void_p;cms.cmsCreateTransform.argtypes=[C.c_void_p,C.c_uint32,C.c_void_p,C.c_uint32,C.c_uint32,C.c_uint32];cms.cmsCreateTransform.restype=C.c_void_p;cms.cmsDoTransform.argtypes=[C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32];cms.cmsDeleteTransform.argtypes=[C.c_void_p];cms.cmsCloseProfile.argtypes=[C.c_void_p];cms.cmsGetEncodedCMMversion.restype=C.c_uint32;assert cms.cmsGetEncodedCMMversion()==2160

def managed(raw,profile):
 words=struct.unpack('<'+'H'*(len(raw)//2),raw);source=C.create_string_buffer(profile);dest_bytes=(root/'wml2/src/srgb.icc').read_bytes();destination=C.create_string_buffer(dest_bytes);sp=cms.cmsOpenProfileFromMem(source,len(profile));dp=cms.cmsOpenProfileFromMem(destination,len(dest_bytes));assert sp and dp;tr=cms.cmsCreateTransform(sp,(4<<16)|(3<<3)|2,dp,(4<<16)|(3<<3)|2,1,0x100);assert tr
 rgb=[v for i,v in enumerate(words) if i%4!=3];inp=(C.c_uint16*len(rgb))(*rgb);result=(C.c_uint16*len(rgb))();cms.cmsDoTransform(tr,inp,result,len(words)//4);rgba=bytes((result[i*3+c]*255+32767)//65535 if c<3 else (words[i*4+3]*255+32767)//65535 for i in range(len(words)//4) for c in range(4));cms.cmsDeleteTransform(tr);cms.cmsCloseProfile(sp);cms.cmsCloseProfile(dp);return rgba

def parsed(text):
 image=[int(v) for v in next(l for l in text.splitlines() if l.startswith('image ')).split()[1:]];frames=[[int(v) for v in l.split()[1:]] for l in text.splitlines() if l.startswith('frame ')];return image,frames
report={'scope':'Trusted-frozen-fixture core and callback-transport evaluation; shipping animation remains unsupported, no production admission or performance/RSS claim','base_sha':'6a48db924b338c5ae7095bea6105fb4dbd436e95','checkout_sha':run(['git','rev-parse','HEAD'],'checkout').stdout.strip(),'rustc':run(['rustc','-vV'],'rustc').stdout,'C_libjxl':'0.11.2','LittleCMS':'2.16','cases':[],'invalid_timebase_checks':'The callback model rejects zero numerator/denominator and arithmetic overflow; these are utility arguments, not claimed validly encoded zero-timebase JXL files','decision':'HOLD production animation integration pending explicit entrypoint/timing/frame-publication and cumulative decoder+collector+metadata budget contract'}
manifest={'oracle':'C libjxl0.11.2 coalesced RGBA16 plus noncoalesced coded-layer metadata; LittleCMS2.16 RGB16 relative/NOOPTIMIZE with alpha copied and final8-bit rounding','construction':'C-generated animations; preview compound bit-splices a C-generated animation-syntax preview frame into a fingerprinted C global header. C validates both preview and all main frames. WML2/Rust never generates expected pixels.','cases':[]}
for name,mode,bits_,ori,num,den,loops,profile in cases:
 source=dest/f'jxl-animation-{name}.jxl';data=source.read_bytes()
 if not a.write_fixtures:
  assert old and next(c['sha256'] for c in old['cases'] if c['name']==name)==sha(data),'input changed'
 command=[out/'reference',source,out/('c-'+name)]
 if name=='preview':command.append(out/'c-preview.preview.rgba16le')
 c=run(command,'C-'+name);image,frames=parsed(c.stdout);assert image[4:7]==[num,den,loops];assert len(frames)==3
 layered=run([out/'reference',source,out/('layers-'+name),'--layers'],'C-layers-'+name);_,layers=parsed(layered.stdout)
 r=run([out/'probe',source,out/('rust-'+name),root/'wml2/src/srgb.icc'],'rust-'+name);coded=[l.split() for l in r.stdout.splitlines() if l.startswith('coded ')];main_coded=[l for l in coded if l[3]=='false'];visible=[l.split() for l in r.stdout.splitlines() if l.startswith('visible ')];assert len(visible)==len(frames)==3;assert len(main_coded)==len(layers)
 for raw_layer,layer in zip(main_coded,layers):
  assert [int(raw_layer[5]),int(raw_layer[6]=='true'),int(raw_layer[9]),int(raw_layer[10]),int(raw_layer[7]),int(raw_layer[8]),int(raw_layer[13]),int(raw_layer[14]),int(raw_layer[11])]==[layer[1],layer[3],layer[4],layer[5],layer[6],layer[7],layer[8],layer[9],layer[10]]
 comparisons=[]
 for i,frame in enumerate(frames):
  assert int(visible[i][3])==frame[1] and (visible[i][4]=='true')==bool(frame[3]);expected_ms=Fraction(frame[1]*1000*den,num);assert math.isclose(float(visible[i][2]),float(expected_ms),rel_tol=0,abs_tol=1e-9)
  native=(out/f'c-{name}.{i}.rgba16le').read_bytes();actual=(out/f'rust-{name}.{i}.rgba16le').read_bytes();assert len(native)==len(actual)==image[0]*image[1]*8;oracle=struct.unpack('<'+'H'*(len(native)//2),native);values=struct.unpack('<'+'H'*(len(actual)//2),actual);difference=max(abs(v-w) for v,w in zip(values,oracle));assert difference<=1
  srgb=managed(native,(out/f'c-{name}.icc').read_bytes());actual_srgb=(out/f'rust-{name}.{i}.srgb.rgba').read_bytes();rgb=max(abs(v-w) for j,(v,w) in enumerate(zip(srgb,actual_srgb)) if j%4!=3);alpha=all(v==w for j,(v,w) in enumerate(zip(srgb,actual_srgb)) if j%4==3);assert rgb<=2 and alpha
  for suffix,content in [('rgba16le',native),('srgb.rgba',srgb)]:
   fixture=dest/f'jxl-animation-{name}.{i}.{suffix}'
   if a.write_fixtures:fixture.write_bytes(content)
   else:assert fixture.read_bytes()==content,fixture
  comparisons.append(dict(index=i,ticks=frame[1],timecode=frame[2],last=bool(frame[3]),milliseconds_exact=str(expected_ms),native_max_difference=difference,managed_RGB_max_difference=rgb,managed_alpha_exact=alpha,rgba16_sha256=sha(native),srgb_sha256=sha(srgb)))
 if name=='preview':
  run([out/'reference',source,out/'preview-skip'],'C-preview-skip')
  for i in range(3):assert (out/f'c-preview.{i}.rgba16le').read_bytes()==(out/f'preview-skip.{i}.rgba16le').read_bytes()==(out/f'c-variable.{i}.rgba16le').read_bytes()
 reasons=[]
 if any(Fraction(f[1]*1000*den,num).denominator!=1 for f in frames):reasons.append('fractional milliseconds require explicit policy')
 if any(f[1]==0 for f in frames):reasons.append('visible final zero-duration frame needs explicit presentation contract')
 if image[7]:reasons.append('SMPTE timecodes not exposed by pinned public frame APIs')
 if image[8]:reasons.append('preview+animation allocation/publication contract not qualified')
 transport=None
 if not reasons:
  waits=','.join(str(f[1]*1000*den//num) for f in frames);transport=run([out/'contract',image[0],image[1],loops,waits,out/('c-'+name),source],'transport-'+name).stdout.strip()
 entry=dict(name=name,file=source.name,sha256=sha(data),source_orientation=ori,coded_canvas=[7,5],output_canvas=image[:2],color_bits=bits_,alpha_bits=bits_,tps_numerator=num,tps_denominator=den,loop_count=loops,have_timecodes=bool(image[7]),have_preview=bool(image[8]),C_noncoalesced_layers=layers,frames=comparisons)
 oracle_entry=dict(entry);oracle_entry['frames']=[{k:v for k,v in f.items() if k not in ['native_max_difference','managed_RGB_max_difference','managed_alpha_exact']} for f in comparisons];manifest['cases'].append(oracle_entry);report['cases'].append(dict(**entry,core_coded_headers=len(coded),core_main_headers=len(main_coded),strict_legacy_transport_reasons=reasons,strict_legacy_transport=transport))
# Truncated queried frame envelopes must not publish experiment output. The
# shipping APIs reject animation independently; this does not certify a future adapter.
negative=[]
for name in ['variable','reference','internal','icc16','preview']:
 data=(dest/f'jxl-animation-{name}.jxl').read_bytes();path=out/f'truncated-{name}.jxl';path.write_bytes(data[:-1]);c=run([out/'reference',path,out/f'bad-C-{name}'],'negative-C-'+name,False);r=run([out/'probe',path,out/f'bad-rust-{name}'],'negative-rust-'+name,False);assert c.returncode!=0 and r.returncode!=0;assert not list(out.glob(f'bad-rust-{name}.*.rgba16le'));negative.append(dict(name=name,C_rejected=True,core_header_walk_rejected=True,core_pixel_output_files=0))
report['truncated']=negative
# Direct access to timecode is intentionally unavailable, not a guessed lossless API.
proof=out/'timecode-access.rs';proof.write_text('fn read(h:&jxl_codec::headers::frame_header::FrameHeader)->u32{h.timecode}\nfn main(){}\n');failure=run(['rustc','--edition','2024',proof,'--extern',f'jxl_codec={artifact("libjxl-*.rlib")}','-L',f'dependency={target/"release/deps"}','-o',out/'timecode-access'],'private-timecode',False);assert failure.returncode!=0 and 'E0616' in failure.stderr
report['private_timecode_compile_rejection']='E0616';report['native_frame_comparisons']=sum(len(c['frames']) for c in report['cases']);report['managed_frame_comparisons']=report['native_frame_comparisons'];report['strict_legacy_transport_cases']=sum(c['strict_legacy_transport'] is not None for c in report['cases']);report['callback_model']='Full coalesced canvas Override/None; first canvas retained plus N frame buffers = (N+1)*width*height*4. Abort at each of first3 next calls stops later draws and terminates once. This is a proposed transport experiment, not production animation/Abort/cancellation support.'
if a.write_fixtures:
 manifest_path.write_text(json.dumps(manifest,indent=2)+'\n')
 rows=[]
 for c in manifest['cases']:
  n=c['name'];oracles=', '.join('include_bytes!("../../wml2/tests/fixtures/review/jxl-animation-'+n+'.'+str(i)+'.rgba16le").as_slice()' for i in range(3));ticks=','.join(str(f['ticks']) for f in c['frames']);rows.append('    (include_bytes!("../../wml2/tests/fixtures/review/jxl-animation-'+n+'.jxl").as_slice(), ['+oracles+'], ['+ticks+'], '+str(c['tps_numerator'])+', '+str(c['tps_denominator'])+', '+str(c['loop_count'])+', ('+','.join(map(str,c['output_canvas']))+')),')
 (root/'wml2-test/scripts/jpegxl_animation_evaluation_cases.rs').write_text('// Generated frozen independent-C fixture matrix.\n[\n'+'\n'.join(rows)+'\n]\n')
report['fixture_manifest_sha256']=sha(manifest_path.read_bytes());report['root_lock_sha256']=sha((root/'Cargo.lock').read_bytes());(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ['native_frame_comparisons','managed_frame_comparisons','strict_legacy_transport_cases','private_timecode_compile_rejection','decision']},indent=2))
