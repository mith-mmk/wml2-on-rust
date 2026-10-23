# Short-Huffman restart fixtures

Public synthetic 64x32 RGB gradient, independently encoded with Pillow 11.1.0
(libjpeg), quality90, subsampling0 (4:4:4) /2 (4:2:0), with/without one-row
restart intervals. No private image or metadata is used. The row-restart outputs
contain three / one RST markers respectively and decode to the same RGBA bytes
as their plain counterparts with both the old and new WML2 binaries.

Reproduce from the repository root:

```python
from pathlib import Path
from PIL import Image

root = Path("wml2/tests/fixtures/review")
rgb = bytes(v for y in range(32) for x in range(64)
            for v in ((x * 7 + y * 3) % 256,
                      (x * 13 + y * 11) % 256,
                      (x * 31 + y * 17) % 256))
image = Image.frombytes("RGB", (64, 32), rgb)
for sub in (0, 2):
    for rows in (0, 1):
        image.save(root / f"jpeg-huffman-s{sub}-rows{rows}.jpg",
                   quality=90, subsampling=sub, restart_marker_rows=rows)
```

A separate local experiment with `restart_marker_blocks=2` produces within-row
markers and exposes a pre-existing WML2 raster mismatch versus the plain file.
It is identical before/after short-code optimization and is not fixed here. The
positive row-boundary regression does not claim general restart-marker coverage.
