#!/usr/bin/env python3
"""Regenerate JPEG reduced-IDCT goldens. Pillow 12.3.0 encodes new inputs;
libjpeg-turbo 2.1.5 ISLOW, scale_num=1, fancy=FALSE decodes all references.
Build jpeg_scaled_oracle.c with cc -O2 jpeg_scaled_oracle.c -ljpeg -o .test-oracle
and pass that executable as argv[1]. Existing restart fixtures are unchanged.
"""
from pathlib import Path
import hashlib, subprocess, sys, tempfile, zlib
from PIL import Image
root=Path(__file__).resolve().parent
oracle=Path(sys.argv[1]).resolve()
names=[]
for mode,w,h,sub in [('RGB',129,75,0),('RGB',129,75,1),('RGB',129,75,2),('RGB',9,7,0),('L',17,13,0),('L',1,1,0)]:
    im=Image.new(mode,(w,h))
    if mode=='RGB': im.putdata([((x*19+y*7)%256,(x*5+y*29)%256,(x*31+y*13)%256) for y in range(h) for x in range(w)])
    else: im.putdata([(x*37+y*19)%256 for y in range(h) for x in range(w)])
    name=f'jpeg-scaled-{mode}-{w}x{h}-s{sub}'
    im.save(root/(name+'.jpg'),quality=91,subsampling=sub)
    names.append(name)
im=Image.open(root/'jpeg-scaled-RGB-129x75-s0.jpg')
name='jpeg-scaled-direct-rgb';im.save(root/(name+'.jpg'),quality=91,keep_rgb=True);names.append(name)
names += ['restart-'+p+'-i0' for p in ['minimal','bgray','b444','b420','odd420']]
manifest=['libjpeg-turbo 2.1.5; JDCT_ISLOW; do_fancy_upsampling=FALSE; JCS_EXT_RGBA; scale_num=1', 'golden filename | width height | SHA256 uncompressed RGBA']
with tempfile.TemporaryDirectory(prefix='.test-jpeg-scaled-') as temp:
    raw=Path(temp)/'rgba'
    for name in names:
        for scale in [2,4,8]:
            r=subprocess.run([str(oracle),str(root/(name+'.jpg')),str(scale),str(raw)],capture_output=True,check=True)
            data=raw.read_bytes();out=f'{name}-d{scale}.rgba.zlib'
            (root/out).write_bytes(zlib.compress(data,9))
            manifest.append(f'{out} | {r.stderr.decode().strip()} | {hashlib.sha256(data).hexdigest()}')
(root/'jpeg-scaled-oracles.txt').write_text('\n'.join(manifest)+'\n')

Image.open(root.parents[3]/"test/images/bundled/sample.bmp").convert("RGB").save(root/"jpeg-scaled-bench.jpg",quality=90,subsampling=2)
Image.open(root.parents[3]/"test/images/bundled/sample.bmp").convert("L").save(root/"jpeg-scaled-bench-gray.jpg",quality=90)
