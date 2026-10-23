"""Compare before/after release binaries in ABBA order; see docs/jpeg-tuning.md."""
import argparse
import csv
import json
import os
from pathlib import Path
import statistics
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path, help="inputs and before/after-single/multi.exe binaries")
parser.add_argument("--repeats", type=int, default=5)
parser.add_argument("--warmups", type=int, default=2)
args = parser.parse_args()
p = args.directory.resolve()
files = sorted(p.glob("*.jpg")) + sorted(Path(__file__).parent.glob("jpeg-icc-*.jpg"))
rows, failures = [], []
for file in files:
    for feature in ("single", "multi"):
        for run, revision in enumerate(("before", "after", "after", "before")):
            mode = f"{revision}-{feature}"
            dump = p / f".test-rgba-{mode}"
            dump.mkdir(exist_ok=True)
            executable = p / (mode + (".exe" if os.name == "nt" else ""))
            result = subprocess.run(
                [str(executable), str(args.repeats), str(args.warmups), str(file)],
                env={**os.environ, "JPEG_BENCH_RGBA_DIR": str(dump)},
                capture_output=True, text=True, timeout=120,
            )
            if result.returncode:
                failures.append({"mode": mode, "run": run, "file": file.name, "error": result.stderr})
                if file.name != "jpeg-icc-cmyk-progressive.jpg":
                    raise RuntimeError(result.stderr)
                continue
            rows.extend({"mode": mode, "run": run, **row}
                        for row in csv.DictReader(result.stdout.splitlines()))
        print(file.name, feature, "complete", flush=True)
for file in files:
    paths = [p / f".test-rgba-{mode}" / f"{file.name}.rgba"
             for mode in ("before-single", "after-single", "before-multi", "after-multi")]
    if all(path.exists() for path in paths):
        expected = paths[0].read_bytes()
        assert all(path.read_bytes() == expected for path in paths), file.name
    else:
        assert file.name == "jpeg-icc-cmyk-progressive.jpg", file.name
with (p / "timings.csv").open("w", newline="") as output:
    writer = csv.DictWriter(output, fieldnames=list(rows[0]))
    writer.writeheader()
    writer.writerows(rows)
summary = []
for file in sorted({row["file"] for row in rows}):
    result = {"file": file}
    for mode in ("before-single", "after-single", "before-multi", "after-multi"):
        result[mode] = statistics.median(int(row["decode_ns"]) / 1e6
                                        for row in rows if row["file"] == file and row["mode"] == mode)
    summary.append(result)
(p / "summary.json").write_text(json.dumps(summary, indent=2))
(p / "failures.json").write_text(json.dumps(failures, indent=2))
print(json.dumps(summary, indent=2))
