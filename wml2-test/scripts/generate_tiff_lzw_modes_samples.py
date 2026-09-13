#!/usr/bin/env python3
"""Generate independent TIFF LZW mode fixtures.

The fixtures deliberately keep code packing separate from TIFF FillOrder:
standard TIFF is MSB/early-change, the historical LibTIFF stream is
LSB/late-change, and the WML2 extension is LSB/early-change.  Each mode is
written with both FillOrder values so the verifier records whether an
independent reader accepts the tag combination.  The payload is long enough
to cross the 9, 10 and 11 bit code-width boundaries.
"""
import argparse
import hashlib
import json
from pathlib import Path

import sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from generate_tiff_review_samples import make_tiff, page


def lzw_encode(data, *, lsb, early_change):
    table = {bytes([i]): i for i in range(256)}
    clear, end = 256, 257
    width, next_code = 9, 258
    output = bytearray()
    bit_buffer = 0
    bit_count = 0

    def emit(code, code_width):
        nonlocal bit_buffer, bit_count
        if lsb:
            bit_buffer |= code << bit_count
            bit_count += code_width
            while bit_count >= 8:
                output.append(bit_buffer & 0xff)
                bit_buffer >>= 8
                bit_count -= 8
        else:
            bit_buffer = (bit_buffer << code_width) | code
            bit_count += code_width
            while bit_count >= 8:
                shift = bit_count - 8
                output.append((bit_buffer >> shift) & 0xff)
                bit_buffer &= (1 << shift) - 1 if shift else 0
                bit_count -= 8

    emit(clear, width)
    if not data:
        emit(end, width)
        return bytes(output)
    current = bytes([data[0]])
    for value in data[1:]:
        candidate = current + bytes([value])
        if candidate in table:
            current = candidate
            continue
        emit(table[current], width)
        if next_code < 4096:
            table[candidate] = next_code
            next_code += 1
            threshold = 1 << width
            if width < 12 and ((next_code == threshold) if early_change else (next_code > threshold)):
                width += 1
        else:
            emit(clear, width)
            table = {bytes([i]): i for i in range(256)}
            width, next_code = 9, 258
        current = bytes([value])
    emit(table[current], width)
    threshold = 1 << width
    if width < 12 and ((next_code + 1 == threshold) if early_change else (next_code == threshold)):
        width += 1
    emit(end, width)
    if bit_count:
        output.append((bit_buffer & 0xff) if lsb else ((bit_buffer << (8 - bit_count)) & 0xff))
    return bytes(output)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dest", type=Path, required=True)
    args = parser.parse_args()
    args.dest.mkdir(parents=True, exist_ok=True)
    raw = bytes((index * 37 + index // 127) % 256 for index in range(4096))
    rgba = []
    for value in raw:
        rgba.extend((value, value, value, 255))
    manifest = {}
    cases = [
        ("standard_msb_fill1", False, True, 1, "standard_msb", "required_match"),
        ("libtiff_lsb_late_fill1", True, False, 1, "libtiff_lsb_late", "required_match"),
        ("wml2_lsb_early_fill2", True, True, 2, "wml2_lsb_early", "wml2_custom"),
        ("standard_msb_fill2", False, True, 2, "standard_msb", "observe"),
        ("libtiff_lsb_late_fill2", True, False, 2, "libtiff_lsb_late", "observe"),
        ("wml2_lsb_early_fill1", True, True, 1, "wml2_lsb_early", "wml2_custom"),
    ]
    for name, lsb, early, fill_order, mode, oracle_policy in cases:
        encoded = lzw_encode(raw, lsb=lsb, early_change=early)
        filename = f"lzw_{name}_9_10_11bit.tif"
        add = dict(width=256, height=16, expected_rgba=rgba,
                   tags={"Compression": 5, "FillOrder": fill_order}, lzw_mode=mode)
        add["oracle_policy"] = oracle_policy
        (args.dest / filename).write_bytes(
            make_tiff([page(256, 16, [8], 1, 1, [encoded], compression=5,
                        fill_order=fill_order)])
        )
        manifest[filename] = add
    (args.dest / "manifest.json").write_text(
        json.dumps({"format": "TIFF LZW mode separation 2026-09-13", "samples": manifest}, indent=2) + "\n",
        encoding="utf-8")
    (args.dest / "README.md").write_text(
        "# TIFF LZW mode fixtures\n\n"
        "These original, generated fixtures cross the 9, 10 and 11 bit code-width boundaries.\n"
        "`standard_msb` and `libtiff_lsb_late` are compared with ImageMagick/LibTIFF.\n"
        "`wml2_lsb_early` is the WML2-specific stream; FillOrder is varied independently.\n",
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
