# JPEG pipeline tuning (NERO, 2026-10-23)

Base: WML2 0.0.39 / `40c43cef96253d27fd57bf97b5529611ff4c185f`.
This work changes no public API, numerical/color kernel, metadata policy, dependency,
or release version. JPEG EXIF fixes remain in place.

## Scope and bounds

- Reuse the coefficient Vec in sequential baseline decoding. Preserve the original
  per-job coefficient allocation and ownership transfer in parallel mode.
- Start the baseline worker only above 256 DCT blocks, rather than above eight
  MCUs. A 4:4:4 MCU and a 4:2:0 MCU have different work, so a fixed MCU-only cutoff
  needlessly disabled useful parallelism at some boundaries in the trial.
- Preserve the eight-MCU pending limit, bounded channels, raster callback order,
  cancellation flag, worker join/error propagation, and DecodeLimits enforcement.
  The small-image cutoff is a measured local heuristic, not a universal crossover.

Larger prototypes were rejected: four-MCU channel batches did not improve
throughput reliably; fixed-array IDCT output reduced sequential allocations but
repeatedly slowed large parallel baseline cases. A shared color-conversion
signature change and reserve-based worker buffer preparation were also dropped
to preserve the established large-image paths. None of these changes is included.

## Measurement

AMD Ryzen 7 7700, 8 physical / 16 logical cores, Windows NT 10.0.26300,
x86_64-pc-windows-msvc, Rust 1.99.0 (b940084d7), LLVM 23.1.1.
Release opt-level=3, no RUSTFLAGS, no concurrent benchmark/build/test processes.
Features: `--no-default-features --features jpeg,idct_llm,exif`, adding
`multithread` for parallel-enabled builds. The latter can still choose sequential
processing for small baseline files; progressive decoding is not parallelized here.

The unmetered `jpeg_bench` example reads encoded bytes before timing. Timed work
includes metadata parsing, output allocation, callbacks and decoding; file I/O,
output hashes, printing and optional RGBA dumps are excluded. First-draw latency
is also recorded. Existing `review_bench` was inspected, but its global allocator
atomics would perturb the producer/consumer comparison, so they are not used in
these timings. Allocation reductions are properties of the code, not a measured
RSS or peak-heap claim.

For each image and feature configuration, separate processes run in
before/after/after/before (ABBA) order. Each process/file has two warmups and five measured iterations;
reported medians pool ten samples per revision. This is one host and two process
launches per revision, without confidence intervals or a universal speed claim.
Background applications are not controlled or closed. An earlier whole-corpus
ABBA attempt exhibited substantial time-dependent variation; image-local ABBA
shortens the interval between paired measurements. Remaining variability and
slower cases are reported rather than interpreted as universal improvements.
Raw data and all case medians accompany this document.

Inputs are public synthetic gradients (quality 90, 4:4:4 and 4:2:0, baseline and
progressive) at 128x128, 1024x768 and 3072x2048; a resized bundled 8x8 fixture at
16x16; additional baseline boundaries at 32x32, 64x64 and 256x256; and the six
bundled `jpeg-icc-*.jpg` fixtures. Generator: Pillow 11.1.0 / NumPy 2.5.3.
Twenty-seven inputs decode successfully. The CMYK progressive fixture already
fails on the base (`Nothing marker but 7f offset:000000dc`), as documented by
`jpeg_progressive_cmyk_metadata_uses_the_same_source_profile_rule`; it is excluded
from speed comparisons and has the same failure after this patch.

## Reproduction

Use independent checkouts of the base and this branch. Copy only the identical
`wml2/examples/jpeg_bench.rs` timing harness into the base checkout. For each:

```sh
cargo build -p wml2 --release --locked --no-default-features --features jpeg,idct_llm,exif --example jpeg_bench
cargo build -p wml2 --release --locked --no-default-features --features jpeg,idct_llm,exif,multithread --example jpeg_bench
```

Preserve the resulting executables as `before-single`, `before-multi`,
`after-single`, `after-multi` (with `.exe` on Windows) in `.test-bench/`.
Then, from the repository root:

```sh
python wml2/tests/fixtures/review/generate_jpeg_bench.py .test-bench
python wml2/tests/fixtures/review/benchmark_jpeg_pipeline.py .test-bench
```

The runner compares full RGBA bytes across all four binaries, not just hashes.
No private JPEG, private metadata, local paths, or private screenshots are included.

## Regression coverage

Final source verification on NERO:

- `cargo test -p wml2 --release --locked --offline --lib --tests`:
  267 passed, zero failures, four existing ignores.
- Same command with `--features multithread`: 268 passed, zero failures,
  four existing ignores.
- `--no-default-features --features jpeg,exif,idct_aan,multithread` and
  `jpeg,exif,idct_slower,multithread`, each with
  `--lib --test jpeg_encode --test jpeg_exif --test callback_abort`:
  33 passed for each configuration.
- `cargo test -p wml2 --release --locked --offline --doc --features multithread`:
  12 passed.
- `cargo check -p wml2 --locked --offline --examples --features multithread`,
  `cargo fmt --all --check`, and `git diff --check`: passed.
- The generator reproduces all 22 measured synthetic inputs with identical SHA256;
  a separate runner smoke check passes with all four binaries.

The callback suite retains the 64x64 case and additionally uses 512x512 JPEG to
exercise cancellation with filled parallel queues after the new cutoff. Both
pixel and expanded-byte limit rejection are checked. Existing tests cover worker
panic propagation, truncation, metadata errors, EXIF orientation and limits.
Successful benchmark cases are compared byte for byte before/after, single/multi;
decoded synthetic PNG renderings are inspected locally for readability.

## Final results

Times below are pooled medians in milliseconds. Raw samples are in
[`jpeg-tuning-timings.csv`](jpeg-tuning-timings.csv), and all cases, input hashes,
binary hashes and build conditions are in
[`jpeg-tuning-results.json`](jpeg-tuning-results.json).

| Baseline input | Build | Before ms | After ms | Elapsed change |
|---|---|---:|---:|---:|
| 32x32, 4:4:4 | multithread enabled | 0.1308 | 0.0558 | -57.3% |
| 64x64, 4:4:4 | multithread enabled | 0.3548 | 0.2075 | -41.5% |
| 64x64, 4:2:0 | multithread enabled | 0.1980 | 0.1275 | -35.6% |
| 128x128, 4:2:0 | multithread enabled | 0.4717 | 0.4161 | -11.8% |
| 256x256, 4:4:4 | multithread enabled | 2.9725 | 3.3159 | +11.6% |
| 1024x768, 4:4:4 | multithread enabled | 33.7745 | 33.9385 | +0.5% |
| 3072x2048, 4:4:4 | multithread enabled | 262.9752 | 260.7684 | -0.8% |
| 3072x2048, 4:2:0 | multithread enabled | 124.4083 | 122.9238 | -1.2% |
| 3072x2048, 4:4:4 | single | 336.0761 | 326.9249 | -2.7% |
| 3072x2048, 4:2:0 | single | 183.9807 | 190.0306 | +3.3% |

For 32x32 4:4:4 the first-draw median falls from 0.0772 to 0.0114 ms;
for 64x64 4:2:0 it falls from 0.0891 to 0.0195 ms. These small jobs no longer
start a worker. Larger baseline jobs keep the original worker queue and numerical
path; their roughly flat times do not establish a throughput improvement.

The slower 256x256 result and other positive elapsed deltas are retained. We do
not claim a universal speedup or statistical equivalence. Progressive output is
unchanged byte for byte, but measured times still vary (the large 4:2:0
multithread-enabled case is +0.4%). On this host the convincing benefit is
avoiding startup on small baseline jobs; the performance crossover on other CPUs
and under different background load is unverified.

The private regression JPEG also decodes successfully in all four binaries and
produces identical RGBA bytes with the input SHA checked locally. The file,
metadata and output are absent from the public timing corpus and repository.
