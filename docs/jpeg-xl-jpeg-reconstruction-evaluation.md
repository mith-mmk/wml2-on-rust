# JPEG XL original-JPEG reconstruction evaluation (#48)

Assessment: 2026-10-30, WML2 main `6a48db924b338c5ae7095bea6105fb4dbd436e95`.
This is a source/API feasibility assessment, not an implemented or validated
reconstruction feature. Runtime dependencies and supported inputs are unchanged.

## Decision and blockers

A Pure Rust route exists in **jxl-oxide**, but it is a separate decoder stack;
adding `jxl-jbr` to the existing decoder is not a small adapter. First evaluate
an optional, isolated reconstruction backend using `jxl-oxide` with default
features disabled. Keep ordinary pixel decoding on the existing bounded adapter.
Do not enable a reconstruction capability until pre-allocation admission,
expanded reconstruction-data limits and the byte-identity oracle below pass.

The alternative is upstream bounded reconstruction support in `jxl-rs`. An
independent implementation would require parsing reconstruction headers, Brotli
payloads, JPEG markers/Huffman tables/scans and recovering quantized coefficients
from the JXL frame; it should be treated as substantial codec work. No effort or
completion-date estimate is supported by this assessment.

## What the inspected APIs actually provide

| Source | Finding | Consequence |
| --- | --- | --- |
| WML2 [adapter](../wml2/src/jpegxl.rs), `unpack()` | Retains an uncompressed outer `jbrd` as indexed opaque source bytes; does not validate/decompress its reconstruction payload | Box presence and successful pixel decoding do not establish reconstructability |
| Published `jxl` 0.7.4 and 0.8.0, BSD-3-Clause | Both are decoder libraries; inspected public APIs expose no original-JPEG writer. 0.8 adds `JxlAuxBoxType::JPEG_RECONSTRUCTION_DATA` and raw auxiliary-box access | Neither a version bump nor the `jbrd` constant supplies JPEG reconstruction; #40 remains separate |
| `jxl-jbr` 0.2.1, MIT OR Apache-2.0 | `JpegBitstreamData::try_parse/feed_bytes/finalize`, then `reconstruct(frame, icc, exif, xmp, pool)` and a writer | Requires `jxl_frame::Frame` and `jxl_threadpool`, not WML2 RGBA pixels or `jxl-rs` frame types |
| `jxl-oxide` 0.12.6, MIT OR Apache-2.0 | `JxlImage::jpeg_reconstruction_status()` and `reconstruct_jpeg(impl Write)` integrate the frame, original ICC, Exif and XMP with `jxl-jbr` | A plausible complete Pure Rust candidate, not yet adopted or qualified for WML2 |
| libjxl reference, BSD-3-Clause | Decoder reconstruction event and JPEG output-buffer API; encoder can retain reconstruction data when losslessly transcoding a single original JPEG | Suitable independent fixture/oracle tool, not a Pure Rust runtime dependency |

In `jxl-oxide`, an `Available` status can still end in reconstruction failure.
`jxl-jbr::try_parse()` uses a Brotli `DecompressorWriter<Vec<u8>>`; the expected
expanded size is checked at `finalize()`, after decompression. A capped final
JPEG writer alone therefore does **not** bound reconstruction metadata memory.
The source inspection does not establish a safe whole-decoder memory bound.

The reference flow uses `JxlEncoderStoreJPEGMetadata` plus
`JxlEncoderAddJPEGFrame`, then `JXL_DEC_JPEG_RECONSTRUCTION`,
`JxlDecoderSetJPEGBuffer`, repeated `JXL_DEC_JPEG_NEED_MORE_OUTPUT` handling and
`JxlDecoderReleaseJPEGBuffer`. Every accumulated chunk must count against a
maximum JPEG output budget. Re-encoding decoded pixels is not this operation.

## Proposed WML2 contract and integration gate

Use a separate explicit original-JPEG operation, with errors distinguishing
missing reconstruction data, incomplete/corrupt input, incompatible frame data,
resource rejection and output I/O failure. Never silently fall back to a newly
encoded JPEG. Names and public signatures remain undecided.

Bound input, box count, aggregate raw and expanded boxes, ICC/Exif/XMP,
reconstruction headers, frame/coefficient storage, output bytes and work before
allocating or decompressing. Reject arithmetic overflow and cap 32-bit targets
explicitly. Preserve #37 limits and #31 cancellation; any backend lacking hooks
requires upstream changes or a reviewed fork before admission. A writer error
may leave a partial stream: expose this contract or stage bounded output before
committing a file.

`jxl-oxide` defaults to rayon; disable it for an initial single-thread candidate.
Its optional `lcms2` feature introduces a native CMS and must remain off. Original
JPEG reconstruction should preserve source profile bytes, not transform pixels.
Linux/macOS/Windows and `wasm32-unknown-unknown` need independent compile/runtime
qualification; Pure Rust and `Write` APIs alone do not prove browser execution,
thread support, cancellation or bounded allocation. libjxl/bindings need C/C++
build/link infrastructure and a separate WASM toolchain; retain them as oracles.

## Required oracle and refusal matrix

1. Preserve licensed **original JPEG files**, SHA-256 and length, and pin the
   independent libjxl version/build/options used to produce reconstructable JXL.
   Cover gray and RGB, baseline/progressive scans, subsampling, restart markers,
   comments, legal padding/tail bytes, and ICC/Exif/XMP singly and together.
2. Require complete output byte equality and SHA-256 equality to each original,
   also checking the independent reference reconstruction. Pixel equality or
   simply producing a decodable JPEG is insufficient. Verify metadata marker
   ordering/chunking through whole-file equality.
3. Missing/empty/invalid `jbrd`, mismatched frame or metadata lengths, truncated
   headers/Brotli/coefficient data, duplicate or malformed boxes and writer
   failures must fail explicitly without panic or fallback. Include incremental
   input if exposed, and the supported outer-box forms without assuming a
   forbidden box type becomes valid merely because Brotli support exists.
4. Test exact resource boundaries, output-limit-minus-one, decompression bombs,
   overflow, cancellation during reconstruction and allocation peaks. Reject
   oversized inputs before expansion. Check feature-off behavior and all target
   platforms, including an actual WASM host, before advertising support.
5. Rerun existing opaque-box retention and pixel/orientation/ICC/refusal tests;
   their behavior must remain independent of reconstruction failure. Existing
   [jbrd fixture manifest](../wml2/tests/fixtures/review/jxl-jbrd-fixtures.json)
   records pixel/profile references, not original-JPEG identity evidence.

## Evidence and limits

Inspected published archives (not just development branches), their manifests,
`jxl` public API/box parser, `jxl-jbr/src/lib.rs` and `reconstruct.rs`, and
`jxl-oxide/src/lib.rs` reconstruction path. Registry latest stable versions were
0.8.0 / 0.2.1 / 0.12.6 respectively at assessment time. Archive SHA-256:

- `jxl` 0.7.4: `43b041e4bd5bb931cf73b271bc3304eb135bb473914877d64bf77fdf5b4d4687`.
- `jxl` 0.8.0: `b8379fc83d79477900c9aed78cb8370a556d834081bebcb771c6010381c4e230`.
- `jxl-jbr` 0.2.1: `e35d032bcec660647828527ff42c6f5776d2fd44b8357f9f6d9ac6dc07218e46`.
- `jxl-oxide` 0.12.6: `d36c662923f47586880211f3bc7c0d83fb3a9b410d278c7bde93450748abeef3`.

Primary sources: [jxl 0.7.4 source](https://docs.rs/crate/jxl/0.7.4/source/src/),
[jxl 0.8.0 source](https://docs.rs/crate/jxl/0.8.0/source/src/),
[jxl-jbr 0.2.1 source](https://docs.rs/crate/jxl-jbr/0.2.1/source/src/lib.rs),
[jxl-oxide 0.12.6 source](https://docs.rs/crate/jxl-oxide/0.12.6/source/src/lib.rs),
[libjxl decoder header](https://github.com/libjxl/libjxl/blob/3de4501eda3f9802e4cfd333e3faab828d8d9223/lib/include/jxl/decode.h),
[encoder header](https://github.com/libjxl/libjxl/blob/3de4501eda3f9802e4cfd333e3faab828d8d9223/lib/include/jxl/encode.h),
[license](https://github.com/libjxl/libjxl/blob/3de4501eda3f9802e4cfd333e3faab828d8d9223/LICENSE),
[build requirements](https://github.com/libjxl/libjxl/blob/3de4501eda3f9802e4cfd333e3faab828d8d9223/BUILDING.md).

No backend build, reconstruction execution, image-display check, benchmark,
fuzzing or platform run was performed for this documentation-only assessment.
#48 remains an implementation/qualification task; this report supplies the
feasibility decision and its blockers, not feature completion.
