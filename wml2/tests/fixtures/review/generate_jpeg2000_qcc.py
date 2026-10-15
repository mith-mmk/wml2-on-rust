"""Create active-packet QCC overrides and check with OpenJPEG.

Run at repository root with Python/Pillow. Production sources are unchanged.
The default QCD is 40 40 (Mb=9). QCC 40 48 raises one component to Mb=10.
Adding a zero to that packet's unary zero-plane tag preserves coded bit depth.
The source is one 1x1 block per component, no RCT, one resolution and layer.
"""
from pathlib import Path
import hashlib
from PIL import Image, features, __version__

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
manifest = [f'Pillow {__version__} OpenJPEG {features.version_codec("jpg_2000")}']


class Bits:
    def __init__(self, data):
        self.data, self.pos, self.left, self.last, self.bits = data, 0, 0, 0, []

    def bit(self):
        if self.left == 0:
            byte = self.data[self.pos]
            self.pos += 1
            self.left = 7 if self.last == 255 else 8
            assert self.last != 255 or byte < 128
            self.last = byte
        self.left -= 1
        value = (self.last >> self.left) & 1
        self.bits.append(value)
        return value

    def number(self, n):
        result = 0
        for _ in range(n):
            result = (result << 1) | self.bit()
        return result

    def passes(self):
        if not self.bit():
            return 1
        if not self.bit():
            return 2
        value = self.number(2)
        if value != 3:
            return 3 + value
        value = self.number(5)
        return 6 + value if value != 31 else 37 + self.number(7)

    def align(self):
        self.left = 0
        if self.last == 255:
            assert self.data[self.pos] < 128
            self.pos += 1


def pack(bits):
    output, position, last = bytearray(), 0, 0
    while position < len(bits):
        width, byte = (7 if last == 255 else 8), 0
        for i in range(width):
            byte = (byte << 1) | (bits[position + i] if position + i < len(bits) else 0)
        position += width
        output.append(byte)
        last = byte
    if last == 255:
        output.append(0)
    return bytes(output)


source = (ROOT / 'wml2/tests/fixtures/review/jp2_rgb_1x1_d0_noise_mct0.j2k').read_bytes()
sot = source.index(bytes.fromhex('ff90'))
payload, packets, position = source[sot + 14:-2], [], 0
for component in range(3):
    bits = Bits(payload[position:])
    assert bits.bit() == bits.bit() == 1  # packet present; single-block inclusion
    while not bits.bit():
        pass  # unary zero-plane tag
    passes, lblock = bits.passes(), 3
    while bits.bit():
        lblock += 1
    length = bits.number(lblock + passes.bit_length() - 1)
    bits.align()
    body = payload[position + bits.pos:position + bits.pos + length]
    assert len(body) == length
    position += bits.pos + length
    packets.append((bits.bits, body))
assert position == len(payload)

for component in range(3):
    rebuilt = []
    for i, (bits, body) in enumerate(packets):
        updated = bits[:]
        if i == component:
            updated.insert(2, 0)
        rebuilt.append(pack(updated) + body)
    tile = b''.join(rebuilt)
    qcc = bytes.fromhex('ff5d0005') + bytes([component, 0x40, 0x48])
    result = bytearray(source[:sot] + qcc + source[sot:sot + 14] + tile + bytes.fromhex('ffd9'))
    updated_sot = sot + len(qcc)
    result[updated_sot + 6:updated_sot + 10] = (14 + len(tile)).to_bytes(4, 'big')
    path = OUT / f'jp2_qcc_1x1_d0_c{component}.j2k'
    path.write_bytes(result)
    with Image.open(path) as oracle:
        assert oracle.size == (1, 1) and oracle.tobytes() == bytes([31, 83, 197])
    raw = path.with_suffix('.rgb')
    raw.write_bytes(bytes([31, 83, 197]))
    for file in [path, raw]:
        manifest.append(f'{hashlib.sha256(file.read_bytes()).hexdigest()} {file.name}')
    print(f'{path.name}: OpenJPEG exact RGB 31,83,197')
(OUT / 'jpeg2000-qcc-manifest.txt').write_text('\n'.join(manifest) + '\n')
