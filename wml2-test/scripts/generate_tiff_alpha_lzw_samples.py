#!/usr/bin/env python3
"""Generate the TIFF alpha/LZW follow-up corpus."""
import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from generate_tiff_review_samples import lzw_encode, make_tiff, page, p16, p32


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

    # Associated-alpha fixtures retain enough precision to catch a decoder
    # that shifts to RGBA8 before unassociating the color samples. The
    # expected values are calculated from the original integer ratios.
    alpha16 = [100, 200, 300, 300]
    gray16 = [100, 300]
    # Keep the 32-bit external-oracle samples in the high 16 bits so
    # ImageMagick Q16 can inspect their integer values without reducing a
    # low-alpha probe to its own internal 16-bit cache. The Rust regression
    # still uses a low-alpha 32-bit case to exercise the precision bug.
    alpha32 = 0xFFFF0000
    rgb32 = [alpha32 // 3, alpha32 * 2 // 3, alpha32, alpha32]
    gray32 = [alpha32 // 3, alpha32]
    for bits, rgb, gray, pack, label in [
        (16, alpha16, gray16, p16, "16"),
        (32, rgb32, gray32, p32, "32"),
    ]:
        for be, endian in ((False, "le"), (True, "be")):
            alpha8 = 1 if bits == 16 else 255
            expected_rgb = [85, 170, 255, alpha8]
            expected_gray = [85, 85, 85, alpha8]
            add(args.dest, manifest, f"associated_rgb{label}_{endian}.tif",
                make_tiff([page(1, 1, [bits] * 4, 4, 2,
                                 [b"".join(pack(value, be) for value in rgb)], extra=[1])], be=be),
                width=1, height=1, expected_rgba=expected_rgb,
                oracle_policy="expected_only", tags={"BitsPerSample": [bits], "ExtraSamples": [1]})
            add(args.dest, manifest, f"associated_gray{label}_{endian}.tif",
                make_tiff([page(1, 1, [bits] * 2, 2, 1,
                                 [b"".join(pack(value, be) for value in gray)], extra=[1])], be=be),
                width=1, height=1, expected_rgba=expected_gray,
                oracle_policy="expected_only", tags={"BitsPerSample": [bits], "ExtraSamples": [1]})

    # WhiteIsZero associated alpha must normalize M-S before unassociating.
    # Keep partial and fully transparent samples for both byte orders at every
    # legacy bit depth; native U16 is checked by the Rust regression.
    white_zero_cases = [
        (8, [200, 128], [200, 0], None, "8"),
        (16, [50_000, 60_000], [50_000, 0], p16, "16"),
        (32, [0xC8C8_C8C8, 0x8080_8080], [0xC8C8_C8C8, 0], p32, "32"),
    ]
    for bits, partial, transparent, pack, label in white_zero_cases:
        if bits == 8:
            pack = lambda value, be: bytes([value])
        for be, endian in ((False, "le"), (True, "be")):
            for suffix, samples, expected in [
                ("partial", partial, {8: [110, 110, 110, 128],
                                      16: [66, 66, 66, 233],
                                      32: [110, 110, 110, 128]}[bits]),
                ("transparent", transparent, [0, 0, 0, 0]),
            ]:
                add(args.dest, manifest, f"whiteiszero_gray{label}_{endian}_{suffix}.tif",
                    make_tiff([page(1, 1, [bits, bits], 2, 0,
                                    [b"".join(pack(value, be) for value in samples)],
                                    extra=[1])], be=be),
                    width=1, height=1, expected_rgba=expected,
                    oracle_policy="expected_only",
                    tags={"PhotometricInterpretation": 0, "BitsPerSample": [bits, bits],
                          "ExtraSamples": [1]})

    (args.dest / "manifest.json").write_text(
        json.dumps({"format": "TIFF alpha-lzw follow-up 2026-09-13", "samples": manifest}, indent=2) + "\n",
        encoding="utf-8")
    (args.dest / "README.md").write_text(
        "# TIFF alpha and LZW fixtures\n\n"
        "The CMYK ExtraSamples=2 cases preserve explicit unassociated alpha and nonzero K.\n"
        "CMYK associated alpha and Palette alpha cases are explicit unsupported inputs.\n"
        "Gray/RGB associated-alpha 16/32-bit LE/BE cases use formula-based expected RGBA.\n"
        "WhiteIsZero Gray associated-alpha 8/16/32-bit partial/transparent cases\n"
        "also use formula-based expected RGBA;\n"
        "the local verifier records, but does not require, version-specific oracle output.\n"
        "All files are original generated test data.\n",
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
