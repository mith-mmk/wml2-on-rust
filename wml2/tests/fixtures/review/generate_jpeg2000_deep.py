"""Original multilevel reversible Gray fixtures; OpenJPEG is test-only."""
from pathlib import Path
from PIL import Image, features, __version__
import hashlib
out=Path('wml2/tests/fixtures/review')
print('Pillow',__version__,'OpenJPEG',features.version('jpg_2000'))
shapes=[(4,4,2),(5,7,2),(8,8,3),(9,11,3),(16,16,4),(17,19,4),(32,32,5),(33,35,5),(65,49,2),(65,49,5),(257,129,3),(257,129,5),(1024,1024,5)]
for w,h,level in shapes:
 for pattern in (['ramp'] if w==1024 else ['noise','mixed']):
  gray=bytes((x*73+y*151+x*y*17+31)%256 if pattern=='noise' else
             (128 if (x//16+y//16)%3==0 else 128+(x+y)%16 if (x//16+y//16)%3==1 else (x*13+y*11)%256) if pattern=='mixed' else
             (x*9+y*7)%256 for y in range(h) for x in range(w))
  source=Image.frombytes('L',(w,h),gray);stem=f'jp2_deep_{w}x{h}_d{level}_{pattern}'
  (out/f'{stem}.gray').write_bytes(gray)
  for ext in ['j2k','jp2']:
   path=out/f'{stem}.{ext}'
   source.save(path,format='JPEG2000',no_jp2=ext=='j2k',num_resolutions=level+1,codeblock_size=(16,16),irreversible=False)
   assert Image.open(path).tobytes()==gray
   print(hashlib.sha256(path.read_bytes()).hexdigest(),path.name)
