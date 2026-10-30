#!/usr/bin/env python3
"""Construct TIFF/XMP metadata independently; verify with Pillow, ElementTree
and C libjxl. Optional brob bodies use C libbrotli, not WML2/Rust serialization.
"""
import ctypes as C
import ctypes.util
import hashlib
import json
from pathlib import Path
import struct
import xml.etree.ElementTree as ET
from PIL import Image
root=Path(__file__).resolve().parents[2]; out=root/'wml2/tests/fixtures/review'

def tiff(endian,orientation):
 order='<' if endian=='little' else '>'
 make=b'LittleMaker\0' if endian=='little' else b'BigMaker\0'
 header=(b'II' if endian=='little' else b'MM')+struct.pack(order+'HI',42,8)
 tags=struct.pack(order+'H',2)
 tags+=struct.pack(order+'HHI',0x112,3,1)+struct.pack(order+'H',orientation)+b'\0\0'
 tags+=struct.pack(order+'HHII',0x10f,2,len(make),38)+struct.pack(order+'I',0)
 data=header+tags+make
 exif=Image.Exif();exif.load(data)
 assert exif[0x112]==orientation and exif[0x10f]==make[:-1].decode()
 return data
little=tiff('little',6);big=tiff('big',3)
xmp=('<?xpacket begin="\ufeff" id="W5M0MpCehiHzreSzNTczkc9d"?>'
 '<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">'
 '<rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" dc:title="JPEG XL メタデータ &amp; UTF-8"/>'
 '</rdf:RDF></x:xmpmeta><?xpacket end="w"?>').encode()
xmp2=b'<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description second="yes"/></rdf:RDF>'
assert ET.fromstring(xmp).tag=='{adobe:ns:meta/}xmpmeta'
assert ET.fromstring(xmp2).tag=='{http://www.w3.org/1999/02/22-rdf-syntax-ns#}RDF'
for name,data in [('jxl-meta-little.tiff',little),('jxl-meta-big.tiff',big),('jxl-meta.xmp',xmp),('jxl-meta-second.xmp',xmp2)]: (out/name).write_bytes(data)
brotli=C.CDLL(ctypes.util.find_library('brotlienc'))
brotli.BrotliEncoderCompress.argtypes=[C.c_int,C.c_int,C.c_int,C.c_size_t,C.c_void_p,C.POINTER(C.c_size_t),C.c_void_p]
brotli.BrotliEncoderCompress.restype=C.c_int
brotli.BrotliEncoderVersion.restype=C.c_uint32

def compress(data):
 size=C.c_size_t(len(data)+1024); output=C.create_string_buffer(size.value)
 assert brotli.BrotliEncoderCompress(5,22,0,len(data),C.create_string_buffer(data),C.byref(size),output)
 return output.raw[:size.value]
def box(kind,data):return struct.pack('>I',len(data)+8)+kind+data
raw=(out/'jxl-rgba8.jxl').read_bytes();assert raw[:2]==b'\xff\x0a'
le=b'\0'*4+little;be=struct.pack('>I',5)+b'junk!'+big
cases=[('jxl-meta-little.jxl',[(b'Exif',le),(b'xml ',xmp)],False),
       ('jxl-meta-big-offset.jxl',[(b'Exif',be),(b'xml ',xmp2)],False),
       ('jxl-meta-brob.jxl',[(b'Exif',le),(b'xml ',xmp)],True),
       ('jxl-meta-multiple.jxl',[(b'Exif',le),(b'xml ',xmp),(b'Exif',be),(b'xml ',xmp2)],False),
       ('jxl-meta-brob-invalid.jxl',[(b'Exif',b'\xff'*4),(b'xml ',b'<not-closed'),(b'Exif',le),(b'xml ',xmp)],True)]
# Independent C libjxl pixel + (decompressed) metadata extraction.
jxl=C.CDLL(ctypes.util.find_library('jxl'))
def fn(name,result,args):
 f=getattr(jxl,name);f.restype=result;f.argtypes=args;return f
create=fn('JxlDecoderCreate',C.c_void_p,[C.c_void_p]);destroy=fn('JxlDecoderDestroy',None,[C.c_void_p])
subscribe=fn('JxlDecoderSubscribeEvents',C.c_int,[C.c_void_p,C.c_int]);setinput=fn('JxlDecoderSetInput',C.c_int,[C.c_void_p,C.c_void_p,C.c_size_t]);close=fn('JxlDecoderCloseInput',None,[C.c_void_p]);process=fn('JxlDecoderProcessInput',C.c_int,[C.c_void_p]);decompress=fn('JxlDecoderSetDecompressBoxes',C.c_int,[C.c_void_p,C.c_int])
boxtype=fn('JxlDecoderGetBoxType',C.c_int,[C.c_void_p,C.c_void_p,C.c_int]);boxbuffer=fn('JxlDecoderSetBoxBuffer',C.c_int,[C.c_void_p,C.c_void_p,C.c_size_t]);release=fn('JxlDecoderReleaseBoxBuffer',C.c_size_t,[C.c_void_p])
class Format(C.Structure): _fields_=[('channels',C.c_uint32),('datatype',C.c_int),('endian',C.c_int),('align',C.c_size_t)]
imagebuffer=fn('JxlDecoderSetImageOutBuffer',C.c_int,[C.c_void_p,C.POINTER(Format),C.c_void_p,C.c_size_t]);version=fn('JxlDecoderVersion',C.c_uint32,[])

def verify(data,expected):
 dec=create(None);source=C.create_string_buffer(data);pixels=C.create_string_buffer(140);fmt=Format(4,2,0,0);active=None;buffer=None;actual=[]
 try:
  assert subscribe(dec,0x4000|0x1000)==0 and decompress(dec,1)==0 and setinput(dec,source,len(data))==0;close(dec)
  for _ in range(100):
   status=process(dec)
   if status in (0,0x4000):
    if active is not None:
     actual.append((active,buffer.raw[:len(buffer)-release(dec)]));active=None
    if status==0:break
    kind=C.create_string_buffer(4);assert boxtype(dec,kind,1)==0
    if kind.raw in (b'Exif',b'xml '):
     active=kind.raw;buffer=C.create_string_buffer(8192);assert boxbuffer(dec,buffer,len(buffer))==0
   elif status==5:assert imagebuffer(dec,C.byref(fmt),pixels,len(pixels))==0
   elif status==0x1000:pass
   else:raise RuntimeError(status)
  assert actual==expected
  assert pixels.raw==(out/'jxl-rgba8-djxl.bin').read_bytes()
 finally:destroy(dec)
entries=[]
for name,metadata,compressed in cases:
 data=b'\0\0\0\x0cJXL \r\n\x87\n'+box(b'ftyp',b'jxl \0\0\0\0jxl ')+box(b'jxlc',raw)
 for kind,payload in metadata:data+=box(b'brob',kind+compress(payload)) if compressed else box(kind,payload)
 verify(data,metadata);(out/name).write_bytes(data)
 entries.append({'file':name,'sha256':hashlib.sha256(data).hexdigest(),'metadata':[{'type':k.decode(),'sha256':hashlib.sha256(v).hexdigest()} for k,v in metadata]})
manifest={'oracle':f'Pillow {Image.__version__} TIFF EXIF + Python ElementTree UTF-8/XML namespaces + C libjxl {version()} decompressed metadata and pixels','compression':f'C libbrotli {brotli.BrotliEncoderVersion():#x}, quality5 lgwin22','license':'Deterministic generated test metadata; existing codestream/djxl pixel provenance in jxl-fixtures.json','expected':{'little_orientation':6,'big_orientation':3,'little_make':'LittleMaker','big_make':'BigMaker','big_payload_offset':5,'selection':'first valid EXIF/XMP in file order'},'cases':entries}
(out/'jxl-meta-fixtures.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n');print(json.dumps(manifest,indent=2,ensure_ascii=False))
