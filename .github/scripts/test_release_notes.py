from pathlib import Path
import subprocess
import tempfile
import unittest

from release_notes import prepare


BODY = "# WML2 0.0.33\n\n## Changes\n- Fix WebP.\n\n## Compatibility\n- Encoded bytes change.\n\n## Pull requests\n- None: direct commits.\n"


class ReleaseNotesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix=".test-release-", dir=Path(__file__).resolve().parents[2])
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "wml2").mkdir()
        (self.root / "docs/releases").mkdir(parents=True)
        (self.root / "wml2/Cargo.toml").write_text('[package]\nversion = "0.0.33"\n', encoding="utf-8")
        self.notes = self.root / "docs/releases/0.0.33.md"
        self.notes.write_text(BODY, encoding="utf-8")

    def test_exact_body_and_version(self):
        self.assertEqual(prepare(self.root, "v0.0.33"), BODY)

    def test_wrong_or_unsafe_tag(self):
        for tag in ("v0.0.32", "0.0.33", "v0.0.33-rc1", "v../../x", "v0.0.33\n"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                prepare(self.root, tag)

    def test_missing_body(self):
        self.notes.unlink()
        with self.assertRaises(FileNotFoundError):
            prepare(self.root)

    def test_empty_misversioned_or_incomplete_body(self):
        for body in ("", "   ", BODY.replace("# WML2 0.0.33", "# WML2 0.0.32"),
                     BODY.replace("- Fix WebP.", ""), BODY.replace("- Fix WebP.", "- TBD"),
                     BODY.replace("## Pull requests", "## Other"), BODY + "\n## Changes\n- Duplicate.\n"):
            with self.subTest(body=body), self.assertRaises(ValueError):
                self.notes.write_text(body, encoding="utf-8")
                prepare(self.root)

    def test_exact_sha_and_tag_commit(self):
        def git(*args):
            return subprocess.check_output(["git", *args], cwd=self.root, text=True, stderr=subprocess.DEVNULL).strip()
        git("init", "--quiet")
        git("add", ".")
        git("-c", "user.name=Test", "-c", "user.email=test@example.invalid", "-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "fixture")
        sha = git("rev-parse", "HEAD")
        git("tag", "v0.0.33")
        self.assertEqual(prepare(self.root, "v0.0.33", sha), BODY)
        with self.assertRaises(ValueError):
            prepare(self.root, "v0.0.33", "0" * 40)
        git("-c", "user.name=Test", "-c", "user.email=test@example.invalid", "-c", "commit.gpgsign=false", "commit", "--quiet", "--allow-empty", "-m", "next")
        with self.assertRaises(ValueError):
            prepare(self.root, "v0.0.33", git("rev-parse", "HEAD"))
        git("checkout", "--quiet", sha)
        git("tag", "-d", "v0.0.33")
        git("-c", "user.name=Test", "-c", "user.email=test@example.invalid", "-c", "tag.gpgsign=false", "tag", "-a", "v0.0.33", "-m", "annotated")
        self.assertEqual(prepare(self.root, "v0.0.33", sha), BODY)

    def test_cli_copies_body_and_fails_without_output_on_bad_tag(self):
        import sys
        script = Path(__file__).with_name("release_notes.py")
        output = self.root / "body.md"
        command = [sys.executable, "-B", str(script), "--root", str(self.root), "--output", str(output)]
        subprocess.run(command + ["--tag", "v0.0.33"], check=True)
        self.assertEqual(output.read_text(encoding="utf-8"), BODY)
        output.unlink()
        result = subprocess.run(command + ["--tag", "v0.0.32"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
