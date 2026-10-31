# Bounded embedded-preview skipping for JPEG XL

The ordinary, EX and explicit sRGB APIs return the **main image** from supported
still codestreams with an embedded preview. Preview presence alone is no longer
an error. No preview image is substituted for the main image, no preview callback
is published, and preview is not treated as animation or an additional output
frame. Output precision, alpha, ICC and orientation contracts remain unchanged.
`jxl =0.7.4` stays pinned; no dependency, public image API or metadata key is added.

## What is supported

One terminal visible preview frame is structurally checked and skipped before
the existing main sequence is scanned/decoded. The main sequence still permits
at most 64 internal layers and one visible still image. Animated files and
intrinsic display size remain separate unsupported cases in this branch.
There is no first-frame animation fallback, progressive-output change, preview
extraction API or new dimension-information API (#45/#57).

All file-global colour/depth/extra-channel/orientation rules remain enabled:
integer 8/9/10/12/16; Gray/RGB with at most one same-depth full-resolution straight
alpha; existing HDR/ICC restrictions. JPEG XL preview/main share these global
properties; they do not have independent global ICC profiles or channel models.
The fixtures deliberately have different dimensions, RGB/Gray sample content
and alpha values. Gray+alpha still uses the existing RGBA EX layout; this change
does not introduce a two-channel EX format. Source header bits, including preview
information and original orientation, remain in `JPEG XL Source Header`.

## Why the pinned codec is safe to skip only this subset

Published jxl0.7.4 has `JxlDecoderOptions.skip_preview`, default true; WML2 sets
it explicitly. Its codestream parser still reads the preview frame header and
TOC. For a visible preview it selects `ProcessMode::Skip(false)`, bypasses
`Frame::from_header_and_toc` and the pixel/reference decoder state, and advances
through section byte ranges. A terminal preview clears state before the main
frame and does not produce a public frame notification.

However, a non-visible preview overwrites that decision with `SkipOutput`,
which **does allocate/decode pixel state**. Therefore WML2 rejects non-visible or
non-terminal preview headers before the codec runs. This is a checked subset,
not a claim that every possible preview sequence is supported. A future broader
contract must account for those internal layers instead of assuming that setting
`skip_preview=true` always prevents their pixel allocations.

Audited code at the crate's recorded revision
[`624ce908`](https://github.com/libjxl/jxl-rs/blob/624ce908afcf8eb0dc2585a671535eaabb5d88cc/jxl/src/api/inner/codestream_parser/mod.rs)
and [frame construction/section skipping](https://github.com/libjxl/jxl-rs/blob/624ce908afcf8eb0dc2585a671535eaabb5d88cc/jxl/src/api/inner/codestream_parser/frame_info.rs).
This conclusion is about actual published jxl0.7.4, not equivalence with C libjxl
or an assumption that upgrading to0.8 preserves the bounded preflight (#40).

## Bounds and failure behaviour

The existing header/frame/TOC preflight now has a preview phase with its own
nonserialized dimensions. It reuses the same checked size, crop-coordinate,
TOC entry/permutation and complete-payload extent checks as main layers.

| Resource | Rule |
| --- | --- |
| Input | Existing whole-file/caller limit and fixed64 MiB cap; all container assembly/auxiliary validation stays enabled |
| Preview dimensions | Pinned header parser max4096 per axis; independently require `max(width,16) * max(height,16) <=4 Mi` both for the declared preview and actual padded frame geometry |
| Main output | Caller `pixels`, `frames` and `animation_bytes` continue to describe the main canvas: a 32x32 preview of a 2x3 main still works with pixels=6, frames=1, animation_bytes=24 |
| Header/TOC | Existing64 KiB parse windows, max4096 TOC entries, checked sums/offsets and conservative8 MiB permutation allowance before parsing; one supported preview plus at most64 main layers |
| Combined workspace | Existing input/profile/header/main-layer base and TOC allowances share `expanded_bytes`; no preview f32 plane/retained-image allowance is invented because this path does not construct those planes |
| Codec sample guard | The maximum of declared/actual preview padded area and main scanned layers supplies the pinned codec's sample limit, which still checks previews even while skipping; the fixed per-image limits are not disabled |
| Metadata | Source header/profile/source-box budgets remain enabled; no preview pixel buffer or new preview metadata structure is retained |

The conservative TOC allowance may remain reserved in the combined high-water
estimate after a preview TOC has been dropped; this does not relax a cap.
Arithmetic overflow, out-of-range/missing preview sections, missing/truncated main,
unsupported preview shape and limit failures stop before initialization, drawing
or metadata publication. Existing begin-image/Abort lifecycle remains unchanged.
Tests verify one initializer and one draw for successful main decoding, no
callbacks on preflight errors, and termination without preview/metadata output
when the caller aborts initialization. This is a known-buffer/request admission
contract, not process RSS or every allocator/callback allocation guarantee.

**Skipped entropy is opaque.** Headers, TOC and byte extents are validated, but
preview pixel entropy, semantic content and reconstruction are not decoded or
fully validated. A deliberately damaged preview payload can therefore be ignored
while the main remains readable. Independent C libjxl main-only decoding agrees;
requesting that same damaged preview from C libjxl fails. This distinction is
explicit rather than claiming full-file pixel validation.

## Preview extraction and cancellation

Preview retrieval is deferred. The pinned codec can expose preview decoding when
skipping is disabled; this document does not claim that all upstream preview
extraction is unavailable. WML2's current APIs describe a main-image result, so a
separate extraction API needs a separate dimensions/layout/colour/alpha/limit and
failure contract. Returning its first frame through a main-image API would be
incorrect. No such API is necessary for this main-reading fix.

PR #32's cancellation API is not copied into this main-based branch. Its existing
safe-point contract was verified on an isolated combination: preview preflight,
header/TOC parsing and serial section skipping are still **non-interruptible**;
there is no preview pixel task in which to run a probe. Cancellation is observed
at existing surrounding stages/runner tasks/CMS batches. No bounded-latency or
mid-skip interruption promise is added. Existing input caps still bound accepted
bytes. Parent PR #32 and this branch remain separate and unchanged by that test.

## Independent fixtures and local verification

`generate_jpegxl_preview_fixtures.py` uses C libjxl0.11.2 to encode standalone
frames, then inserts a preview header and those independently encoded frame
payloads. Explicit field locations belong to fingerprinted fixtures, not a new
runtime serializer. C libjxl then decodes **both** preview and main and verifies
that main-only skip yields identical main pixels. The encoded data and expected
samples never come from WML2 or a second call to its decoder. Independent
LittleCMS2.16 converts C's normalized16-bit main samples using its actual output
profile to WML2's saved sRGB profile, before one final8-bit rounding.

Six cases cover RGBA8, embedded-ICC RGBA16, orientation6 with ICC, Gray+alpha8,
a preview larger than the main, and a64x48 lossy XYB main with8x8 preview.
The equivalent no-preview lossy C source is retained for performance/display
comparison. The 4096x4096 declaration is a **header-admission negative**, with a
small body; it is not misrepresented as an independently valid oversized-pixel
encoding. Provenance, hashes and exact frame boundaries are in
`wml2/tests/fixtures/review/jxl-preview-fixtures.json`.

```sh
python -B wml2-test/scripts/generate_jpegxl_preview_fixtures.py \
  --include <libjxl-0.11.2-include-directory> --work .test-preview
cargo test -p wml2 --locked --no-default-features \
  --features jpeg-xl,image-buffer-ex,color-management \
  --lib --test jpeg_xl --test jpeg_xl_additional --test jpeg_xl_layers
python -B wml2-test/scripts/verify_jpegxl_wasm.py --output .test-preview-wasm
```

Preview tests live inside the existing `jpeg_xl` integration target, so its
existing Linux/MSRV/stable, macOS, Windows/32-bit Windows and compatible-feature
CI commands exercise them without a separate unselected test target. The actual
Node WASM harness adds15 cases (76 total). Its compressed expected pixels use
the existing Cargo miniz artifact explicitly, not Rust std's private miniz crate.
See [verification evidence](jpeg-xl-preview-validation.json) for exact tested
commits, feature counts, WASM/source hashes and performance/display limitations.

## Local results (2026-10-30)

Linux x86_64 / Rust1.91.0: 59 expanded native tests pass (25 unit,28 original
integration including6 preview tests,3 additional,3 layer). Minimal JXL without
EX/CMS passes45; disabled detection passes1. Actual Node24.19.0 WASM passes76
with no host imports. Scoped Clippy has the same11 existing warnings and no new
preview diagnostic;35 repository CI selection/release tests pass. Formatting and
local links are checked. The64x48 managed export was visually inspected without
corruption; every independent reference byte differs by at most1/255 and alpha
is exact.

An isolated combination of PR #32 `da4f457` plus source checkpoint `de79d21`
passes65 native tests. The standalone checker
`wml2-test/scripts/check_jpegxl_preview_cancel.rs` checks the independent C/CMS
reference and cancels at every observed probe position for RGBA8 (17), oriented
ICC16 (17), and lossy XYB (52):86 deterministic points, no returned image after
cancellation, and pre-cancel versus limit-error precedence. To reproduce, apply
that source checkpoint in a detached PR #32 worktree and build the checker as an
isolated Cargo binary depending on that worktree's wml2 with
`jpeg-xl,image-buffer-ex,color-management`, plus `miniz_oxide=0.7`. Pass the
preview fixture directory as its only argument. These check surrounding safe
points; they do not measure or claim interruption inside preview scanning/skipping.

Release measurements compare unchanged main `6a48db9` and source checkpoint
`de79d21`, identical `jpeg-xl,image-buffer-ex,color-management` features and
registry versions/checksums verified against the repository Cargo.lock. Three
alternating process pairs are pinned to CPU0, with3 warmups and resident inputs.
Small cases use150 timed calls; the ordinary/managed large bench use40/5.
Medians of process medians:

| Existing path | Before µs | After µs |
| --- | ---: | ---: |
| RGBA16 ordinary | 55.329 | 54.117 |
| Small ICC sRGB | 131.378 | 134.077 |
| 606x500 ordinary | 16293.601 | 16226.035 |
| 606x500 sRGB | 1058690.873 | 1061808.937 |

All full-buffer hashes match. Changes span -2.19% to+2.05% in these local samples;
this is not a general throughput or guaranteed-zero-overhead claim. The managed
large case has only5 timed calls per process. In the candidate, the independently
identical64x48 C main without/with its8x8 preview measures635.138/636.585 µs
ordinary (300 timed calls/process) and10974.429/10836.181 µs sRGB (50 calls/process),
again three alternating pairs with identical output hashes. Individual process
medians and binary/lock hashes are retained in the machine-readable evidence.
No process RSS bound or platform-independent timing claim is made.
