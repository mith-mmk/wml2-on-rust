# Published jxl 0.8.0 migration evaluation

Decision for [#40](https://github.com/mith-mmk/wml2-on-rust/issues/40): **hold the
runtime upgrade**. Valid fixture decoding is promising, but 0.8.0 removes the
public parser APIs used to admit ICC and internal-layer allocations before
pixel decoding. Its public sample/profile-level limits do not replace WML2's
expanded-workspace, metadata and cumulative retained-layer contracts. This PR
adds development evidence, not a migrated decoder or new supported formats.
The runtime remains pinned to published `jxl =0.7.4` with default features off.

## Revisions and scope

Evaluation on 2026-10-30 in Linux x86_64 managed cloud, Rust 1.91.0:

- WML2 runtime source: main `6a48db924b338c5ae7095bea6105fb4dbd436e95`.
- Probe source: `5a1d8af2e7e9f8bda7f362f7b6febc6a9764d78b`; source hashes,
  fixture-manifest hashes, commands and measurements are in the
  [machine-readable evidence](jpeg-xl-08-evaluation.json).
- Published 0.8.0 crate source revision:
  `9a08ca643fd2d01f803dd0be6a5bebc9cc11e597` from `.cargo_vcs_info.json`.
  The [official release](https://github.com/libjxl/jxl-rs/releases/tag/v0.8.0)
  documents the event API, orientation and alpha changes. Upstream speed and
  memory claims are not treated as WML2 measurements here.
- [PR #32](https://github.com/mith-mmk/wml2-on-rust/pull/32) is independent:
  its cancellation branch and commit `da4f45764995110502a1a0a3cd9e933d86a5872f`
  are not included or changed. Its eventual contract must survive a migration.

## Reproduced compatibility boundary

Changing only the exact dependency version to 0.8.0 and running
`cargo check -p wml2 --no-default-features --features jpeg-xl,color-management,image-buffer-ex`
fails with **19 compiler errors** on the runtime source above. The temporary
manifest and lock changes were restored. The error classes include:

- `E0603`: `bit_reader`, `headers`, `entropy_coding` and their public 0.7.4
  header/TOC/ICC interfaces are private in the
  [0.8.0 library](https://github.com/libjxl/jxl-rs/blob/9a08ca643fd2d01f803dd0be6a5bebc9cc11e597/jxl/src/lib.rs).
- Removed `states` / `ProcessingResult`, and changed process signatures:
  [event-based decoder](https://github.com/libjxl/jxl-rs/blob/9a08ca643fd2d01f803dd0be6a5bebc9cc11e597/jxl/src/api/decoder.rs).

The event-loop rewrite is manageable. Removing preflight or moving its checks
after `BasicInfo` is not an acceptable workaround: source ICC has already been
read/expanded before that event, and output canvas dimensions do not bound the
storage of invisible reference layers. `scan_frames_only` is not proof of
bounded ICC, TOC permutation or cumulative layer allocations. The public basic
info also omits intrinsic-size information needed to preserve the current
explicit refusal. Reimplementing the pinned entropy/header parser locally or
maintaining a fork would be a separate maintenance and licensing decision.

Prefer an upstream bounded-admission interface exposing/checking the necessary
ICC and internal-frame storage costs before allocation. A narrowly maintained
fork exposing equivalent facilities is an alternative to evaluate explicitly;
this report does not select or ship one. Do not simply remove the caps or copy
a second decoder into WML2 to make the new version compile.

## Contract mapping for a future adapter

| Existing WML2 contract | 0.8.0 implications / remaining work |
| --- | --- |
| Input/container/metadata checks before publication | Retain WML2 envelope assembly and aggregate metadata checks; preserve trailing-data rejection and use `close_input`/`Complete` correctly |
| ICC pre-admission and retained source/reconstruction profiles | Need bounded profile admission before expansion; query the optional output profile after setting pixel format, not the source profile or an early default |
| Internal padded dimensions, signed extents, TOC complexity, retained layers and workspace | Current public header/TOC parsers are private; `sample_limit` and `max_profile_level` alone are insufficient |
| Ordinary / native EX / explicit sRGB output | Preserve depth/layout/rounding and refusal semantics separately; the prototype always uses RGBA16 and is not an EX adapter |
| Orientation applied once with source metadata | Set `adjust_orientation = true` explicitly; the option is now consumed. The eight independent orientation probes pass |
| Straight same-depth alpha and transparent RGB | Keep `AlphaMode::AsIs` for the currently admitted inputs; new alpha modes do not automatically authorize associated/mixed-depth/subsampled input |
| Explicit Level 5/10 containers within WML2 quotas | Audit new `ProfileLevel` enforcement and interaction with WML2's codestream extraction; do not blindly use the new Main5 default or treat Main10 as a memory budget |
| No preview, animation, intrinsic-size, HDR/float or general extras | Preserve explicit rejection; defaults such as `skip_preview` must not silently extend support |
| PR #32 cooperative cancellation | Fallible sequential `JxlParallelRunner` remains public. Port both runner overrides and typed error mapping; then rerun its real-codec, ICC-batch and no-publication tests. No migrated WML2 cancellation test is claimed here |
| Feature/MSRV/platform policy | 0.8.0 declares Rust 1.89; both cores compile with WML2's Rust 1.91. Keep `default-features = false`, including no implicit SIMD or Brotli. WASM compile succeeds; runtime platform evidence is still required |

## Executed regression and oracle evidence

Unchanged 0.7.4 WML2 ordinary/EX/sRGB targets pass **53 tests** (25 unit,
22 original JPEG XL, 3 additional, 3 internal-layer). These include rejection,
truncation, budgets and callback Abort. This is baseline evidence, not a claim
that those admission tests pass with an unimplemented 0.8 adapter.

The reproducible development-only probe runs **64 successful comparisons**:
32 cases for each published core, directly against existing independent
libjxl/djxl 0.11.2 and LittleCMS 2.16 references, never one decoder against itself.
Coverage is 16 base integer/ICC/lossy cases, 8 orientations, 2 normalized 9/10-bit
cases, and 6 sRGB cases including official JPEG-derived and internal-layer input.
The 0.8 event wrapper explicitly selects oriented, high-precision RGBA16,
straight supported alpha and Main10 allowance for this trusted fixture set.

Native lossless/normalization/orientation comparisons allow one u16 unit;
lossy comparisons retain the existing 32-u16 or 2-u8-scale tolerance. Managed
RGB/Gray uses the actual reconstruction ICC and the own ICC engine with alpha
excluded, then RGBA8 rounding; maximum observed managed difference is 1/255.
This managed probe is a separate validation implementation, not a new WML2 API.
Dimensions are checked. The fixtures include alpha and invisible RGB.

Both cores and the probe compile for `wasm32-unknown-unknown`, with default
features off. This is **compile evidence only**: no new Node/browser, Windows,
macOS or private-corpus execution is claimed. The existing historical browser
records remain separate. A migrated WML2 must still run all ordinary/EX/sRGB,
metadata/refusal/limits and PR #32 contracts on native and WASM/browser targets.

## Measured core cost

Three alternating process pairs pinned to CPU 0; median of process medians.
Small 7x5 input uses 150 iterations, larger inputs 13, with the first 3 omitted.
The same binary links both cores and instruments the System allocator. These
measure **core RGBA16 decoding plus reconstruction-profile extraction**, excluding
WML2 input assembly, admission, callbacks and CMS. They are not before/after
WML2 end-to-end performance or safe-limit measurements. All three measured
workloads have identical output hashes between cores.

| Trusted fixture | 0.7.4 median ms | 0.8.0 median ms | Allocated bytes 0.7.4 / 0.8.0 | Peak extra live bytes 0.7.4 / 0.8.0 |
| --- | ---: | ---: | ---: | ---: |
| ICC lossless 7x5 | 0.055294 | 0.057937 | 4,316,259 / 121,935 | 4,204,459 / 89,825 |
| JPEG-derived bench 606x500 | 17.953463 | 11.904633 | 12,711,617 / 4,000,479 | 7,015,766 / 3,767,489 |
| Layered sunset 924x1386 | 163.254751 | 142.834865 | 47,989,983 / 39,430,815 | 39,336,322 / 39,301,704 |

The small ICC case is slightly slower; larger cases are faster in this local
probe. Requested allocator bytes exclude stack, allocator bookkeeping, mapped
memory and process RSS. Peak is additional live requested bytes relative to the
pre-decode baseline. In particular, the layered peak barely changes despite
lower total allocation traffic: no reduced workspace cap follows from this table.
Finite fixtures and successful decodes cannot establish bounds for hostile inputs.

## Reproduction and next work

```console
cargo test -p wml2 --no-default-features --features jpeg-xl,color-management,image-buffer-ex --lib --test jpeg_xl --test jpeg_xl_additional --test jpeg_xl_layers
python3 wml2-test/scripts/evaluate_jxl08.py --output .test-jxl08
# Only for an otherwise idle machine; Linux CPU pinning for the recorded method:
taskset -c 0 python3 wml2-test/scripts/evaluate_jxl08.py --output .test-jxl08 --bench
cargo check --target wasm32-unknown-unknown --manifest-path .test-jxl08/Cargo.toml
```

The script creates an isolated temporary manifest with exact published core
versions and builds the separate [Rust probe](../wml2-test/scripts/jxl08_probe.rs).
It checks input hashes from the existing fixture manifests; `report.json` and
per-process CSVs are written under the requested `.test*` directory. Run with
Rust/Cargo available and install the WASM standard target before the compile
check. The probe is restricted by convention to these trusted fixtures and
provides **no WML2-equivalent admission** for arbitrary files.

1. Resolve bounded ICC/header/layer admission with upstream or a reviewed fork.
2. Port event state and exact supported-input semantics; integrate the eventual
   #32 cancellation contract without taking that branch as an accidental base.
3. Run the complete existing WML2 oracle, refusal, limits, metadata and platform
   matrix on the real migrated adapter. Only then compare end-to-end performance
   and reassess the conservative budget design in #37.
4. Stage later capabilities separately: #41/#50 metadata, #42 alpha/extras,
   #43 animation, #44 preview, #45 intrinsic dimensions (aligned with #35),
   #46 HDR and #47 incremental output. #48 JPEG reconstruction and #49 encoding
   require separate backend feasibility; 0.8.0 remains a decoder dependency.

No runtime pin, public WML2 API, security cap or support boundary is changed by
this evaluation. No merge, release or issue closure is implied.
