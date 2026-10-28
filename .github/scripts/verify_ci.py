#!/usr/bin/env python3
"""A stable required-check name; only explicitly unselected jobs may skip."""
import json
import os

ALWAYS = {'select', 'release-metadata', 'test', 'platform', 'feature-contracts', 'compile', 'format'}
CONDITIONAL = {'jpeg-xl-platform': 'jpegxl', 'tiff-oracle': 'tiff'}
CONDITIONAL.update({'avif-oracles': 'avif', 'avif-test': 'avif', 'codec-smoke': 'smoke'})


def verify(needs):
    if set(needs) != ALWAYS | set(CONDITIONAL):
        raise ValueError('missing or unexpected prerequisite')
    for job in ALWAYS:
        if needs[job]['result'] != 'success':
            raise ValueError(f'{job}: {needs[job]["result"]}')
    outputs = needs['select']['outputs']
    modes = json.loads(outputs.get('modes', '{}'))
    if set(modes) != {'avif', 'jpeg2000', 'jpeg-xl'} or any(v not in {'none', 'smoke', 'full'} for v in modes.values()):
        raise ValueError('missing or invalid codec modes')
    for codec, flag in [('avif', 'avif'), ('jpeg2000', 'jpeg2000'), ('jpeg-xl', 'jpegxl')]:
        if outputs.get(flag) != str(modes[codec] == 'full').lower():
            raise ValueError(f'inconsistent full selection for {codec}')
    if outputs.get('smoke') != str('smoke' in modes.values()).lower():
        raise ValueError('inconsistent smoke selection')
    for job, flag in CONDITIONAL.items():
        if outputs.get(flag) not in {'true', 'false'}:
            raise ValueError(f'missing selection flag: {flag}')
        expected = 'success' if outputs[flag] == 'true' else 'skipped'
        if needs[job]['result'] != expected:
            raise ValueError(f'{job}: expected {expected}, got {needs[job]["result"]}')


if __name__ == '__main__':
    verify(json.loads(os.environ['CI_NEEDS']))
    print('All selected checks succeeded; only unselected checks were skipped.')
