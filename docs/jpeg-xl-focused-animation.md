# Bounded JPEG XL animation

The original-profile RGB8/16 base shipped in WML2 0.0.42 through [#103](https://github.com/mith-mmk/wml2-on-rust/pull/103),
based on the 0.8 migration [#102](https://github.com/mith-mmk/wml2-on-rust/pull/102)
(`282056388c3d4c442be9f7275d102128e807972e`).
Transplants #93/#94's animation processing, not their preview/HDR/extra-plane or
public resource-policy dependencies. No encoder implementation or changes.

The working-tree follow-up adds XYB RGB and SDR Gray/Gray+alpha; these additions
are not in the published 0.0.42 package. See [Gray support](jpeg-xl-gray-animation.md)
and [SDR integration validation](jpeg-xl-sdr-validation.md).

## Contract

Common image/callback APIs decode admitted Gray/RGB8/16 (original-profile or XYB) with optional
matching straight full-resolution Alpha. Codec coalescing applies crop/reference,
blend and orientation once. Each displayed frame is a full canvas; invisible
layers consume the unchanged 64-layer cap, not playback frames. Loop zero means
indefinite repetition. The poster is frame0, not an extra playback delay.

The existing callback clock is integer milliseconds. Unrepresentable fractional
waits are refused before init; long waits use checked u128/u64 arithmetic.
`animation_rgba16_from_with_limits_and_cancel` preserves exact rational ticks
with coalesced native-endian RGBA16 and source/output ICC. No rounding or playback
policy is invented. Visible final-zero frames remain visible with zero delay.

EX callbacks declare Rgba16 LE, retaining original coded depth in metadata. The
collector exposes immutable `AnimationFrameEx` through `animation()` and raw
`loop_count()`. Existing precision policies still apply. Static picking/conversion
refuses animated state rather than losing frames. This adds no encoder.
Explicit `image_from_srgb*` uses the existing CMS per frame, excluding alpha.
Raw reconstruction coordinates/profile tagging and sRGB output remain distinct.
XYB reconstruction stays in `jxl`; output ICC can differ from source ICC (for
example, embedded ICC XYB falls back to sRGB). Consumers must use output ICC.

All decoding, final stream validation, CMS and output preparation precede
init/draw; owned errors/cancellation expose no partial image. Callback side effects
remain incremental. Existing Abort is stop-success, with exactly one terminate;
callback errors propagate. Identified animation failures terminate before init.
Earlier container/header failures retain the existing begin-only behavior.

Existing input/dimension/pixel/TOC/Main5/120fps caps and DecodeLimits remain.
Frame count, source canvases, prepared output, collector poster/frames, metadata
and scratch are admitted cumulatively with checked arithmetic and fallible
buffer reservations. A private operation-local allowance uses expanded_bytes;
there is no new public resource policy, codec heap tracking or RSS guarantee.

Preview-bearing animation, timecodes, HDR, associated/subsampled or mixed-depth
Alpha and general extra channels remain explicit refusals. These are limitations
of this bounded animation profile, not features silently converted or discarded.
The old large-image restriction remains; it is not a prerequisite for this series.

## Initial validation (bde4518)

112 enabled tests passed across library, EX collector/contracts, still JXL,
additional JXL and seven animation tests. Minimal jpeg-xl-only: six animation
tests passed. Feature-off target compiles with zero animation tests. Existing
independent C/djxl/LCMS oracles cover all canvases, variable/fractional/long/final-zero
timing, loops, crop/reference/alpha blend, orientation, ICC and sRGB. 64-layer
success/65-layer refusal, truncation at every byte, every callback Abort/error
boundary, cooperative cancellation checkpoints, cumulative output boundary and
actual allocation refusal are checked. Decoder outputs were rendered and inspected.
No fresh C-oracle generation is claimed: frozen goldens are replayed with source
commit and hashes in [oracle manifest](jpeg-xl-focused-animation-oracle.json).

Actual wasm32/Node24.19 passed 236 cases, including native rational animation,
common callbacks, EX, cancellation and frame limits. Other native 32-bit platforms
are CI coverage, not locally executed claims.

Three balanced release benchmark blocks use the same Rust1.91/source path/features
and existing allocator-meter harness. Relative to migration-only, still medians
change by about 0–2%; recorded peak requested allocations are unchanged on those
cases. Variable ICC16 animation takes about 0.182ms and peaks at about 4.20MB on
this host. Against main, photo peak decreases 5,995,746 → 4,382,783 bytes; tiny RGB
peak increases 64,222 → 71,912. Requested bytes are observed allocation deltas, not
RSS or an allocation contract. No speed claim is made for all inputs.
Raw trials: [validation JSON](jpeg-xl-focused-animation-validation.json).

These results describe the original implementation checkpoints, not a fresh
release-tag CI run. See [0.0.42 release notes](releases/0.0.42.md) for release
scope and validation. The separate XYB-animation follow-up #104 is not included.

## PR103 review: resident metadata during nested decode

The common path retains its reader and expanded container metadata while the
native animation decoder unpacks the container again. Previously their existing
allowance was claimed only after native decoding, leaving that peak unaccounted.
It is now deducted from expanded_bytes before nested input assembly, Brotli,
preflight and pixels, then carried into the returned private allowance exactly
once. Native-only calls reserve zero external bytes. No public API/cap changes.

New tests reject before unpack at the assembly boundary, exercise exact quota
transfer, overflow-sized reservations, cancellation precedence and recovery. A
four-brob container retains all four expanded payloads across successful decode;
the exact workspace boundary, failure before image publication, truncation and
retry are checked. Current verification: 36 library + 6 brob + 8 animation tests;
minimal jpeg-xl-only 29 library + 7 animation tests; actual WASM 236 cases passed.
All three rendered sRGB frames retain their inspected hashes. Three alternating
release blocks against bde4518 give median ratios 1.024 (plain) / 0.927 (metadata),
with identical pixels and observed allocation peaks. Shared-host variation does
not establish a speed improvement. Raw trials are in the validation JSON.

## XYB animation follow-up

XYB is now admitted through the existing bounded RGB8/16 animation path. The
0.7.4 preflight's `!xyb_encoded` is cross-checked against the 0.8 codec's
`uses_original_profile`, rather than requiring it to be true. No codec code,
public API, hard cap or allocation policy changes. XYB reconstruction and
coalescing remain in jxl 0.8.0. Source and output ICC are retained separately;
CMS continues to use the reconstruction/output ICC and excludes alpha.

Validation on Windows / Rust 1.99.0:

- The two reported failures, libjxl conformance `animation_icos4d` and
  `animation_spline`, now decode all 48 and 60 frames, with exact 50ms and 20ms
  waits and infinite loops. Native RGBA16 (rounded for the independent 8-bit
  oracle), ordinary, EX and explicit sRGB paths match every reference canvas:
  RGB tolerance one 8-bit unit, alpha exact. The references are generated with
  ImageMagick 7.1.2-21 / libjxl 0.11.2, not the Rust decoder. Provenance,
  source hashes and reference hashes are in
  [the oracle manifest](jpeg-xl-xyb-animation-oracle.json). The adjacent
  `generate_jxl_xyb_animation.py` reproduces the compressed raw references.
- New coverage also verifies frame/output/workspace/metadata/pixel limits,
  truncation, trailing bytes, cooperative cancellation, recovery, and no
  callbacks before failure. 115 existing focused library/JXL tests plus five
  new tests passed. Minimal jpeg-xl-only: 11 animation tests passed; the new
  feature-off test target compiles and runs zero tests.
- Rendered frames 0/12/24 (icos4d) and 0/20/40 (spline) were inspected: geometry,
  color and animation changes are readable, without visible corruption.
- Five alternating release benchmark blocks against main `4e6e6baa`, 200
  decodes/case after warmup: median ratios 1.033 (tiny RGB8), 0.995 (lossy still),
  1.012 (existing ICC16 animation). These small shared-host differences do not
  establish a material regression or improvement. Raw trials and method are in
  [the benchmark record](jpeg-xl-xyb-animation-benchmark.json).

At this original XYB checkpoint, no new embedded-ICC fixture, browser playback
QA or fresh WASM execution was claimed. The later SDR follow-up below adds those
checks within its stated scope. Independent 16-bit coverage is recorded below.
Existing original-profile 16-bit/ICC, crop/blend/orientation and security tests
remain passing. CI completion is not awaited; no merge or release is performed.


## Independent 16-bit XYB animation comparison

The follow-up fixture is a deterministic synthetic 32x24 RGBA16 APNG with three
frames. Official libjxl cjxl 0.12.0 (`4128790`, distance 1, effort 7) encodes XYB;
its container is stripped without changing the codestream. The parser test
asserts XYB, integer16 RGB and integer16 straight full-resolution alpha, D65/sRGB
primaries and transfer, relative rendering intent. The decoded timing is
50/90/130 ticks at 1000 ticks/second, with loop count zero. cjxl does not preserve
the synthetic APNG's loop count of three, so the oracle contract checks the
actual encoded JXL loop count rather than claiming APNG loop preservation.

Independent djxl explicitly outputs **16-bit PAM**, all three coalesced frames,
in `RGB_D65_SRG_Rel_SRG`, without alpha blending. The PAM header requires MAXVAL
65535 and four channels. PAM's big-endian samples are stored as raw little-endian
u16 goldens. Neither reference nor WML2 output passes through RGBA8, ImageMagick,
CMS, premultiplication, or a background composite in this comparison. 8,979 of
9,216 reference samples are not divisible by 257, ruling out 8-bit expansion.

Both native RGBA16 and EX Rgba16/Exact compare every sample directly. On Windows
x64 / Rust 1.99.0, all three frames have maximum RGBA error `[1,1,1,0]` in units
of 1/65535. Per-frame sums of absolute RGBA error are `[25,18,19,0]`,
`[25,15,29,0]`, and `[32,14,24,0]`. In total 201 of 6,912 RGB samples differ by
one unit; all 2,304 alpha samples match both libjxl and the source alpha formula
exactly. Tests enforce RGB tolerance **1/65535** and alpha tolerance **zero**.
This is decoder reconstruction agreement, not lossless RGB encoding: the source
was intentionally encoded with lossy XYB distance 1.

The fixture, independent raw golden, manifest, and standard-library-only
`generate_jxl_xyb_animation16.py` are under `wml2/tests/fixtures/review/`.
The script requires the recorded official cjxl/djxl version and records source,
codestream, reference and ICC SHA256 hashes. Native and EX tests pass, as do the
previous XYB tests. Representative 16-bit renders were inspected after comparison;
visualization alone is not precision evidence. No production decoder code changed.
Final focused regression run: 122 tests passed across the library and eight JXL
integration targets. The XYB target passes four tests with jpeg-xl only and
compiles/runs zero tests with the feature disabled. The generator was rerun and
produced the same codestream and reference hashes. `rustfmt --check` and
`git diff --check` passed. CI/CD completion is not awaited.

## SDR follow-up

[The integration report](jpeg-xl-sdr-validation.md) records the distinct
source/output ICC fixture, Gray support, current native/WASM checks, Viewer
playback and before-after benchmarks. Explicit `image_from_srgb*` returns sRGB
RGBA8 for admitted supported profiles or fails before image publication. Ordinary
and EX APIs intentionally preserve reconstruction coordinates and tag the actual
output ICC; their samples must not be assumed to be sRGB.

The verified ICC subset is standard SDR RGB matrix/TRC, plus simple Gray signaling
expanded to a matching neutral-axis RGB profile. Existing original-profile RGB
ICC conversion remains available through the bounded `icc-profile` engine. The
XYB embedded-ICC fixture verifies jxl's sRGB reconstruction fallback with an
input profile that cannot be represented by one simple transfer function. This
is not a promise for arbitrary ICC LUT/device-link/CMYK profiles or opaque HDR
curves. Unsupported CMS inputs fail; no new special-ICC or HDR handling is added.
Expansion of arbitrary Gray reconstruction/output ICC to RGBA remains explicitly
refused even by the managed API. Embedded source ICC is separate; the tested
refusal fixture uses original-profile Gray. XYB Gray fallback combinations are
not claimed as verified.
