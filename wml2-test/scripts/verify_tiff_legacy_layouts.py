#!/usr/bin/env python3
"""Independent Pillow checks for bounded legacy TIFF compatibility.

Requires an existing to_png executable. Optionally checks the externally fetched
smallliz.tif against its pinned corpus hash; never uses a private sample.
All generated artifacts stay under the explicitly named .test directory.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

from PIL import Image


def raw_rgb(be, missing):
    order = ">" if be else "<"
    width, height = 240, 164
    pixels = bytes(v for y in range(height) for x in range(width)
                   for v in (x * 255 // (width - 1), y * 255 // (height - 1), (x + y) % 256))
    fields = [(256, 4, 1, width), (257, 4, 1, height),
              (258, 3, 3, None), (259, 3, 1, 1), (262, 3, 1, 2),
              (273, 4, 1, None), (277, 3, 1, 3), (284, 3, 1, 1)]
    if not missing:
        fields += [(278, 4, 1, height), (279, 4, 1, len(pixels))]
    fields.sort()
    extra = 8 + 2 + 12 * len(fields) + 4
    output = bytearray(b"MM" if be else b"II")
    output += struct.pack(order + "HIH", 42, 8, len(fields))
    for tag, kind, count, value in fields:
        value = extra if tag == 258 else extra + 6 if tag == 273 else value
        output += struct.pack(order + "HHI", tag, kind, count)
        output += struct.pack(order + "H", value) + b"\0\0" if kind == 3 and count == 1 else struct.pack(order + "I", value)
    output += struct.pack(order + "I3H", 0, 8, 8, 8) + pixels
    return bytes(output), (width, height), pixels


def check(converter, source, output, size, expected=None, tolerance=0):
    subprocess.run([str(converter), str(source), str(output)], check=True,
                   capture_output=True, timeout=30)
    with Image.open(source) as reference, Image.open(output / (source.name + ".png")) as result:
        assert reference.size == result.size == size
        a, b = reference.convert("RGBA").tobytes(), result.convert("RGBA").tobytes()
        assert len(a) == len(b) == size[0] * size[1] * 4
        if expected is not None:
            rgba = bytes(v for i in range(0, len(expected), 3)
                         for v in (*expected[i:i + 3], 255))
            assert a == b == rgba, source
        delta = [abs(x - y) for x, y in zip(a, b)]
        assert max(delta) <= tolerance, (source, max(delta))
        assert a[3::4] == b[3::4], "alpha differs"
        return {"name": source.name, "size": size, "rgba_bytes": len(b),
                "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                "max_abs_error": max(delta), "mean_abs_error": sum(delta) / len(delta)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--converter", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=Path(".test-tiff-legacy"))
    parser.add_argument("--smallliz", type=Path)
    args = parser.parse_args()
    assert args.output.name.startswith(".test"), "output must be a .test directory"
    args.output.mkdir(parents=True, exist_ok=True)
    images = args.output / ".test-png"
    images.mkdir(exist_ok=True)
    results = []
    for be in [False, True]:
        for missing in [False, True]:
            data, size, expected = raw_rgb(be, missing)
            source = args.output / f"rgb-{'be' if be else 'le'}-{'missing' if missing else 'explicit'}.tif"
            source.write_bytes(data)
            results.append(check(args.converter.resolve(), source, images, size, expected))
    if args.smallliz:
        assert hashlib.sha256(args.smallliz.read_bytes()).hexdigest() == "e8f722bc322b9a7621d1d858bec36ea78d31b85221f314a2b9872d3af91d600a"
        results.append(check(args.converter.resolve(), args.smallliz, images, (160, 160), tolerance=3))
    report = {"oracle": "Pillow " + Image.__version__, "results": results,
              "private_XING_T24_tested": False}
    encoded = json.dumps(report, indent=2) + "\n"
    (args.output / "results.json").write_text(encoded)
    print(encoded, end="")


if __name__ == "__main__":
    main()
