import copy
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch
import tomllib

import check_ffmpeg
import select_ci as selection
import verify_ci
import run_codec_smoke

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = tomllib.loads((ROOT / 'wml2/Cargo.toml').read_text())
LOCK = tomllib.loads((ROOT / 'Cargo.lock').read_text())


class SelectionTests(unittest.TestCase):
    def classify(self, paths, pairs=None):
        return selection.affected(paths, lambda path: pairs[path])

    def test_isolated_codec_and_union(self):
        for path, expected in [
            ('wml2/src/avif/reconstruction.rs', 'avif'),
            ('wml2/src/highres/avif.rs', 'avif'),
            ('wml2/tests/highres_avif_encode.rs', 'avif'),
            ('wml2/src/jpeg2000_tier1.rs', 'jpeg2000'),
            ('wml2/src/jpegxl.rs', 'jpeg-xl'),
        ]:
            with self.subTest(path=path):
                codecs, full, _ = self.classify([path])
                self.assertFalse(full)
                self.assertEqual(codecs, {expected})
        self.assertEqual(self.classify(['wml2/src/jpegxl.rs', 'wml2/src/avif/ex.rs'])[0], {'jpeg-xl', 'avif'})

    def test_shared_unknown_workflow_fixture_and_submodule_force_full(self):
        for path in ['wml2/src/draw.rs', 'wml2/src/limits.rs', 'wml2/src/metadata.rs',
                     'wml2/src/highres/mod.rs', 'wml2/src/color.rs', 'Cargo.toml',
                     '.github/workflows/ci.yml', '.github/scripts/select_ci.py',
                     'avif', 'avifenc', 'wml2/tests/fixtures/new.avif',
                     'docs/generate.py', 'new-file.rs', 'wml2-test/Cargo.toml']:
            with self.subTest(path=path):
                self.assertTrue(self.classify(['wml2/src/avif/ex.rs', path])[1])

    def test_local_codecs_exif_and_embedded_jpeg_dependencies(self):
        for path, expected in [
            ('wml2/src/jpeg/decoder.rs', {'jpeg', 'tiff'}),
            ('wml2/src/tiff/ifd.rs', selection.LOCAL_CODECS | {'jpeg-xl'}),
            ('wml2/src/highres/tiff.rs', selection.LOCAL_CODECS | {'jpeg-xl'}),
            ('wml2/src/metadata/exif.rs', selection.LOCAL_CODECS | {'jpeg-xl'}),
            ('wml2/src/png/decoder.rs', {'png', 'tiff'}),
        ]:
            with self.subTest(path=path):
                codecs, full, _ = self.classify([path])
                self.assertFalse(full)
                self.assertEqual(codecs, expected)
                self.assertEqual(codecs & selection.CODECS, expected & selection.CODECS)

    def test_shared_types_cfg_and_unknown_changes_never_reduce_to_smoke(self):
        # Classification never tries to infer Rust semantics from text hunks.
        for path in ['wml2/src/error.rs', 'wml2/src/lib.rs', 'wml2/src/decode_guard.rs',
                     'wml2/src/image_buffer_ex.rs', 'wml2/src/warning.rs',
                     'wml2/src/highres/types.rs', 'wml2/src/metadata.rs',
                     'wml2/src/tiff/header.rs', 'wml2/src/tiff/mod.rs']:
            self.assertTrue(self.classify(['wml2-test/examples/converter.rs', path])[1])

    def test_smoke_none_full_and_matrix_partition(self):
        for paths, expected in [
            (['wml2-test/examples/converter.rs'], dict.fromkeys(selection.CODECS, 'smoke')),
            (['README.md'], dict.fromkeys(selection.CODECS, 'none')),
            (['wml2/src/tiff/ifd.rs'], {c: 'full' if c == 'jpeg-xl' else 'none' for c in selection.CODECS}),
            (['wml2/src/draw.rs'], dict.fromkeys(selection.CODECS, 'full')),
            (['wml2/src/jpeg2000.rs', 'wml2-test/examples/converter.rs'],
             {'jpeg2000': 'full', 'avif': 'smoke', 'jpeg-xl': 'smoke'}),
        ]:
            with self.subTest(paths=paths), patch.object(selection, 'git', side_effect=[b'base', '\0'.join(paths).encode() + b'\0']):
                result = selection.plan('pull_request', 'base', 'head', MANIFEST)
                self.assertEqual(result['modes'], expected)
                self.assertEqual(result['smoke'], 'smoke' in expected.values())
                self.assertTrue(result['core_matrix']['include'])
        result = selection.plan('workflow_call', '', '', MANIFEST)
        core, avif = result['core_matrix']['include'], result['avif_matrix']['include']
        self.assertEqual(len(core) + len(avif), 46)
        self.assertTrue(all(not row['avif'] for row in core))
        self.assertTrue(all(row['avif'] for row in avif))
        self.assertEqual({(r['rust'], r['features']) for r in core + avif},
                         {(r['rust'], r['features']) for r in result['matrix']['include']})
        for row in core + avif:
            self.assertEqual(row['profile'], 'ci' if 'jpeg2000' in row['features'] else 'test')
            self.assertEqual(row['ffmpeg'], 'avifenc' in row['features'])

    def test_manifest_only_existing_avif_versions(self):
        updated = copy.deepcopy(MANIFEST)
        updated['dependencies']['avif_codec']['version'] = '0.1.99'
        self.assertTrue(selection.manifest_avif_only(MANIFEST, updated))
        for mutate in [
            lambda m: m['features']['avif'].append('jpeg2000'),
            lambda m: m['dependencies']['icc_profile'].update(version='9'),
            lambda m: m['dependencies']['avif_codec'].update(git='https://example.invalid/repo'),
            lambda m: m['dependencies']['avif_codec'].update(optional=False),
            lambda m: m['dependencies']['avif_codec'].update(features=['encode']),
        ]:
            changed = copy.deepcopy(updated)
            mutate(changed)
            self.assertFalse(selection.manifest_avif_only(MANIFEST, changed))

    def test_lock_checks_sources_edges_and_shared_dependencies(self):
        updated = copy.deepcopy(LOCK)
        for p in updated['package']:
            if p['name'] in selection.AVIF_PACKAGES:
                p.update(version='0.1.99', checksum='new')
        self.assertTrue(selection.lock_avif_only(LOCK, updated))
        for name, values in [
            ('avif-image-buffer', {'version': '0.1.99'}),
            ('avif-rust', {'dependencies': ['new-shared-dependency']}),
            ('rav1e', {'checksum': 'different'}),
        ]:
            changed = copy.deepcopy(updated)
            next(p for p in changed['package'] if p['name'] == name).update(values)
            self.assertFalse(selection.lock_avif_only(LOCK, changed))
        changed = copy.deepcopy(updated)
        changed['package'].append(copy.deepcopy(next(p for p in changed['package'] if p['name'] == 'avif-rust')))
        with self.assertRaises(ValueError):
            selection.lock_avif_only(LOCK, changed)
        changed = copy.deepcopy(updated)
        next(p for p in changed['package'] if p['name'] == 'avif-rust')['source'] = 'git+https://example.invalid'
        with self.assertRaises(ValueError):
            selection.lock_avif_only(LOCK, changed)

    def test_transitive_feature_enables_ffmpeg(self):
        m = {'features': {'default': ['wrapper'], 'wrapper': ['avifenc'], 'avifenc': ['avif'], 'avif': ['dep:avif_codec']}}
        self.assertTrue(selection.uses_avif('default', m))
        self.assertTrue(selection.uses_avif('wrapper', m))
        self.assertFalse(selection.uses_avif('minimal', m))
        self.assertFalse(selection.uses_avif('jpeg2000', MANIFEST))
        self.assertTrue(selection.uses_avif('avifenc,high-bit-depth', MANIFEST))
        self.assertFalse(selection.uses_avif('multithread', MANIFEST))

    def test_future_default_optional_codec_keeps_full_mode_and_preparation(self):
        manifest = copy.deepcopy(MANIFEST)
        manifest['features']['default'] += ['jpeg2000', 'avif']
        with patch.object(selection, 'git', side_effect=[b'base', b'README.md\0']):
            result = selection.plan('pull_request', 'base', 'head', manifest)
        self.assertTrue(result['avif'])
        self.assertTrue(result['jpeg2000'])
        self.assertEqual(result['modes']['avif'], 'full')
        self.assertEqual(result['modes']['jpeg2000'], 'full')
        self.assertTrue(any(row['features'] == 'default' and row['profile'] == 'ci'
                            for row in result['avif_matrix']['include']))

    def test_matrix_and_non_pr_full_events(self):
        for event in ['push', 'workflow_dispatch', 'workflow_call', 'schedule', 'unknown']:
            result = selection.plan(event, '', '', MANIFEST)
            self.assertTrue(result['full'])
            self.assertEqual(len(result['matrix']['include']), 46)
        with patch.object(selection, 'git', side_effect=[b'base', b'wml2/src/avif/ex.rs\0']):
            result = selection.plan('pull_request', 'base', 'head', MANIFEST)
        self.assertFalse(result['full'])
        self.assertFalse(result['jpeg2000'])
        self.assertFalse(result['jpegxl'])
        features = {row['features'] for row in result['matrix']['include']}
        self.assertNotIn('jpeg2000', features)
        self.assertEqual(len(result['matrix']['include']), 24)
        self.assertEqual(sum(row['avif'] for row in result['matrix']['include']), 12)

    def test_empty_missing_base_and_malformed_toml_fallback(self):
        for effects in [
            [subprocess.CalledProcessError(1, 'git')],
            [b'base', b''],
            [b'base', b'Cargo.lock\0', b'invalid toml ['],
        ]:
            with patch.object(selection, 'git', side_effect=effects):
                self.assertTrue(selection.plan('pull_request', 'base', 'head', MANIFEST)['full'])

    def test_rename_retains_shared_old_path(self):
        with patch.object(selection, 'git', side_effect=[b'base', b'wml2/src/draw.rs\0wml2/src/avif/moved.rs\0']) as git:
            self.assertTrue(selection.plan('pull_request', 'base', 'head', MANIFEST)['full'])
            self.assertIn('--no-renames', git.call_args.args)


class GateTests(unittest.TestCase):
    def needs(self, full):
        n = {k: {'result': 'success'} for k in verify_ci.ALWAYS}
        n['select']['outputs'] = {flag: str(full).lower() for flag in verify_ci.CONDITIONAL.values()}
        n.update({k: {'result': 'success' if full else 'skipped'} for k in verify_ci.CONDITIONAL})
        n['select']['outputs'].update(modes=json.dumps(dict.fromkeys(selection.CODECS, 'full' if full else 'none')),
                                    jpeg2000=str(full).lower(), smoke='false')
        n['codec-smoke']['result'] = 'skipped'
        return n

    def test_full_and_selected_pass(self):
        for full in [True, False]:
            verify_ci.verify(self.needs(full))

    def test_failures_cancellation_missing_and_unexpected_skips_fail(self):
        for job in verify_ci.ALWAYS | set(verify_ci.CONDITIONAL):
            for result in ['failure', 'cancelled', 'skipped']:
                n = self.needs(True)
                if n[job]['result'] == result:
                    continue
                n[job]['result'] = result
                with self.subTest(job=job, result=result), self.assertRaises(ValueError):
                    verify_ci.verify(n)
        n = self.needs(False)
        n['tiff-oracle']['result'] = 'failure'
        with self.assertRaises(ValueError):
            verify_ci.verify(n)

    def test_smoke_selection_and_inconsistent_flags(self):
        n = self.needs(False)
        n['select']['outputs'].update(modes=json.dumps(dict.fromkeys(selection.CODECS, 'smoke')), smoke='true')
        n['codec-smoke']['result'] = 'success'
        verify_ci.verify(n)
        for flag, value in [('smoke', 'false'), ('jpeg2000', 'true'), ('avif', 'true'), ('modes', '{}')]:
            wrong = copy.deepcopy(n)
            wrong['select']['outputs'][flag] = value
            with self.subTest(flag=flag), self.assertRaises(ValueError):
                verify_ci.verify(wrong)
        n = self.needs(False)
        del n['test']
        with self.assertRaises(ValueError):
            verify_ci.verify(n)
        n = self.needs(False)
        n['select']['outputs'] = {}
        with self.assertRaises(ValueError):
            verify_ci.verify(n)


class FFmpegTests(unittest.TestCase):
    def test_missing_tools_rejected(self):
        with patch.object(check_ffmpeg.shutil, 'which', return_value=None):
            self.assertFalse(check_ffmpeg.usable())

    def test_available_but_broken_codec_rejected(self):
        with patch.object(check_ffmpeg.shutil, 'which', return_value='/bin/tool'), patch.object(check_ffmpeg.subprocess, 'run', side_effect=subprocess.CalledProcessError(1, 'ffmpeg')):
            self.assertFalse(check_ffmpeg.usable())

    def successful_results(self):
        probe = json.dumps({'streams': [{'width': 16, 'height': 16, 'pix_fmt': 'gbrp', 'codec_name': 'av1'}]}).encode()
        return [subprocess.CompletedProcess([], 0, stdout=data) for data in [b'', probe, bytes(768), bytes(1024)]]

    def test_all_oracle_operations_required(self):
        with patch.object(check_ffmpeg.shutil, 'which', return_value='/bin/tool'), patch.object(check_ffmpeg.subprocess, 'run', side_effect=self.successful_results()) as run:
            self.assertTrue(check_ffmpeg.usable())
            self.assertEqual([call.args[0][0] for call in run.call_args_list], ['ffmpeg', 'ffprobe', 'ffmpeg', 'ffmpeg'])

    def test_invalid_probe_wrong_pixel_length_or_missing_operation_fail(self):
        for index, replacement in [(1, b'{}'), (1, b'invalid JSON'), (2, b''), (3, bytes(512))]:
            results = self.successful_results()
            results[index] = subprocess.CompletedProcess([], 0, stdout=replacement)
            with self.subTest(index=index), patch.object(check_ffmpeg.shutil, 'which', return_value='/bin/tool'), patch.object(check_ffmpeg.subprocess, 'run', side_effect=results):
                self.assertFalse(check_ffmpeg.usable())
        for index in range(4):
            results = self.successful_results()
            results[index] = subprocess.TimeoutExpired('ffmpeg', 30)
            with patch.object(check_ffmpeg.shutil, 'which', return_value='/bin/tool'), patch.object(check_ffmpeg.subprocess, 'run', side_effect=results):
                self.assertFalse(check_ffmpeg.usable())


class SmokeTests(unittest.TestCase):
    def test_optimized_profile_preserves_runtime_safety_checks(self):
        profile = tomllib.loads((ROOT / 'Cargo.toml').read_text())['profile']['ci']
        self.assertEqual(profile['inherits'], 'test')
        self.assertIs(profile['debug-assertions'], True)
        self.assertIs(profile['overflow-checks'], True)

    def test_none_full_have_no_smoke_commands(self):
        for mode in ['none', 'full']:
            self.assertEqual(run_codec_smoke.commands(dict.fromkeys(selection.CODECS, mode)), [])

    def test_smoke_has_exact_filters_and_no_full_corpus(self):
        commands = run_codec_smoke.commands(dict.fromkeys(selection.CODECS, 'smoke'))
        self.assertEqual(len(commands), 9)
        for command in commands:
            self.assertIn('--locked', command)
            self.assertIn('ci', command)
            self.assertNotIn('--lib', command)
            self.assertNotIn('all_400_native_owned_streams', ' '.join(command))
        self.assertTrue(all('--exact' in command for command in commands[:7]))


if __name__ == '__main__':
    unittest.main()
