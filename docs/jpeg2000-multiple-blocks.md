# JPEG2000 reversible Gray: multiple code-blocks

Historical multiple-block milestone. See [multilevel extension](jpeg2000-multilevel.md)
for the current 0–5-level profile.

This extends the native Rust decoder at `15263f540f83abff2d6b1ebd2c38b8d727326499`.
The supported image size is now **1–1024 pixels on each axis**, using multiple
16×16 code-blocks. Single unsigned Gray8, one tile/tile-part/layer, LRCP, style
zero, implicit precincts, full passes, and reversible 5/3 with zero/one levels
remain the supported profile. A one-level transform requires both axes >=2.
RGB, 9/7, two or more levels, larger images, alternate code-block sizes, explicit
precincts, partial passes, multiple tiles/layers and other unsupported settings
continue to return errors before callbacks. Package version is still 0.0.35;
default features, dependencies and existing WML2 API are unchanged.

## What changed

The packet reader now creates inclusion and zero-bit-plane tag trees per band.
Parents share already-decoded lower bounds and values across leaves; traversal
is root to leaf, with code-block leaves visited in raster order. Each included
block has its own plane count, pass count, segment length and fresh MQ state.
Its coefficients are copied to the appropriate band rectangle before inverse
5/3 synthesis. Edge code-blocks use their actual width/height, including odd
canvas dimensions. Empty/excluded blocks remain zero. A single-block band uses
the direct-bit path and does not allocate trees. Entire images <=16×16 use the
previous packet parser unchanged, preserving its allocation and scan behavior.
Both paths share the existing bit reader, Tier-1 decoder and inverse lifting.

The new tree implementation follows T.800 B.10.2/4/5; it does not call or link
OpenJPEG or another product decoder. Tier-1 math and inverse lifting are unchanged. Neighbor context formation
now has an explicit inline hint so coefficient-loop bounds remain visible to
the optimizer; MQ/significance/context rules and resulting pixels are unchanged. At the selected dimension bound, the implicit 32768×32768
precinct covers each resolution, so multiple-precinct handling is not implied.

## Memory and malformed input

SIZ is validated and the existing pixel/frame/expanded limits are checked before
any coefficient, tag-tree or block descriptor allocation. At most 64×64 blocks
occupy a full-resolution band. Tree depth is at most seven, leaves at most 4096;
zero-plane thresholds remain bounded by supported coefficient precision (<=16).
Packet lengths and bodies use checked slices; the declared length does not
allocate a block buffer. Bounds/metadata/callback validation from the previous
milestone remains in force.

The conservative workspace charge remains `64 * width * height + 8192` bytes,
with input charged separately. On a 64-bit host, a worst full-resolution band
has 5461 nodes in each two-byte-node tree, small per-level vectors, and at most
4096 block descriptors (<=56 bytes each). Combined with the coefficient/RGBA
planes, one-block Tier-1 scratch and lifting rows/columns, this remains below the
existing charge. Thin images have fewer leaves; the fixed 8192-byte allowance
covers the small-image scratch. Geometry is bounded at 1024 before arithmetic,
so multiplication also fits 32-bit usize. No unsafe code is introduced.

## Oracle fixtures and reproduction

`generate_jpeg2000_multi.py` creates 22 original Gray sources and 44 J2K/JP2
fixtures. Pillow 12.3.0/OpenJPEG 2.5.4 independently decode each file back to its
source; Rust tests compare every RGBA byte. SHA256 values are in the adjacent
`jpeg2000-multi-manifest.txt`. Cases cover 17×19, 31×33, 65×49, 257×129,
1×513, 513×1 and 1024×1024, and both supported decomposition counts. Mixed
patterns put zero, low-amplitude and high-amplitude blocks together, exercising
shared inclusion nodes and different missing bit-plane values. Existing 88
small-image oracle fixtures remain covered.

```
python wml2/tests/fixtures/review/generate_jpeg2000_multi.py
cargo test -p wml2 --locked --release --no-default-features \
  --features jpeg2000 --lib --test jpeg2000
cargo build -p wml2 --locked --release --no-default-features \
  --features jpeg2000 --example jpeg2000_verify
python wml2/tests/fixtures/review/check_jpeg2000_random.py \
  /absolute/path/to/release/examples/jpeg2000_verify --max-dimension 129 --cases 64
```

Additional focused checks cover hand-derived shared-node tag trees, exact and
one-byte-short workspace budgets, pixel budgets, oversized/zero SIZ dimensions,
rejected decomposition counts 2/32/255, and repaired Psot/EOC around truncated
multiple-block packet headers/bodies. The existing malformed-prefix, mutation,
feature-off and callback tests are retained. Larger entropy mutations also run
without panicking; an altered entropy stream can be a legal alternate image,
so exhaustive corruption detection is not claimed.

Actual Rust output for 257×129 one-level mixed JP2 was exported through
`image_load`, compared exactly with OpenJPEG, and displayed with nearest-neighbor
scaling to inspect edges/block boundaries.

## Final local verification (2026-10-11)

Rust 1.91.0. The final version retaining the original small-image packet parser passed:

| Check | Result |
| --- | --- |
| JPEG2000-only release unit + integration | 25 passed |
| Default codecs + jpeg2000 + high-bit-depth release lib/tests | 237 passed, 4 ignored |
| Feature-off minimal lib/tests | 26 passed |
| jpeg2000 + noretoro release unit + integration | 25 passed |
| Additional random Gray oracles, seed 200053, dimensions 1–129 | 64/64 exact matches |
| Clippy minimal/jpeg2000 lib, integration and example | Passed; no new diagnostics versus previous milestone |
| wasm32-unknown-unknown jpeg2000-only cargo check | Passed; compile only |
| cargo fmt --all --check; git diff --check | Passed |
| Actual public-API image display | Passed; exact OpenJPEG pixel comparison |

The general multiple-block implementation also passed the complete debug
regression suite before the allocation-free single-block refinement. Focused
multiple-block debug tests covered the 1024 bound and passed. Full Windows/macOS
execution, AVIF-specific optional matrix, remote CI and unsupported JPEG2000
profiles were not run. This is still a deliberately restricted experimental
profile, not a general Part 1 conformance claim.

## Final performance comparison

The final inline-context version was measured after other builds/tests completed,
with identical example source, one fixed CPU and B/H/H/B order. Small inputs:
100 warmups and 20,000 measured decodes. All paired output SHA256 values match.
Whole-child process CPU time includes startup/export/warmups and is divided by
20,101 public-API calls; wall time reported by the example times only the 20,000
measured calls.

| 16×16 one-level JP2 | Base mean CPU us/call | Head mean CPU us/call | Change |
| --- | ---: | ---: | ---: |
| Noise | 96.795 | 99.499 | +2.79% |
| Ramp | 39.125 | 35.635 | -8.92% |
| Checker | 22.842 | 22.972 | +0.57% |

Noise wall samples ranged from 78.956 to 120.194 us across runs on this shared
host, considerably more than the final mean CPU difference. These measurements
do not prove zero regression on every machine; no consistent large slowdown
remained in the final comparison. The original small-image parser is retained,
and the context inline hint resolved the larger difference observed during
implementation without changing context rules or pixels.

New profiles (no supported base comparison), 100 warmups + 20 timed calls:
65×49 noise 1.788 ms/decode, 257×129 mixed 13.326 ms/decode, 1024×1024 ramp
313.729 ms/decode. This remains a correctness-focused implementation rather than
a throughput guarantee. Raw data: `jpeg2000-multiple-blocks-benchmark.json`.
Reproduction helper: `wml2/tests/fixtures/review/benchmark_jpeg2000_multi.py`,
accepting absolute paths to base/head `jpeg2000_verify` release binaries.
