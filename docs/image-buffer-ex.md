# ImageBufferEx: checked integer storage and explicit legacy views

Repository branch work based on main `e42d2cf` and the existing EX branch
`c1203f4`. WML2 remains 0.0.35. Enable `image-buffer-ex` explicitly; default
features, production dependencies and existing legacy fields/signatures remain
unchanged. This work has not been merged or released.

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
| Native codec source | Static PNG/TIFF emit Gray8/16 or RGBA8/16; low sample bits retained on EX path |
| Typed highres handoff | Pixel-only U8/U16 full-resolution Gray or straight RGBA at uniform 8/12/16 bits |
| Unsupported | YUV, F32/HDR/signed/10-bit, premultiplied alpha, EX animation, general color/ICC transforms |

Existing converters, rectangular pickers, highres adapters and PNG/TIFF EX
decoding were ported commit-by-commit, rather than recreated or replacing main
with the older EX branch snapshot. The new continuation adds source/policy
accessors, native pixel borrowing, the owning legacy bridge and borrowed encoder
adapter. It also moves highres pixel-budget validation ahead of sample scanning
and allocation. Unrelated current codecs/release files remain intact.

## Declarations and precision

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
and callback metadata are exposed as before. Use existing EncodeOptions and
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

EX decoding remains limited to enabled static PNG/TIFF. PNG packed Gray expands
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

JPEG2000 high-depth acceptance is a later codec task. This continuation does not
change JPEG2000 PR #10 or add a new codec's high-depth implementation. The EX
contract here preserves supported unsigned integer samples; signed/float/HDR,
subsampling, alpha models and color transforms need separate scoped decisions.

## Validation and reproduction

Current-source local results, display checks, performance measurements and
immutable-source hashes are recorded in `image-buffer-ex-validation.json`.
Historical EX branch results are not counted as newly executed tests.

```sh
cargo test --workspace --release --features wml2/image-buffer-ex,wml2/high-bit-depth,wml2/tga,wml2/pcx,wml2/dds,wml2/q4
cargo test -p wml2 --no-default-features --features image-buffer-ex
cargo test -p wml2 --no-default-features
cargo build -p wml2 --release --target wasm32-unknown-unknown --no-default-features --features image-buffer-ex,high-bit-depth,png,tiff
cargo run -p wml2 --release --features image-buffer-ex,png,tiff --example ex_decode -- input.tif preview.png native.rgba16le
```

The continuation includes independent exhaustive nearest/monotonic/complement
checks, 209,664 Exact lattice decisions, RGBA16 alpha/transparent-RGB boundaries,
odd padding/rectangle partition checks, late/clipped conversion failure
atomicity, declaration/pixel/legacy bridge/encoder tests and early highres pixel
budget validation. Runtime results are distinguished from build-only and
unexecuted platform/viewer/CI checks in the validation record.
