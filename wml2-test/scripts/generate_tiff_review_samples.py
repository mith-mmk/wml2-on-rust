#!/usr/bin/env python3
"""Generate small, reproducible TIFF review samples.

The script has no third-party dependency.  It writes only below --dest and
also emits manifest.json containing the expected legacy RGBA result or an
explicit rejection expectation for each sample.
"""
import argparse
import json
import struct
import zlib
from pathlib import Path


def p16(v, be): return struct.pack(">H" if be else "<H", v & 0xffff)
def p32(v, be): return struct.pack(">I" if be else "<I", v & 0xffffffff)
def p64(v, be): return struct.pack(">Q" if be else "<Q", v & 0xffffffffffffffff)


def typed(typ, values, be):
    if typ == 1 or typ == 7:
        return bytes(values)
    if typ == 2:
        return bytes(values) if not isinstance(values, str) else values.encode() + b"\0"
    if typ == 3:
        return b"".join(p16(x, be) for x in values)
    if typ == 4:
        return b"".join(p32(x, be) for x in values)
    if typ == 16:
        return b"".join(p64(x, be) for x in values)
    raise ValueError(f"unsupported TIFF type {typ}")


def lzw_encode(data):
    """Encode TIFF LZW (MSB-first, clear=256, EOI=257)."""
    table = {bytes([i]): i for i in range(256)}
    next_code, width = 258, 9
    codes = [256]
    w = b""
    for byte in data:
        k = bytes([byte])
        if w + k in table:
            w += k
            continue
        codes.append(table[w])
        if next_code < 4096:
            table[w + k] = next_code
            next_code += 1
            if next_code == (1 << width) and width < 12:
                width += 1
        w = k
    if w:
        codes.append(table[w])
    codes.append(257)
    out, bitbuf, nbits = bytearray(), 0, 0
    next_code, width = 258, 9
    # Reconstruct dictionary growth at code emission boundaries.
    previous = None
    for code in codes:
        bitbuf = (bitbuf << width) | code
        nbits += width
        while nbits >= 8:
            nbits -= 8
            out.append((bitbuf >> nbits) & 0xff)
        if code == 256:
            next_code, width, previous = 258, 9, None
        elif code == 257:
            previous = None
        elif previous is not None and next_code < 4096:
            next_code += 1
            if next_code == (1 << width) and width < 12:
                width += 1
        previous = code if code not in (256, 257) else None
    if nbits:
        out.append((bitbuf << (8 - nbits)) & 0xff)
    return bytes(out)


def packbits(data):
    out = bytearray()
    for pos in range(0, len(data), 128):
        chunk = data[pos:pos + 128]
        out.append(len(chunk) - 1)
        out.extend(chunk)
    return bytes(out)


def page_fields(page, be, big, blocks):
    off_type = 16 if big else 4
    offsets = [0] * len(blocks)
    counts = [len(x) for x in blocks]
    f = [
        (254, 4, [page.get("new_subfile", 0)]),
        (255, 3, [page.get("subfile", 0)]),
        (256, 4, [page["width"]]), (257, 4, [page["height"]]),
        (258, 3, page["bits"]), (259, 3, [page["compression"]]),
        (262, 3, [page["photo"]]), (266, 3, [page.get("fill_order", 1)]),
        (277, 3, [page["samples"]]), (278, 4, [page.get("rows", page["height"])]),
        (284, 3, [page.get("planar", 1)]), (317, 3, [page.get("predictor", 1)]),
    ]
    if page.get("sample_format") is not None:
        f.append((339, 3, page["sample_format"]))
    if page.get("tile"):
        f += [(322, 4, [page["tile"][0]]), (323, 4, [page["tile"][1]]),
              (324, off_type, offsets), (325, off_type, counts)]
    else:
        f += [(273, off_type, offsets), (279, off_type, counts)]
    if page.get("extra") is not None: f.append((338, 3, page["extra"]))
    if page.get("color_map") is not None: f.append((320, 3, page["color_map"]))
    if page.get("ink_set") is not None: f.append((332, 3, [page["ink_set"]]))
    if page.get("number_inks") is not None: f.append((334, 3, [page["number_inks"]]))
    if page.get("ink_names") is not None: f.append((333, 2, page["ink_names"]))
    if page.get("icc") is not None: f.append((34675, 7, page["icc"]))
    return f


def make_tiff(pages, *, be=False, big=False):
    """Build one or more Classic/BigTIFF pages with strip or tile blocks."""
    head = bytearray((b"MM" if be else b"II"))
    if big:
        head += p16(43, be) + p16(8, be) + p16(0, be) + p64(16, be)
        entry_size, count_size, next_size, slot = 20, 8, 8, 8
    else:
        head += p16(42, be) + p32(8, be)
        entry_size, count_size, next_size, slot = 12, 2, 4, 4
    ifd_offsets, sizes = [], []
    for page in pages:
        count = len(page_fields(page, be, big, page["blocks"]))
        ifd_offsets.append(len(head) + sum(sizes))
        sizes.append(count_size + count * entry_size + next_size)
    out = head + bytearray(sum(sizes))
    payload_at = len(out)
    encoded_pages = []
    for page in pages:
        blocks = page["blocks"]
        fields = page_fields(page, be, big, blocks)
        # External values and image blocks share the same aligned payload area.
        external = []
        for index, (tag, typ, values) in enumerate(fields):
            raw = typed(typ, values, be)
            if len(raw) > slot:
                if payload_at & 1: out.append(0); payload_at += 1
                external.append((index, payload_at, raw))
                out.extend(raw)
                payload_at += len(raw)
        image_offsets = []
        for block in blocks:
            if payload_at & 1: out.append(0); payload_at += 1
            image_offsets.append(payload_at)
            out.extend(block); payload_at += len(block)
        for index, (tag, typ, values) in enumerate(fields):
            if tag in (273, 324): fields[index] = (tag, typ, image_offsets)
            elif tag in (279, 325): fields[index] = (tag, typ, [len(x) for x in blocks])
        # Recompute external payloads for fields whose placeholder changed.
        for index, at, raw in external:
            tag, typ, values = fields[index]
            updated = typed(typ, values, be)
            out[at:at + len(updated)] = updated
        directory = bytearray()
        directory += p64(len(fields), be) if big else p16(len(fields), be)
        for field_index, (tag, typ, values) in enumerate(fields):
            raw = typed(typ, values, be)
            directory += p16(tag, be) + p16(typ, be)
            directory += p64(len(values), be) if big else p32(len(values), be)
            if len(raw) <= slot:
                directory += raw + b"\0" * (slot - len(raw))
            else:
                # Match the field index, including updated offset arrays.
                found = next((at for external_index, at, _ in external
                              if external_index == field_index), None)
                if found is None: raise AssertionError((tag, typ))
                directory += p64(found, be) if big else p32(found, be)
        page_index = len(encoded_pages)
        next_ifd = ifd_offsets[page_index + 1] if page_index + 1 < len(ifd_offsets) else 0
        directory += p64(next_ifd, be) if big else p32(next_ifd, be)
        encoded_pages.append((ifd_offsets[page_index], directory))
    for at, directory in encoded_pages:
        out[at:at + len(directory)] = directory
    return bytes(out)


def page(width, height, bits, samples, photo, blocks, compression=1, **kw):
    return dict(width=width, height=height, bits=bits, samples=samples,
                photo=photo, blocks=blocks, compression=compression, **kw)


def add_sample(dest, manifest, name, data, **info):
    (dest / name).write_bytes(data)
    manifest[name] = info


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dest", type=Path, required=True,
                    help="directory receiving TIFF samples and manifest.json")
    args = ap.parse_args()
    args.dest.mkdir(parents=True, exist_ok=True)
    manifest = {}
    raw16 = bytes([7]) * 256
    for compression, encoded in [
        (1, raw16), (5, lzw_encode(raw16)), (8, zlib.compress(raw16)),
        (32946, zlib.compress(raw16)), (32773, packbits(raw16)),
    ]:
        name = {1: "none", 5: "lzw", 8: "deflate8", 32946: "deflate32946", 32773: "packbits"}[compression]
        add_sample(args.dest, manifest, f"r1_tile_{name}_classic_le.tif",
                   make_tiff([page(1, 1, [8], 1, 1, [encoded], compression=compression, tile=(16, 16))]),
                   width=1, height=1, expected_rgba=[7, 7, 7, 255], limit_expanded_bytes=4)
    # R2 files model the stale four-element SampleFormat in an RGB output.
    for big in (False, True):
        for compression in (1, 5, 8):
            data = [10, 20, 30]
            block = data if compression == 1 else (lzw_encode(bytes(data)) if compression == 5 else zlib.compress(bytes(data)))
            suffix = "bigtiff" if big else "classic"
            add_sample(args.dest, manifest, f"r2_opaque_rgba_stale_sampleformat_{suffix}_{compression}.tif",
                       make_tiff([page(1, 1, [8, 8, 8], 3, 2, [block], compression=compression,
                                      sample_format=[1, 1, 1, 1])], big=big),
                       width=1, height=1, expect_error=True)
    add_sample(args.dest, manifest, "r2_source_opaque_rgba_sampleformat4.tif",
               make_tiff([page(1, 1, [8, 8, 8, 8], 4, 2, [[10, 20, 30, 255]], extra=[2],
                               sample_format=[1, 1, 1, 1])]),
               width=1, height=1, expected_rgba=[10, 20, 30, 255], check_encoder=True)
    for new_subfile, label in ((0, "subfiletype1"), (2, "newsubfiletype2")):
        p1 = page(1, 1, [8], 1, 1, [[1]])
        p2 = page(1, 1, [8], 1, 1, [[2]], new_subfile=new_subfile, subfile=1 if new_subfile == 0 else 0)
        add_sample(args.dest, manifest, f"r3_multipage_{label}.tif", make_tiff([p1, p2]),
                   width=1, height=1, pages=2, expected_pages_rgba=[[1, 1, 1, 255], [2, 2, 2, 255]])
    add_sample(args.dest, manifest, "r4_gray_extra_samples_unspecified.tif",
               make_tiff([page(1, 1, [8, 8], 2, 1, [[128, 0]], extra=[0])]),
               width=1, height=1, expected_rgba=[128, 128, 128, 255], expect_native_error="unknown extra channel")
    cmap = [0] * 48
    for i, value in ((1, 17), (2, 34)):
        cmap[i] = cmap[16 + i] = cmap[32 + i] = value * 257
    for be, label in ((False, "le"), (True, "be")):
        add_sample(args.dest, manifest, f"r5_palette_4bit_fillorder2_{label}.tif",
                   make_tiff([page(2, 1, [4], 1, 3, [[0x48]], fill_order=2, color_map=cmap)], be=be),
                   width=2, height=1, expected_rgba=[17,17,17,255,34,34,34,255], tags={"FillOrder": 2})
    add_sample(args.dest, manifest, "r6_rows_per_strip_u32_max.tif",
               make_tiff([page(1, 2, [8], 1, 1, [[4, 5]], rows=0xffffffff)]),
               width=1, height=2, expected_rgba=[4,4,4,255,5,5,5,255], tags={"RowsPerStrip": 4294967295})
    add_sample(args.dest, manifest, "inkset2_non_cmyk_separation.tif",
               make_tiff([page(1, 1, [8,8,8,8], 4, 5, [[0,0,0,0]], ink_set=2, number_inks=4,
                               ink_names=b"Cyan\0Magenta\0Yellow\0Black\0")]),
               width=1, height=1, expect_error=True)
    # Native/high-bit-depth boundary samples, with all endian/variant pairs.
    native_pages = [
        ("gray16", page(2, 1, [16], 1, 1, [[]]), [0x1234, 0xabcd], [18,18,18,255,171,171,171,255]),
        ("gray16_whiteiszero", page(1, 1, [16], 1, 0, [[]]), [0], [255,255,255,255]),
        ("rgb16", page(1, 1, [16,16,16], 3, 2, [[]]), [0x1234,0x5678,0x9abc], [18,86,154,255]),
        ("rgba16", page(1, 1, [16,16,16,16], 4, 2, [[]], extra=[2]), [0x1234,0x5678,0x9abc,0xffff], [18,86,154,255]),
    ]
    for stem, pg, expected, expected_rgba in native_pages:
        for be, endian in ((False, "le"), (True, "be")):
            for big in (False, True):
                # Re-encode the supplied 16-bit samples in the requested byte order.
                vals = expected
                raw = b"".join(p16(v, be) for v in vals)
                pg2 = dict(pg, blocks=[raw])
                suffix = f"{endian}_{'bigtiff' if big else 'classic'}"
                native_values = [0xffff - v for v in vals] if pg["photo"] == 0 else vals
                add_sample(args.dest, manifest, f"native_{stem}_{suffix}.tif", make_tiff([pg2], be=be, big=big),
                           width=pg["width"], height=pg["height"], native_samples=native_values,
                           expected_rgba=expected_rgba)
    for be, label in ((False, "le"), (True, "be")):
        planes = [b"".join(p16(value, be) for value in (0x1234, 0x1234)),
                  b"".join(p16(value, be) for value in (0x5678, 0x5678)),
                  b"".join(p16(value, be) for value in (0x9abc, 0x9abc))]
        add_sample(args.dest, manifest, f"native_planar_rgb16_{label}.tif",
                   make_tiff([page(2, 1, [16,16,16], 3, 2, planes, planar=2)], be=be),
                   width=2, height=1, native_samples=[0x1234,0x5678,0x9abc] * 2,
                   expected_rgba=[18,86,154,255,18,86,154,255], tags={"PlanarConfiguration": 2})
    (args.dest / "manifest.json").write_text(json.dumps({"format": "TIFF review a08bf058", "samples": manifest}, indent=2) + "\n", encoding="utf-8")
    print(f"generated {len(manifest)} samples in {args.dest}")


if __name__ == "__main__":
    main()
