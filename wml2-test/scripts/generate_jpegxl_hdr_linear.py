#!/usr/bin/env python3
"""Independent70-digit Decimal reference for BT.2100 transfer-only conversion.
Inputs are committed independent libjxl C decoded F32 encoded coordinates.
No WML2 output is used to construct the oracle. HLG uses published rounded a,b,c.
"""
import decimal,hashlib,json,pathlib,struct,zlib
root=pathlib.Path(__file__).resolve().parents[2]
f=root/'wml2/tests/fixtures/review'
D=decimal.Decimal
records=[]
with decimal.localcontext() as ctx:
 ctx.prec=70
 for transfer in ['pq','hlg']:
  for kind in ['rgb','rgba','oriented']:
   name=f'jxl-encoded-hdr-{transfer}-{kind}'
   raw=zlib.decompress((f/(name+'.f32le.zlib')).read_bytes())
   values=struct.unpack('<'+'f'*(len(raw)//4),raw);out=[]
   for i,v in enumerate(values):
    e=D.from_float(v)
    if i%4==3:y=e
    elif transfer=='pq':
     p=e**(D(32)/2523)
     y=D(10000)*(max(p-D(3424)/4096,D(0))/(D(2413)/128-D(2392)/128*p))**(D(16384)/2610)
    elif e<=D('.5'):y=e*e/3
    else:y=(((e-D('.55991073'))/D('.17883277')).exp()+D('.28466892'))/12
    out.append(float(y))
   result=struct.pack('<'+'f'*len(out),*out)
   target=name.replace('encoded-hdr','linear-hdr')+'.f32le.zlib'
   (f/target).write_bytes(zlib.compress(result,9))
   records.append(dict(input=name+'.f32le.zlib',input_sha256=hashlib.sha256(raw).hexdigest(),output=target,output_sha256=hashlib.sha256(result).hexdigest()))
report=dict(reference='BT.2100 / Khronos Data Format1.4 sections13.5.2 and13.6.1',url='https://registry.khronos.org/DataFormat/specs/1.4/dataformat.1.4.inline.html',decimal_precision=70,rounding='Decimal exact F32 input; Decimal transcendental; binary32 final round',hlg_constants=['0.17883277','0.28466892','0.55991073'],generator_sha256=hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),records=records)
(root/'docs/jpeg-xl-hdr-linear-oracle.json').write_text(json.dumps(report,indent=2)+'\n')
