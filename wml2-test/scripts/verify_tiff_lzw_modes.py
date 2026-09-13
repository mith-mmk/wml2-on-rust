#!/usr/bin/env python3
"""Verify TIFF LZW code order independently from FillOrder.

The two original interoperability cases (standard MSB and historical
LibTIFF LSB, both FillOrder=1) require an ImageMagick/LibTIFF pixel match.
Additional tag-independence cases always require WML2's expected pixels, but
record external reader results without making reader-version-specific
FillOrder behavior a portability requirement.
"""
import argparse
import json
import subprocess
from pathlib import Path
from PIL import Image


def run(args):
    return subprocess.run(args, capture_output=True, timeout=30)


def rgba(path):
    with Image.open(path) as image:
        return list(image.convert("RGBA").tobytes())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--samples", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--converter", required=True)
    parser.add_argument("--metadata", required=True)
    parser.add_argument("--magick", required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((args.samples / "manifest.json").read_text(encoding="utf-8"))
    results = []
    for name, spec in manifest["samples"].items():
        source = args.samples / name
        case = args.output / Path(name).stem
        case.mkdir()
        converted = run([args.converter, str(source), "-o", str(case), "-f", "png"])
        metadata = run([args.metadata, str(source)])
        policy = spec.get("oracle_policy", "required_match")
        row = {"name": name, "mode": spec["lzw_mode"], "oracle_policy": policy,
               "converter_exit": converted.returncode, "metadata_exit": metadata.returncode}
        metadata_lines = metadata.stdout.decode(errors="replace").splitlines()
        row["metadata_shape"] = {
            "width": next((line.split(":", 1)[1].strip() for line in metadata_lines
                           if line.startswith("width:")), None),
            "height": next((line.split(":", 1)[1].strip() for line in metadata_lines
                            if line.startswith("height:")), None),
            "pages": next((line.split(":", 1)[1].strip() for line in metadata_lines
                           if line.startswith("image pages:")), None),
        }
        metadata_ok = row["metadata_shape"] == {"width": str(spec["width"]),
                                                  "height": str(spec["height"]), "pages": "1"}
        outputs = list(case.glob("*.png"))
        actual = rgba(outputs[0]) if len(outputs) == 1 else None
        wml2_ok = (converted.returncode == 0 and metadata.returncode == 0
                   and metadata_ok and actual == spec["expected_rgba"])
        reference = run([args.magick, str(source), "-alpha", "on", "-depth", "8", "rgba:-"])
        row["oracle_exit"] = reference.returncode
        row["oracle_matches"] = (reference.returncode == 0 and actual is not None
                                  and list(reference.stdout) == actual)
        if policy == "required_match":
            row["ok"] = wml2_ok and row["oracle_matches"]
            if not row["ok"]:
                row["error"] = "required WML2/ImageMagick LZW pixel match failed"
        elif policy == "observe":
            row["ok"] = wml2_ok
            if not row["ok"]:
                row["error"] = "WML2 expected pixels failed for tag-independence case"
        elif policy == "wml2_custom":
            # A future external reader may learn this private stream.  Until
            # then rejection is expected; if it accepts, it must match WML2.
            row["ok"] = wml2_ok and (reference.returncode != 0 or row["oracle_matches"])
            if not row["ok"]:
                row["error"] = "WML2 custom LSB stream or external agreement failed"
        else:
            row["ok"] = False
            row["error"] = f"unknown oracle policy: {policy}"
        results.append(row)
    report = {"cases": len(results), "passed": sum(row["ok"] for row in results), "results": results}
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report["passed"] == report["cases"] else 1)


if __name__ == "__main__":
    main()
