#!/usr/bin/env python3
"""Check the actual libaom AVIF encode/probe/decode path before installing FFmpeg."""
import shutil
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
                 '-frames:v', '1', '-c:v', 'libaom-av1', '-cpu-used', '8', '-still-picture', '1', sample],
                ['ffprobe', '-v', 'error', '-show_streams', sample],
                ['ffmpeg', '-v', 'error', '-i', sample, '-frames:v', '1', '-f', 'null', '-'],
            ]
            for command in commands:
                subprocess.run(command, check=True, capture_output=True, timeout=30)
        return True
    except (OSError, subprocess.SubprocessError):
        return False


if __name__ == '__main__':
    raise SystemExit(0 if usable() else 1)
