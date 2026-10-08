#!/usr/bin/env python3
"""Generate EX regression TIFFs with Pillow/libtiff, independently of WML2."""
import argparse
import struct
from pathlib import Path
from PIL import Image, features


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dest', type=Path, required=True)
    args = parser.parse_args()
    if not args.dest.is_dir():
        parser.error('--dest must be an existing directory')
    width, height = 257, 3
    for depth in (8, 16):
        values = [(x * 37 + y * 83 + (x // 7) * 11) % 256 if depth == 8
                  else (x * 257 + y * 4660 + (x // 17) * 31) % 65536
                  for y in range(height) for x in range(width)]
        raw = bytes(values) if depth == 8 else struct.pack('<' + 'H' * len(values), *values)
        image = Image.frombytes('L' if depth == 8 else 'I;16', (width, height), raw)
        for name, compression in (('lzw', 'tiff_lzw'), ('packbits', 'packbits')):
            path = args.dest / f'ex-gray{depth}-{name}.tif'
            image.save(path, compression=compression)
            with Image.open(path) as check:
                check.load()
                assert list(check.get_flattened_data()) == values, path
                assert check.tag_v2[259] == (5 if name == 'lzw' else 32773)
            print(path, path.stat().st_size)
    print('libtiff', features.version('libtiff'))


if __name__ == '__main__':
    main()
