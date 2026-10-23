# JPEG restart intervals, including the final MCU row

Independent correctness branch based on main / WML2 0.0.39 at
`40c43cef96253d27fd57bf97b5529611ff4c185f`. The small/large JPEG performance
commits are not dependencies and are not included in this patch. The progressive
performance recheck is paused, with its script/builds retained in the other
worktree; no performance conclusion is drawn from that unfinished work.

## Minimal existing-version reproduction

Public synthetic 64x32 RGB, Pillow 11.1.0/libjpeg, quality90, 4:4:4, baseline:
`restart-minimal-i0.jpg` has no restart markers; `restart-minimal-i1.jpg` uses
one-MCU restart intervals. They encode the same source pixels and quantization.

With only this patch's tests/fixtures added to unmodified 0.0.39, run:

```sh
cargo test -p wml2 --release --locked --test jpeg_restart
```

The original code fails with `pixel mismatch: restart-minimal-i1.jpg`. Its decoded
RGBA differs from the plain reference at **448 of 2048 pixels**. The fixed decoder
has **zero** mismatches; both decoded renderings were inspected locally. The old
decoder can return a partially filled image rather than the complete raster.
No private picture, metadata, log or output is used for the published fixtures.

## Cause and correction

- Both baseline and interleaved progressive paths gated restart handling on
  `mcu_y < mcu_y_max - 1`, excluding every interval in the last MCU row. The new
  condition checks whether any MCU remains in raster order. It deliberately does
  not request an extra restart marker after the final MCU, including exact
  interval multiples.
- Restart markers are consumed at known unit boundaries, discarding only the
  previous byte's padding. A shared reader checks the RST0..RST7 cycle, beginning
  with RST0 for each scan. An RST encountered inside entropy-coded data is no
  longer silently skipped. The old fallback also extracted its RST number from
  the `0xff` prefix instead of the marker byte.
- DC predictors reset after each accepted restart. Progressive EOBRUN also resets
  at restarts and at scan starts.
- Noninterleaved progressive scans count **component blocks**, not frame MCUs.
  Component-coordinate row/column bounds keep padded/odd-size scans and their
  restart counters consistent. Interleaved scans continue to count frame MCUs.
- Progressive DRI segments between scans now update the interval and require
  their specified four-byte length; the old branch misidentified DRI as DNL.
  The bit-reader reset at each SOS also resets the expected RST cycle.

The numerical kernels, color conversion, worker count, queue bound, EXIF handling,
public API and version are unchanged. Existing callback behavior is retained:
Abort terminates once; an ordinary draw error propagates without inventing a
terminate callback. Worker shutdown/join remains on the established path.

## Fixtures and regression coverage

`generate_jpeg_restart.py` recreates 56 small public JPEGs, approximately 280 KiB
in total, using only the deterministic RGB formula in that script. Grayscale uses
Pillow's RGB-to-L conversion. SHA256/size and marker counts are recorded in
[`jpeg-restart-fixtures.json`](jpeg-restart-fixtures.json).

The eight groups cover minimal 64x32, 128x48 baseline/progressive 4:4:4 and 4:2:0,
odd baseline 4:2:0 at 129x49, and baseline/progressive grayscale at 65x33. Each has
a plain reference and restart intervals 1, 2, 3, 8, 100 plus one-row intervals.
All **48 restarted cases** exactly match their plain reference's complete RGBA
buffer. This checks DC/EOB reset, bit padding, row/non-row boundaries, exact and
incomplete final intervals, intervals longer than a scan, RST wrap, and SOS/DRI
changes. Plain references use the same WML2 IDCT; this is a restart consistency
oracle, not a claim of bit-exact color agreement with another decoder.

Negative tests mutate RST0 and the next RST0 after wrap, remove required markers,
truncate markers, corrupt an inter-scan DRI length, and enforce pixel/expanded-byte
limits. Callback Abort and deliberate draw failure exercise the parallel queue
shutdown path. Tests run with/without multithread, and with LLM, AAN and slow IDCT.

Final checks on Windows x64 MSVC / Rust1.99.0:

- release default features: 271 passed; with multithread: 272 passed. Zero
  failures, four existing ignores in each configuration.
- AAN and slow-IDCT focused suites: 37 passed each.
- debug multithread restart suite: four passed.
- release multithread doc tests: 12 passed; examples compile; formatting and
  `git diff --check` pass.
- The private JPEG's input SHA was checked locally; its full RGBA matches old
  0.0.39 and fixed single/multi output. Existing EXIF and Orientation tests pass.

## Ordinary-image performance control

No-RST inputs were compared to 0.0.39 using the same existing `jpeg_bench` harness,
release opt-level3, `--no-default-features --features jpeg,idct_llm,exif` with/without
multithread, on NERO Ryzen7 7700 (8 physical/16 logical), Rust1.99.0. For each
input/configuration: ABBA processes, two warmups and five timed samples per process.
File I/O, hashing and output dumps are outside the decode timer. The temporary
harness is not included in this correctness patch. All four outputs are identical.

| No-RST input | Single before/after ms | Multi enabled before/after ms |
|---|---:|---:|
| 1024x768 baseline 4:4:4 | 43.070 / 41.186 | 35.136 / 34.202 |
| 3072x2048 baseline 4:2:0 | 190.748 / 182.101 | 130.524 / 128.651 |
| 3072x2048 progressive 4:4:4 | 534.954 / 522.413 | 513.668 / 531.038 |

The progressive multi-enabled control is approximately 3.4% slower in this local
sample. Background applications were not controlled; no confidence interval or
universal speed/absence-of-regression claim is made. No unrelated performance
change was added to chase this value. Raw control data are retained in
[`jpeg-restart-benchmark.json`](jpeg-restart-benchmark.json). Timing the previously
broken RST files against their partial old raster would compare different work
and is intentionally not presented as a performance result.
