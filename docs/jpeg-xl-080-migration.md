# Published JPEG XL 0.8.0 decoder migration

This development change addresses #40, building on #72 and #73 at `8fa0b978c4f68e998439e1bffe0b567f46024074`. Pixel decoding moves to published **jxl =0.8.0**. Published **jxl =0.7.4 remains as a temporary parser-only compatibility dependency**. No all-0.8 dependency graph, merge, release, wider supported profile or full heap guarantee is claimed. Published WML2 0.0.41 is not relabeled as containing this unreleased change.

## Dependency and admission boundary

| Alias | Exact registry package | Role |
| --- | --- | --- |
| jxl_codec | jxl 0.8.0 | Event decoder, profiles, pixel format/output, cooperative runner |
| jxl_preflight | jxl 0.7.4 | Dimension prefix, FileHeader, ICC entropy preamble/skip, frame geometry and TOC admission only |

Both are optional under `jpeg-xl` with default features disabled. Feature-off builds activate neither. Both versions declare Rust 1.89; WML2's Rust 1.91 policy remains. Normal registry dependencies propagate to consuming applications without a root Cargo patch. The dual graph adds both generations of jxl_macros/simd/transforms; it is an explicit temporary maintenance and build-size cost, not vendoring or an external fork. No adapter call instantiates the 0.7.4 pixel decoder.

#54 (`7c81dea`) correctly identified the private-parser obstacle and retained the then-current runtime. This implementation does not remove its checks or move ICC admission after BasicInfo: it isolates the existing published readers and changes the actual reconstruction backend. The bridge removal criterion is a supported, equivalently bounded 0.8-or-later pre-allocation interface, or a separately approved parser/source maintenance strategy. Merely deleting the alias does not satisfy that criterion. #54 remains a historical evaluation; no other PR is merged or overwritten.

The [validation snapshot](jpeg-xl-080-migration-validation.json) pins crate checksums, source revisions, audited files, commands/results and measurements. The package is named `jxl`, not a guessed alternate package. 0.8.0 is source commit `9a08ca643fd2d01f803dd0be6a5bebc9cc11e597`; the bridge is `624ce908afcf8eb0dc2585a671535eaabb5d88cc`.

## API and supported subset

The adapter closes its complete input and requires BasicInfo → FrameHeader → FrameComplete(false) → Complete. NeedMoreInput is a truncated-input error; unexpected events fail closed. File length is checked before publishing the image. Output-profile access stays borrowed through metadata selection; equal simple source/output encodings reuse the validated serialized envelope through an admitted fallible copy (avoiding a second factory call), while raw ICCs avoid equality scans; raw ICC/header copies and cumulative adapter/CMS admission from #72/#73 remain fallible and intact. Cancellation is passed through every process call and the real codec runner; no partial canvas, pixels or metadata are published on refusal/cancellation.

The new public BasicInfo projection must agree with bounded preflight on oriented dimensions, original orientation, integer source depth, absent animation, extra-channel count/type and straight alpha. ICC and internal-frame admission still occur before the decoder is initialized. Public 0.8 BasicInfo is not used as a substitute for intrinsic, ICC, TOC or invisible-reference checks.

`adjust_orientation=true`, `AlphaMode::AsIs`, high precision, skipped preview and disabled spot rendering are explicit. `ProfileLevel::Main5` preserves 0.7.4's three former force_level5 defaults (splines, patches, modular channels). Original validated Level5/Level10 container labels are still retained/accepted by the envelope parser; extracted codestreams do not activate Main10-only complexity. New animation, associated/subsampled/general extras, HDR/float and preview-extraction capabilities are not automatically enabled. All fixed input, ICC, geometry/padding, TOC, reference-layer and workspace screens remain.

Wire review finds identical FileHeader, size/preview and bit-depth grammar definitions. ImageMetadata changes remove an unused constructor and a loose extra-count check; WML2 still admits at most one straight full-resolution alpha. TOC/frame macro arguments use shorthand without changing serialized fields. 0.8's extra-channel dim_shift fixes concern inputs already rejected by the bridge. Color-matrix validation is stronger and malformed profiles can reject earlier. BitReader overread bookkeeping, entropy row fast paths and LZ77 scratch reuse do not change the admitted wire fields; independent fixtures exercise complete ICC skipping, chunking, frame/TOC scanning/layers and orientation. This source review and regression evidence is not a formal proof for every malformed stream or an allocator-wide bound.

## Validation and measured costs

Local Rust 1.91.0 Linux tests pass 139 ordinary/EX/sRGB/metadata/refusal/resource/prefix checks, including 50 library checks with allocator refusal and the added event/projection disagreement test. Minimal JXL passes 80; disabled-feature configuration passes two. The full compatible all-target feature combination compiles. The existing matrix continues to test default-off, precision and unrelated codecs.

Actual WASM executes 229 composed checks with no imports, including native depths, mixed alpha, all orientations, intrinsic/preview composition, metadata/brob, jbrd, truncation and limits. The resource verifier passes 12 groups at each actual 32 MiB and 64 MiB linear-memory ceiling, including CMS/cancellation and explicit memory.grow refusal. Local i686-musl is **compile-only** because this host cannot execute ELF32; executable Windows 32-bit and macOS coverage comes from CI and is reported separately. Only repository fixtures/generated inputs were used; historical external/private corpora are not claimed as rerun.

Release comparisons rebuild the #73 baseline, migrated source and baseline again at the same staged source path, with identical Rust 1.91.0/features/release settings. Baseline binaries match byte-for-byte. Two pinned logical CPUs run six balanced process blocks per input, with documented warmups, process medians, variability and bootstrap intervals; all output hashes match. This is end-to-end admission/decode/metadata/rounding plus the existing allocation-meter and pixel-hash probe, not #54's trusted-core-only benchmark.

The 606×500 photo takes about 0.784× baseline time and the layered image about 0.895–0.905×. Tiny RGB takes about 1.023–1.044× and tiny ICC about 1.100–1.109× baseline: this is a real small-input regression, not a universal speedup or a cleared all-case performance gate. Exact absolute times, requested allocations and peaks are recorded in the snapshot. The sample release benchmark executable grows 3,744,496 → 4,031,440 bytes (+286,944; about 7.7%); this is one executable/configuration, not a crate/RSS size promise. These tradeoffs require review before adoption.

Reproduce ordinary/EX/sRGB tests with the selected feature flags in the snapshot. Run `python -B wml2-test/scripts/verify_jpegxl_wasm.py --output .test-jxl080-wasm --exif` and `verify_jpegxl_resources_wasm.py --output .test-jxl080-resources`. Prime locked packages with `cargo fetch --locked` before the offline benchmark. The benchmark recipe in `wml2-test/scripts/benchmark_jxl080.py` requires local Git history containing the #73 baseline; use a fresh `.test*` directory on an otherwise idle Linux host. Shared-runner latency is not a CI gate.

## Resource work and pending decisions

#72's finite resource APIs and #73's owned-copy changes are direct prerequisites, preserved on this branch; this PR is stacked on #73. It is a sibling of #74, not based on its evaluation branch. #74's 0.7.4 entropy patch is not applied to either published dependency. Its cumulative-lease/failure-test principles can inform future work, but 0.8 moves/reclaims LZ77 scratch between readers and retained scratch owners. Charging only the old reader lifetime would release too early; private module access and every preflight/decoder reader root need fresh qualification. Frame/modular/reference/metadata/CMS paths remain outside a complete shared-codec ledger. #37/#63 and the 2560×1440 admission issue remain unresolved.

The user authorized the 0.8.0 decoder update. This draft's temporary 0.7.4 parser dependency, small-input slowdown and size cost are explicit review points. No external repository, upstream contribution, dependency release, PR approval or merge is performed here.
