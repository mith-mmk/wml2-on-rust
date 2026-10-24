# Progressive performance regression check

Completed on NERO, 2026-10-24, without changing the JPEG implementation.
Before: `1be4b9f9777fbe101e0941885d20b7d3513721ac`.
After: `68ff410d9089f69251000ead54a66567ff2a71aa` (short Huffman lookup).
The independent restart fix/PR28 stays at `4c14f51041d72ae5697bfb6af74ac34b033746af`;
it was neither modified nor merged into this branch.

## Decision

The earlier single-run approximately +4.8% progressive 4:4:4 multi-enabled result
does not reproduce as a stable slowdown. Separate-process results change sign and
are sensitive to load/drift and aggregation. A source-matched same-process
comparison favors the new decoder in seven of eight target blocks, with median
block change **-5.9%**, while baseline multi-enabled blocks retain approximately
13% improvement. There is no evidence here that warrants disabling the short-code
path for progressive or changing the working baseline optimization.

For these inputs/host, treat the isolated +4.8% value as measurement variation,
not an established implementation regression. This is not proof of no slowdown on
every progressive profile, CPU or binary layout. Production code remains intact;
this follow-up adds evidence and a reproduction helper only.

## 1. Repeat the original separate-process method

Same release opt-level3, Windows x64 MSVC / Rust1.99.0, Ryzen7 7700 (8 physical,
16 logical), no default features, `jpeg,idct_llm,exif`, optionally `multithread`.
No background application was closed or reconfigured.

For each large progressive 4:4:4/4:2:0 input and single/multi configuration, run
three ABBA/BAAB cycles: six processes per revision, two warmups and five measured
decodes per process (30 timed samples per revision). Pair adjacent processes and
compare each process's median. Baseline controls use one ABBA cycle per input.
File I/O, hashes and RGBA comparison are outside the timer.

| Progressive input | Build | Pooled before / after ms | Median adjacent-pair change | Pair range |
|---|---|---:|---:|---:|
| 3072x2048 4:4:4 | multi enabled | 515.44 / 535.42 | -2.29% | -13.23% to +13.04% |
| 3072x2048 4:4:4 | single | 536.90 / 511.80 | -2.74% | -18.73% to -0.60% |
| 3072x2048 4:2:0 | multi enabled | 277.03 / 284.09 | +3.30% | -8.54% to +11.75% |
| 3072x2048 4:2:0 | single | 268.67 / 269.58 | +2.01% | -14.26% to +3.63% |

Pooled medians describe all collected samples, but do not isolate causality when
process-level conditions drift. On the target case, their positive delta coexists
with a negative adjacent-pair median and large sign reversals. Neither number is
discarded or presented as conclusive by itself. Therefore add the following
within-process control instead of tuning against the original isolated number.

## 2. Same-process source comparison

Archive both immutable revisions locally. Rename only the two temporary Cargo
packages, remove unused test/example target declarations, and link their unchanged
library sources/dependencies/features into one diagnostic executable. No product
source, runtime flag, profiler or decoder branch is inserted.

Read the input once. Obtain an untimed reference output, then run two warmups per
revision and eight ABBA/BAAB blocks. Each block measures two decodes per revision;
compare the mean elapsed time within each block, then take the median of eight
block changes. Every returned RGBA buffer must equal the full reference bytes.
The clock surrounds `image_from` including output allocation; comparison and I/O
are untimed. Before/after libraries use the same adapter within this experiment.

| 3072x2048 input | Build | Before / after median ms | Median block change | Slower blocks |
|---|---|---:|---:|---:|
| progressive 4:4:4 | multi enabled | 633.14 / 602.85 | -5.91% | 1 / 8 |
| progressive 4:2:0 | multi enabled | 354.64 / 336.95 | -6.66% | 1 / 8 |
| progressive 4:4:4 | single | 632.46 / 592.91 | -5.68% | 1 / 8 |
| progressive 4:2:0 | single | 348.54 / 333.93 | -4.27% | 0 / 8 |
| baseline 4:4:4 | multi enabled | 343.13 / 296.02 | -13.31% | 0 / 8 |
| baseline 4:2:0 | multi enabled | 168.01 / 145.61 | -13.07% | 0 / 8 |
| baseline 4:4:4 | single | 424.66 / 389.35 | -8.68% | 1 / 8 |
| baseline 4:2:0 | single | 253.86 / 222.36 | -12.93% | 0 / 8 |

The target multi-enabled block range is -11.65% to +3.14%. The single-thread
4:4:4 control still has one large positive outlier; background variability is not
eliminated. There are only eight blocks and no formal confidence-interval claim.
This diagnostic also changes binary layout and uses `image_from` rather than the
original Sink adapter: do not compare its absolute milliseconds directly to the
separate-process table. Its paired source comparison supports the decision above;
it is not a universal substitute for application benchmarks.

Raw samples, paired process medians, all block changes, binary/input hashes and
conditions: [jpeg-progressive-review.json](jpeg-progressive-review.json),
[single CSV](jpeg-progressive-dual-single.csv),
[multi CSV](jpeg-progressive-dual-multi.csv).

## Reproduction

Use the existing public synthetic input generator and a new temporary directory:

```sh
python wml2/tests/fixtures/review/generate_jpeg_bench.py .test-inputs
python wml2/tests/fixtures/review/build_jpeg_revision_comparison.py .test-dual-review --before 1be4b9f9777fbe101e0941885d20b7d3513721ac --after 68ff410d9089f69251000ead54a66567ff2a71aa
.test-dual-review/.test-dual-multi.exe .test-inputs/large-s0-progressive.jpg .test-inputs/large-s2-progressive.jpg .test-inputs/large-s0-baseline.jpg .test-inputs/large-s2-baseline.jpg
.test-dual-review/.test-dual-single.exe .test-inputs/large-s0-progressive.jpg .test-inputs/large-s2-progressive.jpg .test-inputs/large-s0-baseline.jpg .test-inputs/large-s2-baseline.jpg
```

The builder uses cached dependencies with `--offline`, rejects an existing output
directory and never edits either original checkout. The two temporary package
names are `wml2-before`/`wml2-after`; library source is copied byte for byte from
Git. The helper was rebuilt in a second fresh directory and its full-pixel smoke
comparison passed. Executable suffixes above are for Windows, the verified host.

## Final regression validation and remaining scope

Private JPEG input SHA was checked locally; its complete RGBA is identical across
both revisions, single and multi. No private picture, metadata or pixels appear
in these records. Production-branch release suites (including EXIF Orientation,
corruption/limits/callback contracts) pass: 267 tests without multithread and 268
with it, with four existing ignored tests in each run. All 12 doc tests and
formatting pass. An initial file-I/O test attempt used a missing TEMP directory;
creating the isolated directory resolved those environment failures without any
code change.

This review's completion conditions are met: repeat/controlled comparison, retained
baseline benefit, full-pixel/private/EXIF and single/multi regression validation,
and saved reproducible evidence. Other platforms, real-world progressive corpora,
and throughput after integrating the separate restart fix are not measured here.
No main merge, release, CI waiting/monitoring, or restart PR update is performed.
