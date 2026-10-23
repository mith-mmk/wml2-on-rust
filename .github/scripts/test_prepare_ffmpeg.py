import json
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import prepare_ffmpeg as preparation


class PreparationTests(unittest.TestCase):
    def test_usable_runner_tools_require_no_apt_or_cache(self):
        with patch.object(preparation.check_ffmpeg, 'usable', return_value=True), patch.object(preparation, 'run') as run, patch.object(preparation, 'output') as output:
            preparation.plan()
            run.assert_not_called()
            output.assert_called_once_with(ready='true', package_key='unused')

    def test_authenticated_plan_pins_version_and_keys_dependency_transaction(self):
        read_text = Path.read_text
        def read(path, *args, **kwargs):
            if path.as_posix() == '/etc/os-release':
                return 'ID=ubuntu\nVERSION_ID="24.04"\n'
            return read_text(path, *args, **kwargs)
        keys = []
        with tempfile.TemporaryDirectory(prefix='.test-ffmpeg-plan-', dir='.') as directory:
            root = Path(directory).resolve()
            for digest in ['a' * 64, 'b' * 64]:
                transaction = "'https://archive.ubuntu.com/ubuntu/pool/f/ffmpeg_6.1_amd64.deb' ffmpeg_6.1_amd64.deb 42 SHA256:" + digest
                metadata = f'Package: ffmpeg\nFilename: pool/f/ffmpeg_6.1_amd64.deb\nSize: 42\nSHA256: {digest}\n'
                with patch.object(preparation.check_ffmpeg, 'usable', return_value=False), patch.object(preparation, 'ROOT', root), patch.object(preparation, 'STATE', root / 'state.json'), patch.object(Path, 'read_text', read), patch.object(preparation, 'run', side_effect=['amd64', '', '  Candidate: 7:6.1.1-3ubuntu5\n', transaction, metadata]) as run, patch.object(preparation, 'output') as output:
                    preparation.plan()
                    state = json.loads((root / 'state.json').read_text())
                    self.assertEqual(state['version'], '7:6.1.1-3ubuntu5')
                    self.assertIn('ffmpeg=7:6.1.1-3ubuntu5', run.call_args_list[-2].args[0])
                    self.assertEqual(state['packages']['ffmpeg_6.1_amd64.deb']['sha256'], digest)
                    sources = (root / '.test-ffmpeg.sources').read_text()
                    self.assertIn('https://archive.ubuntu.com/ubuntu', sources)
                    self.assertIn('https://security.ubuntu.com/ubuntu', sources)
                    self.assertIn('Signed-By:', sources)
                    self.assertNotIn('trusted=yes', sources)
                    keys.append(output.call_args.kwargs['package_key'])
        self.assertNotEqual(*keys)

    def installation(self, results, usable=True):
        with tempfile.TemporaryDirectory(prefix='.test-ffmpeg-install-', dir='.') as directory:
            root = Path(directory)
            state = root / 'state.json'
            state.write_text(json.dumps({'version': '7:fixed', 'options': [], 'packages': {}}))
            archives = root / '.test-debs'
            archives.mkdir()
            (archives / 'bad.deb').write_bytes(b'corrupt')
            with patch.object(preparation, 'STATE', state), patch.object(preparation, 'ARCHIVES', archives), patch.object(preparation, 'run', side_effect=results) as run, patch.object(preparation.check_ffmpeg, 'usable', return_value=usable), patch.dict(os.environ, {'GITHUB_PATH': ''}):
                preparation.install()
                return run.call_args_list, (archives / 'bad.deb').exists()

    def test_corrupt_cache_recovers_once_then_verifies_pinned_version(self):
        calls, stale_exists = self.installation([subprocess.CalledProcessError(1, 'apt-get'), '', '7:fixed'])
        self.assertFalse(stale_exists)
        self.assertEqual(len(calls), 3)
        self.assertEqual(calls[0].args[1], 480)
        self.assertEqual(calls[1].args[1], 480)

    def test_install_failure_timeout_wrong_version_or_bad_runtime_never_skip(self):
        for error in [subprocess.CalledProcessError(1, 'apt-get'), subprocess.TimeoutExpired('apt-get', 480)]:
            with self.assertRaises(type(error)):
                self.installation([error, error])
        with self.assertRaisesRegex(RuntimeError, 'pinned'):
            self.installation(['', '7:different'])
        with self.assertRaisesRegex(RuntimeError, 'required encode'):
            self.installation(['', '7:fixed'], usable=False)

    def test_timeout_covers_sudo_process_group(self):
        with patch.object(preparation.subprocess, 'check_output') as check:
            preparation.run(['sudo', 'apt-get', 'update'], 180)
            self.assertEqual(check.call_args.args[0][:3], ['timeout', '--kill-after=10s', '180s'])
            self.assertEqual(check.call_args.kwargs['timeout'], 200)

    def test_only_archives_matching_fresh_index_sha256_are_reused(self):
        with tempfile.TemporaryDirectory(prefix='.test-ffmpeg-hashes-', dir='.') as directory:
            root = Path(directory)
            (root / 'valid.deb').write_bytes(b'valid')
            (root / 'corrupt.deb').write_bytes(b'wrong')
            (root / 'unrequested.deb').write_bytes(b'irrelevant')
            expected = {'size': 5, 'sha256': hashlib.sha256(b'valid').hexdigest()}
            with patch.object(preparation, 'ARCHIVES', root):
                preparation.validate_archives({'valid.deb': expected, 'corrupt.deb': expected})
            self.assertTrue((root / 'valid.deb').exists())
            self.assertFalse((root / 'corrupt.deb').exists())
            self.assertFalse((root / 'unrequested.deb').exists())

    def test_untrusted_origin_or_missing_strong_hash_fail_closed(self):
        for uri, metadata in [
            ('https://third-party.invalid/pool/f/foo.deb', ''),
            ('https://archive.ubuntu.com/ubuntu/pool/f/foo.deb', 'Filename: pool/f/foo.deb\nSize: 42\nMD5sum: abc\n'),
        ]:
            transaction = f"'{uri}' foo_1_amd64.deb 42 MD5Sum:abc"
            with patch.object(preparation, 'run', return_value=metadata), self.assertRaises(ValueError):
                preparation.package_hashes(transaction, [])


if __name__ == '__main__':
    unittest.main()
