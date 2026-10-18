#!/usr/bin/env python3
"""Check WML2's explicit JPEG IDCT conflict and broad compatible features.

`--all-features` is intentionally invalid because it enables three JPEG IDCTs.
Compatible checks cover every decoder and, separately, the noretoro disable flag.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not args.output.name.startswith('.test'):
        parser.error('--output directory must start with .test')
    root = Path(__file__).resolve().parents[2]
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    cargo = ['cargo']
    metadata = json.loads(subprocess.check_output(
        cargo + ['metadata', '--locked', '--format-version=1', '--no-deps'],
        cwd=root, text=True))
    package = next(p for p in metadata['packages'] if p['name'] == 'wml2')
    names = sorted(set(package['features']) - {'idct_aan', 'idct_slower'})
    required = {'jpeg-xl', 'image-buffer-ex', 'idct_llm'}
    if not required.issubset(names):
        raise RuntimeError('compatible matrix must include JPEG XL, EX and one IDCT')
    common = ['-p', 'wml2', '--locked']
    conflict = subprocess.run(cargo + ['check', *common, '--lib', '--all-features'],
                              cwd=root, capture_output=True, text=True)
    conflict_log = conflict.stdout + conflict.stderr
    (output / 'all-features.log').write_text(conflict_log)
    expected = 'Choose only one JPEG IDCT feature'
    if conflict.returncode == 0 or expected not in conflict_log:
        raise RuntimeError('all-features did not produce the expected IDCT conflict; '
                           f'see {output / "all-features.log"}')
    # noretoro is a disabling feature; including it alone would hide every
    # retro codec even when their individual format features are enabled.
    profiles = {'all-codecs': [n for n in names if n != 'noretoro'],
                'noretoro': names}
    for profile, features in profiles.items():
        checks = [
            ['check', *common, '--all-targets', '--no-default-features', '--features', ','.join(features)],
            ['test', *common, '--test', 'jpeg_xl', '--no-default-features', '--features', ','.join(features)],
        ]
        for index, command in enumerate(checks):
            result = subprocess.run(cargo + command, cwd=root, capture_output=True, text=True)
            (output / f'{profile}-{index}.log').write_text(result.stdout + result.stderr)
            result.check_returncode()
    report = {
        'expected_all_features_conflict': expected,
        'compatible_profiles': profiles,
        'all_targets_check': 'PASS',
        'jpeg_xl_integration_tests': 'PASS',
        'target': os.environ.get('CARGO_BUILD_TARGET', 'host'),
    }
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
