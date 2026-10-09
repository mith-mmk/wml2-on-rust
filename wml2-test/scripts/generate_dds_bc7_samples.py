#!/usr/bin/env python3
"""Build independent BC7 bitstream fixtures and decode them with Pillow.

Requires Pillow 12.3.0 to reproduce the committed oracle exactly. No WML2
encoder/decoder is used. Partition/anchor numeric data is read from the pinned
Microsoft DirectXTex reference (MIT), checked against the normative Khronos
BPTC specification during implementation. The corresponding license notice is
retained with the adapted production table data in wml2/src/dds.rs.

Run with --dest pointing to an existing directory, e.g. a .test* directory.
"""

import argparse
import random
import re
import urllib.request
from pathlib import Path

from PIL import Image

REFERENCE = "https://raw.githubusercontent.com/microsoft/DirectXTex/1acf4ebe6330e06b7eb52e8db63d4f824ad77427/DirectXTex/BC6HBC7.cpp"
MODES = {
    # subsets, partition bits, color bits, alpha bits, pbit style, index bits
    0: (3, 4, 4, 0, 1, 3),
    1: (2, 6, 6, 0, 2, 3),
    2: (3, 6, 5, 0, 0, 2),
    3: (2, 6, 7, 0, 1, 2),
    7: (2, 6, 5, 5, 1, 2),
}


def reference_tables():
    source = urllib.request.urlopen(REFERENCE).read().decode()

    def extract(name, width):
        text = source.split(name, 1)[1].split("};", 1)[0]
        text = re.sub(r"//[^\n]*", "", text)
        rows = re.findall(r"\{\s*([\d,\s]+)\s*\}", text)
        values = [[int(v) for v in row.split(",") if v.strip()] for row in rows]
        assert len(values) == 192 and all(len(v) == width for v in values)
        return [values[i * 64:(i + 1) * 64] for i in range(3)]

    return extract("g_aPartitionTable[3][64][16]", 16), extract("g_aFixUp[3][64][3]", 3)


def container(blocks, width=4, height=None):
    height = len(blocks) * 4 if height is None else height
    assert len(blocks) == ((width + 3) // 4) * ((height + 3) // 4)
    header = bytearray(148)
    header[:4] = b"DDS "
    header[84:88] = b"DX10"
    for offset, value in [(4, 124), (8, 0x81007), (12, height), (16, width),
                          (20, len(blocks) * 16), (76, 32), (80, 4),
                          (108, 0x1000), (128, 98), (132, 3), (140, 1)]:
        header[offset:offset + 4] = value.to_bytes(4, "little")
    return header + b"".join(blocks)


def targeted_block(mode, partition, kind, patterns, anchors):
    subsets, partition_bits, color_bits, alpha_bits, pbit_style, index_bits = MODES[mode]
    fields = [(1 << mode, mode + 1), (partition, partition_bits)]
    channels = 4 if alpha_bits else 3
    maximum = (1 << color_bits) - 1
    for channel in range(channels):
        for endpoint in range(subsets * 2):
            choices = [0, maximum, maximum if endpoint % 2 else 0,
                       (endpoint * 13 + channel * 7) % (maximum + 1),
                       maximum if (endpoint + channel) % 2 else 0, 0, maximum]
            fields.append((choices[kind], color_bits))
    if pbit_style == 1:
        for endpoint in range(subsets * 2):
            bit = (endpoint + kind) % 2 if kind < 5 else kind - 5
            fields.append((bit, 1))
    elif pbit_style == 2:
        for subset in range(subsets):
            fields.append(((subset + kind) % 2, 1))
    pattern = patterns[subsets - 1][partition]
    fixups = anchors[subsets - 1][partition]
    for pixel in range(16):
        bits = index_bits - (pixel == fixups[pattern[pixel]])
        choices = [0, (1 << bits) - 1, pixel % (1 << bits),
                   (pixel * 3 + 1) % (1 << bits), (pixel * 5 + 3) % (1 << bits), 0, 0]
        fields.append((choices[kind], bits))
    value = 0
    shift = 0
    for field, bits in fields:
        assert 0 <= field < 1 << bits
        value |= field << shift
        shift += bits
    assert shift == 128
    return value.to_bytes(16, "little")


def save(dest, name, blocks, width=4, height=None):
    encoded = container(blocks, width, height)
    path = dest / (name + ".dds")
    path.write_bytes(encoded)
    with Image.open(path) as image:
        decoded = image.convert("RGBA")
        rgba = decoded.tobytes()
        assert len(rgba) == decoded.width * decoded.height * 4
        (dest / (name + ".rgba")).write_bytes(rgba)
    print(name, len(blocks), "blocks", len(rgba), "RGBA bytes")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dest", required=True, type=Path)
    args = parser.parse_args()
    if not args.dest.is_dir():
        parser.error("--dest must be an existing directory")
    if Image.__version__ != "12.3.0":
        parser.error("Pillow 12.3.0 is required for the committed oracle")
    patterns, anchors = reference_tables()
    rng = random.Random(0xBC7456)
    singles = []
    for mode in [4, 5, 6]:
        for _ in range(64):
            block = bytearray(rng.randbytes(16))
            block[0] = (block[0] & (~((1 << (mode + 1)) - 1) & 255)) | (1 << mode)
            singles.append(bytes(block))
    save(args.dest, "dds-bc7-single-subset", singles)

    rng = random.Random(0xBC701237)
    random_blocks = []
    for mode, (_, partition_bits, *_) in MODES.items():
        for partition in range(1 << partition_bits):
            for _ in range(2):
                value = int.from_bytes(rng.randbytes(16), "little")
                low_bits = mode + 1 + partition_bits
                value = (value >> low_bits << low_bits) | (1 << mode) | (partition << (mode + 1))
                random_blocks.append(value.to_bytes(16, "little"))
    save(args.dest, "dds-bc7-partitioned", random_blocks)

    targeted = []
    for mode, (_, partition_bits, *_) in MODES.items():
        for partition in [0, (1 << partition_bits) - 1]:
            for kind in range(5):
                targeted.append(targeted_block(mode, partition, kind, patterns, anchors))
    # Mode7 minimum/maximum RGBA with uniformly zero/one endpoint pbits.
    targeted += [targeted_block(7, 63, kind, patterns, anchors) for kind in [5, 6]]
    save(args.dest, "dds-bc7-targeted", targeted)
    # Four different modes/partitions in one non-multiple-of-four image.
    mixed = [targeted_block(mode, partition, 3, patterns, anchors)
             for mode, partition in [(0, 15), (1, 63), (2, 63), (7, 63)]]
    save(args.dest, "dds-bc7-mixed-clipped", mixed, 5, 7)


if __name__ == "__main__":
    main()
