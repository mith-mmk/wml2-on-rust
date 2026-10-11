# Reversible Gray JPEG2000: up to five decomposition levels

This extends the native Rust decoder at `3c824158974a4cd6d2fce93d31e0d3e57eb658ec`.
The supported profile now includes **0–5 reversible 5/3 decomposition levels**.
Each image axis must be at least `2^levels` and at most 1024. Thus 4×4 at level 2,
8×8 at level 3, 16×16 at level 4 and 32×32 at level 5 reach the supported minimum;
odd dimensions such as 33×35 at level 5 are supported as well.

Gray8, one tile/tile-part/layer, LRCP, implicit precincts, 16×16 code-blocks,
style zero and full coding passes remain required. RGB/alpha, 9/7, levels above
five, dimensions too small for the chosen level, multiple tiles/layers and
previously unsupported JP2/codec settings still fail explicitly. The existing
WML2 public API, ordinary opaque RGBA8 output, 1024 dimension cap, dependencies,
default features and version 0.0.35 are maintained. There is no external decoder
call or FFI; OpenJPEG is only used for fixture generation/oracle verification.

## Subbands and reconstruction

QCD now requires exactly `1 + 3 * levels` band exponents. The packet reader lays
out deepest LL followed by each resolution's HL/LH/HH from coarse to fine,
matching QCD and LRCP order. Band rectangles are disjoint nested quadrants;
code-block grids/tag trees remain independent for each band. Ceil division
preserves odd dimensions at every resolution.

Synthesis grows the LL region from the deepest level to the full canvas, doing
horizontal then vertical integer lifting at every step (T.800 F.3). Regional
rows/columns always use the original canvas stride, so coefficients outside the
current LL rectangle are not accidentally included or overwritten. Existing
zero/one-level synthesis and small-image packet paths are retained. Multilevel
synthesis is kept out of the common decode path by an explicit outline hint.
The frequently called significance helper is explicitly inlined to keep caller
coordinate bounds visible; coefficient/context rules remain unchanged.

New multilevel lifting uses i64 intermediates, floor division for negative
values, whole-sample symmetric edges, and checked conversion back to i32
coefficient storage. Out-of-range coefficients return an error before callbacks.
The final DC shift clamps the coefficient to -128..127 before adding 128,
which is equivalent to a wide shift followed by byte-range clamping and cannot
overflow even for i32 extremes. No unsafe code is introduced.

## Why five levels

The cap bounds processing to six packets and sixteen subbands. The minimum-axis
rule prevents empty subbands in this profile and permits 1×1 deepest LL/high
bands at the boundary. All block and tree allocations derive from geometry
validated before decoding. The existing workspace charge remains
`64 * width * height + 8192`, with separately charged input and metadata. Only
one full coefficient plane is retained, with small per-resolution descriptors
and trees; lifting uses temporary rows/columns, not full intermediate images.
The widest new row/column scratch remains below 16 KiB at the 1024-axis bound,
well within the existing charge for those geometries.

For coefficients bounded by magnitude M, one-dimensional inverse 5/3 is
bounded by `2*M + ceil(M/2)`. Applying that bound twice per stage, five times
from the maximum 16-bit magnitude 65535, gives 624995833, below i32::MAX.
Checked conversion makes the invariant explicit even for malformed coefficient
input. i64 arithmetic safely handles sums of i32 values. Larger depths would
need a separate precision/storage/empty-band assessment and are not implied.

## Independent fixtures and tests

`generate_jpeg2000_deep.py` creates 25 original Gray sources, 50 J2K/JP2
encodings, and verifies exact OpenJPEG decoding to the sources. Pillow 12.3.0 /
OpenJPEG 2.5.4 tool versions and input SHA256 values are recorded in
`jpeg2000-deep-manifest.txt`. Fixtures span levels 2–5, exact minimum dimensions,
odd dimensions, 1×1 shrinking bands, mixed empty/low/high-amplitude blocks,
multiple blocks, and the 1024×1024 limit. Rust compares every RGBA byte with
checked-in source `.gray` data. Existing 132 zero/one-level fixtures remain
covered by the same integration target.

Focused checks include coefficient overflow, negative floor rounding, exact
workspace/pixel budgets, insufficient axes, rejected level counts 6/32/255,
mismatched QCD band counts, unsupported coefficient precision, repaired Psot/EOC
around short multilevel packet streams, and bounded mutations. As before,
JPEG2000 entropy mutations can encode legal alternate pixels, so universal
corruption detection is not claimed.

```
python wml2/tests/fixtures/review/generate_jpeg2000_deep.py
cargo test -p wml2 --locked --release --no-default-features \
  --features jpeg2000 --lib --test jpeg2000
cargo build -p wml2 --locked --release --no-default-features \
  --features jpeg2000 --example jpeg2000_verify
python wml2/tests/fixtures/review/check_jpeg2000_random.py \
  /absolute/path/to/release/examples/jpeg2000_verify \
  --max-dimension 129 --cases 96 --max-levels 5
```

Initialize submodules and generate the ignored workspace Cargo.lock before
using `--locked` in a fresh checkout.

## Final local verification (2026-10-11)

Rust 1.91.0; no failures in the final runs:

| Check | Result |
| --- | --- |
| JPEG2000-only release unit + integration | 29 passed |
| Default codecs + jpeg2000 + high-bit-depth release lib/tests | 241 passed, 4 existing ignored |
| Feature-off minimal lib/tests | 26 passed |
| jpeg2000 + noretoro release unit + integration | 29 passed |
| Additional random oracles, dimensions 1–129, seed 200053 | 96/96 exact matches |
| Clippy JPEG2000 lib, integration and example | Passed, no new diagnostics versus base |
| wasm32-unknown-unknown JPEG2000-only compile | Passed, not execution |
| cargo fmt --all --check; git diff --check | Passed |
| Actual 257×129, five-level JP2 display | Exact OpenJPEG pixels; inspected nearest-neighbor image |

Random cases included 18/23/18/16/13/8 images at levels 0/1/2/3/4/5, respectively.
Tests cover all 182 fixed J2K/JP2 oracle images across the three milestones.
No full external JP2 corpus, unsupported RGB/9/7 profiles, Windows/macOS execution,
AVIF-specific optional matrix or remote CI was run for this unit.

## Performance on the final code

Release builds, fixed CPU affinity, no concurrent local builds/tests, B/H/H/B
order, identical exported output hashes for each pair. Small inputs use 100
warmups and 10,000 timed calls; larger existing inputs use 100 warmups and 100
calls. Whole-process CPU measurements include startup, export and warmups,
divided by total decode calls; the example's wall measurement excludes those.

| Existing one-level JP2 | Base mean CPU us/call | Head mean CPU us/call | Change |
| --- | ---: | ---: | ---: |
| 16×16 noise | 82.396 | 80.065 | -2.83% |
| 16×16 ramp | 35.719 | 42.572 | +19.19% |
| 65×49 noise | 1766.940 | 1868.470 | +5.75% |
| 257×129 mixed | 11518.346 | 11708.316 | +1.65% |

One ramp head sample took 49.356 us versus 35.789 us in the other. A six-process
B/H/H/B/H/B follow-up with 30,000 calls each gave mean CPU 29.346 us on base and
29.614 us on head (+0.91%); all wall samples were 28.106–31.027 us. This suggests
a transient contribution to the initial +19% result rather than a consistently
large regression. The larger-input differences remain measured observations;
zero regression on all systems is not claimed. Full data, including the initial
ramp sample, are in `jpeg2000-multilevel-benchmark.json` and the follow-up in
`jpeg2000-multilevel-ramp-repeat.json`.

New five-level profiles (no supported base comparison), 100 warmups + 20 timed
calls: 33×35 noise 0.713 ms, 257×129 mixed 12.732 ms and 1024×1024 ramp
303.522 ms/decode. The helper `benchmark_jpeg2000_deep.py` reproduces the paired
protocol when passed absolute paths to the base/head release examples. These
shared-host samples are not a general throughput guarantee.
