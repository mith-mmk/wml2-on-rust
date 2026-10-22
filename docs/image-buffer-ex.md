# ImageBufferEx: checked integer storage and explicit legacy views

Available in WML2 0.0.37 through the explicit `image-buffer-ex` feature.
The original continuation incorporated main's0.0.36 update; the historical
[main merge record](image-buffer-ex-main-merge.md) retains those checkpoints.
Default features and existing legacy fields/signatures remain unchanged.
Optional AVIF dependencies now use published registry0.1.2 with one shared
buffer source. Direct consumers of standalone AVIF re-exports must align
versions and sources. See the [0.0.37 release notes](releases/0.0.37.md).

## Current support

| Area | Supported contract |
| --- | --- |
| Storage / source / output | Gray8/12/16 and Rgba8/12/16, separately declared |
| Byte layout | Interleaved, byte stride; 12/16-bit samples occupy two LE bytes, no alignment assumption |
| Sample and alpha range | Unsigned 0–255 / 0–4095 / 0–65535; RGBA straight alpha; 12-bit high four bits must be zero |
| Conversion | Full normalized integer range, including alpha; Exact rejects loss, Round uses nearest integer, ties upward |
| Channel conversion | Gray replicated to RGB plus opaque alpha; RGBA→Gray requires an external explicit color conversion |
| Picking | Explicit output-format/stride/policy rectangles via PickCallbackEx; native borrowed single-pixel bytes |
| Legacy input | Checked ownership transfer of static RGBA8 ImageBuffer pixels, retaining background/callback metadata |
| Legacy encoding | Explicit whole-canvas to_rgba8 or borrowed legacy_picker with per-rectangle conversion |
| Native codec source | Static PNG/TIFF emit Gray8/16 or RGBA8/16; bounded AVIF emits RGBA16; bounded JPEG2000 EX emits native Gray/RGBA12/16, without ICC application; JPEG2000 EX emits unsigned Gray12/16 or RGBA12/16 from RGB/explicit straight RGBA in the reversible or bounded 9/7 profile |
| Typed highres handoff | Pixel-only U8/U16 full-resolution Gray or straight RGBA at uniform 8/12/16 bits |
| Unsupported | YUV, F32/HDR/signed/10-bit, premultiplied alpha, EX animation, general color/ICC transforms |

Existing converters, rectangular pickers, highres adapters and PNG/TIFF EX
decoding were ported commit-by-commit, rather than recreated or replacing main
with the older EX branch snapshot. The new continuation adds source/policy
accessors, native pixel borrowing, the owning legacy bridge and borrowed encoder
adapter. It also moves highres pixel-budget validation ahead of sample scanning
and allocation. Unrelated current codecs/release files remain intact.

## Declarations and precision

For a runnable example of input declarations, independent storage, encoder
requests, ICC collection and failure behavior, see the
[Callback implementation guide](image-buffer-ex-callbacks.md) and
[five-scenario example](../wml2/examples/ex_callback_contracts.rs). The guide
distinguishes current metadata delivery order from future API proposals.

`InitOptionsEx::source_format` and `source_stride` describe bytes delivered by
subsequent draw rectangles. `ImageBufferEx::with_storage` independently selects
stored format and source-to-storage precision policy. `PixelOutputEx` chooses
each picker output. There is no implicit narrowing merely because an old
encoder or display wants RGBA8.

`source_format()` reports the successful callback declaration, or None before
initialization/after failed initialization. It does **not** report the original
encoded-file color type: PNG RGB/palette/GrayAlpha may have expanded into RGBA,
and packed Gray may have expanded into Gray8. `format()` and `stride()` describe
the stored canvas. `precision_conversion()` reports the draw conversion policy;
picker/encoder requests choose their own policy.

`source_stride()` is the explicit stride per incoming rectangle; None means
packed for that rectangle's width. It is meaningful while source_format is Some.
`from_bytes` stores the supplied padded canvas layout, but subsequent draws use
packed source rectangles. Padding is included after the last source row, ignored
as sample data, and required in exact buffer-length validation. Output padding
is zeroed. Odd strides are supported without typed casts.

Precision conversion scales `sample * destination_max / source_max`. Exact
requires an integral result, which can reject widening as well as narrowing
(for example, most 12→16 values). Round explicitly chooses nearest normalized
samples, including alpha. It does not apply transfer functions, luminance,
premultiplication, ICC conversion or a display/HDR mapping. Transparent RGB is
retained. Gray→RGBA copies Gray into RGB and supplies destination-opaque alpha.

## Native access and legacy handoff

`pixel_bytes(x, y)` borrows the stored pixel with no allocation or conversion.
Read its representation from `format()`; low bits and straight alpha remain
intact. Coordinates must be inside the canvas, padding is excluded, and canvas
pixel/byte budgets apply. Mutable raw byte access is deliberately absent so
validated 12-bit/storage invariants cannot be invalidated accidentally.

```rust
# use wml2::draw::*;
let old = ImageBuffer::from_buffer(1, 1, vec![1, 2, 3, 0]);
let image = ImageBufferEx::try_from(old)?;
assert_eq!(image.pixel_bytes(0, 0)?, &[1, 2, 3, 0]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`TryFrom<ImageBuffer>` checks the publicly mutable legacy dimensions and exact
RGBA8 byte length, transfers the owned pixel Vec without copying and retains
legacy background/callback metadata. Any animation collection (including empty),
current-frame index, delay or nonzero loop configuration is rejected. The inert
`Some(0)` loop count installed by ordinary legacy static initialization is
accepted. This is an explicit consuming conversion; failed conversion returns
an error, and callers needing to retain their original buffer can use the
existing borrowed PickCallbackEx path instead. No animation or highres metadata
is synthesized.

`legacy_picker(policy)` borrows EX as the unchanged legacy PickCallback. It
converts each requested rectangle to packed RGBA8 using the supplied policy and
leaves native storage untouched. Requests must be nonempty and fully in-bounds.
`encode_start` validates canvas/budgets; with Exact, an unrepresentable sample
fails when its rectangle is picked, returning no partial rectangle. Background
and callback metadata are exposed with source/output ICC identity preserved:
Gray ICC is retained as `Source ICC Profile` when Gray expands to RGBA.
See the [ICC metadata and lifecycle contract](icc-consistency-metadata.md).
Use existing EncodeOptions and
image_encoder; image_to retains its existing ImageBuffer signature.

```rust
# use wml2::draw::*;
# use wml2::util::ImageFormat;
# #[cfg(feature = "png")]
# {
let image = ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Gray16, None,
    vec![1, 0])?;
let png = image_encoder(&mut EncodeOptions {
    debug_flag: 0,
    drawer: &mut image.legacy_picker(PrecisionConversion::Round),
    options: None,
}, ImageFormat::Png)?;
assert_eq!(image.pixel_bytes(0, 0)?, &[1, 0]);
# }
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Codec and highres boundaries

EX decoding supports enabled static PNG/TIFF, the
[bounded AVIF adapter](image-buffer-ex-avif.md), and
[unsigned Gray/RGB/declared straight RGBA12/16 JPEG2000](jpeg2000-ex-97.md). AVIF encoding consumes an
explicit EX8/12-bit output request; RGBA16 storage stays independent.
PNG packed Gray expands
to Gray8; palette/RGB/GrayAlpha expand to straight RGBA at their sample depth.
TIFF accepts supported unsigned 8/16-bit Gray/RGB with optional straight alpha,
retains orientation metadata without rotating samples, and rejects premultiplied
or multiple display pages. APNG and other codecs do not fall back to implicit
RGBA8 through the EX entry point. Color metadata is retained as metadata; there
is no automatic color/display transform.

`from_highres_pixels` / `to_highres_pixels` explicitly copy **pixels only**.
Highres metadata, timing and color declarations stay with the caller's original
frame and must be attached explicitly. Do not treat legacy HashMap metadata as
interchangeable with those typed declarations. Import checks checked pixel
count before sample validation/copy; storage/output byte budgets are checked
before allocation. Other EX operations validate dimensions/layout/sample values
and budgets and preserve storage when conversions/picks/draws fail. Expanded
limits are individual buffers, not a promise about allocator overhead or total
process RSS.

JPEG2000 EX retains unsigned Gray/RGB12/16 and JP2 RGBA12/16 with complete
straight-alpha CDEF, using reversible 5/3/RCT or scalar-quantized 9/7/ICT
(at most six decomposition levels). RGB gains opaque
alpha in the existing RGBA formats. The legacy RGBA8 decoder keeps its acceptance;
high depth is not silently narrowed. Mixed/signed precision, mixed filters,
implicit or premultiplied alpha, other coded precisions and effective ICC remain
unsupported. See the [current profile](jpeg2000-ex-97.md). EX still has no signed/float/HDR/YUV storage or
automatic color transform; no large-image streaming API is added.

## Validation and reproduction

The [Callback guide and validation](image-buffer-ex-callbacks.md) add executable
examples and contract tests without changing runtime code. Its separate
[validation record](image-buffer-ex-callback-validation.json) identifies the
checks rerun for that documentation follow-up.

The [TIFF follow-up](image-buffer-ex-tiff-review.md) and its
[current validation record](image-buffer-ex-tiff-validation.json) document the
FillOrder fix, SampleFormat rejection contract, 408 passing workspace tests,
82 actual WASM groups and 54 exact native oracle exports. Production TIFF
source matches `93ebfe1`; the clean measurement/WASM checkpoint is `637dec1`.
The original supplied review ZIP could not be downloaded through Library;
the follow-up distinguishes that gap from independently executed equivalents.

The [initial continuation validation record](image-buffer-ex-validation.json) preserves source,
fixture/helper/output hashes and links independent review and raw measurements.
Final native runtime source matches clean checkpoint
`59082ef43ce53d4d8805b5285e5bf2e31aab2907`; later evidence changes do not modify
production code. Historical EX branch results are not counted as newly executed
tests. Initial continuation results (before the TIFF follow-up):

| Configuration / check | Result |
| --- | --- |
| Ordinary workspace, EX + high-bit-depth + optional retro codecs | 394 passed, 0 failed, 13 existing ignored; 48 targets including doctests |
| Minimal EX, no default codecs | 70 passed, 0 failed; 30 targets including doctests |
| Feature-off minimal | 39 passed, 0 failed; 30 targets including doctests |
| Format / diff checks | Passed |
| Clippy all-targets and focused new targets | Passed with existing/inherited library/test warnings; no new access/boundary-test warning, not a -Dwarnings gate |
| Actual minimal wasm32 runtime, Node v24.19.0 | 13 smoke groups passed, no host imports; six formats/highres/legacy view/PNG/four TIFF fixtures |
| Native pixel/display checks | Five images retain every native16bit sample exactly; explicit Round preview also matches independent expectations |

The [WASM record](image-buffer-ex-wasm.json) is execution evidence, not only a
build. Its13smoke groups do not claim the complete native workspace matrix ran
in WASM. An initial standalone harness misstated existing TIFF geometry/pattern;
that harness assertion was corrected to the existing generator/manifest, with
no production changes. The committed portable helper passes from a clean tree.
[Display sheet](image-buffer-ex-display.png) and [pixel/output hashes](image-buffer-ex-display.json)
cover8/16bit LZW/PackBits Gray and independently generated RGBA16 with low bits.

An [independent API review](image-buffer-ex-review.json) closed the static-loop
compatibility finding and checked native access, bounded conversion, source
contracts, shared legacy fields and preservation of main's newer codecs/release
files. Independent boundary tests pass; the review is not a general security or
complete codec-conformance proof.

The [paired main comparison](image-buffer-ex-benchmark.json) uses identical
measurement helpers, immutable release binaries, fixed CPU affinity,
B/C/C/B/C/B,3processes/version,2warmup and7timed batches per case/process.
Eight legacy draw/pick/JPEG/TIFF cases have matching measured pixel digests and
unchanged allocation counts. No material increase appears in these samples;
large shared-host variation includes baseline outliers. All raw rows are
retained, and negative means do not establish guaranteed speedups. Timing is
per-body wall time, with process CPU/startup separately recorded.

The [same-row encoder-view comparison](image-buffer-ex-encoder-view-benchmark.json)
retains a1024×768RGBA16 canvas and requests the same packed RGBA8 row using
explicit Round. Whole-canvas copy followed by picking allocates3,149,824bytes;
the borrowed adapter allocates4,096bytes. Pixel hashes match. These samples
average15.931ms versus23.776μs for that one-row request, not a whole-image codec
throughput comparison. The allocation reduction confirms that borrowing avoids
an additional complete RGBA8 canvas while retaining native samples.

Browser viewer/UI, other OS, the full CI matrix and every optional AVIF/ICC
configuration were not executed here. There was no CI wait, merge, publication,
version change or new PR creation.

```sh
cargo test --workspace --release --features wml2/image-buffer-ex,wml2/high-bit-depth,wml2/tga,wml2/pcx,wml2/dds,wml2/q4
cargo test -p wml2 --no-default-features --features image-buffer-ex
cargo test -p wml2 --no-default-features
cargo build -p wml2 --release --target wasm32-unknown-unknown --no-default-features --features image-buffer-ex,high-bit-depth,png,tiff
python3 wml2-test/scripts/verify_image_buffer_ex_wasm.py --output .test-ex-wasm
python3 wml2-test/scripts/generate_ex_access_sample.py --output .test-ex-access.tif
cargo run -p wml2 --release --features image-buffer-ex --example review_bench -- ex-view 9
cargo run -p wml2 --release --features image-buffer-ex,png,tiff --example ex_decode -- input.tif preview.png native.rgba16le
```

The continuation includes independent exhaustive nearest/monotonic/complement
checks, 209,664 Exact lattice decisions, RGBA16 alpha/transparent-RGB boundaries,
odd padding/rectangle partition checks, late/clipped conversion failure
atomicity, declaration/pixel/legacy bridge/encoder tests and early highres pixel
budget validation. Runtime results are distinguished from build-only and
unexecuted platform/viewer/CI checks in the validation record.
