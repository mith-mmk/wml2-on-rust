#!/usr/bin/env python3
"""Independent LittleCMS float references for native Gray/RGB layout conversion."""
import csv,ctypes as C,ctypes.util,pathlib
root=pathlib.Path(__file__).resolve().parents[2];fixtures=root/'wml2/tests/fixtures/review'
cms=C.CDLL(ctypes.util.find_library('lcms2'))
cms.cmsOpenProfileFromMem.argtypes=[C.c_void_p,C.c_uint32];cms.cmsOpenProfileFromMem.restype=C.c_void_p
cms.cmsCreateTransform.argtypes=[C.c_void_p,C.c_uint32,C.c_void_p,C.c_uint32,C.c_uint32,C.c_uint32];cms.cmsCreateTransform.restype=C.c_void_p
cms.cmsDoTransform.argtypes=[C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32];cms.cmsDeleteTransform.argtypes=[C.c_void_p];cms.cmsCloseProfile.argtypes=[C.c_void_p]
cms.cmsGetEncodedCMMversion.restype=C.c_uint32
assert cms.cmsGetEncodedCMMversion()==2160,'frozen references require LittleCMS2.16'
rows=[]
for route,source_name,destination_name,ic,oc in [('gray-rgb','avif-icc-gray-linear.icc','avif-icc-linear.icc',1,3),('rgb-gray','avif-icc-linear.icc','avif-icc-gray-linear.icc',3,1)]:
 source=(fixtures/source_name).read_bytes();dest=(fixtures/destination_name).read_bytes();so=C.create_string_buffer(source);do=C.create_string_buffer(dest);sp=cms.cmsOpenProfileFromMem(so,len(source));dp=cms.cmsOpenProfileFromMem(do,len(dest));assert sp and dp
 transform=cms.cmsCreateTransform(sp,(1<<22)|((3 if ic==1 else 4)<<16)|(ic<<3)|4,dp,(1<<22)|((3 if oc==1 else 4)<<16)|(oc<<3)|4,1,0x100);assert transform
 for bits in [8,10,12,16,32]:
  maximum=1 if bits==32 else (1<<bits)-1
  patterns=[[0,0,0],[maximum,0,0],[0,maximum,0],[0,0,maximum],[maximum]*3,[maximum/3,maximum*2/3,maximum/7],[maximum/2]*3,[maximum/255,maximum/127,maximum/63]] if ic==3 else [[v,0,0] for v in [0,maximum/255,maximum/3,maximum/2,maximum/2+maximum/255,maximum-maximum/255,maximum]]
  for v in patterns:
   v=v if bits==32 else [round(x) for x in v];inp=(C.c_float*ic)(*(x/maximum for x in v[:ic]));out=(C.c_float*oc)();cms.cmsDoTransform(transform,inp,out,1)
   expected=[float(x) if bits==32 else round(max(0,min(1,x))*maximum) for x in out];rows.append([route,bits]+v+expected+[0]*(3-oc))
 cms.cmsDeleteTransform(transform);cms.cmsCloseProfile(sp);cms.cmsCloseProfile(dp)
with (fixtures/'lcms-cross-model.csv').open('w',newline='') as f:
 writer=csv.writer(f);writer.writerow(['route','bits','in0','in1','in2','out0','out1','out2']);writer.writerows(rows)
print(f'{len(rows)} LittleCMS2.16 relative-intent float reference rows;NOOPTIMIZE;native integer ranges quantized last')
