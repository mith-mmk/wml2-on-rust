#!/usr/bin/env python3
"""A stable required-check name; only explicitly unselected jobs may skip."""
import json
import os

ALWAYS = {'select', 'release-metadata', 'test', 'platform', 'feature-contracts', 'compile', 'format'}
CONDITIONAL = {'jpeg-xl-platform': 'jpegxl', 'tiff-oracle': 'tiff'}


def verify(needs):
    if set(needs) != ALWAYS | set(CONDITIONAL):
        raise ValueError('missing or unexpected prerequisite')
    for job in ALWAYS:
        if needs[job]['result'] != 'success':
            raise ValueError(f'{job}: {needs[job]["result"]}')
    outputs = needs['select']['outputs']
    for job, flag in CONDITIONAL.items():
        if outputs.get(flag) not in {'true', 'false'}:
            raise ValueError(f'missing selection flag: {flag}')
        expected = 'success' if outputs[flag] == 'true' else 'skipped'
        if needs[job]['result'] != expected:
            raise ValueError(f'{job}: expected {expected}, got {needs[job]["result"]}')


if __name__ == '__main__':
    verify(json.loads(os.environ['CI_NEEDS']))
    print('All selected checks succeeded; only unselected checks were skipped.')
