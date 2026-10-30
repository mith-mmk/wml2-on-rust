# JPEG XL encoding backend evaluation (#49)

Assessment: 2026-10-30 against WML2 main
`6a48db924b338c5ae7095bea6105fb4dbd436e95`. This report selects a next experiment;
it adds no backend, runtime dependency, encoder capability or save API.

## Decision

Investigate **zune-jpegxl 0.5.2**, pinned with default features disabled, as a
permissively licensed Pure Rust **lossless still-image** candidate. Start with
8-bit Gray/RGB/RGBA and explicit sRGB signaling. Qualify Gray/RGB 16-bit and
16-bit alpha separately rather than inferring support from the advertised
maximum bit depth. Adoption is blocked on the oracle, resource and metadata
contracts below. Do not silently discard ICC or promise a general encoder.

Keep `jxl` pinned for decoding pending #40; its 0.8.0 update does not supply an
encoder. Keep original-JPEG byte reconstruction (#48), animation, HDR/float and
lossy quality design outside the initial implementation. A new independent
encoder would entail Modular transforms/predictors, entropy coding, frame/header
writing, alpha/color metadata and container work, rather than a simple wrapper.

## Candidates and evidence

| Candidate | API/coverage found | License and build implications | Assessment |
| --- | --- | --- | --- |
| Published `jxl` 0.7.4 / 0.8.0 (`libjxl/jxl-rs`) | Decoder APIs, no public pixel encoder | BSD-3-Clause; already used by WML2 | No save backend; migration evaluation is #40 / PR #54 |
| `zune-jpegxl` 0.5.2 | `JxlSimpleEncoder::new(bytes, EncoderOptions).encode(sink)`; lossless Modular, Luma/LumaA/RGB/RGBA, Eight/Sixteen input | MIT OR Apache-2.0 OR Zlib; Rust 1.80; `zune-core ^0.5.1`; no native build script; defaults enable `std` and `threads` | Best scoped prototype candidate, subject to qualification |
| `imazen/jxl-encoder`, revision `554ad6966771bc2b6f79d6a6c569f55ce155644a` | Upstream documents lossless/VarDCT lossy, integer/float, alpha and ICC with limits/cancellation APIs | AGPL-3.0-or-later or commercial; README MSRV 1.93 exceeds WML2 1.91 | Do not adopt in this batch: explicit licensing and MSRV decisions plus independent qualification needed |
| `libjxl`, revision `3de4501eda3f9802e4cfd333e3faab828d8d9223` | `JxlEncoderAddImageFrame`, lossless/distance settings, ICC profile input and chunked output | BSD-3-Clause C++; CMake/compiler/dependency/link infrastructure | Reference oracle; runtime FFI would require a separate Pure Rust policy decision |
| `jpegxl-rs` bindings | Wrap libjxl encoding/reconstruction | Rust wrapper does not remove native libjxl linkage | Not a Pure Rust alternative |

The imazen feature list is upstream's description, not a WML2 test result or
endorsement of completeness/performance. Its commercial terms were not assessed.
A permissive upstream ancestor does not replace the license on this fork.

### zune source constraints that affect a wrapper

The published source accepts exactly Eight or Sixteen depth. Sixteen-bit input
uses **native endian** sample bytes: a WML2 adapter must convert typed samples
explicitly and test endian behavior. `encode()` rejects width or height <= 1,
so even a 1xN image needs an explicit unsupported-dimension contract or an
upstream fix; a blanket valid-dimensions claim would be false.

`prepare_header()` hardcodes sRGB (or D65/sRGB-transfer gray), writes no ICC and
no preview, and does not expose general metadata/profile input. Supplying raw
pixels from another color space would mislabel them. An initial contract should
accept explicitly supported sRGB input and reject unsupported/custom ICC;
full ICC preservation requires upstream work or a reviewed extension. Do not
silently convert a nominally lossless source to sRGB and call that sample-lossless.

Alpha is an extra channel; the header's default extra-channel path is annotated
as 8-bit alpha. Treat full-range RGBA16/GrayAlpha16 correctness as unresolved
until independent sample and bit-depth oracles verify it or upstream changes it.
Eight/Sixteen input acceptance alone is not proof of alpha precision.

`encode_inner()` allocates group buffers before reserving the final writer.
Dimension/stride/group arithmetic and retained buffers need checked admission;
a final capped sink does not bound these allocations. Quality is not exposed
as a lossy mode: `EncoderOptions::effort` controls work/compression, and must not
be mapped to a fictional lossy quality contract.

## Proposed integration contract (not yet an API)

Use a separate optional encoding feature, with dependency defaults disabled and
a deliberate single-thread setting for the first prototype. Keep decode-only
`jpeg-xl` builds unable to encode. Wire the chosen feature consistently into
`image_encoder`/`image_to`, EX encoding if qualified, format inference, extension
advertising and `encoder_supports_format`; return explicit errors for rejected
formats, dimensions, profiles, depths and options. Do not change common EX API
semantics or advertise RGBA16 until its full route passes.

Check nonzero/allowed dimensions, width*height*channels*sample-size, stride and
buffer length, frame/group counts, aggregate working memory and output budgets
before backend calls. A future adapter must reject rather than panic or wrap on
32-bit hosts. Budget the sink reservation too; cancellation must interrupt long
work, not just precede the call. #37 and #31 remain the controlling resource and
cancellation work. No backend limits are disabled by this proposal.

For a lossless-only first feature, reject lossy quality/distance requests rather
than ignore them. Preserve straight-alpha samples, including RGB under zero
alpha, and define orientation/source metadata policy explicitly. Container
Exif/XMP handling and ICC need a supported and tested contract before they are
offered; raw pixel encoding does not automatically preserve source boxes.

## Qualification required before adoption

- Pin backend/transitive versions, features, licenses and independent libjxl
  build/version/options. Encode licensed Gray/RGB/RGBA integer fixtures, ramps,
  random/extreme values, transparent colors, odd and group-boundary dimensions.
  Require independent `djxl`/libjxl **exact sample equality** at the declared
  bit depth without color conversion, alpha premultiplication or orientation
  confounds; inspect channel depths and color signaling independently too.
- Qualify 8- and 16-bit paths separately, including distinct alpha precision
  and native-endian input. Cross-check WML2 decoding, but never use it as the
  sole oracle. Verify metadata/profile bytes when preservation is implemented.
- Test decode-only and fully disabled builds, capability/extension/inference
  agreement with actual save results, invalid lengths, 0/1/oversized dimensions,
  unsupported depths/profiles/options, allocation and output limits at their
  boundaries, sink errors, overflow and cancellation. Reject all unqualified
  paths explicitly.
- Compile and execute on Linux, macOS and Windows with WML2's Rust 1.91 baseline;
  compile `wasm32-unknown-unknown` with threads disabled and run a browser/Node
  smoke/sample-oracle test. Pure Rust/no-native-build metadata is promising,
  not proof of WASM or 32-bit correctness. libjxl remains an external oracle
  built for its own host, not a linked WASM runtime dependency.
- Benchmark the accepted matrix for time, output size, allocation peak and work
  versus input size; do not import upstream performance claims. If lossy support
  is later proposed, define distance/quality ranges and reproducible error or
  perceptual metrics with separate alpha rules, not exact sample equality.

## Reproducible source evidence and limits

Registry queries on the assessment date returned `jxl` 0.8.0 and `zune-jpegxl`
0.5.2 as latest stable. Inspected published source archives and manifests, not
only READMEs. Archive SHA-256:

- `jxl` 0.7.4: `43b041e4bd5bb931cf73b271bc3304eb135bb473914877d64bf77fdf5b4d4687`.
- `jxl` 0.8.0: `b8379fc83d79477900c9aed78cb8370a556d834081bebcb771c6010381c4e230`.
- `zune-jpegxl` 0.5.2: `cba11ecd07cc23351500e840ae03841b7e4997923ec8e4832ee3ae199ea0b6b4`.

Primary sources: [zune source and manifest](https://docs.rs/crate/zune-jpegxl/0.5.2/source/),
[encoder implementation](https://docs.rs/crate/zune-jpegxl/0.5.2/source/src/encoder.rs),
[jxl 0.7.4](https://docs.rs/crate/jxl/0.7.4/source/src/),
[jxl 0.8.0](https://docs.rs/crate/jxl/0.8.0/source/src/),
[imazen README and license links](https://github.com/imazen/jxl-encoder/blob/554ad6966771bc2b6f79d6a6c569f55ce155644a/README.md),
[imazen manifest](https://github.com/imazen/jxl-encoder/blob/554ad6966771bc2b6f79d6a6c569f55ce155644a/jxl-encoder/Cargo.toml),
[reference encoder header](https://github.com/libjxl/libjxl/blob/3de4501eda3f9802e4cfd333e3faab828d8d9223/lib/include/jxl/encode.h),
[reference build requirements](https://github.com/libjxl/libjxl/blob/3de4501eda3f9802e4cfd333e3faab828d8d9223/BUILDING.md),
[Rust libjxl wrapper](https://docs.rs/crate/jpegxl-rs/0.15.0+libjxl-0.12.0).

No candidate was compiled, run, benchmarked, visually checked or fuzzed here.
The tests above are required future work, not completed validation. #49 remains
open for backend qualification and implementation; this report records the
technical choice and blockers without adding runtime support.
