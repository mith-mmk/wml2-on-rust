#!/usr/bin/env python3
"""Conservative PR selection. Unknown/shared changes and non-PR events run all checks."""
import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import tomllib

FEATURES = [
    'default', 'minimal', 'jpeg-xl', 'jpeg-xl,image-buffer-ex',
    'image-buffer-ex', 'image-buffer-ex,high-bit-depth',
    'png,image-buffer-ex,high-bit-depth', 'tiff,image-buffer-ex,high-bit-depth',
    'high-bit-depth', 'color-management', 'jpeg2000', 'jpeg2000,image-buffer-ex',
    'avif,image-buffer-ex', 'avifenc,image-buffer-ex,high-bit-depth',
    'avifenc,image-buffer-ex,high-bit-depth,multithread', 'avif,high-bit-depth',
    'avifenc,high-bit-depth', 'avifenc,high-bit-depth,multithread',
    'jpeg,idct_llm', 'jpeg,idct_aan', 'jpeg,idct_slower',
    'jpeg,idct_llm,multithread', 'png,psd,tiff',
]
CODECS = {'avif', 'jpeg2000', 'jpeg-xl'}
LOCAL_CODECS = {'jpeg', 'png', 'tiff'}
AVIF_PACKAGES = {'avif-rust', 'avifenc-rust'}
BASE_FEATURES = {'default', 'minimal', 'image-buffer-ex',
                 'image-buffer-ex,high-bit-depth', 'high-bit-depth', 'color-management'}


def git(*args):
    return subprocess.check_output(['git', *args])


def manifest_avif_only(before, after):
    """Only existing registry AVIF version requirements may change, not features."""
    a, b = copy.deepcopy(before), copy.deepcopy(after)
    changed = False
    for alias, package in [('avif_codec', 'avif-rust'), ('avifenc_codec', 'avifenc-rust')]:
        for manifest in [a, b]:
            dep = manifest['dependencies'][alias]
            if dep.get('package') != package or any(k in dep for k in ['git', 'path', 'registry']):
                return False
            if not isinstance(dep.get('version'), str):
                return False
        changed |= a['dependencies'][alias]['version'] != b['dependencies'][alias]['version']
        a['dependencies'][alias].pop('version')
        b['dependencies'][alias].pop('version')
    return changed and a == b


def lock_avif_only(before, after):
    """Reject changes to shared/transitive dependencies, edges, sources or package sets."""
    def normalized(value):
        value = copy.deepcopy(value)
        seen = set()
        for package in value['package']:
            if package['name'] in AVIF_PACKAGES:
                if package['name'] in seen or package.get('source') != 'registry+https://github.com/rust-lang/crates.io-index':
                    raise ValueError('ambiguous AVIF package source or duplicate')
                seen.add(package['name'])
                package.pop('version')
                package.pop('checksum')
        if seen != AVIF_PACKAGES:
            raise ValueError('missing AVIF package')
        value['package'].sort(key=lambda p: (p['name'], p.get('version', ''), p.get('source', '')))
        return value
    return before != after and normalized(before) == normalized(after)


def affected(paths, read_pair):
    codecs = set()
    for path in paths:
        if path == 'wml2/Cargo.toml':
            if not manifest_avif_only(*read_pair(path)):
                return CODECS, True, 'shared manifest/feature change'
            codecs.add('avif')
        elif path == 'Cargo.lock':
            if not lock_avif_only(*read_pair(path)):
                return CODECS, True, 'shared/transitive dependency change'
            codecs.add('avif')
        elif path.startswith(('wml2/src/avif/', 'wml2/tests/avif_')) or path in {
            'wml2/src/highres/avif.rs', 'wml2/src/highres/avif_encode.rs',
            'wml2/src/highres/avif_encode_bench.rs',
            'wml2/tests/highres_avif_native_bridge.rs', 'wml2/tests/highres_avif_encode.rs',
        }:
            codecs.add('avif')
        elif path == 'wml2/src/jpegxl.rs' or path.startswith('wml2/tests/jpeg_xl'):
            codecs.add('jpeg-xl')
        elif (path.startswith('wml2/src/jpeg2000') and path.endswith('.rs')) or path.startswith('wml2/tests/jpeg2000'):
            codecs.add('jpeg2000')
        elif path.startswith(('wml2/src/jpeg/', 'wml2/tests/jpeg_')):
            # TIFF's embedded JPEG path depends on this decoder too.
            codecs.update({'jpeg', 'tiff'})
        elif path in {'wml2/src/tiff/header.rs', 'wml2/src/tiff/mod.rs'}:
            # TiffHeaders is embedded in the shared DataMap enum. This file also
            # owns public types/reexports, so path-only selection cannot narrow it.
            return CODECS, True, 'shared TIFF metadata types or module wiring'
        elif path.startswith(('wml2/src/tiff/', 'wml2/tests/tiff_')) or path == 'wml2/src/highres/tiff.rs':
            # JPEG/PNG/WebP/JXL EXIF also call the TIFF directory reader.
            codecs.update(LOCAL_CODECS | {'jpeg-xl'})
        elif path == 'wml2/src/metadata/exif.rs':
            # JXL now uses these EXIF helpers too; JP2/AVIF do not.
            codecs.update(LOCAL_CODECS | {'jpeg-xl'})
        elif path.startswith(('wml2/src/png/', 'wml2/tests/png_')):
            codecs.update({'png', 'tiff'})
        elif path.startswith('wml2-test/examples/') and path.endswith('.rs'):
            # Consumer tools cannot alter wml2's public types or decoder bodies.
            # Check their build plus bounded integration cases for optional codecs.
            codecs.update(c + '-smoke' for c in CODECS)
        elif (path.startswith('docs/') and path.endswith(('.md', '.json', '.png', '.json.gz'))) or path in {'README.md', 'README.ja.md'}:
            # Documentation only. Executable scripts, fixtures, submodules and
            # unknown extensions are deliberately NOT classified by their name.
            continue
        else:
            return CODECS, True, 'shared, unknown or executable path: ' + path
    return codecs, False, 'recognized codec/documentation changes'


def enabled_features(features, manifest):
    todo = ['default'] if features == 'default' else ([] if features == 'minimal' else features.split(','))
    seen = set()
    while todo:
        feature = todo.pop()
        if feature in seen:
            continue
        seen.add(feature)
        todo.extend(manifest.get('features', {}).get(feature, []))
    return seen


def uses_avif(features, manifest):
    return bool(enabled_features(features, manifest) & {'avif', 'avifenc', 'dep:avif_codec', 'dep:avifenc_codec'})


def plan(event, base, head, manifest):
    full, codecs, reason = True, CODECS, 'non-PR events always run the full suite'
    if event == 'pull_request':
        try:
            # Merge-base catches changes across multiple commits. No rename
            # detection: both old and new paths must be classified safely.
            ancestor = git('merge-base', base, head).decode().strip()
            paths = git('diff', '--name-only', '--no-renames', '-z', ancestor, head).decode().split('\0')
            paths = [p for p in paths if p]
            if not paths:
                raise ValueError('empty or indeterminate diff')
            def pair(path):
                return tuple(tomllib.loads(git('show', f'{rev}:{path}').decode()) for rev in [ancestor, head])
            codecs, full, reason = affected(paths, pair)
        except (subprocess.CalledProcessError, ValueError, KeyError, TypeError, UnicodeError) as exc:
            full, codecs, reason = True, CODECS, 'selection unavailable; full suite: ' + type(exc).__name__
    selected = [f for f in FEATURES if full or f in BASE_FEATURES or any(c in f.split(',') or (c == 'avif' and 'avifenc' in f.split(',')) for c in codecs)]
    modes = {c: ('full' if full or c in codecs else 'smoke' if c + '-smoke' in codecs else 'none') for c in CODECS}
    # Feature dependency changes are full when introduced; later PRs must also
    # honor that graph (e.g. a default feature newly enabling an optional codec).
    for features in selected:
        for codec in CODECS & enabled_features(features, manifest):
            modes[codec] = 'full'
    matrix = {'include': [{'rust': rust, 'features': f, 'avif': uses_avif(f, manifest),
                           'ffmpeg': bool(enabled_features(f, manifest) & {'avifenc', 'dep:avifenc_codec'}),
                           'profile': 'ci' if 'jpeg2000' in enabled_features(f, manifest) else 'test'}
                          for rust in ['1.91.0', 'stable'] for f in selected]}
    core_matrix = {'include': [row for row in matrix['include'] if not row['avif']]}
    avif_matrix = {'include': [row for row in matrix['include'] if row['avif']]}
    if avif_matrix['include']:
        modes['avif'] = 'full'
    # Job-level `if` handles the unselected case; keep a valid matrix even then.
    if not avif_matrix['include']:
        avif_matrix = {'include': [{'rust': 'stable', 'features': 'minimal', 'avif': False, 'ffmpeg': False, 'profile': 'test'}]}
    return {'matrix': matrix, 'core_matrix': core_matrix, 'avif_matrix': avif_matrix,
            'full': full, 'avif': modes['avif'] == 'full',
            'jpeg2000': modes['jpeg2000'] == 'full', 'jpegxl': modes['jpeg-xl'] == 'full',
            'tiff': full or 'tiff' in codecs, 'modes': modes,
            'smoke': any(mode == 'smoke' for mode in modes.values()), 'reason': reason}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--event', default=os.environ.get('CI_EVENT', ''))
    parser.add_argument('--base', default=os.environ.get('CI_BASE', ''))
    parser.add_argument('--head', default=os.environ.get('CI_HEAD', 'HEAD'))
    args = parser.parse_args()
    manifest = tomllib.loads(Path('wml2/Cargo.toml').read_text())
    result = plan(args.event, args.base, args.head, manifest)
    print(json.dumps(result, indent=2))
    if output := os.environ.get('GITHUB_OUTPUT'):
        with open(output, 'a') as stream:
            for key, value in result.items():
                if key != 'reason':
                    stream.write(f'{key}={json.dumps(value, separators=(",", ":"))}\n')


if __name__ == '__main__':
    main()
