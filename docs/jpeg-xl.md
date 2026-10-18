# JPEG XL still-image decoding

The opt-in `jpeg-xl` feature uses **published `jxl =0.7.4`**, not upstream main.
Defaults are unchanged; JPEG XL encoding is unavailable. Add `image-buffer-ex`
for high-precision callback output. No C library, image-rs, CMS, SIMD or thread
pool is pulled into the runtime by this feature.

```toml
wml2 = { version = "0.0.37", features = ["jpeg-xl", "image-buffer-ex"] }
```

```rust
use wml2::draw::{image_from_ex, PixelFormatEx, PrecisionConversion};
# let input = include_bytes!("../wml2/tests/fixtures/review/jxl-rgba12.jxl");
let image = image_from_ex(input, PixelFormatEx::Rgba12, PrecisionConversion::Exact)?;
assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba12));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Supported profile and precision

- Bare `ff 0a` codestreams and containers with `jxlc` or ordered `jxlp` chunks.
  Every box, including trailing boxes, is length-checked before decoding.
- Integer **8,12,16-bit** Gray/RGB, optionally one full-resolution straight
  alpha channel with the same depth as colour. Orientation is applied once by
  the decoder. RGB becomes RGBA with opaque alpha; GrayAlpha becomes RGBA.
- The EX source is native Gray8/12/16 or Rgba8/12/16, packed, with12/16-bit
  samples in little-endian u16 slots. Transparent RGB and low bits survive
  lossless decoding. Neither EX nor ordinary decoding goes through RGBA8 first.
- Ordinary `image_from`/`image_decoder` return RGBA8. High-depth samples,
  including alpha, are explicitly rounded over the full normalized range:
  `floor((sample*255 + source_max/2)/source_max)`. Upstream U8 dithering is
  deliberately avoided by decoding at source precision before this step.
- EX storage conversion uses the existing `Exact`/`Round` policy. `Exact` can
  reject12→16 as well as16→8 because normalized integer ranges differ. The
  `high_precision` decoder setting controls intermediate arithmetic; it is not
  a promise that lossy input becomes lossless.

Explicitly rejected: float sources/HDR PQ or HLG/tone-mapping requests,
animations (no silent first-frame fallback), previews, intrinsic display sizes,
premultiplied or mixed-depth/subsampled alpha, multiple/other extra channels,
compressed `brob` metadata, JPEG reconstruction `jbrd`, and unsupported depths.
A Level10 box is accepted for bounded16-bit input; this does not promise the
entire Level10 profile. Native JPEG bitstream reconstruction is not implemented.

## Colour and source metadata

No general CMS conversion or tone mapping is added. JPEG XL's own XYB→RGB
reconstruction stays inside `jxl`. **Output samples need not be in the source
profile**: XYB may use an sRGB fallback when the embedded profile cannot be
used directly. The adapter queries the output profile after setting the pixel
format, not before.

| Metadata key | Meaning |
| --- | --- |
| `ICC Profile` | Actual output colour profile, generated from simple signaling when needed |
| `JPEG XL Source ICC Profile` | Original embedded ICC bytes, or equivalent generated ICC for original simple signaling |
| `JPEG XL Source Header` | Original bit-aligned codestream-header prefix; last byte may also contain following bits; not a standalone file |
| `JPEG XL Source Box <index> <hex-type>` | Uncompressed auxiliary box bytes, including unknown types and duplicate types with distinct indexes |
| `JPEG XL Source Bit Depth` / `JPEG XL Output Bit Depth` | Coded colour depth / declared delivered sample depth |

Gray replicated into RGBA gets a matching neutral-axis RGB profile for simple
Gray signaling. An arbitrary Gray ICC that needs RGBA expansion is explicitly
rejected because this would require a CMS; EX Gray-only output can retain it.
Metadata is delivered after pixels, matching existing static adapters. Source
orientation remains in the raw header; output dimensions/pixels are oriented.
Raw EXIF/XMP boxes are preserved as source bytes, not parsed or reapplied.

## Limits and cancellation

Both entry points use `DecodeLimits` and the shared callback guard. Input,
canvas pixels/bytes, frame count, source/output metadata and callback forwarding
are checked independently. Abort stops later draws/metadata and terminates
once, including the ordinary RGBA8 path. Malformed/truncated input is rejected
before initialization; there is no partial-image success.

Additional fixed profile caps:64MiB input,64KiB header prefix,16384 per image
dimension,4Mi effective padded frame pixels,1024 auxiliary boxes. The conservative
workspace estimate includes input assembly,128 bytes per possible internal
frame pixel (including16-pixel padding and the RGB/RGBA sample ratio),1MiB
header workspace, and ICC workspace. It is charged against `expanded_bytes`;
limits can therefore reject an image even if its final canvas alone fits.
This is **not a process-RSS or arbitrary decoder-internal allocation guarantee**.
Dependency complexity limits remain enabled, and `sample_limit` bounds internal
frames as well as the canvas. Caller `unlimited()` does not disable fixed caps.

For embedded ICC, a pinned0.7.4 preamble probe checks coded and advertised
expanded profile sizes before the main decoder's ICC allocation, allows for two
retained profiles and temporary copies, and reserves8MiB for entropy/LZ77 work.
A conservative16KiB metadata allowance for generated profiles/keys is required
before parsing or generating profiles. The complete retained metadata is checked
again before callback initialization. The dependency's public low-level APIs
used by this probe must be reviewed when upgrading `jxl`.

## Independent verification

`wml2-test/scripts/generate_jpegxl_fixtures.py` and
`jpegxl_fixture_encoder.c` use **libjxl0.11.2**, independently of WML2/jxl-rs.
Lossless expected values are deterministic original sample bytes, including
alpha0/1/mid/max and nonzero RGB under alpha0. Nine Gray/GrayAlpha/RGBA depth
combinations also have `djxl` oracle output. Fixture hashes/provenance are in
`wml2/tests/fixtures/review/jxl-fixtures.json`.

Lossy checks use at most2 output units at8-bit and32 at16-bit against `djxl`.
The ICC/XYB comparison explicitly requests the same sRGB output profile in
`djxl`; comparing its source-profile output to sRGB samples would be invalid.
Other tests cover source/output ICC identity, orientation, EX precision,
containers/chunks/metadata, unsupported inputs, truncations, limits and Abort.

```console
cargo test -p wml2 --locked --no-default-features --features jpeg-xl,image-buffer-ex --test jpeg_xl
cargo test -p wml2 --locked --no-default-features --test jpeg_xl
python3 wml2-test/scripts/generate_jpegxl_fixtures.py --tools /path/to/libjxl/usr --work .test-jxl-fixtures
```

Upstream reference: [published source commit](https://github.com/libjxl/jxl-rs/tree/624ce908afcf8eb0dc2585a671535eaabb5d88cc/jxl).
In particular,0.7.4 has typestate processing and `premultiply_output`; main's
`AlphaMode` API must not be assumed to exist in the published crate.
