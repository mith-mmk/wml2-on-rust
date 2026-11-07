#!/usr/bin/env python3
"""Matched callback metadata benchmark using the existing independent C animation."""
import argparse,csv,hashlib,json,os,statistics,struct,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--before',type=Path,required=True);p.add_argument('--after',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
root=Path(__file__).resolve().parents[2];out=a.output.resolve()
if not out.name.startswith('.test'):p.error('output must start .test')
out.mkdir(parents=True,exist_ok=False)
f=root/'wml2/tests/fixtures/review';source=f/'jxl-animation-main5-icc16-variable.jxl';original=source.read_bytes()
def box(kind,data):return (len(data)+8).to_bytes(4,'big')+kind+data
# Insert ahead of the first codestream box so original control/EOF semantics survive.
at=12
while original[at+4:at+8]not in [b'jxlc',b'jxlp']:
 n=int.from_bytes(original[at:at+4],'big');assert n>=8;at+=n
for name,record in [('opaque-1',box(b'test',b'x'*4096)),('opaque-17',box(b'test',b'x'*256)*17),('opaque-256',box(b'test',b'x'*256)*256),('brob-256',box(b'brob',b'test'+(f/'jxl-brob-empty.br').read_bytes())*256)]:
 (out/(name+'.jxl')).write_bytes(original[:at]+record+original[at:])
raw=(f/'jxl-animation-main5-icc16-variable.0.rgba16le').read_bytes();gold=bytes((v*255+32767)//65535 for(v,)in struct.iter_unpack('<H',raw));srgb=(f/'jxl-animation-main5-icc16-variable.0.srgb8').read_bytes()
cases=[('bare',source,'ordinary',300),('bare-managed',source,'managed',100),('opaque-1',out/'opaque-1.jxl','ordinary',100),('opaque-17',out/'opaque-17.jxl','ordinary',100),('opaque-17-managed',out/'opaque-17.jxl','managed',100),('opaque-256',out/'opaque-256.jxl','ordinary',30),('brob-256',out/'brob-256.jxl','ordinary',30)]
bins={'before':a.before.resolve(),'after':a.after.resolve()};cpu=None
if hasattr(os,'sched_getaffinity'):cpu=min(os.sched_getaffinity(0));os.sched_setaffinity(0,{cpu})
rows=[]
for name,file,mode,n in cases:
 row=dict(name=name,input_sha256=hashlib.sha256(file.read_bytes()).hexdigest(),mode=mode,samples=n);times={k:[]for k in bins};hashes=set()
 for which,exe in bins.items():row[which+'_floor']=json.loads(subprocess.check_output([str(exe),str(file),mode,'floor'],text=True,cwd=root))['workspace_bytes']
 row['floor_delta']=row['after_floor']-row['before_floor'];assert row['floor_delta']>=0
 for i in range(3):
  for which in (['before','after']if i%2==0 else['after','before']):
   args=[str(bins[which]),str(file),mode,str(n)]
   if i==0:args.append(str(out/(name+'-'+which+'.raw')))
   text=subprocess.check_output(args,text=True,cwd=root);(out/f'{name}-{which}-{i}.csv').write_text(text);values=list(csv.DictReader(text.splitlines()));times[which].append(statistics.median(int(v['ns'])for v in values));hashes.update(v['hash']for v in values)
 assert len(hashes)==1
 actual=(out/(name+'-after.raw')).read_bytes();assert actual==(out/(name+'-before.raw')).read_bytes();expected=srgb if mode=='managed'else gold;assert len(actual)==len(expected)
 error=max(abs(x-y)for x,y in zip(actual,expected));assert error<=(2 if mode=='managed'else 1)
 row.update(block_medians_ns=times,ratio=statistics.median(times['after'])/statistics.median(times['before']),poster_max_c_error=error);rows.append(row)
report=dict(method=f'Rust1.91 identical release features/probe; CPU{cpu};3 alternating pairs,3 warmups; timing includes full animation decode and frame/metadata destruction, hash only covers poster; all frames qualified separately',binaries_sha256={k:hashlib.sha256(v.read_bytes()).hexdigest()for k,v in bins.items()},cases=rows)
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
