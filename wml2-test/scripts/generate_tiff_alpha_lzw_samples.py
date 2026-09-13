#!/usr/bin/env python3
"""Generate the 2026-09-13 TIFF alpha/LZW follow-up corpus."""
import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from generate_tiff_review_samples import lzw_encode, make_tiff, page, p16


def cmyk_rgba(samples):
    result = []
    for c, m, y, k, alpha in samples:
        result.extend([
            ((255 - c) * (255 - k) + 127) // 255,
            ((255 - m) * (255 - k) + 127) // 255,
            ((255 - y) * (255 - k) + 127) // 255,
            alpha,
        ])
    return result


def add(dest, manifest, name, data, **info):
    (dest / name).write_bytes(data)
    manifest[name] = info


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dest", type=Path, required=True)
    args = parser.parse_args()
    args.dest.mkdir(parents=True, exist_ok=True)
    manifest = {}

    pixels = [(0, 0, 0, 0, 0), (10, 20, 30, 40, 85),
              (255, 0, 0, 255, 255)]
    raw = bytes(value for pixel in pixels for value in pixel)
    for be, label in ((False, "le"), (True, "be")):
        block = lzw_encode(raw) if label == "le" else raw
        compression = 5 if label == "le" else 1
        add(args.dest, manifest, f"cmyka_extra2_{label}_{'lzw' if compression == 5 else 'none'}.tif",
            make_tiff([page(3, 1, [8] * 5, 5, 5, [block], compression=compression, extra=[2])], be=be),
            width=3, height=1, expected_rgba=cmyk_rgba(pixels), tags={"PhotometricInterpretation": 5,
            "ExtraSamples": [2], "Compression": compression})
    add(args.dest, manifest, "cmyka_extra2_bigtiff_be_lzw.tif",
        make_tiff([page(3, 1, [8] * 5, 5, 5, [lzw_encode(raw)], compression=5, extra=[2])], be=True, big=True),
        width=3, height=1, expected_rgba=cmyk_rgba(pixels), tags={"ExtraSamples": [2], "Compression": 5})

    associated = bytes((10, 20, 30, 40, 85))
    add(args.dest, manifest, "cmyka_associated_alpha8_le_reject.tif",
        make_tiff([page(1, 1, [8] * 5, 5, 5, [associated], extra=[1])]),
        width=1, height=1, expect_error=True, tags={"ExtraSamples": [1]})
    associated16 = b"".join(p16(value * 257, True) for value in (10, 20, 30, 40, 85))
    add(args.dest, manifest, "cmyka_associated_alpha16_be_reject.tif",
        make_tiff([page(1, 1, [16] * 5, 5, 5, [associated16], extra=[1])], be=True),
        width=1, height=1, expect_error=True, tags={"BitsPerSample": [16], "ExtraSamples": [1]})
    add(args.dest, manifest, "cmyka_associated_alpha8_planar_reject.tif",
        make_tiff([page(1, 1, [8] * 5, 5, 5, [[10], [20], [30], [40], [85]], planar=2, extra=[1])]),
        width=1, height=1, expect_error=True, tags={"PlanarConfiguration": 2, "ExtraSamples": [1]})

    cmap = [0] * (256 * 3)
    for index, value in ((1, 0x11), (2, 0x22)):
        cmap[index] = cmap[256 + index] = cmap[512 + index] = value * 257
    for association, label in ((1, "associated"), (2, "unassociated")):
        add(args.dest, manifest, f"palette_alpha_{association}_{label}_le_reject.tif",
            make_tiff([page(1, 1, [8, 8], 2, 3, [[1, 128]], extra=[association], color_map=cmap)]),
            width=1, height=1, expect_error=True, tags={"PhotometricInterpretation": 3,
            "ExtraSamples": [association]})
    tile_raw = bytearray(16 * 16 * 2)
    tile_raw[0:2] = bytes((1, 255))
    add(args.dest, manifest, "palette_alpha2_tile_le_reject.tif",
        make_tiff([page(1, 1, [8, 8], 2, 3, [bytes(tile_raw)], tile=(16, 16), extra=[2], color_map=cmap)]),
        width=1, height=1, expect_error=True, tags={"ExtraSamples": [2], "TileWidth": 16})

    (args.dest / "manifest.json").write_text(
        json.dumps({"format": "TIFF alpha-lzw follow-up 2026-09-13", "samples": manifest}, indent=2) + "\n",
        encoding="utf-8")
    (args.dest / "README.md").write_text(
        "# TIFF alpha and LZW fixtures\n\n"
        "The CMYK ExtraSamples=2 cases preserve explicit unassociated alpha and nonzero K.\n"
        "CMYK associated alpha and Palette alpha cases are explicit unsupported inputs.\n"
        "All files are original generated test data and are compared by the local verifier.\n",
        encoding="utf-8")
    (args.dest / "LICENSE.txt").write_text(
        "Original generated test data, released under CC0 1.0.\n"
        "No third-party image content is included.\n"
        "https://creativecommons.org/publicdomain/zero/1.0/\n",
        encoding="utf-8")
    hashes = {}
    for path in sorted(args.dest.iterdir()):
        if path.is_file() and path.name != "hashes.json":
            hashes[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    (args.dest / "hashes.json").write_text(json.dumps(hashes, indent=2) + "\n", encoding="utf-8")
    print(f"generated {len(manifest)} samples in {args.dest}")


if __name__ == "__main__":
    main()
