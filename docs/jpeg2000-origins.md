# JPEG2000 unaligned tile origins and small edges

Continuation from `44d39af873ffd983f04dd87112825f2c75bd856c`, on
`codex/jpeg2000-rgb-reimplementation-20261011`. Normal remote read/fetch matched
that SHA. Repository AGENTS.md and the empty/absent additional `.agents/skills`
locations were checked; temporary work uses `.test*`. Rust 1.91.0 and the real
wasm32 target are available. Main and stable jpeg2000 are not updated.

## Selected extension

Unaligned tile positioning removes a practical restriction of the existing
multi-tile profile while reusing complete packet/Tier-1 decoding and reversible
5/3/RCT. It needs neither progressive MQ state nor a new transform/quantizer.

The decoder accepts arbitrary positive tile dimensions up to 1024, a canvas up
to 1024×1024, at most 256 raster tiles, and clipped edge tiles down to one sample.
Gray8/RGB8, RCT on/off, levels 0–5, one tile-part/layer per tile, LRCP, full passes,
style zero, and bounded Part 1 blocks remain the profile. Global image and tile
grid origins remain zero; no tile-header overrides are accepted. Tile origins
no longer have to align to 2^levels, and each actual tile dimension need not
reach that factor. Public APIs, RGBA8, other formats, feature defaults,
production dependencies and WML2 0.0.35 remain unchanged.

## Coordinate and reconstruction rules

[T.800 B.5/B.7 and F.3](https://www.itu.int/rec/T-REC-T.800-200208-S/en)
provide the coordinate, global block-grid and lifting rules. Low and high band
bounds now use the actual tile interval at every depth; different band origins
are used for the first/last cropped code-block. The packed rectangles remain
disjoint even when the deepest LL axis is empty. A zero-area resolution has no
precinct/packet; a nonempty resolution whose high bands are empty still has its
packet. Synthesis uses the reduced global origin to choose even/odd interleaving
and whole-sample reflection at each level.

The origin-aware kernel uses i64 lifting intermediates and checked i32 storage.
F.3.6's odd-start singleton uses ordinary division by two, separately from the
floor divisions in update/predict. Random testing found a one-level difference
when negative odd singleton coefficients were incorrectly floor-divided; the
independent reviewer confirmed it at coefficient level, and a fixed native/WASM
compatibility regression now covers it. The original origin-zero packet path,
aligned tile synthesis and small-block Tier-1 specializations remain in use.

Existing SIZ/block/precision guards and the pre-allocation budget
`64 * width * height * components + 8192` remain. Only canvas output and one
tile's scratch coexist. All tiles and EOC finish before drawer callbacks.
Independent static review found no remaining actionable resource/API or
correctness defect after the singleton correction.

## Oracle evidence and its limitation

`generate_jpeg2000_origins.py` records 210 positive encodings (Gray, independent
RGB, RCT RGB; J2K and JP2), all source-exact through OpenJPEG. Cases span levels
0–5, different origin parities, 1/2/3-sample clipped edges, multiple blocks,
asymmetric shapes, 256 tiles and a 1024 canvas. The matrix includes 24 canonical
promotions, 18 normalized empty-packet encodings and 96 one-sample-edge cases.
There are six additional valid-but-over-limit encodings. Their actual headers,
framing, pixels and file/source hashes are independently verified. The positive
manifest SHA256 is `873b0535181e048333acf8232432290a29728e6597d1db3cdd9165e39ffe9ff6`.
Eighteen previously rejected nonaligned/tiny images now match source pixels;
the previous six over-limit inputs remain rejected.

Pillow12.3.0/OpenJPEG2.5.4 sometimes issues incorrect coefficients for deep
transforms with tiny odd-origin edges. This was isolated by entropy-decoding a
block and comparing it with direct forward 5/3 coefficients. The generator
preserves hashes, the mismatch and its derivation in its docstring. For explicit
nonzero/empty-LL tests, it starts from a source-exact NL1 stream, promotes header
band tables/depth, and adds empty packets only where appropriate. Existing
finest nonzero packet bytes are preserved. Both source and promoted encodings
are independently source-exact through OpenJPEG; no MQ encoder implementation
is introduced. Other problematic matrix positions use neutral source-exact
fixtures. The oracle limitation is not attributed to the Rust decoder.

Seeded random validation obtains 24 source-exact geometries / 144 pixel
comparisons. An additional ten geometries / 60 comparisons have encoder/source
mismatches; they are recorded as compatibility-only, and Rust agrees with
OpenJPEG on every one. The fixed `oracle_origins_*singleton` regression preserves
both the original source and separately named OpenJPEG expected pixels, with
provenance JSON; it is not claimed to be a source-exact encoder roundtrip.

## Verification checkpoint

Normal release workspace regression passed 357 tests, 0 failures and 12 existing
ignored tests across 43 targets including doctests. The pixel tests exercise
exact/one-short input, pixels, workspace and one-frame budgets. Additional tests
cover zero resolutions, odd singleton overflow/reflection, repaired short tile
payloads, late framing, marker/packet mutations, callbacks before errors and
rejection of nonzero global origins. Feature-off minimal regression passed 16 tests. Clippy and formatting/diff
checks pass; the 11 existing warnings elsewhere remain, with none in JPEG2000.
Public-API output images were displayed and compared with OpenJPEG, including
Gray, a deep nonzero singleton edge, flat empty packets and a 1024 canvas.
Final minimal and default-codec real WASM execution passed 1078 pixel/API cases
each (1077 source-exact fixed encodings plus the singleton compatibility case)
and 12 over-limit inputs. Node.js v24.19.0 instantiates the actual module without
host imports. Checks include every RGBA byte/dimension, exact and one-short
limits, scope restoration, callbacks/Abort, truncation, late tile framing,
missing tiles and repaired short packet rejection before callbacks.

Both modules report source SHA256
`b3f5e99387519955447f66ba957ceb7ce2d7b6b8c99abae147a7b302be26e9ad`,
matching implementation commit `236ca6a`. Minimal was built from that clean
checkpoint; default was built before commit with the identical source/fixture
hashes, recorded honestly as a dirty-tree snapshot. Linear memory was
63,176,704 bytes (minimal) and 63,307,776 bytes (default). Module/source/manifest
hashes and full results are in `jpeg2000-wasm-runtime.json` and
`jpeg2000-origins-validation.json`. This is real WASM execution, not browser UI.

## Paired performance sample

Immutable release verifiers compare baseline `44d39af` with implementation
`236ca6a`, fixed CPU affinity, B/H/H/B/H/B (three processes per version),
100 warmups, Rust 1.91. No concurrent CPU tests ran during timing. All paired
PPM hashes match. Whole-process CPU microseconds/call, including startup and
export, divided by timed calls plus 101:

| Existing input | Baseline | Extension | Change |
| --- | ---: | ---: | ---: |
| Gray16 | 68.131 | 62.062 | -8.91% |
| Gray65 | 1356.349 | 1248.359 | -7.96% |
| RGB16 | 309.097 | 311.877 | +0.90% |
| RGB65 | 4150.564 | 4246.935 | +2.32% |
| Gray256tiles | 781.621 | 809.338 | +3.55% |
| RGB9tiles | 119534.383 | 124235.669 | +3.93% |

The short sample shows no large deterioration across these six inputs. It
does not establish no regression on other hosts or a cumulative guarantee.
The earlier larger-RGB +6.11% observation used an older baseline and remains
a separate caveat. Raw CPU/wall rows, hashes and measurement conditions are in
`jpeg2000-origins-benchmark.json`. Newly supported geometries cannot be decoded
by the baseline and are not included in the paired timings.

## Reproduction and remaining scope

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_origins.py
cargo test --workspace --release --features wml2/jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 \
  --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_origins_random.py \
  /absolute/path/to/jpeg2000_verify --cases 24
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

Nonzero global image/tile grid origins, multiple layers/parts, arbitrary tile
order, tile coding overrides, 9/7, subsampling, signed/high-bit-depth/alpha,
other progressions, ROI and explicit precinct/SOP/EPH remain unsupported.
Browser UI, other operating systems, optional AVIF matrices and an external
corpus are untested. No merge, release, CI wait or old-recovery/auth loop is done.
