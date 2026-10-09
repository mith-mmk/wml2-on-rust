"""Compare local WML2 RGBA dumps with independently produced oracle PNGs.

Does not execute or download an oracle, create PNGs, or declare correctness
from smoke-test success. Pillow is required only for reading oracle PNGs.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input-dir', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--dump', type=Path, required=True)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--oracle-dir', type=Path)
    parser.add_argument('--oracle-binary', type=Path)
    parser.add_argument('--oracle-version')
    args = parser.parse_args()
    inputs = sorted(p for p in args.input_dir.iterdir() if p.is_file() and p.suffix.lower() == '.q4')
    if not inputs:
        parser.error('no Q4 inputs; comparison is unverified')
    args.output_dir.mkdir(parents=True, exist_ok=True)
    samples = []
    report = {
        'head': subprocess.check_output(['git', '-C', str(args.repo), 'rev-parse', 'HEAD'], text=True).strip(),
        'working_tree_status': subprocess.check_output(['git', '-C', str(args.repo), 'status', '--porcelain'], text=True),
        'dump_sha256': sha(args.dump),
        'oracle_binary_sha256': sha(args.oracle_binary) if args.oracle_binary else None,
        'oracle_version': args.oracle_version,
        'comparison': 'untransformed RGB, exact dimensions; no resize, interpolation, or ICC conversion',
        'samples': samples,
        'execution_status': 'incomplete',
        'compatibility_pass': False,
    }
    # Never leave an earlier successful report looking like this failed run.
    report_path = args.output_dir / 'report.json'
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    for source in inputs:
        row = {'input': source.name, 'input_sha256': sha(source)}
        samples.append(row)
        raw = args.output_dir / (source.name + '.rgba')
        process = subprocess.run([str(args.dump.resolve()), str(source), str(raw)], capture_output=True, text=True)
        if process.returncode:
            row.update(status='wml_decode_failed', error=process.stderr.strip())
            continue
        rgba = raw.read_bytes()
        if len(rgba) != 640 * 400 * 4:
            raise ValueError(f'{source.name}: invalid RGBA storage length')
        rgb = bytes(value for pixel in range(0, len(rgba), 4) for value in rgba[pixel:pixel + 3])
        row.update(rgba_sha256=sha(raw), rgb_sha256=hashlib.sha256(rgb).hexdigest())
        oracle = args.oracle_dir / (source.stem + '.png') if args.oracle_dir else None
        if not oracle or not oracle.is_file():
            row['status'] = 'oracle_missing'
            continue
        from PIL import Image
        with Image.open(oracle) as image:
            if image.size != (640, 400):
                raise ValueError(f'{oracle}: oracle dimensions differ')
            expected = image.convert('RGB').tobytes()
        count = maximum = 0
        first = None
        for offset in range(0, len(rgb), 3):
            actual_pixel, expected_pixel = rgb[offset:offset + 3], expected[offset:offset + 3]
            if actual_pixel != expected_pixel:
                count += 1
                maximum = max(maximum, *(abs(a - b) for a, b in zip(actual_pixel, expected_pixel)))
                if first is None:
                    pixel = offset // 3
                    first = {'x': pixel % 640, 'y': pixel // 640,
                             'actual_rgb': list(actual_pixel), 'oracle_rgb': list(expected_pixel)}
        row.update(status='different' if count else 'equal', differing_pixels=count,
                   maximum_channel_difference=maximum, first_difference=first,
                   oracle_png_sha256=sha(oracle), oracle_rgb_sha256=hashlib.sha256(expected).hexdigest())
    report['compatibility_pass'] = bool(args.oracle_dir and args.oracle_binary and args.oracle_version
                                        and all(row['status'] == 'equal' for row in samples))
    report['execution_status'] = 'completed'
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'count': len(samples), 'compatibility_pass': report['compatibility_pass']}))


if __name__ == '__main__':
    main()
