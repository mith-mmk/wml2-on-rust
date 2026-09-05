"""Generate a small, independent LittleCMS RGB gamma oracle.

Usage: python test/generate_review_icc_oracle.py OUTPUT_DIRECTORY
Requires Pillow with LittleCMS; ordinary Rust tests use the checked-in CSV.
"""
import csv
import io
import struct
import sys
from pathlib import Path
from PIL import Image, ImageCms


def profile(gamma):
    def xyz(x, y, z):
        return b"XYZ " + bytes(4) + struct.pack(">iii", round(x*65536), round(y*65536), round(z*65536))

    curve = b"curv" + bytes(4) + struct.pack(">I", 1) + struct.pack(">H", gamma*256)
    tags = [(b"rXYZ", xyz(1, 0, 0)), (b"gXYZ", xyz(0, 1, 0)),
            (b"bXYZ", xyz(0, 0, 1)), (b"rTRC", curve),
            (b"gTRC", curve), (b"bTRC", curve),
            (b"wtpt", xyz(0.9642, 1, 0.8249))]
    data = bytearray(132 + len(tags)*12)
    data[8:12] = bytes([4, 0, 0, 0])
    data[12:24] = b"mntrRGB XYZ "
    data[36:40] = b"acsp"
    struct.pack_into(">I", data, 64, 1)
    data[68:80] = xyz(0.9642, 1, 0.8249)[8:]
    struct.pack_into(">I", data, 128, len(tags))
    for index, (signature, tag) in enumerate(tags):
        struct.pack_into(">4sII", data, 132+index*12, signature, len(data), len(tag))
        data.extend(tag)
        data.extend(bytes((-len(data)) % 4))
    struct.pack_into(">I", data, 0, len(data))
    return bytes(data)


destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
source, target = profile(2), profile(1)
(destination/"gamma2-rgb.icc").write_bytes(source)
(destination/"linear-rgb.icc").write_bytes(target)
inputs = [(v, (v*73) % 256, 255-v) for v in range(256)]
image = Image.new("RGB", (len(inputs), 1))
image.putdata(inputs)
transform = ImageCms.buildTransformFromOpenProfiles(
    ImageCms.ImageCmsProfile(io.BytesIO(source)),
    ImageCms.ImageCmsProfile(io.BytesIO(target)),
    "RGB", "RGB", renderingIntent=ImageCms.Intent.RELATIVE_COLORIMETRIC,
    flags=ImageCms.Flags.NOOPTIMIZE)
result = ImageCms.applyTransform(image, transform)
with (destination/"lcms-gamma-rgb.csv").open("w", newline="", encoding="utf-8") as file:
    writer = csv.writer(file)
    writer.writerow(["r", "g", "b", "out_r", "out_g", "out_b"])
    writer.writerows((*before, *after) for before, after in zip(inputs, result.getdata()))
print(f"LittleCMS {ImageCms.core.littlecms_version}: {len(inputs)} RGB vectors")
