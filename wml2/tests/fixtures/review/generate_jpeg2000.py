"""Generate deterministic original fixtures using Pillow/OpenJPEG as an oracle.
Run from repository root; never invoked by Cargo or production decoding.
"""
from pathlib import Path
from PIL import Image, features, __version__
import hashlib
out = Path('wml2/tests/fixtures/review')
print('Pillow', __version__, 'OpenJPEG', features.version('jpg_2000'))
for w, h in [(1,1),(2,2),(3,5),(8,8),(15,13),(16,16)]:
    for levels in range(2 if min(w,h)>=2 else 1):
        for pattern in ['noise','ramp','flat','checker']:
            pixels = bytes(((x*73+y*151+x*y*17+31)%256 if pattern=='noise' else
                            (x*9+y*7)%256 if pattern=='ramp' else
                            128 if pattern=='flat' else 255*((x+y)%2))
                           for y in range(h) for x in range(w))
            image=Image.frombytes('L',(w,h),pixels)
            stem=f'jp2_{w}x{h}_d{levels}_{pattern}'
            for ext in ['j2k','jp2']:
                path=out/f'{stem}.{ext}'
                image.save(path,format='JPEG2000',no_jp2=ext=='j2k',irreversible=False,
                           num_resolutions=levels+1,codeblock_size=(16,16),progression='LRCP')
                assert Image.open(path).tobytes()==pixels
                print(hashlib.sha256(path.read_bytes()).hexdigest(),path.name)
            (out/f'{stem}.gray').write_bytes(pixels)
