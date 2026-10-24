"""Build two immutable JPEG decoders into one diagnostic process.

Run from any directory: script .test-new-directory --before SHA --after SHA.
Requires cached Cargo dependencies (--offline). Production sources are untouched.
"""
import argparse
import io
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
parser.add_argument("--before", required=True)
parser.add_argument("--after", required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[4]
output = args.output.resolve()
if not output.name.startswith(".test") or output.exists():
    parser.error("Choose a new directory whose name starts with .test")
output.mkdir(parents=True)
for label, requested in (("before", args.before), ("after", args.after)):
    revision = subprocess.check_output(
        ["git", "rev-parse", "--verify", "--end-of-options", requested + "^{commit}"], cwd=root, text=True,
    ).strip()
    assert re.fullmatch(r"[0-9a-f]{40,64}", revision)
    destination = output / f".test-{label}"
    archive = subprocess.check_output(["git", "archive", revision, "wml2/src", "wml2/Cargo.toml"], cwd=root)
    with tarfile.open(fileobj=io.BytesIO(archive)) as source:
        for member in source.getmembers():
            if not member.isfile():
                continue
            relative = Path(member.name).relative_to("wml2")
            assert ".." not in relative.parts
            target = destination / relative
            if not target.resolve().is_relative_to(destination.resolve()):
                raise ValueError(f"Archive path escapes the output directory: {member.name}")
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(source.extractfile(member).read())
    manifest = (destination / "Cargo.toml").read_text()
    # Distinguish the two path packages; preserve features/dependencies/library code.
    manifest = manifest.replace('name = "wml2"', f'name = "wml2-{label}"', 1)
    manifest = re.sub(r"(?ms)^\[\[(?:test|example|bench|bin)\]\].*?(?=^\[|\Z)", "", manifest)
    (destination / "Cargo.toml").write_text(manifest)
(output / "Cargo.toml").write_text('''[package]
name="jpeg-dual-revision-bench"
version="0.0.0"
edition="2024"
[workspace]
[profile.release]
opt-level=3
[[bin]]
name="jpeg-dual"
path=".test-main.rs"
[features]
multi=["before/multithread","after/multithread"]
[dependencies]
before={package="wml2-before",path=".test-before",default-features=false,features=["jpeg","idct_llm","exif"]}
after={package="wml2-after",path=".test-after",default-features=false,features=["jpeg","idct_llm","exif"]}
''')
(output / ".test-main.rs").write_text('''use std::{hint::black_box,time::Instant};
fn before(bytes:&[u8])->Vec<u8>{before::draw::image_from(bytes).unwrap().buffer.unwrap()}
fn after(bytes:&[u8])->Vec<u8>{after::draw::image_from(bytes).unwrap().buffer.unwrap()}
fn main(){
 println!("file,cycle,step,revision,decode_ns");
 for file in std::env::args().skip(1){
  let input=std::fs::read(&file).unwrap();let expected=before(&input);
  for _ in 0..2{assert_eq!(before(&input),expected);assert_eq!(after(&input),expected);}
  for cycle in 0..8{
   let order=if cycle%2==0{[0,1,1,0]}else{[1,0,0,1]};
   for (step,revision) in order.into_iter().enumerate(){
    let start=Instant::now();let output=if revision==0{before(black_box(&input))}else{after(black_box(&input))};
    let ns=start.elapsed().as_nanos();assert_eq!(output,expected);
    println!("{},{cycle},{step},{revision},{ns}",std::path::Path::new(&file).file_name().unwrap().to_string_lossy());
   }
  }
 }
}
''')
env = {**os.environ, "CARGO_TARGET_DIR": str(output / ".test-target")}
suffix = ".exe" if os.name == "nt" else ""
for mode in ("multi", "single"):
    command = ["cargo", "build", "--release", "--offline", "--manifest-path", str(output / "Cargo.toml")]
    if mode == "multi":
        command += ["--features", "multi"]
    subprocess.run(command, env=env, check=True)
    shutil.copyfile(output / ".test-target" / "release" / f"jpeg-dual{suffix}", output / f".test-dual-{mode}{suffix}")
print(f"Built single/multi diagnostics in {output}")
