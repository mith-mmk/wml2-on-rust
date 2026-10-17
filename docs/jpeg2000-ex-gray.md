# JPEG2000 unsigned Gray12/16 EX (unreleased)

This work starts at PR #13 head `a1bcf033d7835c79176b2073142d102c4db37fb7`
on a separate branch. PR #13 is not rewritten or merged. WML2 remains0.0.36;
no publish, version bump, tag or release is included.

## Contract

Enable `jpeg2000,image-buffer-ex` and use `image_decoder_ex` or `image_from_ex`.
The first profile accepts raw J2K and JP2 Gray with exactly one **unsigned12 or16bit
full-resolution component**, reversible integer5/3 and no scalar quantization.
Classic coding, existing packet progressions/layers/precincts, odd origins,
bounded tiles/tile-parts, and default/component quantization precedence are reused.

The callback gets `init_ex` with **Gray12 or Gray16**, packed byte stride (`None`),
then one full-canvas `draw`, then `terminate`. Samples are two explicitly LE bytes,
with no alignment requirement. Gray12 uses0..4095 and has zero upper four bits;
Gray16 uses0..65535. Original coded precision and low bits are retained, without
normalizing12bit samples into an undeclared16bit range. Source and chosen EX
storage remain independent. Gray-to-RGBA replication adds opaque alpha; Exact
or Round storage/downconversion is the caller's explicit choice.

The ordinary `image_load`, `image_decoder` and `jpeg2000::decode` still accept
only the existing1–8bit profiles and return RGBA8. They reject high-depth input;
there is no new implicit precision reduction. An ordinary legacy callback also
rejects Gray12/16 source declarations through the EX entry point.

RGB/RGBA high depth, signed high depth, irreversible9/7 high depth, coded depths
other than12/16, effective ICC colour declarations, palette/CMAP/CDEF, subsampling, MCT, HT and maxshift ROI are
explicitly unsupported on this first EX path. Existing1–8bit support for those
already implemented legacy features remains intact. Supporting a5/3 filter does
not recover coding passes an encoder deliberately discarded.

## Implementation and budgets

The existing general assembler has separate compile-time output paths. Legacy
output retains u8 component canvases and its RGBA8 renderer. EX keeps checked
signed centered i32 reconstruction through inverse5/3, applies the declared DC
shift in i64, clamps to the sample domain and packs LE bytes directly into its
Gray canvas. EX never routes high-depth samples through the legacy u8 storage or
renderer. No floating reconstruction or ICC/color transform is introduced. JP2 retains
the existing first-understood colour-box policy; ignored later colour declarations
are not interpreted or applied.

Only the new12/16bit path uses the widened reversible coefficient allowance.
The old8bit coefficient limit remains unchanged. Segmented Tier-1 has its own
28-plane cap, and synthesis uses i64 scratch with checked conversion to i32.
Raising SIZ acceptance alone would not provide this contract.

Input/pixel/frame/metadata limits apply before callbacks. EX budgets cover2bytes
per output pixel, conservative tile reconstruction/block state, segment payload
copies, packet/progression directories and replacement ordering state. Worst-case
classic pass segment/contribution capacity is included before packet allocation.
Checked multiplication/addition handles32bit and geometry overflow. Extremely
large nominal tiles are clipped to the cropped image before sample allocation.
These are documented workspace bounds, not a total process RSS or allocator
failure-recovery guarantee. Input and metadata have separate limits.

All tiles/packets and reconstruction complete before initialization. Malformed,
unsupported or budget-exhausted input emits no callback. Abort from init skips
draw and terminates once; Abort from draw terminates once. Other callback errors
propagate without inventing additional callbacks. No streaming, spatial-region
or reduced-resolution API is added; an EX pick rectangle is not decoder ROI.

## Independent fixtures and checks

`generate_jpeg2000_ex_gray.py` uses original seeded U16 values, OpenJPEG CLI
encoding with explicit RAWL precision, and independent OpenJPEG PGX decoding.
RAWL avoids the PGX input loader reducing a flat zero image's coded precision.
All80 positive J2K/JP2 fixtures match both original U16 and decoded PGX exactly:
zero/max, values255/256/257, ramp/noise, odd sizes, singleton edges, image/tile
origins, tile boundaries, six decomposition levels, five progression orders,
multiple layers/parts, classic modes, precincts and QCC marker permutations.

QCC order tests use independently decoded **canonical QCD-before-QCC** pixels as
the reference for equivalent permutations. A reversed OpenJPEG decode is not the
oracle because some versions share the previously fixed marker-order defect.
Twelve independently readable negative streams cover signed16, RGB/RGBA16,
9/7,10bit and maxshift ROI. Declared ICC, JP2/SIZ disagreement, unsupported
precisions, truncations, mutations and overflow are also rejected in tests.

Native and actual wasm32/Node reuse the same pixel/stride/legacy/callback checks.
They verify packed LE, odd byte-stride picking with zero padding, explicit
Exact/Round conversion, default-scope restoration, exact/one-short measured
workspace thresholds, Abort/error sequencing and32bit geometry boundaries.
The minimal/default WASM harness executes80 positives and12 profile rejections
with no host imports. `i686` compile and Windows32bit CI are included separately.
Results, tested source hashes, benchmark/RSS data and limitations are recorded in
[jpeg2000-ex-gray-validation.json](jpeg2000-ex-gray-validation.json).

```sh
# OpenJPEG is fixture/oracle tooling only, never a production dependency.
python3 -B wml2/tests/fixtures/review/generate_jpeg2000_ex_gray.py \
  --compress /path/to/opj_compress --decompress /path/to/opj_decompress \
  --work-dir .test-jpeg2000-oracles
cargo test -p wml2 --locked --release --no-default-features \
  --features jpeg2000,image-buffer-ex --test jpeg2000_ex
cargo run -p wml2 --locked --release --no-default-features \
  --features jpeg2000,image-buffer-ex --example jpeg2000_ex_verify -- \
  wml2/tests/fixtures/review/jp2_ex_gray16_bench.jp2 .test-gray16.u16le 16 100
python3 -B wml2-test/scripts/verify_jpeg2000_ex_wasm.py --overflow-checks --work-dir .test-jpeg2000-ex-wasm
python3 -B wml2-test/scripts/verify_jpeg2000_ex_wasm.py --default-features --overflow-checks --work-dir .test-jpeg2000-ex-wasm-default
```

## Initial local validation checkpoint (af4bdf6)

Rust1.91/Linux workspace:502 passed,0 failed,14 existing ignored. Final focused
lib plus six EX groups:36 passed,0 failed. Minimal/default actual Node24.19.0
WASM each pass80 positives and12 profile rejections, including byte strides,
callbacks and32bit overflow. i686 compile, Clippy, formatting, CI syntax and the
package file list passed. Existing Clippy warnings remain; no new diagnostic is
introduced in the EX tests/helper/example. The earlier feature matrix and final
test-only overflow strengthening are distinguished in the machine record.

Eight longer alternating legacy benchmark pairs retained every output hash.
Median changes ranged-4.52%..+5.07%; a third longer repeat of the small RGB case
measured-1.31%. Initial noisier measurements are retained as well. Post-exec
VmHWM remained similar (about1.4–1.9MiB for these small inputs); absolute EX
128×96 medians were5.68ms at12bit and6.87ms at16bit, about1.6MiB VmHWM.
There is no prior high-depth API to use as a relative baseline. These local
measurements do not guarantee all-input/all-host performance. The initial
wait4-based peak includes launcher memory and is not used as the decode peak.

Builds used two jobs; sampled cgroup peak and the16GiB limit are recorded.
[Actual decoded/reference and low-byte display](jpeg2000-ex-gray-display.png)
was visually checked. Raw logs and measurement harnesses are retained in
[jpeg2000-ex-gray-raw.json.gz](jpeg2000-ex-gray-raw.json.gz).

## Large-origin indexing correction

EX output now subtracts absolute image origins before combining row and column
offsets. A valid 33x29 canvas with X origin `0xffffff80` therefore keeps its
intermediate indices within the checked canvas bounds on 32-bit targets.
Shared native/WASM regression tests preserve all source samples at both depths
and at X origins `0x80000000` and `0xffffff80`. Overflow checks are explicitly
enabled for the recorded WASM runs. The initial checkpoint above remains a
historical record; see the [correction validation record](jpeg2000-ex-gray-origin-fix-validation.json)
for the before/after checks, current source hashes and paired benchmarks.

## Later work

RGB/RGBA12/16 with explicit straight alpha and RCT, irreversible high depth,
other integer precisions, signed native samples, ICC and large-image streaming
are separate stages. EX has no native10bit/signed/YUV representation; simply
reinterpreting those bytes as Gray16 would violate its normalized-range contract.
