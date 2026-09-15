#!/usr/bin/env python3
"""Run converter/metadata against the generated review manifest.

Requires Pillow to inspect generated PNGs and re-encoded TIFFs. Optional
--magick compares source pixels with an independent LibTIFF decoder.
Native U16 values and custom decode limits are covered by the Rust tests;
this runner validates the legacy sample binaries. Outputs go below --output.
Use a fresh output directory so stale converter results cannot pass a check.

Manifest entries use ``oracle_policy=expected_only`` when a reference decoder
has a documented version-specific interpretation difference. The WML2 expected
RGBA check remains mandatory; the external decoder is then recorded as a
diagnostic instead of being treated as a portable TIFF correctness oracle.
"""
import argparse
import json
import subprocess
from pathlib import Path
from PIL import Image


def run(command):
    return subprocess.run(command, capture_output=True, timeout=30)


def check_encoder(args, source, spec, directory):
    results = []
    for big in (False, True):
        for compression, tag in (("none", 1), ("lzw", 5), ("deflate", 8)):
            case = directory / f"encode_{'big' if big else 'classic'}_{compression}"
            case.mkdir()
            result = dict(bigtiff=big, compression=compression)
            try:
                command = [args.converter, str(source), "-o", str(case), "-f", "tiff",
                           "-c", compression, "--exif", "copy"]
                if big:
                    command.append("--bigtiff")
                converted = run(command)
                if converted.returncode:
                    raise ValueError(converted.stderr.decode(errors="replace"))
                output, = case.glob("*.tiff")
                expected_header = b"II+\0" if big else b"II*\0"
                if output.read_bytes()[:4] != expected_header:
                    raise ValueError("output TIFF variant differs")
                with Image.open(output) as image:
                    sample_format = image.tag_v2.get(339, (1,))
                    if isinstance(sample_format, int):
                        sample_format = (sample_format,)
                    if (image.tag_v2.get(277) != 3 or len(sample_format) not in (1, 3)
                            or any(value != 1 for value in sample_format)
                            or image.tag_v2.get(259) != tag or image.n_frames != 1):
                        raise ValueError("re-encoded TIFF tags differ from RGB contract")
                    if (image.size != (spec["width"], spec["height"])
                            or list(image.convert("RGBA").tobytes()) != spec["expected_rgba"]):
                        raise ValueError("Pillow re-encoded pixels differ")
                metadata = run([args.metadata, str(output)])
                if metadata.returncode:
                    raise ValueError(metadata.stderr.decode(errors="replace"))
                result.update(ok=True, file=str(output.relative_to(args.output)))
            except (ValueError, OSError, subprocess.TimeoutExpired) as error:
                result.update(ok=False, error=str(error))
            results.append(result)
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--samples", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--converter", required=True)
    parser.add_argument("--metadata", required=True)
    parser.add_argument("--magick", help="optional ImageMagick executable for source pixel comparison")
    parser.add_argument("--force-oracle", action="store_true",
                        help="run the external comparison even for expected_only entries")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((args.samples / "manifest.json").read_text(encoding="utf-8"))
    results = []
    for name, spec in manifest["samples"].items():
        source = (args.samples / name).resolve()
        if not source.is_relative_to(args.samples.resolve()):
            raise ValueError("manifest path leaves sample directory")
        case = args.output / Path(name).stem
        case.mkdir()
        result = {"name": name}
        try:
            converted = run([args.converter, str(source), "-o", str(case), "-f", "png", "--split"])
            metadata = run([args.metadata, str(source)])
            result.update(converter_exit=converted.returncode, metadata_exit=metadata.returncode)
            rejection = spec.get("expect_error", False)
            if rejection:
                result["expected_rejection"] = rejection
                result["ok"] = converted.returncode != 0 and metadata.returncode != 0
            else:
                if converted.returncode or metadata.returncode:
                    raise ValueError((converted.stderr + metadata.stderr).decode(errors="replace"))
                outputs = sorted(case.glob("*.png"))
                count = spec.get("pages", 1)
                if len(outputs) != count:
                    raise ValueError(f"expected {count} output pages, found {len(outputs)}")
                text = metadata.stdout.decode(errors="replace")
                if f"image pages: {count}" not in text.splitlines():
                    raise ValueError("metadata page count differs from manifest")
                for dimension in ("width", "height"):
                    if f"{dimension}: {spec[dimension]}" not in text.splitlines():
                        raise ValueError("metadata dimensions differ from manifest")
                expected = spec.get("expected_pages_rgba")
                if expected is None and "expected_rgba" in spec:
                    expected = [spec["expected_rgba"]]
                actual = []
                for output in outputs:
                    with Image.open(output) as image:
                        if image.size != (spec["width"], spec["height"]):
                            raise ValueError("output dimensions differ from manifest")
                        actual.append(list(image.convert("RGBA").tobytes()))
                if expected is not None and actual != expected:
                    raise ValueError(f"pixel mismatch: expected {expected}, actual {actual}")
                oracle_policy = "required_match" if args.force_oracle else spec.get("oracle_policy", "required_match")
                if args.magick:
                    oracle_matches = []
                    for index, pixels in enumerate(actual):
                        reference = run([args.magick, f"{source}[{index}]", "-alpha", "on",
                                         "-depth", "8", "rgba:-"])
                        matches = reference.returncode == 0 and list(reference.stdout) == pixels
                        oracle_matches.append(matches)
                        if oracle_policy == "required_match" and not matches:
                            raise ValueError("ImageMagick source pixels differ: " +
                                             reference.stderr.decode(errors="replace"))
                    result["oracle_pages_checked"] = len(actual)
                    result["oracle_matches"] = oracle_matches
                if args.magick and oracle_policy != "required_match":
                    result["oracle_policy"] = oracle_policy
                result.update(ok=True, pages=count, pixels_checked=expected is not None, rgba=actual)
                if spec.get("check_encoder"):
                    result["encoder"] = check_encoder(args, source, spec, case)
                    result["ok"] = all(row["ok"] for row in result["encoder"])
        except (ValueError, OSError, subprocess.TimeoutExpired) as error:
            result.update(ok=False, error=str(error))
        results.append(result)
    report = {"cases": len(results), "passed": sum(r["ok"] for r in results), "results": results}
    encoded = [row for result in results for row in result.get("encoder", [])]
    report.update(encoder_cases=len(encoded), encoder_passed=sum(row["ok"] for row in encoded))
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report["passed"] == report["cases"] else 1)


if __name__ == "__main__":
    main()
