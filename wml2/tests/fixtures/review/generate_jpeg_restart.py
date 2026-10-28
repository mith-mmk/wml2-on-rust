"""Public restart regression fixtures; Pillow 11.1.0, no private inputs."""
from pathlib import Path
from PIL import Image

root = Path(__file__).parent
for width, height, prefix, sub, progressive in [
    (64, 32, "minimal", 0, False),
    (128, 48, "b444", 0, False), (128, 48, "b420", 2, False),
    (129, 49, "odd420", 2, False),
    (128, 48, "p444", 0, True), (128, 48, "p420", 2, True),
    (65, 33, "bgray", 0, False), (65, 33, "pgray", 0, True),
]:
    rgb = bytes(value for y in range(height) for x in range(width)
                for value in ((x*7+y*3)%256, (x*13+y*11)%256, (x*31+y*17)%256))
    image = Image.frombytes("RGB", (width, height), rgb)
    if prefix.endswith("gray"):
        image = image.convert("L")
    for interval in (0, 1, 2, 3, 8, 100):
        image.save(root / f"restart-{prefix}-i{interval}.jpg", quality=90,
                   subsampling=sub, progressive=progressive, restart_marker_blocks=interval)
    image.save(root / f"restart-{prefix}-row.jpg", quality=90,
               subsampling=sub, progressive=progressive, restart_marker_rows=1)
