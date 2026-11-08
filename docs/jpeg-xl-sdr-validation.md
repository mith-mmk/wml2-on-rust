# SDR JPEG XL follow-up validation

This unreleased follow-up completes the bounded XYB work in #104 and adds
Gray/Gray+alpha. It starts from `a072671e`, merges the 0.0.42 documentation/version
base (`2a195f6`), and keeps decoding/coalescing in published `jxl =0.8.0`.
No encoder, codec fork, new public resource policy, HDR path or general extra
channel handling is added. Published WML2 0.0.42 does not contain these changes.

## Supported output and ICC scope

- Integer RGB8/16, original-profile or XYB, and Gray8/16 with optional one
  matching-depth, straight, full-resolution alpha channel are admitted.
- Native and EX animation return full-canvas RGBA16. Ordinary callbacks round
  once to RGBA8. These APIs report the reconstruction/output ICC and do not
  promise that raw coordinates are sRGB.
- `jpegxl::image_from_srgb*`, with `color-management`, converts every displayed
  frame from the actual reconstruction ICC to sRGB RGBA8 before publication.
  Alpha is excluded. Source, reconstruction and delivered profiles stay distinct.
  An unsupported profile/CMS failure is an error, not a raw-color success.
- Verified ICC cases are standard SDR RGB matrix/TRC and simple Gray signaling.
  Simple Gray becomes a matching neutral-axis RGB profile without changing
  sample values. Expansion of an arbitrary Gray reconstruction/output ICC to RGB is refused,
  including on the sRGB route. The original-profile embedded-Gray-ICC fixture
  exercises this refusal. An embedded source ICC is distinct from the codec
  output profile; untested XYB Gray fallback combinations are not claimed.
  Existing still-image behavior is unchanged.
- Existing bounded RGB ICC/CMS support is retained, but this is not blanket
  conformance for arbitrary LUT, device-link, CMYK, opaque HDR curves or unusual
  profiles. PQ/HLG signaling, float inputs and tone mapping remain out of scope.

The existing preview/timecode/alpha/extra-channel restrictions and input,
dimension, pixel, frame, layer, timing and memory caps remain. Common callbacks
still require exact integer milliseconds; native timing preserves rational
ticks. All decode/CMS preparation finishes before init/draw.

## Distinct source/output ICC regression

The new 32x24 RGBA16 fixture has three XYB frames and a standard RGB matrix/TRC
source profile with different channel gammas (2.0, 2.25, 2.5). One JXL simple
transfer function cannot express it, so the original ICC is actually embedded.
The source ICC is 620 bytes; jxl's sRGB reconstruction ICC is 536 bytes and differs
from it. Parser and public API tests assert these facts, exact timing
(50/90/130ms, loop zero), frame count and metadata tagging.

The independent oracle uses official libjxl 0.12.0 `4128790` djxl default
floating-point NPY output. Its actual output ICC is checked as **linear sRGB**.
The generator then applies the standard sRGB transfer to RGB only, clamps and
rounds directly to u16. This avoids quantizing linear-light samples before the
nonlinear transfer. It does not use WML2 or jxl-rs pixels, an 8-bit intermediate,
premultiplication or a background composite. The float oracle is a test-generation
detail, not added float-input support.

All native/EX u16 samples compare with maximum RGBA error `[1,1,1,0]` per frame.
The per-frame sums are `[22,11,25,0]`, `[28,17,19,0]`, `[17,19,25,0]`.
Ordinary and managed RGBA8 match within one RGB unit; alpha is exact. Using the
source ICC again instead of the output ICC would fail this regression. All three
decoded frames were also rendered over a checkerboard and visually inspected.
This measures decoder reconstruction agreement after lossy encoding, not
lossless agreement with the original source.

The standard-library-only generator and hash manifest are
`wml2/tests/fixtures/review/generate_jxl_xyb_animation_icc.py` and
`jxl-animation-xyb-icc16.json`. Rerunning the generator reproduced the codestream
and reference hashes. [Gray validation](jpeg-xl-gray-animation.md) independently
covers Gray8, GrayAlpha8, Gray16 and GrayAlpha16, with every native/EX sample
exactly matching libjxl and all alpha preserved.

## Executed checks

On Windows x64 / Rust 1.99.0:

- Library plus all ten JPEG XL integration targets: **182 passed**. This includes
  existing crop/reference/blend/orientation, clocks/loops, ICC, slice admission,
  limits, allocation refusal, cancellation, truncation and atomic callbacks.
- New Gray and differing-ICC targets with jpeg-xl only: **5 passed**. Both compile
  and run zero tests with the feature disabled.
- Changed Rust files pass rustfmt checks; git diff whitespace checks pass.
- Actual release wasm32/Node 24.22.1 execution: **13 tests passed**, comprising
  the existing XYB, new Gray and differing-ICC test bodies, not merely a build.
- Viewer-core using this local WML2 path: **8 fixtures passed**, with complete
  frame/timing/loop checks against the independent references. Actual Viewer
  GUI playback and its limits are recorded in
  [the playback report](jpeg-xl-sdr-playback-validation.md).

The Viewer validation uses an isolated local checkout/path patch. Viewer main's
published dependency remains `=0.0.40`; no release or dependency deployment is
implied. A debug GUI run hit its existing five-second load timeout on icos4d;
the release build loaded and played it. This is reported rather than hidden by
increasing the timeout or weakening limits.

## Performance and remaining limits

Seven alternating release blocks (10 warmups, 200 decodes per case, same
dependency versions/features) compare against `a072671e`. Current/base median
ratios are 0.939 for tiny RGB8, 0.859 for lossy still, 1.043 for existing ICC16
animation and 1.016 for XYB16 animation. Shared-host variation is substantial;
these trials do not establish a general speed improvement or material slowdown.
They are not RSS or allocation measurements. Raw trials are in
[the benchmark record](jpeg-xl-sdr-benchmark.json); newly supported Gray latency
is recorded separately, not compared to a formerly unsupported path.

Non-sRGB/XYB Gray, Gray-specific cropped blending and every ICC/alpha/orientation
combination are not exhaustively covered. Existing RGB composition tests passing
is not a claim to that missing Gray matrix. Browser-wide compatibility, all OSes,
display color systems, HDR/float and special ICC/extras remain unclaimed.
No merge, release or deployment is performed. CI/CD completion is not awaited.
Exact commands and numeric results: [validation JSON](jpeg-xl-sdr-validation.json).
