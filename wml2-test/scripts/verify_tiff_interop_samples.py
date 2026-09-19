#!/usr/bin/env python3
"""Validate the pinned external TIFF interoperability corpus.

The TIFF binaries remain outside the repository. This command checks that the
configured corpus contains exactly the manifest revisions before a decoder or
oracle is run against it.
"""

import argparse
import hashlib
import json
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    results = []
    failed = False
    for spec in manifest["samples"]:
        path = args.corpus / spec["name"]
        result = {"name": spec["name"], "path": str(path)}
        if not path.is_file():
            result.update(ok=False, reason="missing")
            failed = True
        else:
            actual = sha256(path)
            result.update(
                ok=actual == spec["sha256"],
                bytes=path.stat().st_size,
                sha256=actual,
            )
            if actual != spec["sha256"] or path.stat().st_size != spec["bytes"]:
                result["reason"] = "hash-or-size-mismatch"
                failed = True
        results.append(result)

    report = {
        "corpus": manifest["corpus"],
        "samples": results,
        "passed": sum(item["ok"] for item in results),
        "total": len(results),
    }
    encoded = json.dumps(report, indent=2) + "\n"
    print(encoded, end="")
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
