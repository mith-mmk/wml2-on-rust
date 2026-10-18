#!/usr/bin/env python3
"""Verify exported AVIF registry integration samples with Pillow and FFmpeg.

Create an existing .test directory, then set WML2_AVIF_REGISTRY_ORACLES to it
when running avif_ex and highres_avif_encode (avifenc,image-buffer-ex,high-bit-depth).
This script requires all nine exports, compares every color/alpha sample, and
writes a checkerboard preview plus a machine-readable report in that directory.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

from PIL import Image, ImageDraw, __version__ as pillow_version


def run(*command):
    return subprocess.run(command, check=True, capture_output=True).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    if not out.is_dir() or not out.name.startswith('.test'):
        parser.error('--output must be an existing .test directory')
    names = [f'{label}-{bits}' for label in ['upstream-native', 'upstream-shared', 'wml2-ex']
             for bits in [8, 12]] + ['upstream-legacy-8', 'wml2-legacy-8', 'wml2-highres-10']
    preview = Image.new('RGB', (540, 150 * len(names)), 'white')
    draw = ImageDraw.Draw(preview)
    results = []
    for i, name in enumerate(names):
        path = out / f'{name}.avif'
        bits = int(name.rsplit('-', 1)[1])
        maximum = (1 << bits) - 1
        source = struct.unpack('<16H', (out / f'{name}.rgba16le').read_bytes())
        expected8 = bytes((v * 255 + maximum // 2) // maximum for v in source)
        with Image.open(path) as image:
            assert image.size == (2, 2), name
            rgba = image.convert('RGBA')
            assert rgba.tobytes() == expected8, f'{name}: Pillow color/alpha mismatch'
        streams = json.loads(run('ffprobe', '-v', 'error', '-show_streams', '-of', 'json', str(path)))['streams']
        assert len(streams) == 2, f'{name}: expected independent color/alpha streams'
        color_format = 'gbrp' if bits == 8 else f'gbrp{bits}le'
        alpha_format = 'gray' if bits == 8 else f'gray{bits}le'
        assert streams[0]['pix_fmt'] == color_format, f'{name}: color bitdepth'
        assert streams[1]['pix_fmt'] == alpha_format, f'{name}: alpha bitdepth'
        for stream, fmt, channels in [(0, color_format, [1, 2, 0]), (1, alpha_format, [3])]:
            raw = run('ffmpeg', '-v', 'error', '-threads', '1', '-i', str(path),
                      '-map', f'0:{stream}', '-frames:v', '1', '-threads', '1',
                      '-pix_fmt', fmt, '-f', 'rawvideo', 'pipe:1')
            actual = tuple(raw) if bits == 8 else struct.unpack(f'<{len(raw)//2}H', raw)
            expected = tuple(source[pixel * 4 + c] for c in channels for pixel in range(4))
            assert actual == expected, f'{name}: FFmpeg stream {stream} mismatch'
        # Show decoded RGBA over a checkerboard beside its independent reference.
        draw.text((8, i * 150 + 5), name, fill='black')
        for column, tile in enumerate([rgba, Image.frombytes('RGBA', (2, 2), expected8)]):
            background = Image.new('RGBA', (120, 120), 'white')
            grid = ImageDraw.Draw(background)
            for y in range(0, 120, 15):
                for x in range(0, 120, 15):
                    if (x // 15 + y // 15) % 2:
                        grid.rectangle((x, y, x + 14, y + 14), fill=(190, 190, 190))
            background.alpha_composite(tile.resize((120, 120), Image.Resampling.NEAREST))
            preview.paste(background.convert('RGB'), (255 + column * 135, i * 150 + 25))
        results.append(dict(case=name, depth=bits, rgba_samples=16,
                            sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                            pillow_rgba8_exact=True, ffmpeg_color_alpha_exact=True))
    preview.save(out / 'registry-oracle-display.png')
    report = dict(pillow=pillow_version, ffmpeg=run('ffmpeg', '-version').decode().splitlines()[0],
                  cases=results, scope='2x2 still identity RGB, straight alpha; no AVIS or ICC transform')
    (out / 'independent-readers.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
