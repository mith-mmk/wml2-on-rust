# JPEG XL continuation (2026-10-19)

`codex/jpeg-xl-remaining-20261019` preserves the original JPEG XL branch at
83c1800d21620d4c4e2a316e51916b1a04c16c96. Its feature-branch integration
ce92c0c includes ICC checkpointfbec877d3267228b1081157c54d1d1bf81355003
(BMP, TIFF page identity, GIF retention and AVIF colour boundary), depending
on PR19 and PR20. A later feature-branch integration now includes ICC checkpoint
163ab8cf6a7177c7c3c30c7047e4210b7ca1b0e7: PSD source/output identity,JP2 ICC
acquisition and native Gray↔RGB layouts. Original79951b7 CI passed56/56;
combined-head validation is recorded separately. No merge to main or publication
is part of this work.

## Explicit displayed output

Enable `jpeg-xl,color-management` and call
`wml2::jpegxl::image_from_srgb(bytes)` or `image_from_srgb_with_limits`.
The result is straight sRGB RGBA8, with a real destination `ICC Profile`.
The original embedded profile remains `JPEG XL Source ICC Profile`; the
actual reconstructed samples' profile is `JPEG XL Reconstruction ICC Profile`.
The latter is the CMS input. In particular, applying the embedded source ICC
after an XYB fallback into another profile would be wrong.

The codec reconstructs RGBA16 first, then the checked shared own-library
`icc-profile`0.0.7 adapter applies1/3→3 channels and excludes alpha, then rounds
to RGBA8. No intermediate RGBA8 quantization is used. Gray ICC expansion is
supported on this explicit route; the ordinary source-coordinate API retains
its existing refusal when it cannot assign a matching RGB profile. Native EX
continues retaining Gray ICC and exact native samples. Unsupported transforms
and resource admission fail before display initialization.

No new third-party runtime dependency is added. libjxl, djxl, LittleCMS,
Playwright and Chromium are independent development/reference tools only.

## Orientation and precision

All8orientations use a frozen asymmetric2×3 RGBA pattern, independently
encoded by libjxl0.11.2 and decoded by djxl. Ordinary and EX output match the
oriented reference exactly, including alpha and invisible RGB, and dimensions
swap only for orientations5–8. The codec applies orientation once.

Coded9/10-bit integer sources are accepted and requested as normalized16-bit
output; native EX declares Gray16/Rgba16, while `JPEG XL Source Bit Depth`
retains9/10 and `JPEG XL Output Bit Depth` reports the delivered domain.
Independent all-sample comparison allows one u16 unit for floating scaling;
every original source code is recoverable by inverse normalization. Native
8/12/16-bit storage semantics and explicit EX precision policy are unchanged.

## Reproducible image evidence

`generate_jpegxl_additional_fixtures.py` uses only libjxl/djxl/LittleCMS.
PAM samples are normalized by their actual MAXVAL. Display references use
`jpegxl_color_reference.c` to obtain libjxl RGBA16 and its actual output ICC,
then LittleCMS2.16 relative intent with NOOPTIMIZE and final8-bit rounding.
The original source patterns, profiles, official input hashes/permissions and
new reference hashes are retained in the fixture manifests. Large reference
rasters are zlib-compressed trusted test data, unrelated to `brob` support.

```console
python3 wml2-test/scripts/generate_jpegxl_additional_fixtures.py --tools /path/to/libjxl-tools/usr --work .test-jxl-fixtures
python3 wml2-test/scripts/verify_jpegxl_wasm.py --output .test-jxl-node
python3 wml2-test/scripts/verify_jpegxl_browser.py --output .test-jxl-browser
```

The Node wasm32 harness checks61cases without host imports. The actual
Chromium154 canvas harness checks13cases: all8orientations,3small ICC cases,
and official606×500 RGB /200×200 Gray images. Maximum browser-side difference
is1/255 against the independent reference. The paired displayed rasters were
visually inspected without corruption. Browser execution uses explicitly
provided in-memory test HTML; it needs no browser filesystem navigation.
The saved HTML, screenshot, WASM hash and detailed result are local artifacts.

## Current support scope

This is bounded still-image decoding and explicit SDR ICC display. It does not
add animation, HDR/tone mapping, encoding, SIMD or threading. Mixed-depth or
associated/subsampled alpha and other extra channels remain rejected.
Container `brob` remains rejected: the pinned public auxiliary-box `data()`
API expands into an unbounded Vec and exposes no output-budget parameter.
Enabling the codec's Brotli feature alone would not satisfy our resource
contract. Opaque `jbrd` retention still does not reconstruct JPEG files.

The Windows25-file collection named in the earlier work is absent from this
cloud environment. These independent cloud/browser results do not claim to
execute that Windows-local corpus. Remote CI outcomes and local benchmark
results are recorded at the saved checkpoint; pending jobs are not successes.

## Local checks and performance checkpoint

The ICC feature integration passes43existing unit/integration tests. The
extended EX+CMS targets pass47tests; JPEG XL alone passes37tests and CMS without
EX passes38tests. Counts overlap. Scoped ordinary Clippy and formatting pass;
only pre-existing unrelated diagnostics remain. The CI steps exercise the new
fixture target and the explicit CMS route; no manual workflow dispatch is needed.

Release benchmarks use the preserved83c1800 baseline and3alternating process
pairs pinned toCPU0. Small cases use1100iterations/first100omitted (3000samples
per variant); the606×500 case uses110/first10 (300samples per variant). There
are no concurrent local builds/tests. All existing-path pixel hashes, allocation
counts and allocated bytes remain identical.

| Existing path | Before median | After median | Change |
| --- | ---: | ---: | ---: |
| ordinary-rgba16 | 53.73µs | 54.71µs | +1.82% |
| ordinary-oriented-rgb | 17378.52µs | 17169.83µs | -1.20% |
| ex-gray16 | 30.95µs | 32.19µs | +4.01% |
| ex-rgb-icc16 | 61.67µs | 62.46µs | +1.29% |

The explicit managed route has no equivalent baseline API. Its absolute medians
are below; small cases use300warm samples and official images use9warm samples.
Initial30-sample official-image measurements were consistent with these values.
These are local tiny-fixture/setup and full-image costs, not cross-machine
throughput guarantees. The own ICC engine's inverse parametric curves dominate
the large-image cost; a faster renderer is not claimed.

| Explicit sRGB display | Median |
| --- | ---: |
| jxl-icc-gray16.jxl | 0.109ms |
| jxl-icc-lossless.jxl | 0.129ms |
| jxl-icc-lossy.jxl | 0.247ms |
| jxl-jbrd-bench.jxl | 1084.211ms |
| jxl-jbrd-gray.jxl | 109.760ms |

The display path is separately specialized at compile time. This keeps its
additional CMS branches out of ordinary/EX optimization and resolved an initial
small-image regression. Detailed paired CSVs and all first-pass observations
are preserved in the local evidence directory.
