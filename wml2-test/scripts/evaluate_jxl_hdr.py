#!/usr/bin/env python3
"""Small trusted-fixture HDR/float evaluation, not a production decoder.

Needs cc, installed libjxl.so.0.11, and Rust 1.91+. Only official pinned libjxl
headers and registry crates are fetched. All generated files stay in .test*.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import os
import platform
import resource
import struct
import subprocess
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = Path(__file__).resolve().parent
REV = "332feb17d17311c748445f7ee75c4fb55cc38530"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def contained():
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_CPU, (20, 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def run(args, bounded=False):
    p = subprocess.run([str(x) for x in args], text=True, capture_output=True,
                       timeout=120, preexec_fn=contained if bounded else None)
    if p.returncode:
        raise RuntimeError(f"{args}: {p.returncode}\n{p.stdout}\n{p.stderr}")
    return p


def unpack(data, storage):
    code = {"f32": "f", "f16": "e", "u16": "H"}[storage]
    return struct.unpack(f"<{len(data)//struct.calcsize(code)}{code}", data)


def compare(a, b, storage, channels):
    x, y = unpack(a, storage), unpack(b, storage)
    assert len(x) == len(y)
    return {"exact_bytes": a == b, "max_abs_error": max(abs(i-j) for i,j in zip(x,y)),
            "different_samples": sum(i != j for i,j in zip(x,y)),
            "max_abs_by_channel": [max(abs(i-j) for i,j in zip(x[c::channels],y[c::channels])) for c in range(channels)]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / ".test-hdr")
    args = parser.parse_args()
    work = args.output.resolve()
    if not work.name.startswith(".test"):
        parser.error("output basename must start with .test")
    work.mkdir(parents=True, exist_ok=True)
    headers = work / ".test-headers" / "jxl"
    headers.mkdir(parents=True, exist_ok=True)
    hashes = {}
    for name in ("encode.h", "decode.h", "types.h", "memory_manager.h", "codestream_header.h",
                 "color_encoding.h", "cms_interface.h", "parallel_runner.h", "stats.h"):
        data = urllib.request.urlopen(f"https://raw.githubusercontent.com/libjxl/libjxl/{REV}/lib/include/jxl/{name}", timeout=30).read()
        (headers/name).write_bytes(data)
        hashes[name] = sha(data)
    (headers/"jxl_export.h").write_text("#define JXL_EXPORT\n#define JXL_DEPRECATED\n#define JXL_DEPRECATED_EXPORT\n")
    (headers/"version.h").write_text("/* Generated-header stub: declarations use no version macros. */\n")
    helper = work/".test-reference"
    run(["cc", "-std=c11", "-Wall", "-Wextra", "-Werror", "-O2", "-I", headers.parent,
         SCRIPT/"jxl_hdr_reference.c", "-Wl,-l:libjxl.so.0.11", "-o", helper])
    manifest = work/"Cargo.toml"
    manifest.write_text(f'''[package]
name="wml2-hdr-evaluation"
version="0.0.0"
edition="2024"
[workspace]
[dependencies]
jxl_codec={{package="jxl",version="=0.7.4",default-features=false}}
wml2={{path={json.dumps(str(ROOT/'wml2'))},default-features=false,features=["jpeg-xl","image-buffer-ex","color-management"]}}
[[bin]]
name="hdr-probe"
path={json.dumps(str(SCRIPT/'jxl_hdr_probe.rs'))}
''')
    os.environ["CARGO_TARGET_DIR"] = str(work/".test-target")
    recorded = ROOT/"docs/jpeg-xl-hdr-float-evaluation.json"
    if recorded.exists():
        (work/"Cargo.lock").write_text(json.loads(recorded.read_text())["cargo_lock"])
    else:
        run(["cargo", "generate-lockfile", "--manifest-path", manifest])
    run(["cargo", "build", "--release", "--locked", "--manifest-path", manifest])
    probe = work/".test-target/release/hdr-probe"
    icc = ROOT/"wml2/tests/fixtures/review/jxl-gamma2.icc"
    contracts = run([probe,"contracts",icc], bounded=True).stdout
    cases = []
    definitions = []
    for storage in ("f16", "f32"):
        for channels in (1, 3, 4):
            definitions.append((f"{storage}-c{channels}-linear", storage, channels, "linear", 255, "-", "extended"))
    for transfer in ("pq", "hlg"):
        for channels in (3,4):
            definitions.append((f"u16-c{channels}-{transfer}", "u16", channels, transfer, 1000, "-", "normalized"))
    definitions += [("f32-icc", "f32",3,"linear",255,icc,"normalized"),
                    ("u16-tone-metadata", "u16",3,"srgb",1000,"-","normalized"),
                    ("u16-sdr-control", "u16",3,"srgb",255,"-","normalized")]
    for name, storage, channels, transfer, intensity, profile, pattern in definitions:
        values = []
        extended = [-1.0, -0.25, -0.0, 0.0, 2**-24, 2**-14, 0.1, 0.5, 1.0, 1.5, 16.0, 65504.0]
        if storage=="f32": extended += [2**-149, 1e-40]
        for pixel in range(15):
            for channel in range(channels):
                value = extended[(pixel+channel)%len(extended)] if pattern=="extended" else [0.0, 0.0001, 0.01, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0][(pixel+channel)%9]
                if channels==4 and channel==3: value=[0.0,0.25,0.5,0.75,1.0][pixel%5]
                values.append(value)
        code={"f16":"e","f32":"f","u16":"H"}[storage]
        raw=struct.pack(f"<{len(values)}{code}",*([round(v*65535) for v in values] if storage=="u16" else values))
        source,encoded=work/f".test-{name}.raw",work/f".test-{name}.jxl"
        source.write_bytes(raw)
        run([helper,"encode",source,encoded,5,3,channels,storage,transfer,intensity,profile],bounded=True)
        case={"name":name,"storage":storage,"channels":channels,"transfer":transfer,
              "intensity_target":intensity,"input_sha256":sha(raw),"jxl_sha256":sha(encoded.read_bytes()),
              "input_bytes":len(raw),"encoded_bytes":encoded.stat().st_size,"outputs":[]}
        for output_storage in ("f32","f16","u16"):
            for target in (0,80):
                reference=work/f".test-{name}-{output_storage}-{target}.ref"
                output=work/f".test-{name}-{output_storage}-{target}.out"
                ref=json.loads(run([helper,"decode",encoded,reference,output_storage,target],bounded=True).stdout)
                core=json.loads(run([probe,encoded,output,output_storage,target],bounded=True).stdout)
                a,b=reference.read_bytes(),output.read_bytes()
                result={"storage":output_storage,"desired_intensity_target":target,"oracle":ref,"core":core,
                        "reference_sha256":sha(a),"core_sha256":sha(b),**compare(a,b,output_storage,channels)}
                if output_storage==storage:
                    result["source_vs_reference"]=compare(raw,a,storage,channels)
                if profile!="-":
                    original=Path(profile).read_bytes()
                    result["original_icc_exact"]=Path(str(output)+".source.icc").read_bytes()==original and Path(str(reference)+".icc").read_bytes()==original
                    result["output_icc_exact"]=Path(str(output)+".output.icc").read_bytes()==original
                samples=unpack(b,output_storage)
                result.update(minimum=min(samples),maximum=max(samples),nonfinite=sum(not math.isfinite(v) for v in samples))
                case["outputs"].append(result)
        for i in range(0,6,2):
            case["outputs"][i]["target80_core_bytes_unchanged"]=case["outputs"][i]["core_sha256"]==case["outputs"][i+1]["core_sha256"]
        cases.append(case)
        if name in ("u16-c3-pq", "u16-c3-hlg"):
            icc_path=Path(str(work/f".test-{name}-f32-0.ref")+".icc")
            definitions.append((f"u16-icc-{transfer}","u16",3,"linear",255,icc_path,"normalized"))
    report={"base":"6a48db924b338c5ae7095bea6105fb4dbd436e95","date":"2026-10-30",
            "rustc":run(["rustc","--version"]).stdout.strip(),"platform":platform.platform(),
            "header_revision":REV,"header_sha256":hashes,"cargo_lock":(work/"Cargo.lock").read_text(),
            "contracts":contracts,"containment":"512MiB address space, 20s CPU per trusted child; not production admission",
            "cases":cases}
    (work/"evaluation.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps({"cases":len(cases),"outputs":sum(len(c['outputs']) for c in cases),
                      "exact_core_vs_libjxl":sum(o['exact_bytes'] for c in cases for o in c['outputs']),
                      "max_error":max(o['max_abs_error'] for c in cases for o in c['outputs'])},indent=2))
    # Reproduce the pinned findings, including failures. This does not bless
    # half conversion or a missing tone map as production support.
    assert len(cases)==15
    for case in cases:
        for o in case["outputs"]:
            assert o["oracle"]["version"]==11002 and o["nonfinite"]==0
            if o["storage"]=="f32" and o["desired_intensity_target"]==0:
                assert o["exact_bytes"],case["name"]
            expected_mismatch=(case["name"].endswith("linear") and o["storage"]=="f16") or (
                case["name"] in ("u16-c3-pq","u16-c4-pq") and o["desired_intensity_target"]==80)
            assert o["exact_bytes"] != expected_mismatch,(case["name"],o["storage"])
            for path in ("legacy","ex","display"):
                assert (o["core"][path]=="accepted")==(case["name"]=="u16-sdr-control")
            if "original_icc_exact" in o:
                assert o["original_icc_exact"] and o["output_icc_exact"]
            if case["channels"]==4:
                assert o["max_abs_by_channel"][-1]==0
            if o["desired_intensity_target"]==0:
                assert o["target80_core_bytes_unchanged"]


if __name__=="__main__":
    main()
