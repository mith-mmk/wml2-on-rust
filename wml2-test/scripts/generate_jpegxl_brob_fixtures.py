#!/usr/bin/env python3
"""Generate brob fixtures with system C libbrotli and verify with C libjxl.

Development tools only. This deliberately does not invoke WML2/Rust Brotli.
"""
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
from pathlib import Path
import struct

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',type=Path,default=Path(__file__).resolve().parents[2]/'wml2/tests/fixtures/review')
a=p.parse_args(); root=Path(__file__).resolve().parents[2]; out=a.output
if not out.is_dir(): p.error('--output must be an existing fixture directory')
enc=C.CDLL(ctypes.util.find_library('brotlienc'))
enc.BrotliEncoderVersion.restype=C.c_uint32
enc.BrotliEncoderMaxCompressedSize.argtypes=[C.c_size_t]; enc.BrotliEncoderMaxCompressedSize.restype=C.c_size_t
enc.BrotliEncoderCompress.argtypes=[C.c_int,C.c_int,C.c_int,C.c_size_t,C.c_void_p,C.POINTER(C.c_size_t),C.c_void_p]
enc.BrotliEncoderCompress.restype=C.c_int
jxl=C.CDLL(ctypes.util.find_library('jxl'))
def function(name,result,args):
 f=getattr(jxl,name); f.restype=result; f.argtypes=args; return f
version=function('JxlDecoderVersion',C.c_uint32,[])
create=function('JxlDecoderCreate',C.c_void_p,[C.c_void_p]); destroy=function('JxlDecoderDestroy',None,[C.c_void_p])
subscribe=function('JxlDecoderSubscribeEvents',C.c_int,[C.c_void_p,C.c_int])
setinput=function('JxlDecoderSetInput',C.c_int,[C.c_void_p,C.c_void_p,C.c_size_t]); close=function('JxlDecoderCloseInput',None,[C.c_void_p])
process=function('JxlDecoderProcessInput',C.c_int,[C.c_void_p])
decompress=function('JxlDecoderSetDecompressBoxes',C.c_int,[C.c_void_p,C.c_int])
boxtype=function('JxlDecoderGetBoxType',C.c_int,[C.c_void_p,C.c_void_p,C.c_int])
boxbuffer=function('JxlDecoderSetBoxBuffer',C.c_int,[C.c_void_p,C.c_void_p,C.c_size_t]); release=function('JxlDecoderReleaseBoxBuffer',C.c_size_t,[C.c_void_p])
class Format(C.Structure):
 _fields_=[('channels',C.c_uint32),('datatype',C.c_int),('endian',C.c_int),('align',C.c_size_t)]
imagesize=function('JxlDecoderImageOutBufferSize',C.c_int,[C.c_void_p,C.POINTER(Format),C.POINTER(C.c_size_t)])
imagebuffer=function('JxlDecoderSetImageOutBuffer',C.c_int,[C.c_void_p,C.POINTER(Format),C.c_void_p,C.c_size_t])
def compress(data):
 capacity=C.c_size_t(enc.BrotliEncoderMaxCompressedSize(len(data)))
 result=C.create_string_buffer(max(capacity.value,1)); source=C.create_string_buffer(data)
 assert enc.BrotliEncoderCompress(5,22,0,len(data),source,C.byref(capacity),result)==1
 return result.raw[:capacity.value]
def box(kind,payload): return struct.pack('>I',len(payload)+8)+kind+payload
xml=b'<x:xmpmeta xmlns:x="adobe:ns:meta/"><metadata>JPEG XL bounded metadata</metadata></x:xmpmeta>'
exif=b'\0'*4+b'II\x2a\0\x08\0\0\0'+struct.pack('<H',1)+struct.pack('<HHII',0x112,3,1,6)+b'\0'*4
seed=0x12345678; random=bytearray()
for i in range(16384+31):
 seed=(1664525*seed+1013904223)&0xffffffff; random.append(seed>>24)
payloads={'empty':b'', 'xml':xml, 'exif':exif, 'repeat':b'A'*(65536+123),'random':bytes(random)}
entries=[]; streams={}
for name,data in payloads.items():
 compressed=compress(data); filename=f'jxl-brob-{name}.br'; (out/filename).write_bytes(compressed); streams[name]=compressed
 entries.append({'name':name,'file':filename,'compressed_bytes':len(compressed),'expanded_bytes':len(data),'compressed_sha256':hashlib.sha256(compressed).hexdigest(),'expanded_sha256':hashlib.sha256(data).hexdigest()})
# Generate an explicitly nonstandard large-window stream, independently
# rejected by the C strict RFC 7932 decoder. It must not be admitted as brob.
enc.BrotliEncoderCreateInstance.argtypes=[C.c_void_p,C.c_void_p,C.c_void_p]
enc.BrotliEncoderCreateInstance.restype=C.c_void_p
enc.BrotliEncoderDestroyInstance.argtypes=[C.c_void_p]
enc.BrotliEncoderSetParameter.argtypes=[C.c_void_p,C.c_int,C.c_uint32]
enc.BrotliEncoderSetParameter.restype=C.c_int
enc.BrotliEncoderCompressStream.argtypes=[C.c_void_p,C.c_int,C.POINTER(C.c_size_t),C.POINTER(C.c_void_p),C.POINTER(C.c_size_t),C.POINTER(C.c_void_p),C.POINTER(C.c_size_t)]
enc.BrotliEncoderCompressStream.restype=C.c_int
enc.BrotliEncoderIsFinished.argtypes=[C.c_void_p]; enc.BrotliEncoderIsFinished.restype=C.c_int
state=enc.BrotliEncoderCreateInstance(None,None,None)
try:
 assert state and enc.BrotliEncoderSetParameter(state,6,1) and enc.BrotliEncoderSetParameter(state,2,25)
 source=C.create_string_buffer(xml); buffer=C.create_string_buffer(4096)
 available_in=C.c_size_t(len(xml)); available_out=C.c_size_t(len(buffer)); total=C.c_size_t()
 next_in=C.cast(source,C.c_void_p); next_out=C.cast(buffer,C.c_void_p)
 assert enc.BrotliEncoderCompressStream(state,2,C.byref(available_in),C.byref(next_in),C.byref(available_out),C.byref(next_out),C.byref(total))
 assert enc.BrotliEncoderIsFinished(state)
 large=buffer.raw[:total.value]
finally: enc.BrotliEncoderDestroyInstance(state)
dec=C.CDLL(ctypes.util.find_library('brotlidec'))
dec.BrotliDecoderDecompress.argtypes=[C.c_size_t,C.c_void_p,C.POINTER(C.c_size_t),C.c_void_p]
dec.BrotliDecoderDecompress.restype=C.c_int
strict_output=C.create_string_buffer(4096); strict_size=C.c_size_t(len(strict_output))
assert dec.BrotliDecoderDecompress(len(large),C.create_string_buffer(large),C.byref(strict_size),strict_output)==0
(out/'jxl-brob-large-window-invalid.br').write_bytes(large)
raw=(root/'wml2/tests/fixtures/review/jxl-rgba8.jxl').read_bytes()
assert raw.startswith(b'\xff\x0a')
ftyp=box(b'ftyp',b'jxl \0\0\0\0jxl ')
metadata=[(b'Exif',payloads['exif'],'exif'),(b'xml ',payloads['xml'],'xml'),(b'test',b'','empty'),(b'xml ',payloads['xml'],'xml')]
compressed=b'\0\0\0\x0cJXL \r\n\x87\n'+ftyp+box(b'brob',b'Exif'+streams['exif'])+box(b'jxlc',raw)+b''.join(box(b'brob',kind+streams[name]) for kind,_,name in metadata[1:])
plain=b'\0\0\0\x0cJXL \r\n\x87\n'+ftyp+box(b'Exif',payloads['exif'])+box(b'jxlc',raw)+b''.join(box(kind,data) for kind,data,_ in metadata[1:])

def verify(data):
 dec=create(None); source=C.create_string_buffer(data); active=None; buf=None; extracted=[]; pixels=None
 fmt=Format(4,2,0,0) # UINT8, native endian, packed
 try:
  assert subscribe(dec,0x4000|0x1000)==0 and decompress(dec,1)==0 and setinput(dec,source,len(data))==0
  close(dec)
  for _ in range(100):
   status=process(dec)
   if status in (0,0x4000):
    if active is not None:
     count=len(buf)-release(dec); extracted.append((active,buf.raw[:count])); active=None
    if status==0: break
    kind=C.create_string_buffer(4); assert boxtype(dec,kind,1)==0
    if kind.raw in (b'Exif',b'xml ',b'test'):
     active=kind.raw; buf=C.create_string_buffer(131072); assert boxbuffer(dec,buf,len(buf))==0
   elif status==5:
    size=C.c_size_t(); assert imagesize(dec,C.byref(fmt),C.byref(size))==0
    assert size.value==7*5*4
    pixels=C.create_string_buffer(size.value); assert imagebuffer(dec,C.byref(fmt),pixels,size.value)==0
   elif status==0x1000: pass
   else: raise RuntimeError(f'libjxl event {status}')
  else: raise RuntimeError('libjxl did not finish')
  assert extracted==[(k,v) for k,v,_ in metadata],[(k,len(v)) for k,v in extracted]
  expected=(root/'wml2/tests/fixtures/review/jxl-rgba8-djxl.bin').read_bytes()
  assert pixels is not None and pixels.raw==expected
  return pixels.raw
 finally: destroy(dec)
assert verify(compressed)==verify(plain)
(out/'jxl-brob-metadata.jxl').write_bytes(compressed)
(out/'jxl-brob-metadata-plain.jxl').write_bytes(plain)
manifest={'encoder':f'C libbrotli {enc.BrotliEncoderVersion():#x}, quality5, lgwin22, generic', 'oracle':f'C libjxl {version()}, decompress-boxes + RGBA8; both container variants verified against existing independent djxl pixels','license':'Generated deterministic test data; existing RGBA8 fixture provenance in jxl-fixtures.json','source_jxl_sha256':hashlib.sha256(raw).hexdigest(),'streams':entries,'invalid_streams':{'jxl-brob-large-window-invalid.br':{'sha256':hashlib.sha256(large).hexdigest(),'reason':'C encoder large-window extension lgwin25; C strict decoder rejects'}},'containers':{name:hashlib.sha256(data).hexdigest() for name,data in [('jxl-brob-metadata.jxl',compressed),('jxl-brob-metadata-plain.jxl',plain)]}}
(out/'jxl-brob-fixtures.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(manifest,indent=2))
