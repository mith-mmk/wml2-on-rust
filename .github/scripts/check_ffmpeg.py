#!/usr/bin/env python3
"""Check the actual libaom AVIF encode/probe/decode path before installing FFmpeg."""
import shutil
import json
import subprocess
import tempfile
from pathlib import Path


def usable():
    if not all(shutil.which(tool) for tool in ['ffmpeg', 'ffprobe']):
        return False
    try:
        with tempfile.TemporaryDirectory(prefix='.test-ffmpeg-', dir='.') as directory:
            sample = str(Path(directory) / 'probe.avif')
            commands = [
                ['ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=black:s=16x16',
                 '-frames:v', '1', '-vf', 'format=gbrp', '-c:v', 'libaom-av1',
                 '-cpu-used', '8', '-still-picture', '1', '-usage', 'allintra', '-crf', '0',
                 '-pix_fmt', 'gbrp', '-enable-cdef', '0', '-enable-restoration', '0',
                 '-enable-palette', '1', '-enable-intrabc', '0', '-enable-filter-intra', '0',
                 '-enable-angle-delta', '0', '-enable-cfl-intra', '0', '-enable-rect-partitions', '0',
                 '-enable-1to4-partitions', '0', '-enable-ab-partitions', '0', '-enable-flip-idtx', '0',
                 '-use-intra-default-tx-only', '1', '-color_range', 'pc', '-colorspace', 'rgb', sample],
                ['ffprobe', '-v', 'error', '-select_streams', 'v:0', '-show_entries',
                 'stream=width,height,pix_fmt,codec_name', '-of', 'json', sample],
            ]
            for command in commands:
                completed = subprocess.run(command, check=True, capture_output=True, timeout=30)
            streams = json.loads(completed.stdout)['streams']
            if len(streams) != 1 or any(streams[0].get(k) != v for k, v in
                                       {'width': 16, 'height': 16, 'pix_fmt': 'gbrp', 'codec_name': 'av1'}.items()):
                return False
            for pixel_format, length in [('gbrp', 16 * 16 * 3), ('rgba', 16 * 16 * 4)]:
                completed = subprocess.run(
                    ['ffmpeg', '-v', 'error', '-i', sample, '-frames:v', '1',
                     '-f', 'rawvideo', '-pix_fmt', pixel_format, '-'],
                    check=True, capture_output=True, timeout=30)
                if len(completed.stdout) != length:
                    return False
        return True
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, TypeError):
        return False


if __name__ == '__main__':
    raise SystemExit(0 if usable() else 1)
