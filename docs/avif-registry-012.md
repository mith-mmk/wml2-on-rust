# Published AVIF 0.1.2 integration

Base: `65c5aba` (WML2 0.0.38). Only the two registry dependency requirements and
lock checksums change; no WML2 version, codec submodule revision, or production
adapter implementation changes. `avif-image-buffer` remains one registry 0.1.0.
Cargo metadata confirms no Git dependencies in the resolved workspace graph.
The `avif`/`avifenc` submodules remain fixture sources, not Cargo dependencies.

## Validation

Linux, Rust 1.99.0, tracked lock:

- `cargo test -p wml2 --locked --no-default-features --features avifenc,image-buffer-ex,high-bit-depth,color-management,multithread --lib --tests`: 156 passed, 0 failed, 2 manual benchmarks ignored.
- `cargo test -p wml2 --locked --no-default-features --lib --tests`: 29 passed, 0 failed.
- `cargo test -p wml2 --locked --lib --tests` (default features): 262 passed, 0 failed, 4 existing ignored.
- Matching feature-set Clippy succeeds with existing warnings; formatting and diff whitespace checks pass.
- Existing native/highres tests exercise 8/10/12bit and independent alpha planes.
- Added synthetic 65×67 RGBA12 lossless roundtrip crosses transform boundaries;
  every low-bit color/alpha sample and the original ImageBufferEx storage agree.
- Existing EX Exact/Round rejection, padded/odd strides, grayscale, limits and
  callback finalization contracts pass.
- Nine existing exports from `avif_ex` and `highres_avif_encode` agree exactly
  with Pillow 12.3.0 RGBA8 and FFmpeg 7.1.5 native color/alpha planes. Includes
  legacy, native, shared, WML2 EX and highres writers, 8/10/12bit, straight alpha.
  The independently decoded checkerboard preview was visually inspected.

Reproduce independent verification (synthetic samples only):

```sh
mkdir -p .test-avif-012
WML2_AVIF_REGISTRY_ORACLES="$PWD/.test-avif-012" cargo test -p wml2 --locked --no-default-features --features avifenc,image-buffer-ex,high-bit-depth --test avif_ex --test highres_avif_encode
python3 -B wml2-test/scripts/verify_avif_registry_oracles.py --output .test-avif-012
```

## Output and allocation compatibility

Upstream reconstruction/filter boundary fixes can intentionally change decoded
pixels. Tight strict-native allocation budgets can now reject images because
full transform padding and actual capacities are counted. WML2 EX and highres
use that strict entry point. Reassess caller budgets; this change does not
silently raise WML2 defaults. See [the adapter contract](image-buffer-ex-avif.md).

Existing ICC fixtures pass, but this is not an independent general HDR/ICC
accuracy claim. PQ/HLG display/tone mapping, arbitrary ICC profiles, all HDR/alpha
combinations, and WML2 AVIS are outside this update's validation. The external
reader exports are 2×2 identity RGB cases; the new odd-size low-bit test uses
WML2/its updated backend and is not a second independent codec oracle.
No private user images or extracted planes are included.

## Performance observations

[Machine-readable results](avif-registry-012-validation.json) retain reader hashes
and both benchmark passes. Existing `review_bench` release binaries were built
against the base and updated locks. After builds stopped, three alternating
runs of 100 encode iterations on one pinned CPU (first five discarded) gave:

| 16×16 EX lossless, odd stride | Base median | 0.1.2 median | Change |
| --- | ---: | ---: | ---: |
| Gray8 | 0.671 ms | 0.715 ms | +6.5% |
| Gray12 | 0.430 ms | 0.451 ms | +4.9% |
| RGBA8 | 14.344 ms | 14.430 ms | +0.6% |
| RGBA12 | 16.373 ms | 16.088 ms | -1.7% |

All four encoded hashes and measured peak allocations match the base. Small
monochrome encode has a modest measured slowdown; no universal performance
improvement is claimed. The earlier unpinned pass varied from -2.9% to +15.8%.
For `decode-ex-file 30 samples/WML2Viewer.avif`, three alternating 10-iteration
runs measured 170.63 ms → 155.59 ms (-8.8%), with identical RGBA16 hashes.
The first sequential pass instead measured +3.8%, illustrating timing variance.
Decode peak live allocation rose 14,975,361 → 15,601,665 bytes (+626,304),
consistent with upstream padded-plane accounting/storage. These are local
microbenchmarks, not a throughput or RSS guarantee for arbitrary inputs.
