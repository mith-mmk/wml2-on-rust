#!/usr/bin/env python3
"""Reproduce frozen resource-boundary inputs with independent C libjxl 0.11.2.

Writes only to the explicit .test directory and verifies the committed manifest.
Needs matching libjxl headers and the installed libjxl.so.0.11 runtime.
"""
import argparse
import ctypes
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--include',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if not args.output.name.startswith('.test'):parser.error('--output must start with .test')
    root=Path(__file__).resolve().parents[2];out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    manifest=json.loads((root/'wml2/tests/fixtures/review/jxl-resource-fixtures.json').read_text())
    source=Path(__file__).with_name('jpegxl_fixture_encoder.c')
    assert hashlib.sha256(source.read_bytes()).hexdigest()==manifest['encoder_source_sha256']
    assert ctypes.CDLL('libjxl.so.0.11').JxlEncoderVersion()==manifest['encoder_version']
    encoder=out/'encoder'
    subprocess.run(['cc','-O2','-Wall','-Wextra','-Werror','-I'+str(args.include.resolve()),str(source),'-Wl,-l:libjxl.so.0.11','-o',str(encoder)],check=True)
    for case in manifest['fixtures']:
        raw=out/(case['name']+'.raw');output=out/case['file']
        raw.write_bytes(bytes(case['expected_rgba'][:3])*(case['width']*case['height']))
        subprocess.run([str(encoder),str(raw),str(output),str(case['width']),str(case['height']),'3','8','lossless','-'],check=True)
        assert output.stat().st_size==case['bytes']
        assert hashlib.sha256(output.read_bytes()).hexdigest()==case['sha256']
        print(case['file'],case['sha256'])


if __name__=='__main__':main()
