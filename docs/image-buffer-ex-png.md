# PNG EX encoding

With `png,image-buffer-ex`, `image_encoder_ex` accepts **static** Gray8,
Gray16, RGBA8 and RGBA16 output. The legacy `image_encoder` still writes
RGBA8 PNG/APNG. No Viewer integration or TIFF encoding is included.

```rust
use wml2::draw::{EncodeOptionsEx, PixelFormatEx, PixelOutputEx,
    PrecisionConversion, image_encoder_ex};
use wml2::util::ImageFormat;
// `image` implements PickCallbackEx; its storage may have a different depth.
# let mut image = wml2::draw::ImageBufferEx::from_bytes(
#     1, 1, PixelFormatEx::Rgba12, None, vec![0; 8]).unwrap();
let png = image_encoder_ex(&mut EncodeOptionsEx {
    debug_flag: 0,
    drawer: &mut image,
    output: PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Round),
    options: None,
}, ImageFormat::Png)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The picker applies the existing normalized Exact/Round contract, including
alpha. A 12-bit storage sample is scaled by `65535/4095` when requesting
16-bit PNG; it is never shifted by four bits. Exact can reject this expansion.
The encoder validates the returned dimensions, format, stride and bytes.
Stride is in bytes and includes padding after the final row. Odd stride is
supported; padding is excluded from the PNG. Public `PixelBytesEx` construction
already validates length and sample range; custom callbacks returning a different
valid layout are rejected again at the encoder boundary.

PNG 16-bit samples are written big endian before byte-wise Paeth filtering,
with bytes-per-pixel 1/2/4/8. Straight alpha and RGB in transparent pixels are
preserved. Background hints are not composited or exported. Animation transport
metadata is rejected; use the existing legacy APNG route for animation.

Only `ICC Profile` is embedded, from encoder options or callback metadata
(options take precedence). Its header must declare the output model (`GRAY`
or `RGB `), carry `acsp`, and have the declared profile length. This is a
header/model check, not complete ICC validation or a CMS transform. Source ICC
is provenance and is never implicitly promoted. A known source-model mismatch,
or Gray storage carrying an output ICC expanded to RGB, requires an explicit
compatible output ICC in encoder options or native Gray output. Supplying an
RGB ICC alone does not transform pixels; callers remain responsible for any
required color conversion. Opaque or malformed output ICC bytes are rejected.

The existing `exif` option is supported. Other source metadata and source ICC
are not automatically copied. Resource limits check pixel count, one frame,
requested pixel extent, metadata and an estimate of known adapter buffers before
picking pixels, followed by final output size. The estimate covers picked
pixels, scanlines, rows and chunk assembly; it is not a bound on total process
RSS, caller-owned storage/custom callback allocations or miniz internals.
`encode_end_ex` is called after every successful start call (including `Ok(None)`),
including encode errors;
the original encode error takes precedence over an end error.

## Verification

`wml2/tests/png_encode_ex.rs` uses the independent `png` crate decoder for all
four layouts, lower-byte-sensitive samples, zero/half alpha, odd padded strides,
two-row filter boundaries, Exact/Round (12→16 and16→8), ICC models and adapter
limits/lifecycle. It includes an ignored release benchmark and optional display
artifact:

```sh
cargo test -p wml2 --no-default-features \
  --features png,jpeg,idct_llm,image-buffer-ex \
  --test png_encode_ex --test png_encode --test image_buffer_ex_icc \
  --test jpeg_png_icc_metadata
WML2_PNG_EX_DISPLAY=/tmp/.test-png-ex.png cargo test --release -p wml2 \
  --no-default-features --features png,image-buffer-ex \
  --test png_encode_ex display_and_benchmark -- --ignored --nocapture
```

Local validation (Rust 1.91.0, Linux x86_64, main `6a48db9`): 108 passed,
one manual benchmark ignored in the normal run; the manual test passed
separately. Feature-off and EX-without-PNG builds passed. Clippy completed with
existing library warnings. The generated 512×384 RGBA16 PNG was independently
read by `png` and Pillow 12.3.0, and its opaque/half-alpha/transparent bands were
visually inspected over a neutral background.

Release measurements are local observations, not throughput guarantees:

| Route (512×384) | main | This change | Encoded bytes |
| --- | ---: | ---: | ---: |
| Existing RGBA8 | 4.978ms | 4.935ms | 4349 |
| New EX RGBA16 | — | 18.613ms | 19407 |

RGBA8 uses three alternating baseline/candidate process pairs, each 40 timed
encodes after three warmups, reporting the median of process medians. Individual
baseline medians: 4.852/5.049/4.978ms; candidate: 4.759/4.970/4.935ms. A deterministic
opaque gradient sets R=`x*255/511`, G=`y*255/383`, B=`(x+y)*255/894`.
Both output files have SHA-256
`37539379c5ea11d6a8a71905a7f87bd48c9240252201e352869b6c5d55ee6b0e`.
The EX measurement is one process with 20 timed encodes after three warmups;
its sample generator and complete independent sample comparison are in the
manual test. Other OSes, WASM execution and large-image process RSS were not
measured locally; CI results are reported separately on the PR.
