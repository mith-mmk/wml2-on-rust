# JPEG/PNG source versus output ICC metadata fixtures

The five `jpeg-icc-{gray,gray-progressive,cmyk,cmyk-progressive,ycck}.jpg`
rasters are independently encoded constant8×8 images without ICC metadata.
The tests attach APP2 payloads and compare successful complete decoded rasters
with their untagged originals. The pre-existing progressive decoder rejects
the progressive CMYK fixture (`Nothing marker but 7f`), so that fixture tests
only parsed-header metadata routing and does not claim new raster support. Gray=128; ordinary CMYK=(64,96,128,32).
Pillow encodes baseline/progressive Gray and CMYK; ImageMagick encodes YCCK
(Adobe APP14 transform2) from the independent CMYK JPEG.
No WML2 encoder generates these JPEG references.

Regenerate the five rasters with Python/Pillow and ImageMagick:

```python
from pathlib import Path
from PIL import Image
import subprocess
root = Path(__file__).parent  # this fixtures/review directory
for mode, pixel, kind in [('L', 128, 'gray'), ('CMYK', (64,96,128,32), 'cmyk')]:
    for progressive in [False, True]:
        suffix = '-progressive' if progressive else ''
        Image.new(mode, (8,8), pixel).save(
            root / f'jpeg-icc-{kind}{suffix}.jpg',
            quality=100, progressive=progressive)
subprocess.run([
    'convert', str(root / 'jpeg-icc-cmyk.jpg'), '-strip', '-colorspace',
    'CMYK', '-quality', '100', str(root / 'jpeg-icc-ycck.jpg')], check=True)
```

The existing `jpeg-icc-base.jpg` is the independent RGB reference documented
in `jpeg-icc-base.md`. `linear-rgb.icc` supplies a real RGB profile for native
RGB(A)/palette retention. Source-only tests deliberately use opaque ICC
container bytes: these paths must preserve bytes without requiring the optional
color-management feature or adding another ICC parser. Real Gray/CMYK profiles
were also checked independently in the audit scratch harness.

PNG tests build deterministic1×1 chunks directly, with ordinary zlib scanlines
and iCCP metadata. They cover Gray1/2/4/8/16, GrayAlpha8/16, RGB8/16, RGBA8/16,
and indexed RGB with palette alpha. PNG EX tests retain the existing single
`Source ICC Profile` key for both Gray and RGB storage. The checks do not
claim that numeric Gray/CMYK expansion applies a profile or produces sRGB.

SHA-256 of the checked-in JPEG references:

```text
1f84bba1347816d0ba1abee2a59d8e0af70edfb5bd451d05e10d1db5df0570dd  jpeg-icc-cmyk-progressive.jpg
f279e7ced9e936d9bd6764dedd529d6a937d935c33bbb603c71d5ab4f8ce6593  jpeg-icc-cmyk.jpg
8b934046c762e64fb1915a56047b18449ec6e69db72945317e505431d51bc717  jpeg-icc-gray-progressive.jpg
c167e27698ca03e30d2600085b6058fda938fa524811afcda51e5a52b72366a4  jpeg-icc-gray.jpg
09289dfe378a2b1b6f3abc9666d8ea35f24811b0d18222c2bfc2b1e14feea975  jpeg-icc-ycck.jpg
```
