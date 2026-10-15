"""Generate original deterministic pixels; encode/reference via Pillow/OpenJPEG."""
from pathlib import Path
import hashlib,json,struct
from PIL import Image,features,__version__
root=Path(__file__).resolve().parent
rows=[]
def emit(name,mode,size,options):
    w,h=size;channels=len(mode)
    pixels=bytes(((x*17+y*29+c*83+(x*y)%127)&255) for y in range(h) for x in range(w) for c in range(channels))
    im=Image.frombytes(mode,size,pixels);path=root/('jp2_general_'+name+'.jp2')
    im.save(path,format='JPEG2000',**options)
    with Image.open(path) as decoded:
        decoded.load();reference=decoded.convert('RGBA').tobytes()
    if not options.get('irreversible',False):assert reference==im.convert('RGBA').tobytes()
    raw=path.with_suffix('.rgba');raw.write_bytes(reference)
    rows.append(dict(input=path.name,reference=raw.name,width=w,height=h,max_error=1 if options.get('irreversible',False) else 0,sha256=hashlib.sha256(path.read_bytes()).hexdigest(),options=options))
for mode in ['L','RGB']:
    for irreversible in [False,True]:
        for order in ['LRCP','RLCP','RPCL','PCRL','CPRL']:
            emit(f'{mode}_{order}_{int(irreversible)}',mode,(32,32),dict(progression=order,num_resolutions=4,codeblock_size=(4,4),precinct_size=(16,16),quality_mode='rates',quality_layers=[12,6,3] if irreversible else [8,4,1],irreversible=irreversible,mct=1 if mode=='RGB' else 0))
emit('levels6','L',(64,64),dict(num_resolutions=7,quality_layers=[4,1],codeblock_size=(4,4)))
for mode in ['LA','RGBA']:
    emit(mode,mode,(16,16),dict(num_resolutions=3,quality_layers=[1],codeblock_size=(4,4)))
manifest=dict(pillow=__version__,openjpeg=features.version('jpg_2000'),provenance='Original deterministic pixels; Pillow/OpenJPEG encoder and independent decoder',files=rows)
(root/'jpeg2000-general-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(len(rows))

(root/'jpeg2000-general-manifest.txt').write_text(''.join(f"{r['input']} {r['reference']} {r['width']} {r['height']} {r['max_error']}\n" for r in rows))

# Original 1x1 HT examples, assembled from T.814's normative code words.
# Their decoded values (128 and 129) are independently checked by OpenJPEG.
def segment(marker, payload):
    return struct.pack('>HH', marker, len(payload)+2)+payload
siz = struct.pack('>H8IH3B', 0x4000,1,1,0,0,1,1,0,0,1,7,1,1)
header = b'\xff\x4f'+segment(0xff51,siz)
header += segment(0xff50,struct.pack('>IH',0x00020000,0))
header += segment(0xff52,bytes.fromhex('00000001000000004001'))
header += segment(0xff5c,bytes.fromhex('4040'))
for name, packet, value in [('zero','c0228200',128),('one','c023006200',129)]:
    body=bytes.fromhex(packet)
    data=header+segment(0xff90,struct.pack('>HIBB',0,14+len(body),0,1))+b'\xff\x93'+body+b'\xff\xd9'
    path=root/f'jp2_general_ht_{name}.j2k'
    with Image.open(__import__('io').BytesIO(data)) as decoded:
        assert decoded.convert('RGBA').tobytes()==bytes([value,value,value,255])
    path.write_bytes(data)
    if name == 'one':
        # Same level-zero code block at the unsigned 32-bit grid boundary.
        # Pixel/code words are unchanged; only the SIZ origin and extent move.
        edge=bytearray(data)
        for offset,value in [(8,0xffffffff),(12,0xffffffff),(16,0xfffffffe),
                             (20,0xfffffffe),(32,0xfffffffe),(36,0xfffffffe)]:
            struct.pack_into('>I',edge,offset,value)
        (root/'jp2_general_ht_u32_boundary.j2k').write_bytes(edge)

# Previously rejected, now supported existing fixtures retain independent
# references rather than comparing the decoder with its own reconstruction.
for line in (root/'jpeg2000-general-reactivated-manifest.txt').read_text().splitlines():
    source,reference,width,height,tolerance=line.split()
    with Image.open(root/source) as decoded:
        decoded.load(); assert decoded.size==(int(width),int(height))
        (root/reference).write_bytes(decoded.convert('RGBA').tobytes())
