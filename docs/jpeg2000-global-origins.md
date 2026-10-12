# Bounded nonzero JPEG2000 image and tile-grid origins

The later [bounded tile-part extension](jpeg2000-tile-parts.md) records support
added after this checkpoint.

Continuation of `8d8c209901fd5f29cbeffb5f8d3a288ad1bf1a08` on
`codex/jpeg2000-rgb-reimplementation-20261011`. Normal remote read/fetch matched
that SHA. Repository AGENTS.md and the available `.agents/skills` locations were
checked; temporary folders use `.test*`. Rust 1.91.0 with the real wasm32 target
is available. No main/stable branch update, release, CI wait or recovery/auth
investigation is part of this work.

## Scope and reason

Nonzero global origins extend the previous origin-aware packet and inverse 5/3
kernels without adding MQ continuation state or a new transform/quantizer.
The profile now supports different image and tile-grid starts, first/last tile
clipping and arbitrary per-tile parity. Actual image axes and nominal tile axes
remain 1–1024, with at most 256 raster tiles, one tile-part/layer, Gray8/RGB8,
RCT on/off, LRCP, levels 0–5, full passes and style zero.

Reference-grid ends Xsiz/Ysiz are limited to 32768. Together with nonnegative
origins, this keeps every tile/resolution inside the first implicit 32768-sized
precinct (and the corresponding high-band precinct), avoiding a new multi-
precinct packet parser. Tile-grid starts must not exceed image starts, and the
first nominal tile must intersect the image. Larger reference coordinates are
unsupported even when the actual cropped image is small.

## Coordinates, bounds and API

[T.800 B.3/B.5/B.6](https://www.itu.int/rec/T-REC-T.800-200208-S/en)
defines reference-grid tile intersections. SIZ now computes actual dimensions
as extent minus image start, compares those with JP2 IHDR, and charges pixels
and `64 * actual_width * actual_height * components + 8192` workspace using
those actual dimensions. Tile counts use extent minus tile-grid start. Each
nominal tile is intersected with the image; absolute starts feed coefficient
placement/lifting, while image starts are subtracted for output row placement.
No blank prefix is allocated or exposed through callbacks. One-tile output
still moves directly; multi-tile output retains one tile's scratch at a time.

Checked subtraction rejects reversed/empty intervals. Axis/component/reference
bounds precede grid arithmetic; short-circuit grid-start checks reject oversized
values before additions. Validated additions are at most 32768+1024, safe on
WASM32. The first-intersection rule makes every enumerated tile nonempty.
Tile framing, packets and EOC finish before any drawer callback. Input,
metadata, frame and abort/error contracts remain unchanged.

The independent review of source SHA256
`fffd891662a4e332ad31f48e4f60b48690d07d04c50475afd47fde37f1bf06da`
found no actionable correctness, resource or API defect. Origin-zero fast
paths remain. Public APIs, RGBA8, other codecs, feature defaults, production
dependencies and WML2 0.0.35 are unchanged.

## Fixed and random evidence

`generate_jpeg2000_global_origins.py` creates 126 source-exact encodings:
Gray, RGB/MCT0 and RGB/RCT each contribute 42 J2K/JP2 files. The matrix covers all
levels, different image/grid starts, clipped first/last tiles, parity, large
coordinates, the exact 32768 bound, 1024² actual canvas, 256 tiles and a 1×1
image at nonzero origin. Thirty encodings use explicitly normalized empty
packet headers. Deep nonneutral cases keep clipped axes large enough to avoid
the already-documented OpenJPEG encoder limitation; tiny deep edges use neutral
source-exact cases. No new oracle mismatch or promotion was needed in this
fixed matrix.

The generator independently verifies source pixels, mode/dimensions, JP2 IHDR,
SIZ/COD/QCD/QCC and raster SOT/Psot/SOD/EOC. The positive manifest has 18 fields
(existing 14 plus image-x/image-y/grid-x/grid-y), SHA256
`aca54751a8c1bcc80dfc49c5d532490d46d57b9d93a174a9665f48986feec7f3`.
Twelve valid over-bound encodings have 19-field negative manifest entries.
Encoding/source hashes have been checked. OpenJPEG remains only an encoder
and independent pixel oracle, with no production dependency or copied decoder.

Seeded random testing obtains 24 source-exact geometries / 144 pixel comparisons
at varying image/grid origins and legal block shapes. Two additional geometries
/ 12 comparisons have the known encoder/source mismatch; they are recorded as
compatibility-only and Rust matches OpenJPEG on every one. The comparator was
not expanded into another investigation. The report records both categories
separately, including source/encoded/output hashes and parameters.

## Verification checkpoint

Ordinary release workspace regression passed 360 tests, 0 failures and 12
existing ignored tests across 43 targets including doctests. Exact/one-short
input, pixel, workspace and one-frame budgets use actual cropped sizes, including
large origins with tiny output. Malformed tests cover maximum-u32/reversed/empty
coordinates, uncovered first tiles, invalid grid placement, excessive actual
sizes and reference ends, early/late tile framing, repaired short bodies and
bounded packet mutations. Valid over-reference-bound files reject before
output initialization. Initial Clippy had only the 11 existing warnings
elsewhere; no JPEG2000 warning was added. Feature-off minimal regression passed 16 tests. Formatting/diff/Python syntax
checks pass. Public-API PPM output images were displayed and independently
matched OpenJPEG, including different image/grid starts, level5, 1×1 and the
1024 canvas.

Final minimal and default-codec real WASM runs each pass 1204 pixel/API cases
(1203 source-exact fixed encodings plus the prior singleton compatibility case)
and 24 over-limit inputs. Node.js v24.19.0 instantiates the actual module without
host imports, checking every RGBA byte, dimensions, exact/one-short limits,
limit scope restoration, callbacks/Abort, truncation and late tile/missing/
repaired-short-packet rejection before callbacks. Both modules report source
SHA256 `fffd891662a4e332ad31f48e4f60b48690d07d04c50475afd47fde37f1bf06da`,
matching checkpoint `f47cbdf`. Minimal was built before that commit with the
identical source/fixtures; default was built from the clean checkpoint.
Linear memory was 70,582,272 bytes (minimal) and 70,713,344 bytes (default),
including embedded fixed test data. Module/source/manifest hashes are in
`jpeg2000-wasm-runtime.json`. This is real WASM execution, not browser UI.

## Paired performance

Immutable release verifiers compare baseline `8d8c209` with checkpoint
`f47cbdf`, fixed CPU affinity, B/H/H/B/H/B (three processes per version),
100 warmups, Rust 1.91. No concurrent CPU tests ran during timing, and all
paired output hashes match. Whole-process CPU microseconds/call, including
export/startup and divided by timed calls plus 101:

| Existing input | Baseline | Extension | CPU change |
| --- | ---: | ---: | ---: |
| Gray16 | 54.749 | 54.258 | -0.90% |
| Gray65 | 1301.889 | 1258.846 | -3.31% |
| RGB16 | 318.740 | 313.023 | -1.79% |
| RGB65 | 4285.348 | 4421.379 | +3.17% |
| Gray256tiles | 806.996 | 805.863 | -0.14% |
| RGB9tiles | 126568.036 | 120941.441 | -4.45% |
| RGB2oddtiles | 211.853 | 210.460 | -0.66% |

No major deterioration appears in these seven short samples. Shared-host
variation remains material, and this does not prove no cumulative or cross-
host regression. The historical larger-RGB +6.11% sample concerned an older
baseline and remains a separate caveat. New nonzero-global-origin inputs
cannot be decoded by the baseline and are excluded from paired timings.
Raw CPU/wall rows and binary/input/output hashes are in
`jpeg2000-global-origins-benchmark.json`. Full verification and scope records
are in `jpeg2000-global-origins-validation.json`; random cases are in
`jpeg2000-global-origins-random.json`.

## Reproduction and exclusions

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_global_origins.py
cargo test --workspace --release --features wml2/jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 \
  --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_global_origins_random.py \
  /absolute/path/to/jpeg2000_verify --cases 24
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

Reference ends above 32768, multiple layers/parts, arbitrary tile order,
tile-header coding overrides, other progressions, 9/7, subsampling,
signed/high-bit-depth/alpha, ROI and explicit precinct/SOP/EPH remain unsupported.
Browser UI, other OS, optional AVIF matrices and an external corpus are untested.
