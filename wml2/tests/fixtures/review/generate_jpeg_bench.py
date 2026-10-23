"""Create public synthetic benchmark inputs; requires Pillow and NumPy.

Usage: python generate_jpeg_bench.py .test-bench
No input images or private metadata are copied into the generated gradients.
"""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageFile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
ImageFile.MAXBLOCK = 64 * 1024 * 1024
base = Image.open(Path(__file__).with_name("jpeg-icc-base.jpg")).convert("RGB")
for sub in (0, 2):
    for progressive in (False, True):
        mode = "progressive" if progressive else "baseline"
        base.resize((16, 16)).save(
            args.output / f"tiny-s{sub}-{mode}.jpg",
            quality=90, subsampling=sub, progressive=progressive,
        )
for name, width, height in [
    ("edge32", 32, 32), ("edge64", 64, 64), ("small", 128, 128),
    ("edge256", 256, 256), ("medium", 1024, 768), ("large", 3072, 2048),
]:
    y, x = np.indices((height, width))
    pixels = np.stack(((x * 7 + y * 3) % 256, (x * 13 + y * 11) % 256,
                       (x * 31 + y * 17) % 256), axis=-1).astype("uint8")
    for sub in (0, 2):
        for progressive in ((False,) if name.startswith("edge") else (False, True)):
            mode = "progressive" if progressive else "baseline"
            Image.fromarray(pixels).save(
                args.output / f"{name}-s{sub}-{mode}.jpg",
                quality=90, subsampling=sub, progressive=progressive,
            )
