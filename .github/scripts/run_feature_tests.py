#!/usr/bin/env python3
"""Same feature contract on MSRV/stable, shared by core and AVIF matrix jobs."""
import os
from pathlib import Path
import subprocess


def run(*args, stdout=None):
    subprocess.run(['cargo', *args], check=True, stdout=stdout)


def main():
    features = os.environ['FEATURES']
    profile = os.environ['TEST_PROFILE']
    if profile not in {'test', 'ci'}:
        raise ValueError('unexpected test profile')
    args = [] if features == 'default' else ['--no-default-features']
    if features not in {'default', 'minimal'}:
        args += ['--features', features]
    run('test', '-p', 'wml2', '--locked', '--profile', profile, '--lib', '--tests', *args)
    with Path('clippy.json').open('w', encoding='utf-8') as stream:
        run('clippy', '-p', 'wml2', '--locked', '--profile', profile, '--lib', '--tests',
            *args, '--message-format=json', stdout=stream)
    if features == 'avifenc,image-buffer-ex,high-bit-depth':
        run('run', '-p', 'wml2', '--locked', *args, '--example', 'ex_avif')
    if features == 'default':
        for codec in ['gif', 'bmp']:
            for suffix in ['', ',image-buffer-ex']:
                extra = (['--test', 'gif_callback_abort', '--test', 'gif_icc'] if codec == 'gif'
                         else ['--lib', '--test', 'bmp_icc'])
                run('test', '-p', 'wml2', '--locked', '--no-default-features', '--features',
                    codec + suffix, '--test', 'callback_abort', *extra)
        run('test', '-p', 'wml2', '--locked', '--doc')
        run('check', '-p', 'wml2', '--locked', '--examples')
        run('check', '-p', 'wml2-test', '--locked', '--examples')


if __name__ == '__main__':
    main()
