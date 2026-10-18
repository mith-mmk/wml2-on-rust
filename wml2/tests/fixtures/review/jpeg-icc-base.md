# JPEG ICC parser regression raster

`jpeg-icc-base.jpg` is an independently Pillow-encoded 8x8 constant RGB (51,102,153) JPEG, quality 100. The regression tests insert APP2 metadata after SOI; no WML2 encoder is used to create this raster. Regenerate using Python and Pillow:

```python
from PIL import Image
Image.new("RGB", (8, 8), (51, 102, 153)).save("jpeg-icc-base.jpg", quality=100)
```

The existing `linear-rgb.icc` fixture supplies real RGB profile bytes. A separate 255-chunk case deliberately uses opaque bytes to ensure container assembly does not become a CMS/profile-semantic check.
