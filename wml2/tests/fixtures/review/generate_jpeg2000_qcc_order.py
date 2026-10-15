"""Original fixture permutations: T.800 A.3/A.6.5 tile QCC precedence.

Only the canonical QCD-before-QCC stream is used as the independent oracle:
The Pillow/OpenJPEG build used here also applies reversed tile QCD/QCC in
encounter order. Equivalent
marker permutations must retain the canonical pixels specified by T.800.
"""
from pathlib import Path
from itertools import permutations
import io, struct
from PIL import Image

root=Path(__file__).resolve().parent
raw=(root/'jp2_irreversible_d5_c3_m1_r2.j2k').read_bytes()
p=2
while True:
    marker,length=struct.unpack_from('>HH',raw,p)
    if marker==0xff5c:qcd=raw[p:p+2+length]
    if marker==0xff90:break
    p+=length+2
assert raw[p+12:p+14]==b'\xff\x93'
assert raw[-2:]==b'\xff\xd9'
assert qcd[4]&31==2

def qcc(component,delta):
    q=bytearray(qcd[4:])
    for i in range(1,len(q),2):
        v=struct.unpack_from('>H',q,i)[0]
        struct.pack_into('>H',q,i,(v&0xf800)|((v+delta)&2047))
    return struct.pack('>HHB',0xff5d,len(q)+3,component)+q

markers={'d':qcd,'c0':qcc(0,512),'c2':qcc(2,256)}
def stream(order,multipart=False):
    extra=b''.join(markers[k] for k in order)
    data=bytearray(raw[:p+12]+extra+raw[p+12:])
    struct.pack_into('>I',data,p+6,struct.unpack_from('>I',raw,p+6)[0]+len(extra))
    if multipart:
        data[p+11]=2
        data=data[:-2]+struct.pack('>HHHIBB',0xff90,10,0,14,1,2)+b'\xff\x93\xff\xd9'
    return bytes(data)

jp2=(root/'jp2_irreversible_d5_c3_m1_r2.jp2').read_bytes()
at=jp2.index(b'jp2c')-4
assert struct.unpack_from('>I',jp2,at)[0]==len(jp2)-at
def wrap(data):return jp2[:at]+struct.pack('>I4s',len(data)+8,b'jp2c')+data

rows=[]
reference=root/'jp2_general_qcc_order.rgba'
with Image.open(io.BytesIO(stream(('d','c0','c2')))) as im:
    im.load();width,height=im.size;reference.write_bytes(im.convert('RGBA').tobytes())
for order in permutations(markers):
    for multipart in [False,True]:
        for ext in ['j2k','jp2']:
            data=stream(order,multipart)
            name='jp2_general_qcc_order_'+'_'.join(order)+('_parts' if multipart else '')+'.'+ext
            (root/name).write_bytes(wrap(data) if ext=='jp2' else data)
            rows.append(f'{name} {reference.name} {width} {height} 1\n')
(root/'jpeg2000-qcc-order-manifest.txt').write_text(''.join(rows))
print(f'{len(rows)} equivalent tile-header permutations, canonical independent RGBA oracle')
