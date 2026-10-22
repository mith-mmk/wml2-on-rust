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
        n['select']['outputs'] = {'jpegxl': str(full).lower(), 'tiff': str(full).lower()}
        n.update({k: {'result': 'success' if full else 'skipped'} for k in verify_ci.CONDITIONAL})
        return n

    def test_full_and_selected_pass(self):
        for full in [True, False]:
            verify_ci.verify(self.needs(full))

    def test_failures_cancellation_missing_and_unexpected_skips_fail(self):
        for job in verify_ci.ALWAYS | set(verify_ci.CONDITIONAL):
            for result in ['failure', 'cancelled', 'skipped']:
                n = self.needs(True)
                n[job]['result'] = result
                with self.subTest(job=job, result=result), self.assertRaises(ValueError):
                    verify_ci.verify(n)
        n = self.needs(False)
        n['tiff-oracle']['result'] = 'failure'
        with self.assertRaises(ValueError):
            verify_ci.verify(n)
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

    def test_all_three_operations_required(self):
        with patch.object(check_ffmpeg.shutil, 'which', return_value='/bin/tool'), patch.object(check_ffmpeg.subprocess, 'run') as run:
            self.assertTrue(check_ffmpeg.usable())
            self.assertEqual([call.args[0][0] for call in run.call_args_list], ['ffmpeg', 'ffprobe', 'ffmpeg'])


if __name__ == '__main__':
    unittest.main()
