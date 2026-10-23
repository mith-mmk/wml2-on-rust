#!/usr/bin/env python3
"""Bounded optional-codec integration checks, using the existing oracle tests.

An exact filter must actually run one test: cargo accepts misspelled filters and
otherwise returns success with zero tests. Full runs retain every original test.
"""
import argparse
import json
import subprocess


def commands(modes):
    common = ['cargo', 'test', '-p', 'wml2', '--locked', '--profile', 'ci', '--no-default-features']
    result = []
    if modes.get('jpeg2000') == 'smoke':
        for target, name in [
            ('jpeg2000', 'detection'),
            ('jpeg2000', 'enabled::limits'),
            ('jpeg2000', 'enabled::unsupported'),
            ('jpeg2000', 'enabled::callbacks'),
            ('jpeg2000_icc', 'ordinary_rgb_gray_and_general_alpha_retain_selected_icc_without_changing_pixels'),
            ('jpeg2000_ex', 'exact_round_and_legacy_callback_contracts_are_explicit'),
            ('jpeg2000_ex', 'callback_abort_errors_and_workspace_thresholds_are_transactional'),
        ]:
            result.append(common + ['--features', 'jpeg2000,image-buffer-ex', '--test', target, name, '--', '--exact'])
    if modes.get('avif') == 'smoke':
        result.append(common + ['--features', 'avif,image-buffer-ex,color-management', '--test', 'avif_color_boundary'])
    if modes.get('jpeg-xl') == 'smoke':
        result.append(common + ['--features', 'jpeg-xl,image-buffer-ex', '--test', 'jpeg_xl'])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--modes', required=True, type=json.loads)
    args = parser.parse_args()
    if set(args.modes) != {'avif', 'jpeg2000', 'jpeg-xl'} or any(v not in {'none', 'smoke', 'full'} for v in args.modes.values()):
        parser.error('invalid codec selection')
    selected = commands(args.modes)
    if not selected:
        parser.error('no smoke cases selected')
    for command in selected:
        print(' '.join(command), flush=True)
        output = subprocess.check_output(command, text=True)
        print(output, flush=True)
        expected = '1 passed;' if '--exact' in command else 'test result: ok.'
        if expected not in output or 'running 0 tests' in output:
            raise RuntimeError('selected smoke test did not execute successfully')


if __name__ == '__main__':
    main()
