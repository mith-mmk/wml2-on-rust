"""Rebuild the frozen 8-bit libjxl oracles (requires ImageMagick with JXL).

Usage: python generate_jxl_xyb_animation.py
Run from any directory. Inputs are adjacent conformance fixtures. This changes
only the two .rgba8.zlib files. Provenance/hashes live in
 docs/jpeg-xl-xyb-animation-oracle.json. The checked-in goldens were generated
with ImageMagick 7.1.2-21 Q16 / libjxl 0.11.2.
"""
from pathlib import Path
import hashlib
import subprocess
import zlib

root = Path(__file__).resolve().parent
for name, size, frames in [("icos4d", 128, 48), ("spline", 320, 60)]:
    stem = root / ("jxl-animation-xyb-" + name)
    raw = subprocess.check_output([
        "magick", str(stem) + ".jxl", "-depth", "8", "rgba:-",
    ])
    assert len(raw) == size * size * frames * 4
    Path(str(stem) + ".rgba8.zlib").write_bytes(zlib.compress(raw, 9))
    print(name, hashlib.sha256(raw).hexdigest())
