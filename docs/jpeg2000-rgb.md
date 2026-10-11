# JPEG2000 RGB8 extension

Reimplemented from verified Gray commit `7043bf083203234c1193cf1c22a70ab61d6dc3df`.
The separately preserved unpublished RGB ZIP has not been downloaded or used.
WML2 remains 0.0.35; the nondefault `jpeg2000` feature, existing public API,
opaque RGBA8 callback/output buffer, default features and codec dependencies are
preserved. Rust decoding has no OpenJPEG dependency.

## Stage 1: independent RGB components

Raw J2K and JP2 with three unsigned 8-bit components now decode through the
ordinary public APIs. Components must have identical dimensions and no
subsampling. Existing bounds remain: zero origins, one tile/part/layer, LRCP,
reversible 5/3, levels 0–5, maximum 1024×1024, 16×16 blocks, implicit precincts,
style zero, complete coding passes. RCT is not yet accepted in this stage.

JP2 requires enumerated sRGB (16) for RGB or grayscale (17) for one component;
ihdr and SIZ must agree on dimensions and component count. Main-header QCC
overrides are bounded to the three RGB components, with duplicate/out-of-range
indices rejected and the same precision/quantization checks as QCD. Unsupported
markers, including component-specific coding style overrides, remain errors.

Packet parsing follows resolution/component order within LRCP. Each component
has independent coefficient planes, inclusion/zero-plane tag trees and wavelet
synthesis. The existing small Gray packet path is retained. Pixel/frame/input
budgets and callback behavior are unchanged; conservative expanded workspace
charges `64 * width * height * components + 8192`, preserving the Gray charge
and checking the RGB charge before plane allocation.

## Stage 1 verification (2026-10-11)

Rust 1.91.0, release opt-level 3. `cargo test --workspace --offline --features
wml2/jpeg2000` passed: 339 tests, 0 failures, 12 ignored, including doctests.
This is the ordinary workspace with default codecs, not an isolated harness.
Existing Gray truncation/mutation/bounds/limit and callback tests pass.

`generate_jpeg2000_rgb.py` creates 37 original RGB sources and 74 J2K/JP2
encodings using Pillow 12.3.0 / OpenJPEG 2.5.4 solely as fixture encoder/oracle.
Every oracle output matches its source; the Rust public API matches every source
pixel and emits alpha 255. Coverage includes odd/even and singleton dimensions,
channel extremes, noise, multiple code-blocks, all supported levels and
1024×1024. Source/encoding hashes are in `jpeg2000-rgb-manifest.txt`.

Public-API decoded palette/noise outputs were displayed and matched independent
OpenJPEG pixels. A CPU-affined B/H/H/B release benchmark compared unchanged
Gray base/current output hashes. 16×16 noise means were 22.013/21.641 us;
257×129 level-5 mixed means were 5589.229/5805.184 us (3.9% difference with
variation between runs). This short shared-host sample cannot establish a
general performance guarantee. Raw results: `jpeg2000-rgb-stage1-benchmark.json`.

The wasm32-unknown-unknown standard library was installed with official rustup;
WASM verification and RCT validation follow in the next stage.
