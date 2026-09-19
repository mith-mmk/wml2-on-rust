"""Check the sample converter against ImageMagick, using Pillow to inspect tags.

Outputs stay in the explicit --output-dir. JSON distinguishes unsupported
inputs, executable failures and pixel differences. No corpus files are edited.
"""
import argparse
import json
from pathlib import Path
import subprocess
import warnings
from PIL import Image


def run(args):
    return subprocess.run(args, capture_output=True, timeout=30)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--converter", required=True)
    parser.add_argument("--metadata", required=True)
    parser.add_argument("--magick", default="magick")
    parser.add_argument("--corpus", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--manifest")
    parser.add_argument("--names", nargs="*")
    args = parser.parse_args()
    manifest_samples = {}
    if args.manifest:
        manifest = json.loads(Path(args.manifest).read_text(encoding="utf-8"))
        manifest_samples = {sample["name"]: sample for sample in manifest.get("samples", [])}
    output = Path(args.output_dir)
    output.mkdir(parents=True, exist_ok=True)
    results, skipped = [], []
    for source in sorted(Path(args.corpus).iterdir()):
        if source.suffix.lower() not in (".tif", ".tiff"):
            continue
        if args.names and source.name not in args.names:
            continue
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                with Image.open(source) as image:
                    tags = image.tag_v2
                    bits = tags.get(258, (1,))
                    sf = tags.get(339, (1,))
                    if isinstance(bits, int):
                        bits = (bits,)
                    if isinstance(sf, int):
                        sf = (sf,)
                    compression, photo = tags.get(259, 1), tags.get(262)
                    info = dict(compression=compression, photo=photo, bits=bits,
                                planar=tags.get(284, 1), alpha=tags.get(338), size=image.size,
                                subsampling=tags.get(530), positioning=tags.get(531, 1))
                    supported = (compression in (1, 2, 3, 4, 5, 6, 7, 8, 32946, 32773)
                                 and all(b in (1, 2, 4, 8, 16, 32) for b in bits)
                                 and all(v == 1 for v in sf) and photo in (0, 1, 2, 3, 5, 6))
                    if not supported and not args.names:
                        skipped.append(dict(name=source.name, reason="outside integer TIFF scope", **info))
                        continue
        except Exception as error:
            if not args.names:
                skipped.append(dict(name=source.name, reason="Pillow cannot inspect tags: " + str(error)))
                continue
            info = {}
        case_dir = output / source.name
        case_dir.mkdir(exist_ok=True)
        policy = manifest_samples.get(source.name, {}).get("oracle_policy", "compare")
        result = dict(name=source.name, oracle_policy=policy, **info)
        try:
            converted = run([args.converter, str(source), "-o", str(case_dir), "-f", "png"])
            result["converter_exit"] = converted.returncode
            if converted.returncode:
                result["error"] = converted.stderr.decode(errors="replace")[-1800:]
            metadata = run([args.metadata, str(source)])
            result["metadata_exit"] = metadata.returncode
            if metadata.returncode:
                result["metadata_error"] = metadata.stderr.decode(errors="replace")[-1000:]
            else:
                result["metadata_summary"] = [line for line in metadata.stdout.decode(errors="replace").splitlines()
                    if line.startswith(("TIFF Ver", "image pages:", "compression:", "width:", "height:"))]
            if converted.returncode == 0:
                outputs = list(case_dir.glob("*.png"))
                if len(outputs) != 1:
                    result["error"] = "Expected one PNG result; multipage needs dedicated assertions"
                else:
                    def pixels(path):
                        return run([args.magick, str(path) + "[0]", "-colorspace", "sRGB",
                                    "-alpha", "on", "-depth", "8", "rgba:-"])
                    actual = pixels(outputs[0])
                    if policy == "expected_only":
                        result["oracle"] = "diagnostic-only; WML2 expected RGBA is authoritative"
                    else:
                        if info.get("compression") in (6, 7):
                            # LibTIFF's RGBA path can misinterpret old/new JPEG strips.
                            # Pillow uses libjpeg; chroma upsampling may still differ from WML2.
                            with Image.open(source) as image:
                                reference = subprocess.CompletedProcess([], 0, image.convert("RGBA").tobytes(), b"")
                            result["oracle"] = "Pillow/libjpeg (upsampling differences allowed)"
                        else:
                            reference = pixels(source)
                            result["oracle"] = "ImageMagick"
                        result["oracle_exit"] = reference.returncode
                        if reference.returncode or actual.returncode:
                            result["error"] = (reference.stderr + actual.stderr).decode(errors="replace")[-1200:]
                        elif len(reference.stdout) != len(actual.stdout):
                            result["error"] = "RGBA output lengths differ"
                            result["lengths"] = [len(reference.stdout), len(actual.stdout)]
                        else:
                            maximum = total = different = 0
                            for a, b in zip(reference.stdout, actual.stdout):
                                delta = abs(a - b)
                                maximum = max(maximum, delta)
                                total += delta
                                different += delta != 0
                            result["max_abs_error"] = maximum
                            result["mean_abs_error"] = total / max(1, len(actual.stdout))
                            result["different_channels"] = different
                            result["channels"] = len(actual.stdout)
        except subprocess.TimeoutExpired:
            result["error"] = "30 second executable timeout"
        results.append(result)
    print(json.dumps(dict(results=results, skipped=skipped), indent=2))


if __name__ == "__main__":
    main()
