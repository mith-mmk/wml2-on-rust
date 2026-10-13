#!/usr/bin/env python3
"""Independent classic TIFF fixture for EX RGBA16 access; no WML2 encoder."""
import argparse
import struct
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    width, height = 65, 33
    values = [v for y in range(height) for x in range(width)
              for v in ((x * 997 + y * 137 + 1) % 65536,
                        (x * 337 + y * 2003 + 129) % 65536,
                        (x * 503 + y * 719 + 32768) % 65536, 65535)]
    raw = struct.pack('<' + 'H' * len(values), *values)
    # Eleven sorted IFD entries at8; BitsPerSample array at146; strip at154.
    entries = [(256, 4, 1, width), (257, 4, 1, height), (258, 3, 4, 146),
               (259, 3, 1, 1), (262, 3, 1, 2), (273, 4, 1, 154),
               (277, 3, 1, 4), (278, 4, 1, height), (279, 4, 1, len(raw)),
               (284, 3, 1, 1), (338, 3, 1, 2)]
    data = (b'II' + struct.pack('<HIH', 42, 8, len(entries))
            + b''.join(struct.pack('<HHII', *entry) for entry in entries)
            + struct.pack('<I4H', 0, 16, 16, 16, 16) + raw)
    args.output.write_bytes(data)


if __name__ == '__main__':
    main()
