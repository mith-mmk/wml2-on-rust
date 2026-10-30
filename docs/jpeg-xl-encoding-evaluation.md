# JPEG XL encoding backend evaluation (#49)

Assessment: 2026-10-30 against WML2 main
`6a48db924b338c5ae7095bea6105fb4dbd436e95`. This report includes an isolated
executable evaluation; it adds no production backend, runtime dependency,
encoder capability or save API.

## Decision

The pinned **zune-jpegxl 0.5.2 + zune-core 0.5.1**, with zune-jpegxl defaults
and threads disabled, is sample-correct on the tested Gray8/GrayAlpha8/RGB8/
RGBA8/Gray16/RGB16 subset. **Reject GrayAlpha16 and RGBA16:** every tested
16-bit-alpha case fails exact independent sample comparison. Keep the candidate
out of production until an enforceable working-memory admission bound and
cancellation strategy exist. No generally safe production subset is established
by these measurements. Custom ICC remains unsupported.

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
upstream fix; a blanket valid-dimensions claim would be false. With the pinned
zune-core, height 0 panics in its height getter before the candidate can return
that error. Reject zero dimensions before constructing/calling backend options.

`prepare_header()` hardcodes sRGB (or D65/sRGB-transfer gray), writes no ICC and
no preview, and does not expose general metadata/profile input. Supplying raw
pixels from another color space would mislabel them. An initial contract should
accept explicitly supported sRGB input and reject unsupported/custom ICC;
full ICC preservation requires upstream work or a reviewed extension. Do not
silently convert a nominally lossless source to sRGB and call that sample-lossless.

Alpha is an extra channel; independent libjxl reports 8-bit alpha even for
Sixteen input. GrayAlpha16 and RGBA16 are now **demonstrated failures**, not
unresolved support: see the executable results below. Eight/Sixteen input
acceptance alone does not establish lossless alpha precision.

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

## Executable evaluation (2026-10-30)

Reproduce on a Linux host with installed libjxl 0.11.2 and `cc`:

```sh
rustup target add wasm32-unknown-unknown
python3 wml2-test/scripts/evaluate_zune_jxl.py --output .test-zune-eval --wasm
```

The isolated Cargo workspace pins both registry crates, disables zune-jpegxl
defaults and threads, and never changes production Cargo files. The transitive
zune-core default feature is empty; its std feature is not enabled. The script
fetches API headers from official libjxl v0.11.2 commit
`332feb17d17311c748445f7ee75c4fb55cc38530`, records their hashes and uses the
installed Debian `libjxl0.11` package `0.11.2-0.1~deb13u2` as independent decoder
(`JxlDecoderVersion() == 11002`). Build-generated export/version headers are
minimal local stubs; no upstream ABI struct or enum is redefined.

Measured on Linux x86_64 little endian, Rust 1.91.0, release optimization.
Synthetic CC0 samples combine extrema, ramps, deterministic noise and nonzero
colors under zero alpha. Native-endian u16 input/output is compared directly,
without a CMS transform, premultiplication or orientation change. libjxl also
confirms dimensions, channel counts, declared depth, straight alpha, orientation
1 and original structured D65/sRGB-transfer signaling. The oracle is never a
candidate self-decoder. [Recorded execution evidence](jpeg-xl-encoding-evaluation.json)
contains all case outcomes, hashes, allocator counters, timings and stderr. The
same JSON object is generated at `.test-zune-eval/evaluation.json`.

| Input | 2x2, 3x5, 17x13, 255x257, 257x259 | 1x1 / 1x17 / 17x1 | Decision |
| --- | --- | --- | --- |
| Gray8, GrayAlpha8, RGB8, RGBA8 | 20/20 exact | All 12 rejected | Sample-qualified only |
| Gray16, RGB16 | 10/10 exact | All 6 rejected | Sample-qualified only |
| GrayAlpha16, RGBA16 | 0/10 exact; libjxl accepts files but reports 8-bit alpha | All 6 rejected | Exclude from integration |

RGBA16 retains RGB in the tested cases but corrupts alpha; GrayAlpha16 also
changes gray samples. At 17x13, differing sample counts per channel are
`[0, 0, 0, 171]` (RGBA16) and `[121, 196]` (GrayAlpha16). All 60 successful native
encodes were independently decoded: 50 exact, 10 mismatches. This includes 18
larger resource runs and two successful output-cap cases beyond the 40 matrix
encodes. The total 95 native invocations also include 24 single-dimension
refusals, seven malformed-input cases and four deliberate sink rejections.

Malformed input results: short/long/empty buffers and CMYK return errors; width
0 returns an error; height 0 panics in `zune-core::EncoderOptions::height()`;
`usize::MAX x 2` with empty input panics in the candidate's checked input-size
`unwrap()`. Panics are caught for evaluation only, never proposed as production
validation. Each native encoder/oracle child is contained with 512 MiB address
space, 20 seconds CPU and a 120-second wall timeout. No process hit those caps;
they are containment limits, not evidence of backend resource admission.

### Resource measurements and remaining bound gap

Peak extra requested Rust heap below excludes the pre-existing input/process
baseline, includes output capacity, and does not measure RSS, allocator metadata,
stack, all transient realloc peaks or a mathematical worst-case bound. Values
are stable across three fresh-process runs; times are medians of three encodes,
not a broad performance comparison.

| Input | Input bytes | Output bytes | Peak extra requested heap | Peak before output reservation | Median encode ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Gray16 512x512 | 524,288 | 720,182 | 1,521,218 | 805,252 | 2.552 |
| Gray16 1024x1024 | 2,097,152 | 2,880,550 | 6,046,306 | 3,169,924 | 9.479 |
| RGB16 512x512 | 1,572,864 | 2,166,311 | 4,540,723 | 2,384,100 | 8.984 |
| RGB16 1024x1024 | 6,291,456 | 8,664,353 | 18,123,613 | 9,468,900 | 32.304 |
| RGBA8 512x512 | 1,048,576 | 1,118,081 | 3,230,605 | 2,119,828 | 8.017 |
| RGBA8 1024x1024 | 4,194,304 | 4,399,671 | 12,811,379 | 8,418,964 | 34.053 |

For a 17x13 RGBA8 input (884 bytes), the candidate writes 1,439 bytes but asks to
reserve 1,471. A reservation cap of 0 or 1,439 rejects only after a 477,096-byte
extra heap peak. A write cap of 0 rejects after the same work/heap use; a write
cap of 1,439 succeeds and 1,438 rejects. This proves a final sink limit does not
bound earlier allocations or CPU work, and reservation hints can exceed actual
output size. No global heap limit or cooperative cancellation hook is exposed
by this candidate API. Source arithmetic still requires checked preflight.

### WASM execution

The same dependency configuration compiles as an evaluation cdylib for
`wasm32-unknown-unknown`. A minimal trusted Node 24.19.0 host needs no imports,
WASI, filesystem or threads. Its eight 17x13 outputs (all channel/depth pairs)
match native encoded bytes exactly; independent libjxl confirms six exact sample
results and the same two 16-bit-alpha failures. Observed linear memory is
1,703,936 bytes after each small call. This is a smoke test, not a browser,
large-image or WASM worst-case memory qualification. The unsafe pointers and
allocator instrumentation are evaluation-only ABI/measurement code, never a
shipping WML2 API or a dependency on native libjxl.

### Exact next integration contract to review

A future implementation may propose only static Gray8/GrayAlpha8/RGB8/RGBA8/
Gray16/RGB16, straight alpha and standard sRGB-transfer data, initially width
and height 2..=1024 and at most 1,048,576 pixels. These caps constrain a prototype;
they do not prove all shapes or hostile inputs safe. Verify exact checked packed
buffer length, u16 native-endian conversion and every size/group operation before
backend options. Reject custom ICC, GrayAlpha16/RGBA16, 0/1 dimensions, float/HDR,
animation, unknown options and lossy quality requests explicitly. Do not silently
convert color or discard transparent RGB. Name a distinct optional encode feature
and keep capability/extension/inference/actual saves consistent with it.

**Do not connect even this subset to production yet.** An audited analytic
working-memory bound or allocation-budget hook must enforce the chosen working
limit before allocation; measurements cannot supply that proof. Output writes
and reservation hints need separately enforced byte limits with documented
partial-output behavior. Cooperative cancellation inside long work also needs a
backend change or an explicitly reviewed execution strategy. Platform, fuzzing,
WML2 callback/EX, metadata-policy and rejection matrices above remain adoption
gates. No production integration contract is approved by this evaluation.

No production encoder/decoder was modified. Image-display validation and
cross-version performance regression claims are therefore not applicable here.
The new measurements do not qualify Windows/macOS execution, big endian or all
32-bit arithmetic. #49 remains open for bounded backend qualification and
implementation; #48 reconstruction remains a separate documented blocker.
