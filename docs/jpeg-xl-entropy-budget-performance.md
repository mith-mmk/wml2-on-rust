# Entropy evaluation performance diagnosis

This extends evaluation draft [#74](https://github.com/mith-mmk/wml2-on-rust/pull/74), comparing its original `af18090a8766ca46cc79b702f2e56c2ee420c498` patch with the separate-lease histogram storage change. Product dependencies, source, API and fixed caps remain unchanged. [Machine-readable measurements and source ablations](jpeg-xl-entropy-budget-performance.json) preserve process medians, variability, bootstrap intervals, build/source hashes, negative controls and untimed reservation counts.

## Controlled result

The photo regression reproduces under matched builds. Keeping the histogram's hot storage as the original raw `Vec<u8>`, with its non-cloneable lease in a separate private field, returns measured decode time to the original-codec level. The earlier small-image 3% slowdown does not consistently reproduce under these controls. No platform-wide speedup or future decoder-wide budget overhead is inferred.

All ratios below divide a process median by its paired original-codec process median. Brackets are bootstrap 95% intervals over process pairs, not over individual iterations.

| Case | Previous patch / original | Updated patch / original |
| --- | --- | --- |
| Photo, logical CPU 0 | 1.0676 [1.0631, 1.0732] | 0.9969 [0.9940, 1.0024] |
| Photo, logical CPU 1 | 1.0722 [1.0690, 1.0761] | 0.9950 [0.9935, 1.0045] |
| Photo, allocation-meter + timed-hash probe | 1.0575 [1.0519, 1.0687] | 0.9944 [0.9903, 1.0034] |

The final four-condition experiment uses original, byte-identical original rebuild, previous patch, and updated patch. Each input/core has 12 process blocks with five warmups per process; measured decodes per process are simple 200, ICC 100, photo 20 and optional large ICC 20. Two pinned logical CPUs run sequentially. The photo therefore has 480 measured decodes per condition. All hashes match. Builds finish before timing. Pixel hashing occurs outside the primary decode timer; an independent check repeats the previous allocation-meter/timed-hash probe. Each condition occupies every process position equally: rotate a full cycle before reversing the next cycle. A four-condition ordering pilot reversed too frequently; it is excluded from final qualification. The reproducer's self-check verifies position balance.

The original codec is rebuilt twice at identical source/manifest paths, with the same registry lock, Rust 1.91.0 and explicit release settings (opt3, codegen16, no LTO/incremental/debug, unwind). Both rebuilds produce byte-identical decode and allocation-meter binaries. A/A paired photo medians are 0.9984 and 0.9989; their intervals include or approach 1, providing a noise control. Final updated ratios for simple are 0.9991/0.9965, ICC 0.9745/0.9730 and optional large ICC 0.9927/0.9931. These results qualify the measured ordinary cases only. WASM performance and native 32-bit performance are not measured.

## Attribution and safety

Untimed diagnostic copies count zero reservations and zero lease drops on ordinary photo decoding, both before and after. Those legacy calls supply no entropy budget, so budget synchronization cannot explain their slowdown. In explicit budgeted probes, 100 maps cause 100 reservations; 100 windows plus their histogram map cause 101 reservations. Drops match. No per-symbol reservation is introduced. These counters are added only to scratch diagnostic copies after latency runs and are absent from the evaluation patch and timed binaries.

Matched source ablations find that a capacity-check-only change, compact wrapper metadata alone and fixed-window lease metadata alone do not remove the photo regression. Combining the fixed-window experiment with raw histogram storage is also inferior to raw histogram storage alone. The evidence supports a storage-representation/code-generation interaction, not a claim about a specific CPU instruction, virtual call, or hardware-counter bottleneck. No sampling profiler was used. On this 64-bit host, the selected change reduces Histograms from 176 to 168 bytes; SymbolReader remains 72 and standalone EntropyVec remains 48. Size alone is not established as the cause, because the compact-size control does not recover performance.

Parsing retains the owned EntropyVec until all fallible histogram construction finishes, then moves its Vec and reservation directly into Histograms. The raw Vec is private; resize goes through a fallible helper that admits the entire replacement while the old allocation remains charged. It frees old storage before replacing/releasing the old lease. Field declaration order likewise frees retained map storage before its lease on histogram destruction. No budget arithmetic, quota, allocation-failure behavior, reference count or ownership interval is weakened. The LZ77 window and symbol loop are unchanged.

Fourteen native checks now include actual histogram-resize allocator refusal, unchanged old data after failure, successful retry, and a deallocation observer proving that 48 reserved bytes remain live while the old 16-byte backing store is freed, and 32 while the replacement is freed. Standalone map/resize/window refusal, exact/minus-one limits, nested maps, clones/moves/checkpoints and shared admission tests remain. The modified codec also passes 84 existing WML2 integration tests, six actual WASM case groups under 64 MiB, and displayed CMS pixels within RGB error 1 with exact alpha. This does not connect a budget throughout the decoder or finish #37/#63.

## Reproduce

With the recorded Rust 1.91.0 toolchain, cached locked dependencies, Linux affinity support and local Git history containing `af18090`:

```sh
python -B wml2-test/scripts/diagnose_jpegxl_entropy_budget.py --check-order
python -B wml2-test/scripts/diagnose_jpegxl_entropy_budget.py --output .test-entropy-perf --offline --diagnostics
```

Use a fresh `.test*` directory. Omit `--offline` to permit normal Cargo fetching. Optional `--extra-input /path/large.jxl` includes the independent 1 MiB ICC scratch input described in #73's snapshot; its hash is recorded, and it is not a new compatibility fixture. `--ablations --blocks 24` also reconstructs the archived, non-adopted diagnostic source variants from their recorded deltas. They are experimental controls, not additional product implementations. The previous patch is read from local Git, never from a CI artifact download. The script fixes all source/build paths within the experiment, verifies crate/patch hashes and registry versions, builds all variants before timing, checks byte-identical A/A rebuilds and pixel hashes, records raw CSV and variability, then optionally builds untimed event counters. CI checks process ordering and functional/failure tests; shared-runner timing is not a CI performance gate.
