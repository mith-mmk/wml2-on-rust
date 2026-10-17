"""Generate independent SampleFormat TIFFs and inspect native LibTIFF scanlines."""
import argparse, ctypes, ctypes.util, hashlib, json, struct
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument("--output-dir", required=True)
work=Path(parser.parse_args().output_dir)
work.mkdir(parents=True, exist_ok=True)
lib=ctypes.CDLL(ctypes.util.find_library("tiff"))
lib.TIFFGetVersion.restype=ctypes.c_char_p
lib.TIFFOpen.argtypes=[ctypes.c_char_p,ctypes.c_char_p]; lib.TIFFOpen.restype=ctypes.c_void_p
lib.TIFFClose.argtypes=[ctypes.c_void_p]
lib.TIFFGetField.argtypes=[ctypes.c_void_p,ctypes.c_uint32]; lib.TIFFGetField.restype=ctypes.c_int
lib.TIFFScanlineSize64.argtypes=[ctypes.c_void_p]; lib.TIFFScanlineSize64.restype=ctypes.c_uint64
lib.TIFFReadScanline.argtypes=[ctypes.c_void_p,ctypes.c_void_p,ctypes.c_uint32,ctypes.c_uint16]; lib.TIFFReadScanline.restype=ctypes.c_int
rows=[]
for name, depth, sf, fmt, values, be in [
 ("signed8",8,2,"b",[-128,-1,0,127],False),
 ("signed16",16,2,"h",[-32768,-1,0,32767],False),
 ("float16",16,3,"e",[-1.0,-0.0,0.0,65504.0],False),
 ("float32",32,3,"f",[-1.0,-0.0,0.0,1.0],False),
 ("unsigned16le",16,1,"H",[32768,65535,0,32767],False),
 ("unsigned16be",16,1,"H",[32768,65535,0,32767],True),
]:
 endian=">" if be else "<"
 short=lambda x: (3,1,struct.pack(endian+"H",x))
 long=lambda x: (4,1,struct.pack(endian+"I",x))
 raw=struct.pack(endian+fmt*len(values),*values)
 tags=[(256,long(4)),(257,long(1)),(258,short(depth)),(259,short(1)),(262,short(1)),(273,long(0)),(277,short(1)),(278,long(1)),(279,long(len(raw))),(284,short(1)),(339,short(sf))]
 offset=8+2+12*len(tags)+4
 tags[5]=(273,long(offset))
 encoded=(b"MM\x00*" if be else b"II*\x00")+struct.pack(endian+"I",8)+struct.pack(endian+"H",len(tags))
 for tag,(kind,count,data) in tags:
  encoded+=struct.pack(endian+"HHI",tag,kind,count)+data.ljust(4,b"\x00")
 encoded+=bytes(4)+raw
 path=work/("ex-review-sampleformat-"+name+".tif"); path.write_bytes(encoded)
 handle=lib.TIFFOpen(str(path).encode(),b"r")
 assert handle
 actual_depth=ctypes.c_uint16(); actual_sf=ctypes.c_uint16()
 assert lib.TIFFGetField(handle,258,ctypes.byref(actual_depth))==1
 assert lib.TIFFGetField(handle,339,ctypes.byref(actual_sf))==1
 assert (actual_depth.value,actual_sf.value)==(depth,sf)
 count=lib.TIFFScanlineSize64(handle)
 output=ctypes.create_string_buffer(count)
 assert lib.TIFFReadScanline(handle,output,0,0)==1
 lib.TIFFClose(handle)
 assert output.raw==struct.pack("="+fmt*len(values),*values), (name,output.raw.hex())
 actual=struct.unpack("="+fmt*len(values),output.raw)
 assert list(actual)==values, (name,actual,values)
 rows.append(dict(file=path.name,sha256=hashlib.sha256(encoded).hexdigest(),depth=depth,sample_format=sf,byte_order="big" if be else "little",expected=values,libtiff_samples=actual,scanline_sha256=hashlib.sha256(output.raw).hexdigest(),native_output_kind="reject" if sf!=1 else "accept"))
result=dict(oracle=lib.TIFFGetVersion().decode().splitlines()[0],rows=rows,passed=len(rows),failed=0,notes="LibTIFF typed scanline interpretation only; no WML2 runtime result is inferred from the oracle.")
(work/"oracle.json").write_text(json.dumps(result,indent=2)+"\n")
print(json.dumps({k:v for k,v in result.items() if k!="rows"},indent=2))
