# Large JPEG decoding: sampled profile and short Huffman codes

Baseline: `1be4b9f9777fbe101e0941885d20b7d3513721ac`, retaining the earlier
small-image cutoff and sequential coefficient-buffer reuse. The objective here is
large-image decode throughput, not just image startup. No IDCT arithmetic, color
conversion, thread count, queue depth, public API or EXIF policy is changed.

## Profile before selecting the change

Same NERO Ryzen 7 7700 (8 physical / 16 logical), Windows x64 MSVC, Rust 1.99.0,
release opt-level 3. As in the earlier benchmark, features are
`jpeg,idct_llm,exif` without defaults, with/without `multithread`.

Temporary instrumentation samples every 64th call using Instant and atomic
counters. Each process decodes four times (one warmup plus three repetitions).
The following are extrapolated sampled wall milliseconds per decode, not CPU time
or a statistical partition of total elapsed time. Sampling can be biased by input
periodicity, clocks and counter overhead. Quantization, setup, output allocation,
some buffer management and scheduling remain outside these individual intervals.
Parallel intervals overlap and must not be added together.

| 3072x2048 baseline input, single | Entropy before/after | IDCT before/after | Color + allocation before/after | Draw before/after |
|---|---:|---:|---:|---:|
| 4:4:4 | 206.20 / 172.26 | 34.92 / 34.86 | 52.83 / 51.79 | 4.52 / 4.40 |
| 4:2:0 | 99.64 / 83.69 | 17.18 / 17.16 | 45.99 / 46.03 | 2.62 / 3.08 |

Parallel producer entropy intervals fall from approximately 209 to 179 ms
(4:4:4) and 102 to 88 ms (4:2:0). Sampled sends are approximately 4--14 ms and
receives 4--9 ms before the change, including synchronization overhead as well as
waiting. These are not pure blocked-thread durations. They and the lower sampled
IDCT/color intervals suggest the producer is the main bottleneck on these inputs;
that is an inference, subsequently checked by uninstrumented end-to-end timing.
Raw counters and estimates are in [jpeg-large-profile.json](jpeg-large-profile.json).

File I/O is excluded from the Rust decode timer. Separate warm-cache Python
`Path.read_bytes` measurements (two warmups, seven repetitions) take medians of
1.7656 ms for the 8,869,329-byte 4:4:4 input and 0.9346 ms for the 4,651,609-byte
4:2:0 input. This includes Python/allocation overhead and cached filesystem access;
it does not measure cold storage or Rust I/O performance. I/O and decoding are
therefore distinct measurements, not silently combined.

## Implementation and invariants

Each expanded Huffman table has a 256-entry, 512-byte short-code lookup containing
the symbol and consumed bit length. Only codes of at most eight bits entirely
present in the current entropy byte use it. Other codes follow the original
bitwise decoder. An empty byte is filled exactly when the old first-bit read would
fill it; the optimization never prefetches the next byte or entropy marker.

The constructor directly fills canonical prefix ranges to limit setup cost.
Truncated/oversubscribed or inconsistent tables leave the cache empty, retaining
the legacy decoding/error path rather than creating a new rejection policy.
The fixed overhead is 512 bytes per expanded table (at most eight in the existing
DC/AC layout); no image-sized buffers or unbounded cache is added.

Exhaustive unit comparisons cover all 256 current-byte values at all nine available
bit counts, comparing symbols/errors, reader offsets, residual bits and byte state
to the original bitwise decoder. The table includes 1/2/4/8-bit cached codes and
9/16-bit fallbacks. Additional checks cover stuffing, EOI/restart/fill markers,
truncation and cache suppression on malformed tables. Release regressions cover
corruption, limits, callbacks/cancellation, worker join failures and EXIF including
normal Orientation 1--8 and the malformed-metadata fallback.

Independent 4:4:4 / 4:2:0 row-restart fixtures also match their plain equivalents
and all four before/after single/multi binaries. A separate within-row restart
experiment exposes a pre-existing raster mismatch; its output is identical
before/after this optimization. That bug is not fixed or treated as new format
support here. See the [fixture provenance](../wml2/tests/fixtures/review/jpeg-huffman-restart.md).

## Final uninstrumented results

Image-local ABBA order, two processes per revision, two warmups and five timed
iterations per process; medians of ten samples, milliseconds. File I/O, hashes,
RGBA dumps and profiler instrumentation are outside these decode timings.

| Input | Single before / after | Multi enabled before / after |
|---|---:|---:|
| 1024x768 baseline 4:4:4 | 42.603 / 37.556 (-11.8%) | 35.241 / 30.109 (-14.6%) |
| 1024x768 baseline 4:2:0 | 23.327 / 20.895 (-10.4%) | 16.379 / 14.220 (-13.2%) |
| 3072x2048 baseline 4:4:4 | 327.485 / 288.383 (-11.9%) | 270.027 / 238.801 (-11.6%) |
| 3072x2048 baseline 4:2:0 | 185.317 / 165.796 (-10.5%) | 125.761 / 110.373 (-12.2%) |
| 3072x2048 progressive 4:4:4 | 559.658 / 474.720 (-15.2%) | 523.096 / 548.133 (+4.8%) |
| 3072x2048 progressive 4:2:0 | 281.252 / 258.662 (-8.0%) | 286.947 / 267.365 (-6.8%) |

The baseline improvement repeats the earlier prototype's approximately 11--14%
result. Progressive results vary more: the slower 4:4:4 multi-enabled result is
retained, not hidden. Background applications are not controlled or closed, and
the data do not establish universal speedups or confidence intervals. The tiny
16x16 baseline 4:4:4 multi-enabled median remains approximately 0.015 ms after
optimizing lookup construction; this does not negate setup overhead on other tiny
inputs. All case medians and raw samples are retained separately from the earlier
small-image work in [jpeg-large-results.json](jpeg-large-results.json) and
[jpeg-large-timings.csv](jpeg-large-timings.csv).

### Previously slower case

The earlier +3.3% single-thread 3072x2048 baseline 4:2:0 result was rechecked
against original 0.0.39 as well as the earlier branch. Three independent process
launches per revision, two warmups and five samples each used orders ABC/CBA/ABC.

| Revision | Single ms | Multi enabled ms |
|---|---:|---:|
| 0.0.39 | 185.854 | 125.305 |
| Prior branch 1be4b9f | 182.607 | 126.428 |
| New short-code decoder | 167.069 | 109.529 |

The earlier 3% single-thread regression does not recur in this recheck. The new
decoder is approximately 10% faster than 0.0.39 in single-thread mode and 13%
faster with multithread enabled on this case. This is a local repeat measurement,
not evidence that every observed older regression was only noise.

The 27 successful benchmark inputs and the private 6667x4000 regression JPEG
produce identical full RGBA bytes before/after and single/multi. Private input,
metadata, logs and output are absent from the repository and public timing corpus.

## Validation

- Release default features, `--lib --tests`: 271 passed, zero failures;
  four existing ignores. With `multithread`: 272 passed, zero failures;
  four existing ignores.
- `jpeg,exif,idct_aan,multithread` and `jpeg,exif,idct_slower,multithread`
  without defaults, `--lib --test jpeg_encode --test jpeg_exif --test callback_abort`:
  37 passed each, zero failures.
- Release doc tests with `multithread`: 12 passed. Examples compile, formatting
  and `git diff --check` pass.
- Full RGBA comparisons for the 27-input public corpus, private JPEG and four
  independent restart/plain fixtures; representative baseline/progressive decoded
  renderings inspected locally.
- Profile instrumentation generated and compiled separately, then restored;
  the committed production source contains no profiling counters or timers.

## Reproduction

The earlier [benchmark procedure](jpeg-tuning.md) and its input generator/ABBA
runner are unchanged. Use this document's baseline commit rather than 0.0.39 for
`before-*`, and the new revision for `after-*`. Compare the same 27 successful
synthetic/bundled inputs byte for byte. The pre-existing unsupported CMYK
progressive fixture remains excluded from timings with its original error.

For sampled stage profiling, use isolated `.test*` checkouts of each revision:

```sh
python wml2/tests/fixtures/review/instrument_jpeg_stages.py wml2/src/jpeg/decoder.rs .test-instrumented.rs
```

The generator leaves its input intact. In the isolated checkout only, copy the
generated file over `decoder.rs`, build `jpeg_bench` with the same features and run
`jpeg_bench 3 1 <input>`. The final stderr `PROFILE` arrays are cumulative call
counts and sampled nanoseconds for entropy block, IDCT block, color/MCU, send,
receive and draw/MCU respectively. Multiply sampled ns by 64 / 4 for the table's
per-decode estimates. Restore the uninstrumented source before normal benchmarks,
tests or commits. Production code contains no profiler, timer or atomic counters.
