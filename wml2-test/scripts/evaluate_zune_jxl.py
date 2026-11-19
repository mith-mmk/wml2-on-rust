#!/usr/bin/env python3
"""Isolated zune-jpegxl/libjxl evaluation. No WML2 dependency changes.

Requires Rust 1.91+, cc, and installed libjxl.so.0.11 (Debian 0.11.2 used).
Downloads only pinned official libjxl API headers and Cargo registry packages.
All generated files go under the chosen .test* directory.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import resource
import struct
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = Path(__file__).resolve().parent
LIBJXL_REV = "332feb17d17311c748445f7ee75c4fb55cc38530"  # v0.11.2


def sha(data):
    return hashlib.sha256(data).hexdigest()


def limits():
    # Bounded child processes; this is containment, not a backend admission API.
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (20, 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def run(args, *, bounded=False):
    return subprocess.run(
        [str(a) for a in args], capture_output=True, text=True, timeout=120,
        check=False, preexec_fn=limits if bounded else None,
    )


def checked(args):
    p = run(args)
    if p.returncode:
        raise RuntimeError(f"{args}: {p.stdout}\n{p.stderr}")
    return p


def pixels(width, height, channels, bits):
    # Original synthetic CC0 test samples: extrema, ramps and xorshift noise.
    state = 0x19700403
    maximum = (1 << bits) - 1
    alpha = [0, 1, maximum, maximum // 2, maximum - 1, 255, 256, 257, 32768]
    samples = []
    for i in range(width * height):
        for c in range(channels):
            state ^= (state << 13) & 0xFFFFFFFF
            state ^= state >> 17
            state ^= (state << 5) & 0xFFFFFFFF
            value = (state ^ (i * 257) ^ (c * 131)) & maximum
            if i < 4:
                value = [0, maximum, 1, maximum - 1][(i + c) % 4]
            if channels in (2, 4) and c == channels - 1:
                value = alpha[i % len(alpha)] & maximum
            samples.append(value)
    if bits == 8:
        return bytes(samples)
    return struct.pack(f"={len(samples)}H", *samples)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / ".test-zune-eval")
    parser.add_argument("--wasm", action="store_true", help="Compile cdylib and run a Node smoke matrix")
    args = parser.parse_args()
    work = args.output.resolve()
    if not work.name.startswith(".test"):
        parser.error("output directory basename must start with .test")
    work.mkdir(parents=True, exist_ok=True)
    headers = work / ".test-headers" / "jxl"
    headers.mkdir(parents=True, exist_ok=True)
    header_hashes = {}
    for name in ("decode.h", "types.h", "memory_manager.h", "codestream_header.h",
                 "color_encoding.h", "cms_interface.h", "parallel_runner.h"):
        data = urllib.request.urlopen(
            f"https://raw.githubusercontent.com/libjxl/libjxl/{LIBJXL_REV}/lib/include/jxl/{name}",
            timeout=30,
        ).read()
        (headers / name).write_bytes(data)
        header_hashes[name] = sha(data)
    # Export/version headers are build-generated upstream. These local stubs
    # affect symbol decoration only; API structs/enums come from pinned headers.
    (headers / "jxl_export.h").write_text(
        "#define JXL_EXPORT\n#define JXL_DEPRECATED\n#define JXL_DEPRECATED_EXPORT\n")
    (headers / "version.h").write_text("/* No API declaration here uses version macros. */\n")
    manifest = work / "Cargo.toml"
    manifest.write_text(f'''[package]
name = "wml2-zune-evaluation"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
zune-jpegxl = {{ version = "=0.5.2", default-features = false }}
zune-core = {{ version = "=0.5.1", default-features = false }}
[[bin]]
name = "probe"
path = {json.dumps(str(SCRIPTS / "zune_jxl_probe.rs"))}
[lib]
path = {json.dumps(str(SCRIPTS / "zune_jxl_wasm_probe.rs"))}
crate-type = ["cdylib"]
''')
    # Keep the evaluation target separate from the production workspace/cache.
    os.environ["CARGO_TARGET_DIR"] = str(work / ".test-target")
    checked(["cargo", "generate-lockfile", "--manifest-path", manifest])
    checked(["cargo", "build", "--release", "--locked", "--manifest-path", manifest])
    oracle = work / ".test-oracle"
    checked(["cc", "-std=c11", "-O2", "-I", headers.parent,
             SCRIPTS / "zune_jxl_oracle.c", "-Wl,-l:libjxl.so.0.11", "-o", oracle])
    probe = work / ".test-target" / "release" / "probe"
    cases = []

    def evaluate(name, width, height, channels, bits, data, cap=16 << 20, reserve_cap=16 << 20):
        source, encoded, decoded = [work / f".test-{name}.{ext}" for ext in ("raw", "jxl", "decoded")]
        source.write_bytes(data)
        p = run([probe, width, height, channels, bits, source, encoded, cap, reserve_cap], bounded=True)
        result = {"name": name, "width": width, "height": height,
                  "channels": channels, "bits": bits, "input_sha256": sha(data),
                  "output_cap": cap, "reserve_cap": reserve_cap,
                  "process_returncode": p.returncode, "stderr": p.stderr.strip()}
        if p.returncode:
            result["process_stdout"] = p.stdout
        else:
            result.update(json.loads(p.stdout))
        if result.get("status") == "ok":
            result["output_sha256"] = sha(encoded.read_bytes())
            q = run([oracle, encoded, decoded, bits], bounded=True)
            result["oracle_returncode"] = q.returncode
            result["oracle_stderr"] = q.stderr.strip()
            if not q.returncode:
                output = decoded.read_bytes()
                result["oracle"] = json.loads(q.stdout)
                result["decoded_sha256"] = sha(output)
                result["exact_samples"] = output == data
                result["different_bytes"] = sum(a != b for a, b in zip(data, output)) + abs(len(data) - len(output))
                if len(output) == len(data):
                    sample_type = "B" if bits == 8 else "H"
                    original = memoryview(data).cast(sample_type)
                    decoded_samples = memoryview(output).cast(sample_type)
                    result["different_samples_by_channel"] = [
                        sum(a != b for a, b in zip(original[c::channels], decoded_samples[c::channels]))
                        for c in range(channels)]
        cases.append(result)
        return result

    dimensions = [(1, 1), (1, 17), (17, 1), (2, 2), (3, 5), (17, 13), (255, 257), (257, 259)]
    for channels in (1, 2, 3, 4):
        for bits in (8, 16):
            for width, height in dimensions:
                evaluate(f"c{channels}-b{bits}-{width}x{height}", width, height, channels, bits,
                         pixels(width, height, channels, bits))
    raw = pixels(17, 13, 4, 8)
    for suffix, w, h, c, data in [
        ("short", 17, 13, 4, raw[:-1]), ("long", 17, 13, 4, raw + b"x"),
        ("empty", 17, 13, 4, b""), ("zero-width", 0, 13, 4, b""),
        ("zero-height", 17, 0, 4, b""), ("unsupported-cmyk", 17, 13, 5, raw),
        ("overflow", (1 << (struct.calcsize("P") * 8)) - 1, 2, 4, b""),
    ]:
        evaluate(suffix, w, h, c, 8, data)
    base = evaluate("cap-baseline", 17, 13, 4, 8, raw)
    for name, cap, reservation in [
        ("reserve-zero", 16 << 20, 0), ("write-zero", 0, 16 << 20),
        ("write-exact", base["output_bytes"], 16 << 20),
        ("write-minus-one", base["output_bytes"] - 1, 16 << 20),
        ("reserve-output-size", 16 << 20, base["output_bytes"]),
    ]:
        evaluate(name, 17, 13, 4, 8, raw, cap, reservation)
    for channels, bits in [(1, 16), (3, 16), (4, 8)]:
        for size in (512, 1024):
            data = pixels(size, size, channels, bits)
            for repetition in range(3):
                evaluate(f"resource-c{channels}-b{bits}-{size}-r{repetition}",
                         size, size, channels, bits, data)
    wasm = {"requested": args.wasm, "runtime_tested": False}
    if args.wasm:
        p = run(["cargo", "build", "--lib", "--release", "--locked", "--target", "wasm32-unknown-unknown",
                 "--manifest-path", manifest])
        wasm.update(returncode=p.returncode, stdout=p.stdout, stderr=p.stderr)
        if not p.returncode:
            binary = work / ".test-target/wasm32-unknown-unknown/release/wml2_zune_evaluation.wasm"
            jobs = []
            for channels in (1, 2, 3, 4):
                for bits in (8, 16):
                    name = f"c{channels}-b{bits}-17x13"
                    jobs.append({"name": name, "width": 17, "height": 13, "channels": channels,
                                 "bits": bits, "input": str(work / f".test-{name}.raw"),
                                 "output": str(work / f".test-wasm-{name}.jxl")})
            jobs_path = work / ".test-wasm-jobs.json"
            jobs_path.write_text(json.dumps(jobs))
            q = run(["node", "--max-old-space-size=64", SCRIPTS / "zune_jxl_wasm_host.mjs", binary, jobs_path])
            wasm.update(host_returncode=q.returncode, host_stderr=q.stderr, binary_sha256=sha(binary.read_bytes()))
            if not q.returncode:
                wasm.update(json.loads(q.stdout), runtime_tested=True)
                for job, result in zip(jobs, wasm["results"]):
                    decoded = work / f".test-wasm-{job['name']}.decoded"
                    oracle_result = run([oracle, job["output"], decoded, job["bits"]], bounded=True)
                    result["oracle_returncode"] = oracle_result.returncode
                    result["oracle_stderr"] = oracle_result.stderr.strip()
                    result["matches_native_encoded"] = Path(job["output"]).read_bytes() == (work / f".test-{job['name']}.jxl").read_bytes()
                    if not oracle_result.returncode:
                        result["exact_samples"] = decoded.read_bytes() == Path(job["input"]).read_bytes()
                        result["oracle"] = json.loads(oracle_result.stdout)
    report = {"schema": 1, "date": "2026-10-30", "base_commit": "1b567d75eb1040ab4fedbb835051dc28a6db9d0d",
              "rustc": checked(["rustc", "--version"]).stdout.strip(),
              "platform": platform.platform(), "byteorder": sys.byteorder,
              "crate_lock": (work / "Cargo.lock").read_text(),
              "feature_tree": checked(["cargo", "tree", "-e", "features", "--manifest-path", manifest]).stdout,
              "header_revision": LIBJXL_REV, "header_sha256": header_hashes,
              "allocator_metric": "Rust requested heap bytes during encode; input/process baseline excluded; output capacity included; not RSS or proven bounds",
              "containment": "per child: 512MiB RLIMIT_AS, 20s CPU, 120s wall timeout; not backend limits",
              "icc": "No profile input in candidate API; source inspection only, no custom ICC supplied",
              "wasm": wasm, "cases": cases}
    (work / "evaluation.json").write_text(json.dumps(report, indent=2) + "\n")
    summary = {"cases": len(cases), "encode_ok": sum(x.get("status") == "ok" for x in cases),
               "exact": sum(x.get("exact_samples", False) for x in cases),
               "oracle_failed": [x["name"] for x in cases if x.get("oracle_returncode", 0)],
               "sample_mismatch": [x["name"] for x in cases if x.get("exact_samples") is False],
               "panics": [x["name"] for x in cases if x.get("status") == "panic"],
               "wasm": wasm.get("returncode")}
    print(json.dumps(summary, indent=2))
    # Validate the recorded qualification split for these exact pinned crates.
    # Expected candidate failures are evidence, never a conformance pass.
    assert base.get("exact_samples"), "independent baseline oracle must agree"
    matrix = [x for x in cases if x["name"].startswith("c") and x["name"] != "cap-baseline"]
    assert len(matrix) == 64
    for case in matrix:
        if case["width"] == 1 or case["height"] == 1:
            assert case["status"] == "error", case["name"]
        else:
            assert case["oracle_returncode"] == 0, case["name"]
            expected = not (case["bits"] == 16 and case["channels"] in (2, 4))
            assert case["exact_samples"] == expected, case["name"]
            info = case["oracle"]
            assert (info["width"], info["height"], info["channels"], info["bits"]) == (
                case["width"], case["height"], case["channels"], case["bits"])
            assert info["uses_original_profile"] == info["structured_profile"] == 1
            assert info["alpha_premultiplied"] == 0 and info["orientation"] == 1
            assert info["white_point"] == 1 and info["transfer_function"] == 13
            assert info["alpha_bits"] == (8 if case["channels"] in (2, 4) else 0)
    assert summary["panics"] == ["zero-height", "overflow"]
    assert summary["oracle_failed"] == [] and summary["exact"] == 50
    if args.wasm:
        assert wasm.get("host_returncode") == 0 and wasm["runtime_tested"]
        assert wasm["imports"] == []
        for result in wasm["results"]:
            assert result["matches_native_encoded"] and result["oracle_returncode"] == 0


if __name__ == "__main__":
    main()
