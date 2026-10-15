"""Original multiple-block Gray8 fixtures; OpenJPEG is only the test oracle."""
from pathlib import Path
from PIL import Image, features, __version__
import hashlib
out=Path('wml2/tests/fixtures/review')
print('Pillow',__version__,'OpenJPEG',features.version('jpg_2000'))
for w,h in [(17,19),(31,33),(65,49),(257,129),(1,513),(513,1),(1024,1024)]:
 for level in range(2 if min(w,h)>1 else 1):
  for pattern in (['ramp'] if w==1024 else ['noise','mixed']):
   gray=bytes((x*73+y*151+x*y*17+31)%256 if pattern=='noise' else
              (128 if ((x//16+y//16)%3==0) else (x+y)%16+128 if (x//16+y//16)%3==1 else (x*13+y*11)%256) if pattern=='mixed' else
              (x*9+y*7)%256 for y in range(h) for x in range(w))
   source=Image.frombytes('L',(w,h),gray)
   stem=f'jp2_multi_{w}x{h}_d{level}_{pattern}'
   (out/f'{stem}.gray').write_bytes(gray)
   for ext in ['j2k','jp2']:
    path=out/f'{stem}.{ext}'
    source.save(path,format='JPEG2000',no_jp2=ext=='j2k',num_resolutions=level+1,codeblock_size=(16,16),irreversible=False)
    assert Image.open(path).tobytes()==gray
    print(hashlib.sha256(path.read_bytes()).hexdigest(),path.name)
