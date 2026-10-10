# DDS BC1 performance follow-up

Baseline: `e773fe2072987c3e878ad4a60ac0119ffd9270ee`. The pre-fix revision is `1da8950016059c2591541a87a65e188068be3d19`; the final change adds only `#[inline(always)]` to the existing BC1 kernel, shared by BC2/BC3. Pixel arithmetic, callbacks and format handling are unchanged.

Rust 1.91 release builds used separate fresh targets and the same benchmark package/source. Each process warms up 10 decodes and times 100 `image_load` calls on a 512×512 input. CPU affinity is fixed with `taskset`; base/head order alternates. Output RGBA size and hash agree for every pair. Final measurements ran after the other builds/tests finished. This is a shared-host synthetic benchmark, not a guarantee for other machines or inputs.

## Reproduced regression before the fix

| CPU | Median paired difference | Ratio quartiles (%) | Median base / head (µs per decode) | Same-binary control median |
|---|---:|---|---|---:|
| 0 | +3.58% | 2.03, 3.58, 5.53 | 752.04 / 777.07 | -0.08% |
| 1 | +3.39% | 1.13, 3.39, 5.77 | 759.03 / 785.30 | +0.40% |
| 2 | +2.98% | 0.74, 2.98, 5.42 | 776.93 / 802.51 | -0.49% |

There are 40 pairs and 16 same-binary controls per CPU. Both execution-order groups retain a positive median on all three CPUs. The old BC1 arithmetic and dispatcher instructions are unchanged apart from addresses, while the surrounding decode function has different stack/register allocation. The once-per-image DX10 branch does not directly explain a per-block cost. Inlining the small BC1 kernel removes its per-block call and return-buffer boundary, without duplicating the compressed-image loop or changing alpha handling. The precise microarchitectural cause is not established.

## Final measurements

24 pairs per CPU and format; negative percentages are faster than baseline. BC2/BC3 are included because they use the same kernel.

| Format | CPU0 median | CPU1 median | CPU0 base / final (µs) | CPU1 base / final (µs) |
|---|---:|---:|---|---|
| bc1 | -7.80% | -7.41% | 748.69 / 686.76 | 734.32 / 679.53 |
| bc2 | -0.48% | -0.32% | 997.55 / 995.04 | 1016.50 / 1013.34 |
| bc3 | -0.29% | +0.65% | 1075.78 / 1067.69 | 1095.89 / 1109.86 |
| bc7 | +1.64% | +1.75% | 2142.32 / 2202.13 | 2108.13 / 2149.69 |
| bc7-partitioned | +1.24% | +1.46% | 1864.49 / 1893.68 | 1900.86 / 1919.83 |
| rgba | -1.35% | -3.28% | 3149.76 / 3107.24 | 3254.43 / 3136.91 |

The BC1 regression is resolved on these measurements. Small BC7 differences remain; zero regression across all formats is not claimed. Earlier runs concurrent with other test activity were discarded from this final table.

## Reproduction identifiers

- Base binary SHA256: `a833d12ae9fc9e1ffadb526379d5576817291e278f47a33782b2450e4b66e8e1`
- Final binary SHA256: `2370b0ab99e0199cd2d937070aec82c25d6a425ba026a3de9f91a7cfd9196228`
- Final DDS source SHA256: `f02229e4f0d90db1600fa25c6a09f82f08bdbc1f3f737b3d6498b849a032e042`
- BC1 input SHA256: `e6338cff6ee6ecd054655cd474481a0dfd0f303644f56898ad420ee036cfb900`

BC1 input uses a legacy DXT1 header, width/height 512, and 16,384 repetitions of block `00f8e007e4e4e4e4`. BC2/BC3 prepend eight `ff` alpha bytes to each block and use DXT3/DXT5 respectively. The other cases use the earlier fixed BC7 and RGBA fixtures. Raw paired samples and hashes are retained in the local review artifacts `.test-bc1-recheck.json` and `.test-bc1-fixed-clean.json`.

Validation: codec-only 125 passed (7 ignored), default plus high-bit-depth 290 passed (11 ignored), formatting, diff checks, Clippy and wasm32 check passed. The final BC1 image matched Pillow in all 262,144 RGBA pixels and was displayed. Ignored/manual tests and wasm runtime execution are not claimed by these counts.
