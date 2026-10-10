"""Validate and copy the canonical, version-specific GitHub Release body."""

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tomllib


def prepare(root, tag=None, expected_sha=None):
    version = tomllib.loads((root / "wml2/Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("Only numeric major.minor.patch releases are supported")
    if tag is not None and (not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag) or tag != f"v{version}"):
        raise ValueError(f"Tag {tag!r} must match Cargo version v{version}")
    if expected_sha is not None:
        if tag is None or not re.fullmatch(r"[0-9a-f]{40}", expected_sha):
            raise ValueError("An exact commit SHA and tag are required")
        for ref in ("HEAD", f"refs/tags/{tag}^{{commit}}"):
            actual = subprocess.check_output(["git", "rev-parse", "--verify", ref], cwd=root, text=True).strip()
            if actual != expected_sha:
                raise ValueError(f"{ref} does not match the release event SHA")
    body = (root / f"docs/releases/{version}.md").read_text(encoding="utf-8").strip()
    if not body or body.splitlines()[0] != f"# WML2 {version}":
        raise ValueError("Release body must start with the exact WML2 version heading")
    if re.search(r"\b(TODO|TBD|PLACEHOLDER)\b", body, re.IGNORECASE):
        raise ValueError("Release body contains an unfinished placeholder")
    for heading in ("Changes", "Compatibility", "Pull requests"):
        sections = re.findall(rf"^## {re.escape(heading)}\n(.*?)(?=^## |\Z)", body, re.MULTILINE | re.DOTALL)
        if len(sections) != 1 or not re.search(r"^- \S", sections[0], re.MULTILINE):
            raise ValueError(f"Release body requires one nonempty {heading} section with a bullet")
    return body + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--tag")
    parser.add_argument("--expected-sha")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        body = prepare(args.root, args.tag, args.expected_sha)
        if args.output:
            args.output.write_text(body, encoding="utf-8", newline="\n")
        else:
            sys.stdout.write(body)
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as exc:
        parser.exit(1, f"Release notes validation failed: {exc}\n")


if __name__ == "__main__":
    main()
