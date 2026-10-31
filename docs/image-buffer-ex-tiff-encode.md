# TIFF EX encoding

`image_encoder_ex` accepts static, single-page **Gray16 or RGBA16** TIFF with
`tiff,image-buffer-ex`. This extends the EX entry point introduced by PNG
PR #53; it does not change legacy 8-bit TIFF or require the PNG feature.

```rust
use wml2::draw::{EncodeOptionsEx, ImageBufferEx, PixelFormatEx, PixelOutputEx,
    PrecisionConversion, image_encoder_ex};
use wml2::util::ImageFormat;
let mut image = ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba12,
    None, vec![0; 8])?;
let tiff = image_encoder_ex(&mut EncodeOptionsEx {
    debug_flag: 0, drawer: &mut image,
    output: PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Round),
    options: Some(std::collections::HashMap::from([
        ("compression".into(), wml2::metadata::DataMap::Ascii("deflate".into())),
        ("predictor".into(), wml2::metadata::DataMap::UInt(2)),
    ])),
}, ImageFormat::Tiff)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The output request is explicit. The picker applies normalized Exact/Round to
all samples including alpha (12→16 uses `65535/4095`, never a bit shift).
Storage remains unchanged. Return dimensions, format, stride and bytes are
checked again by the encoder. Odd byte strides and padding after the final row
are supported; padding is removed. TIFF stores unsigned samples and straight
alpha (`ExtraSamples=2`). RGBA16 retains four channels even if opaque, including
RGB under zero alpha. Gray16 uses BlackIsZero; no new color conversion is added.

The initial output is **one strip on one page**. Its RowsPerStrip is the image
height. Defaults are uncompressed, no predictor, Classic TIFF, little endian.
The EX route reuses the existing compression/predictor/BigTIFF option parsers:

| Option | EX values |
| --- | --- |
| `compression` | `none`/`uncompressed`/1; `lzw`/`lzw_msb`/5; `deflate`/`adobe_deflate`/8; `deflate_legacy`/32946 |
| `predictor` | `none`/1; `horizontal`/2 with LZW or Deflate |
| `bigtiff` | Existing true/false or 0/1 selection |
| `endian` | EX-only `little` (default) or `big` |
| `ICC Profile` | Explicit compatible output ICC bytes, overriding callback output ICC |
| `exif` | Existing Raw/Exif/`copy`, explicitly selected and budgeted |

The existing sample-based 16-bit predictor is applied to LE callback samples
before an optional swap to BE. Differencing resets on every row. Standard
MSB-first LZW code packing is independent of TIFF sample byte order. JPEG,
nonstandard `lzw_lsb`, EX 8/12-bit output, and animation transport are rejected.
Use the legacy entry point for existing 8-bit/multipage output.

PNG and TIFF share a crate-private ICC validator, independent of codec features.
Only `ICC Profile` describes output. `Source ICC Profile` and source TIFF ICC
tags are never automatically promoted. The validator checks ICC header length,
`acsp` and GRAY/RGB model, including Gray→RGBA expansion rules. It does not
validate all ICC tags or apply CMS. Callers prepare any needed color conversion.
Descriptive EXIF tags are preserved when explicitly requested; structural tags
are rebuilt for the output, including bit depth, sample format and alpha.

Pixel, one-frame, requested extent, metadata and known-buffer workspace limits
are checked before pixel picking. Shared EXIF selection/sizing borrows Raw/copy
bytes and checks structured serialization before materialization. The workspace
estimate includes adapter buffers, header assembly and metadata serialization;
it is not total process RSS and excludes callback-owned storage/allocations,
allocator overhead and internal codec dictionaries/workspaces. After a successful
start call, even `Ok(None)`, `encode_end_ex` runs on all result paths. A start
error does not call end; the first encode error takes precedence over end errors.

## Validation

The independent `tiff` crate decoder checks samples and tags across 56 cases:
Gray16/RGBA16 × compression/predictor × Classic/BigTIFF × LE/BE. Tests include
lower-byte values and carry/borrow boundaries, zero/half/full alpha, odd input
and output strides, strip offsets/counts, ICC and sample tags. Additional cases
cover 12→16 Exact/Round for each color/alpha, mismatched ICC/source provenance,
callback failures/layout mismatches, limits and explicitly selected EXIF.

```sh
cargo test -p wml2 --locked --no-default-features \
  --features png,tiff-jpeg,idct_llm,image-buffer-ex --lib \
  --test tiff_encode_ex --test tiff_encode --test tiff_lzw_modes \
  --test tiff_ex_sample_format --test tiff_ex_fill_order \
  --test tiff_review_regressions --test png_encode_ex --test png_encode \
  --test image_buffer_ex_icc --test jpeg_png_icc_metadata
WML2_TIFF_EX_DISPLAY_PREFIX=/tmp/.test-tiff-ex cargo test --release -p wml2 \
  --locked --no-default-features --features tiff,image-buffer-ex \
  --test tiff_encode_ex display_and_benchmark -- --ignored --nocapture
```

Local verification used Rust 1.91.0 / Linux x86_64. The combined matrix above
passed 126 tests, with the PNG/TIFF manual benchmarks normally ignored. The TIFF
manual benchmark passed separately. TIFF-only EX metadata tests also passed;
feature-off, EX-only and TIFF-without-EX builds passed. Clippy completed with
existing library warnings. Pillow 12.3.0 independently opened all three generated
RGBA16 TIFFs and their alpha bands were visually inspected over a neutral field.

Release measurements are local observations, not throughput guarantees. The
legacy comparison uses three alternating baseline/candidate process pairs pinned
to CPU 0, 20 timed encodes per codec after three warmups, and no concurrent local
builds/tests. Baseline is PNG PR #53 commit `b9c9644` (legacy TIFF remains main's
implementation). The deterministic 512×384 opaque gradient has R=`x*255/511`,
G=`y*255/383`, B=`(x+y)*255/894`. LZW/Deflate use horizontal predictor.

| Legacy RGB8 route | Baseline median of medians | Candidate | Bytes (identical) |
| --- | ---: | ---: | ---: |
| none | 0.471ms | 0.462ms | 590052 |
| LZW | 44.073ms | 44.312ms | 19819 |
| Deflate | 3.298ms | 3.355ms | 5032 |

The small timing differences are within the observed local run variation.
Individual baseline/candidate medians in ms (none/LZW/Deflate):
`0.445/42.535/3.137` vs `0.444/43.167/3.355`;
`0.471/44.073/3.298` vs `0.485/44.312/3.296`;
`0.473/44.152/3.320` vs `0.462/45.001/3.531`.

SHA-256, equal before/after for each compression:

- none: `bc3bb7173a2d4506875448df7aaa7ab3507fc08ca276f9cb16d70a2f17fbd43c`
- LZW: `b4d3f89dcf7f3ff81cac3ffccb6bcb4f716c2d90a86ac4d2449359e509ec7401`
- Deflate: `864e4824cbd28ef61da58fe8bdc84cb228a930e54cd0b07f4748e0c2a1c7ddb8`

New EX RGBA16 single-process medians (20 timed + three warmups): none 0.148ms /
1573126 bytes, LZW 118.305ms / 86658 bytes, Deflate 10.849ms / 19279 bytes. The manual
test retains the complete sample comparison and generator. These numbers do not
compare equivalent 8/16-bit work. No total-process memory or other-OS/WASM runtime
measurement is claimed; CI results are reported separately on the PR.
