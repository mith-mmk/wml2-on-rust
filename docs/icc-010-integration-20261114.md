# Published ICC 0.1.0 integration (2026-11-14)

Status: WML2 0.0.46 preparation only. No WML2 merge or registry publication.

The ICC backend is the published crates.io 0.1.0 artifact, SHA256
`b7f43fd73989d11494bf2c99530bbd0dad17b48d3de9604ae6819848324998e8`.
Cargo.lock records the registry source and this checksum; no path patch is used.
ICC release commit: `5de5e73145a1cbc87789d3bb47620e075e835a34`.

## Checks

macOS arm64, Rust 1.97.0; WML2 candidate `3c256281dc5a4557fed49e209696be13922ce81d`:

- `cargo test --locked -p wml2 --lib --tests --features 'color-management psd'`: 373 passed, zero failed, nine intentionally ignored. Expected CMYK backend pinned to transform.
- Focused lib/JPEG/PSD/TIFF/ICC tests with default transform requirement: 141 passed, zero failed, one ignored.
- No-default lib: 18 passed. Color-management disabled with PSD/multithread: 117 passed. Color-management+PSD+multithread lib/JPEG: 99 passed, one ignored.
- CI helper unit tests: 35 passed. `git diff --check` passed.
- Clean-commit `cargo publish --dry-run --locked -p wml2` passed; upload was aborted as intended.
- Dedicated CMYK CI passed Linux/macOS/Windows stable and Linux 1.91.0 on that commit. Full CI is tracked in [run 43531959831](https://github.com/mith-mmk/wml2-on-rust/actions/runs/43531959831); it is not claimed fully passing here.

## Real images

The same five opt-in local JPEG/TIFF/PSD inputs described in
[cmyk-validation](cmyk-validation-20261114.md) decode successfully. All five PNGs
are byte-for-byte identical to the pre-publication candidate outputs; all five
were visually inspected. No source images/profiles are added to Git.

Release-mode 11-decode medians, microseconds: CMYK text JPEG 642; tagged 500x333
JPEG 293155; CMYK TIFF 5695; ordinary PSD 4891; SWOP swatches 487. This run shared
the host with other builds, so it is not a controlled performance comparison.
The remaining cost of tagged CMYK conversion is material for interactive use.
Ordinary PSD is a compatibility sample, not an independent CMYK accuracy oracle.

## Toolchain compatibility correction

The full CI's wasm compile passed, then the JPEG XL runtime verification script
failed to locate miniz because CI Cargo 1.100 reports rlibs below
`release/build/<crate>/<hash>/out`, not always `release/deps`. The script now uses
Cargo-reported target rlibs and their dependency directories, selects only target
WML2/miniz artifacts, and requires exactly one of each. Cargo-reported host
proc-macro directories are also added for transitive metadata resolution. After this script-only
fix, actual Node wasm runs passed 236 cases each with EXIF off and on, with no host
imports. Final-head CI still needs to be read separately.

## Downstream and limits

Viewer can validate this branch with a fixed Git revision while publication is
pending. Production registry consumers need an explicit 0.0.46 requirement after
WML2 publication; 0.0.45 will not select it. Paintcore is deliberately deferred.
No exhaustive ICC profile/intent certification, new independent CMS oracle,
full application GUI/real-device run, or peak-memory measurement is claimed.
