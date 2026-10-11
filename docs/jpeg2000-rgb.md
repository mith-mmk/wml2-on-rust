# JPEG2000 RGB8 extension

Reimplemented from verified Gray commit `7043bf083203234c1193cf1c22a70ab61d6dc3df`.
The separately preserved unpublished RGB ZIP has not been downloaded or used.
WML2 remains 0.0.35; the nondefault `jpeg2000` feature, existing public API,
opaque RGBA8 callback/output buffer, default features and codec dependencies are
preserved. Rust decoding has no OpenJPEG dependency.

Current support includes both independent RGB components and reversible RCT.
The descriptions of stage 1 below record its intermediate commit, not the final
profile. Stable `jpeg2000` is kept at the verified Gray commit; this work is on
`codex/jpeg2000-rgb-reimplementation-20261011`.

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

## Stage 2: reversible color transform and input bounds

COD MCT=1 is accepted only with three equal unsigned RGB8 components and the
already-required reversible 5/3 transform. MCT=0 remains supported. Inverse RCT
follows [ITU-T T.800, Annex G.2.2](https://www.itu.int/rec/T-REC-T.800-200208-S/en):
wavelet synthesis, inverse RCT, then unsigned DC shift and final clamping.
Green is recovered with floor division of the chroma sum by four, then red and
blue are recovered from green and their respective differences. Computation
uses i64 intermediates so reconstructed i32 extremes cannot overflow. Color
conversion does not allocate extra component images or change the RGBA8 API.

QCD is validated even if all components have QCC overrides. QCC indices,
duplicates, quantization style, reserved bits, band count and coefficient
precision are checked before entropy decoding. Gray continues to reject QCC
and MCT. JP2 color/component inconsistencies, signed or high-bit-depth samples,
subsampling, invalid dimensions, irreversible wavelets and MCT values beyond
the supported flag fail before any callback.

The generator now checks 148 RGB J2K/JP2 encodings (37 sources, both MCT flags
and both containers). The public API's bytes match all RGB sources, with opaque
alpha, alongside all 182 existing Gray encodings. Additional integration tests
cover independent and RCT RGB callbacks (Abort and errors), all truncated
prefixes, single-byte mutations, repaired short tile payloads, default and
per-component quantization, and exact input/pixel/expanded/frame/metadata limits.
JPEG2000 has no integrity checksum; a mutation may describe another valid image,
so these tests require rejection of structural violations and safe handling of
entropy mutations, not rejection of every changed byte.

Final ordinary workspace regression: 345 passed, 0 failed, 12 existing ignored.
Minimal feature-off lib + JPEG2000 detection tests: 16 passed. The default codec
configuration plus `jpeg2000` builds for `wasm32-unknown-unknown`; this verifies
compilation, not execution in a browser. The target was installed with the
environment's existing writable Rust toolchain via official `rustup target add`.

Clippy passed for JPEG2000-only lib, integration tests and verification example,
with no diagnostics in JPEG2000 code and existing warnings elsewhere. Formatting
and `git diff --check` passed. Minimal and default-plus-JPEG2000 WASM library
builds both passed. AVIF-specific optional matrices, external JP2 corpora,
Windows/macOS and WASM runtime execution were not checked for this extension.

Additional seeded random comparisons use 128 geometries, random RGB/Gray bytes,
levels 0–5, both containers and both RGB MCT flags. All 768 independent OpenJPEG
and Rust public-API pixel comparisons matched. Level counts across comparisons
were 144/168/138/132/138/48. Reproduction script:
`check_jpeg2000_rgb_random.py`; detailed results: `jpeg2000-rgb-random.json`.
The final RCT-off palette and level-5 RCT-on noise public-API output images were
displayed with nearest-neighbor scaling and matched OpenJPEG pixels exactly.

## Final performance

CPU-affined release builds, no concurrent local tests/builds, B/H/H/B/B/H order,
three processes per version, 100 warmups per process. Existing Gray output
hashes matched for every pair. Means below are timed public-API wall us/decode;
whole-process CPU results are retained in `jpeg2000-rgb-benchmark.json`.

| Existing Gray input | Base | RGB extension | Change |
| --- | ---: | ---: | ---: |
| 16×16 level-1 noise, 20,000 calls | 21.682 | 21.595 | −0.40% |
| 16×16 level-1 ramp, 20,000 calls | 11.499 | 11.691 | +1.67% |
| 65×49 level-1 noise, 1,000 calls | 622.607 | 612.175 | −1.68% |
| 257×129 level-5 mixed, 200 calls | 5470.544 | 5414.187 | −1.03% |

No clear slowdown was observed in these samples. New RGB profiles have no
supported Gray-base comparison: 65×49 level-2 noise was 2.089 ms without RCT and
2.269 ms with RCT (200 timed calls); 1024×1024 level-5 RGB ramp with RCT was
280.871 ms (5 timed calls). This shared-host sample is not a general throughput
guarantee. The benchmark records the final Rust source hash and base SHA.

## Reproduction

Initialize repository submodules and use Rust 1.91 or later; Cargo.lock is
ignored as in the existing workspace. Fixtures are checked in, so OpenJPEG is
needed only to regenerate fixtures or run independent random comparisons.

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_rgb.py
cargo test --workspace --features wml2/jpeg2000
cargo test -p wml2 --no-default-features --lib --test jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 \
  --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_rgb_random.py \
  /absolute/path/to/release/examples/jpeg2000_verify
cargo build -p wml2 --target wasm32-unknown-unknown --features jpeg2000
cargo build -p wml2 --target wasm32-unknown-unknown \
  --no-default-features --features jpeg2000
```

Machine-readable final check summary: `jpeg2000-rgb-validation.json`.
