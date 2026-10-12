# Bounded JPEG2000 multi-tile decoding

Continuation of `a186462192ed4f189dde79004ad3927bbb3df691` on
`codex/jpeg2000-rgb-reimplementation-20261011`. Normal GitHub read/fetch matched
that SHA. Rust 1.91.0, wasm32-unknown-unknown target, and Node v24.19.0 are
available. Repository AGENTS.md was read; no additional `.agents/skills`
instructions were present. Temporary work uses `.test*` folders.

## Scope and implementation

Multi-tile positioning was chosen before layers or irreversible transforms
because it reuses the existing complete packet/Tier-1, reversible 5/3 and RCT
algorithms. The profile accepts Gray8 and equal unsampled RGB8, with or without
RCT, on a canvas up to 1024×1024. Tile dimensions are nonzero and at most 1024;
there are at most 256 tiles, in raster order, with exactly one tile-part each
(TPsot=0, TNsot=1). Image and tile-grid origins remain zero. No tile-header
coding or quantization overrides are accepted.

Every tile origin must align to 2^decomposition-levels; the smallest clipped
tile on either axis must be at least that size. Tile width/height alignment is
required only on axes containing more than one tile. These constraints retain
the existing even-first inverse 5/3 lifting at all levels. Odd wavelet origins
and smaller boundary tiles remain unsupported.

[T.800 B.5/B.7](https://www.itu.int/rec/T-REC-T.800-200208-S/en) defines globally
anchored sub-band code-block grids. An aligned tile's band origin simplifies
to `tile_origin >> depth`. The parser offsets its first block by that origin
modulo the declared block size, clips its first/last blocks, and keeps tag-tree
and block ordering consistent. An independent reviewer confirmed this derivation
against the normative band-origin formula, including level zero.

A whole-canvas RGBA buffer plus one tile's coefficients/output are retained;
tile scratch is released after copying its rows. SIZ bounds, tile-count bound
and the existing `64 * width * height * components + 8192` budget precede
coefficient allocation. Tiles count as one frame. All tile framing, packets
and final EOC must succeed before any output callback. The single-tile path
moves its output directly and retains the existing zero-origin packet and
small-block Tier-1 specializations. Public API, RGBA8, feature defaults,
production dependencies, other codecs and WML2 0.0.35 are unchanged.

## Native and oracle verification

`generate_jpeg2000_tiles.py` produces 162 positive encodings, split equally
among Gray, RGB/MCT0 and RGB/MCT1, with raw J2K and JP2 containers. It covers
levels 0–5, 4/16/64-square and asymmetric blocks, offsets inside blocks, clipped
edges, row/column tiling, 256 tiles, deepest singleton LL and a 1024×1024 canvas.
Flat neutral encodings explicitly use empty packet headers: OpenJPEG's bodyless
0x80 headers are normalized to standard 0x00 headers without changing lengths,
and decoded again by OpenJPEG before recording the fixture.

Another 24 independently valid encodings exercise rejected misaligned origins,
tiny edge tiles and 289-tile layouts. The generator checks source pixels,
dimensions, components and actual SIZ/COD/SOT/SOD/Psot/EOC fields for every file.
Hashes and parameters are in the 14-field positive and 15-field negative
manifests (documented in the generator). OpenJPEG is exclusively an oracle;
production decoding remains independent Rust.

Rust public APIs match all positive source RGBA bytes and dimensions with exact
input, pixel, expanded and one-frame limits; one-short limits fail. Invalid or
unsupported early/late tiles fail before output initialization. Tests include
wrong/duplicate/swapped tile IDs, missing/extra tiles, part count/length/SOD,
early EOC/trailing data, oversized/zero dimensions, repaired short payloads
and bounded packet mutations. Final normalized flat fixtures passed the
focused pixel/limit test. Independent static review found no actionable
correctness, resource or API defect within this scope.

Ordinary release workspace regression passed 353 tests, 0 failed, 12 existing
ignored, across 43 targets including doctests. Minimal-feature feature-off
regression passed 16 tests. Clippy reports the 11 existing warnings elsewhere,
none in JPEG2000; formatting and diff checks pass. Seeded random verification
checks 18 tile/block geometries, 108 pixel comparisons, both containers and all
three component/MCT profiles (`check_jpeg2000_tiles_random.py`).

## Remaining verification at this checkpoint

Actual minimal and default-codec WASM execution and the short paired performance
comparison are in progress; their final results and artifact hashes will be
recorded separately. The previous larger-RGB CPU sample of +6.11% remains a
performance caveat pending the new paired sample. No general speed guarantee
is implied by the native correctness results.

## Reproduction and exclusions

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_tiles.py
cargo test --workspace --release --features wml2/jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 \
  --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_tiles_random.py \
  /absolute/path/to/release/examples/jpeg2000_verify --cases 18
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

Multiple layers/parts, arbitrary tile order, nonaligned wavelet origins, tile
coding overrides, 9/7, subsampling, signed/high-bit-depth/alpha, other progressions,
ROI and explicit precinct/SOP/EPH remain unsupported. Browser UI, other operating
systems, optional AVIF matrices and an external corpus remain untested. This
work does not merge or release, update main/stable jpeg2000, wait for remote CI,
resume old ZIP retrieval, or use GitHub-app writes/approval retries.
