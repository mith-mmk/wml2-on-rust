#!/usr/bin/env python3
"""Generate small TIFF FillOrder fixtures and verify with system TIFFReadScanline.

Only the oracle calls libtiff. No third-party codec is used by WML2.
FillOrder reversal is applied to the complete encoded strip, before decoding.
"""
import ctypes
import ctypes.util
import hashlib
import json
from pathlib import Path
import struct
import zlib


def fixture(depth, endian, codec, fill, predictor, samples=None):
    values = samples if samples is not None else [1, 2, 128, 255] if depth == 8 else [1, 0x1234, 0x8001, 65535]
    encoded_values = values if predictor == 1 else [values[0]] + [
        (values[i] - values[i - 1]) % (1 << depth) for i in range(1, 4)
    ]
    raw = bytes(encoded_values) if depth == 8 else struct.pack(endian + "4H", *encoded_values)
    compression = {"raw": 1, "packbits": 32773, "lzw": 5, "old-lsb": 5, "deflate": 8}[codec]
    if codec == "raw":
        strip = raw
    elif codec == "packbits":
        strip = bytes([len(raw) - 1]) + raw
    elif codec == "deflate":
        strip = zlib.compress(raw)
    elif codec == "lzw":
        # Every code remains nine bits in this deliberately tiny standard stream.
        bits = "".join(f"{code:09b}" for code in [256, *raw, 257])
        bits += "0" * (-len(bits) % 8)
        strip = bytes(int(bits[i:i + 8], 2) for i in range(0, len(bits), 8))
    else:
        codes = [256, *raw, 257]
        stream = sum(code << (9 * i) for i, code in enumerate(codes))
        strip = stream.to_bytes((9 * len(codes) + 7) // 8, "little")
    if fill == 2:
        strip = bytes(int(f"{v:08b}"[::-1], 2) for v in strip)
    tags = [(256, 4, 4), (257, 4, 1), (258, 3, depth), (259, 3, compression),
            (262, 3, 1), (266, 3, fill), (273, 4, 0), (277, 3, 1),
            (278, 4, 1), (279, 4, len(strip)), (284, 3, 1)]
    if predictor == 2:
        tags.append((317, 3, 2))
    tags.sort()
    offset = 8 + 2 + 12 * len(tags) + 4
    output = bytearray(b"II" if endian == "<" else b"MM")
    output += struct.pack(endian + "HIH", 42, 8, len(tags))
    for tag, ty, value in tags:
        value = offset if tag == 273 else value
        output += struct.pack(endian + "HHI", tag, ty, 1)
        output += struct.pack(endian + "H", value) + b"\0\0" if ty == 3 else struct.pack(endian + "I", value)
    output += struct.pack(endian + "I", 0) + strip
    return bytes(output), values


def main():
    out = Path(__file__).resolve().parents[2] / "wml2/tests/fixtures"
    libname = ctypes.util.find_library("tiff")
    lib = ctypes.CDLL(libname)
    lib.TIFFGetVersion.restype = ctypes.c_char_p
    lib.TIFFOpen.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    lib.TIFFOpen.restype = ctypes.c_void_p
    lib.TIFFReadScanline.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint16]
    lib.TIFFReadScanline.restype = ctypes.c_int
    lib.TIFFClose.argtypes = [ctypes.c_void_p]
    records = []
    for depth in [8, 16]:
        for endian, label in [("<", "le"), (">", "be")]:
            for codec in ["raw", "packbits", "lzw", "deflate"]:
                for predictor in ([1, 2] if codec in ["lzw", "deflate"] else [1]):
                    for fill in [1, 2]:
                        data, expected = fixture(depth, endian, codec, fill, predictor)
                        name = f"ex-review-fill{fill}-gray{depth}-{label}-{codec}-p{predictor}.tif"
                        path = out / name
                        path.write_bytes(data)
                        tif = lib.TIFFOpen(str(path).encode(), b"r")
                        if not tif:
                            raise RuntimeError(f"oracle open failed: {name}")
                        buffer = ctypes.create_string_buffer((depth * 4 + 7) // 8)
                        try:
                            status = lib.TIFFReadScanline(tif, buffer, 0, 0)
                            actual = list(buffer.raw) if depth == 8 else list(struct.unpack("=4H", buffer.raw))
                            if status != 1 or actual != expected:
                                raise AssertionError((name, status, actual, expected))
                        finally:
                            lib.TIFFClose(tif)
                        records.append(dict(file=name, sha256=hashlib.sha256(data).hexdigest(), depth=depth,
                                            endian=label, codec=codec, predictor=predictor, fill_order=fill,
                                            samples=expected, oracle_status=status, oracle_samples=actual))
    for first in [1, 64, 65, 127, 128, 255]:
        data, expected = fixture(8, "<", "old-lsb", 2, 1, [first, 2, 128, 255])
        name = f"ex-review-fill2-gray8-le-old-lsb-first{first}.tif"
        path = out / name
        path.write_bytes(data)
        tif = lib.TIFFOpen(str(path).encode(), b"r")
        if not tif:
            raise RuntimeError(f"oracle open failed: {name}")
        buffer = ctypes.create_string_buffer(4)
        try:
            status = lib.TIFFReadScanline(tif, buffer, 0, 0)
            actual = list(buffer.raw)
            if status != 1 or actual != expected:
                raise AssertionError((name, status, actual, expected))
        finally:
            lib.TIFFClose(tif)
        records.append(dict(file=name, sha256=hashlib.sha256(data).hexdigest(), depth=8,
                            endian="le", codec="old-lsb", predictor=1, fill_order=2,
                            samples=expected, oracle_status=status, oracle_samples=actual))
    report = dict(oracle_library=libname, oracle_version=lib.TIFFGetVersion().decode(),
                  oracle_api="TIFFReadScanline", count=len(records), standard_cases=48,
                  canonical_old_lsb_cases=6, cases=records)
    (out / "ex-review-fill-order.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k != "cases"}))


if __name__ == "__main__":
    main()
