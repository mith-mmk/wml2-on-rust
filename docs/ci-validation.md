# CI validation and cost controls

`Required checks` remains the stable aggregate check. No workflow-level path
filters are used: the selector and aggregate always report a result. Failure,
cancellation, missing results and an unexpectedly skipped job fail the aggregate.
Only explicitly unselected jobs may skip. Branch protection is not modified.

## Change selection

`.github/scripts/select_ci.py` classifies the merge-base-to-head diff, including
both sides of renames, with `full > smoke > none` independently for AVIF,
JPEG2000 and JPEG XL. Failure to classify a path, parse a manifest/lockfile or
obtain the diff selects the full suite. Selection does not parse Rust text hunks.

| Change | Optional codec checks |
| --- | --- |
| `wml2/src/jpeg2000*.rs`, `wml2/tests/jpeg2000*` | Full JPEG2000 |
| AVIF adapter/encode paths and AVIF tests | Full AVIF |
| `wml2/src/jpegxl.rs`, `wml2/tests/jpeg_xl*` | Full JPEG XL |
| `wml2/src/jpeg/`, `wml2/tests/jpeg_*` | JPEG IDCT variants and TIFF's embedded JPEG; optional codecs none |
| TIFF implementation/tests, `highres/tiff.rs`, `metadata/exif.rs` | JPEG, PNG, TIFF and default WebP/EXIF coverage; optional codecs none |
| `wml2/src/png/`, `wml2/tests/png_*` | PNG/default/container coverage; optional codecs none |
| `wml2-test/examples/*.rs` | Compile examples and smoke optional codecs |
| Recognized documentation only | Optional codecs none |
| Shared code, API/type definitions, feature/dependency graph, CI, fixtures, unknown paths | Full suite |

`error.rs`, `metadata.rs`, `draw.rs`, `limits.rs`, `decode_guard.rs`,
`image_buffer_ex.rs`, `lib.rs`, shared high-resolution types and color processing
stay full. `tiff/header.rs` and `tiff/mod.rs` also stay full: the public TIFF
metadata types are embedded in the shared `DataMap` enum. A JPEG-only method
added to a shared file does not get an unsafe text-based exemption. A future
refactor into a dedicated codec file can make that boundary narrower.

The TIFF IFD reader is used for JPEG/PNG/WebP EXIF; those formats remain covered
when it changes. JPEG2000 uses its own JP2 boxes and ICC parser, not that reader.
Its `ImgError::is_unsupported` fallback and decode/metadata budgets remain shared
code and therefore trigger full tests when modified. Feature expansion also
honors optional codecs introduced into default features by future changes.

The existing core feature sets always run, as do compatible-feature/all-target
checks, expected IDCT conflict rejection, MSRV/stable tests, cross-target compile
checks, formatting and the platform baseline. Every original full matrix row is
retained; the Linux matrix is partitioned into core and AVIF jobs. They share the
same test/Clippy driver. Core jobs do not depend on FFmpeg preparation.

## Smoke and optimized tests

The smoke job runs on MSRV and stable. It reuses existing tests for small JP2/J2K
inputs, pixel oracles, limits, unsupported input rejection, callbacks, ICC
retention and EX precision conversion/transactional abort. AVIF uses the bounded
ICC/alpha regressions; JPEG XL uses its regular integration contracts. Exact
test filters must execute one test: a misspelled filter returning zero tests is
an error. Full corpus, corruption and independent pixel-oracle checks are still
run for directly affected codecs and all full-suite events.

The `ci` Cargo profile inherits `test`, sets `opt-level = 1`, and explicitly keeps
debug assertions and overflow checks enabled. JPEG2000 full matrix/platform
tests and smoke tests use it. It does not change published builds or the release
profile. Rust caches are separated by feature/profile and compiler; CI package
and Rust cache writes are restricted to trusted main pushes. Smoke can restore
the full JPEG2000 EX dependency cache.

## FFmpeg and AVIF oracles

One read-only job per run prepares the seven existing AVIF oracle fixtures.
Every full AVIF matrix job downloads the same run's data artifact. There is no
cross-run fixture cache, no third-party executable download and no use of this
artifact by release publication jobs.

The encoder rows also need FFmpeg at test runtime. They probe existing tools;
when needed, they install the producer's same-run `.deb` artifact through apt's
signed-index verification, downloading only any missing/different dependencies.
Both producer and consumers use Ubuntu 24.04 amd64. Decoder-only rows avoid this
runtime preparation. The independent encoder/input and encoded-sample/decode
oracles remain enabled; CI sets `WML2_REQUIRE_FFMPEG` so unavailable tools or a
failed encoder cannot silently pass by skipping an external oracle. The bundled
WebP sample test follows the same requirement when its encoder+WebP feature
combination is enabled; this change does not expand the original full matrix.

The runner's FFmpeg and ffprobe are tested before apt is considered. The probe
executes the generator's libaom options, verifies a 16x16 AV1/gbrp stream, and
checks actual gbrp/RGBA raw output lengths. A successful version listing alone
does not qualify a tool. The real fixture generator must still succeed.

Fallback is limited to Ubuntu 24.04 amd64 and official Ubuntu HTTPS repositories
with the Ubuntu archive signing key. It resolves and pins the distro FFmpeg
candidate before installation. Only `.deb` files are cached, with an exact key
covering the distro, architecture, runner image, FFmpeg version, dependency
transaction (including package URIs/checksums), preparation and probe scripts.
There are no prefix restore keys. Each consumer independently obtains the
package SHA256/size from fresh, isolated signed Ubuntu indices and discards
unrequested, same-size-corrupt or otherwise mismatched cached archives before
apt can install them. It does not trust a producer-supplied checksum manifest.
Apt validates newly downloaded packages against signed indices,
installs missing runtime dependencies, and the installed version and actual
operations are checked again. PRs may restore trusted main caches but never save
them. No credentials or apt configuration files are cached.

Index refresh is bounded to 180 seconds. Each installation is bounded to 480
seconds with a 20-second request timeout and one network retry. A failed cached
transaction discards only this workspace's cached `.deb` files and retries once.
The timeout covers child processes. A failed download, missing library, invalid
probe or failed oracle generation fails the job; it never becomes a successful
skip. The fallback uses official archive/security hosts rather than the slow
Azure mirror observed in the investigated run.

## Full runs and release verification

- Directly affected codecs and shared/unknown changes: full in PR checks.
- Main pushes: full, preserving checks for every main commit.
- Manual `checks` workflow dispatch: full on the selected ref.
- Weekly/manual `bounded extended checks`: full reusable checks, plus the
  existing PNG mutation and Miri jobs.
- Release tags: `release.yml` waits for full reusable checks on the exact tag
  commit before builds/publication. A prior PR result is not substituted.

The pre-existing feature-branch push/PR duplication remains removed. Only
superseded PR runs are cancelled; main, manual, weekly and release runs are not.
Release permissions, secrets, branch protection and publication behavior are
unchanged. Re-run full checks before a manual crates.io publish; this change
does not add or authorize registry publication.

## Local verification

Run from the repository root:

```powershell
python -B -m unittest discover -s .github/scripts -p 'test_*.py'
actionlint -shellcheck= -pyflakes=
$env:CARGO_TARGET_DIR = "$PWD/.test-target"
cargo test -p wml2 --locked --profile ci --no-default-features --features jpeg2000,image-buffer-ex --lib --tests
python -B .github/scripts/run_codec_smoke.py --modes '{"jpeg2000":"smoke","avif":"smoke","jpeg-xl":"smoke"}'
```

Actionlint checks workflow expressions/schema; shellcheck and pyflakes can be
enabled when installed. Local Windows tests cannot establish hosted Linux apt
cache behavior or the end-to-end workflow wall-time reduction. Record actual
measurements separately from estimates; do not infer a speedup from different
runner images, queue times or test subsets.

### Implementation validation on NERO

Windows `x86_64-pc-windows-msvc`, Rust 1.99.0. The full
`jpeg2000,image-buffer-ex --lib --tests` configuration passed with `--profile ci`.
All seven exact JPEG2000 smoke cases passed, as did the AVIF color-boundary
suite (10 tests), JPEG XL integration suite (22 tests), and an EX conversion
smoke case with Rust 1.91.0. The real installed FFmpeg passed the expanded probe.
Python selector/gate/probe/recovery tests and actionlint 1.7.12 also passed.
The independent AVIF encoder suite (8 tests) and bundled WebP sample oracle
(1 test, enabled explicitly for local validation) passed with FFmpeg required.
An intentionally missing FFmpeg executable correctly failed the input oracle
with exit 101 instead of passing by skipping it.

One sequential local comparison of the **same** existing test:

| `code_block_geometry_matches_oracle --exact` | Test execution | Wall time including build |
| --- | ---: | ---: |
| `--profile test` | 158.64 s | 163.859 s |
| `--profile ci` | 13.13 s | 13.282 s |

Both runs passed the same independent pixel/geometry oracles and resource-limit
assertions. These are single Windows-local samples, not a measured hosted CI
speedup. The optimized artifact was already built; the baseline build took
5.04 seconds. Existing full runs require 12 Linux AVIF feature/toolchain rows;
fixture generation now occurs once rather than 12 times. Queueing, Linux apt/cache
recovery, artifact fan-out and total hosted workflow duration still require an
actual Actions run. No workflow was manually dispatched or monitored for this
work; ordinary push/PR events may start checks.
