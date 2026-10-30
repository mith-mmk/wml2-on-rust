# Basic image information without pixel expansion

`wml2::image_info` adds a basic information API for JPEG, PNG/APNG, GIF,
TIFF/BigTIFF and PSD v1, using their enabled codec features. It is independent
of EX buffer storage and does not change existing decoding entry points.
No JPEG XL runtime or public metadata keys are changed.

```rust
use wml2::image_info::{image_info_from_reader, ImageCount, ImageStructure};
# fn inspect() -> Result<(), Box<dyn std::error::Error>> {
let mut file = std::fs::File::open("example.png")?;
let info = image_info_from_reader(&mut file)?;
println!("{} × {}", info.dimensions.coded.width, info.dimensions.coded.height);
if let ImageStructure::Animation { frames } = info.structure {
    match frames {
        ImageCount::Known(n) => println!("{n} image blocks counted"),
        ImageCount::Declared(n) => println!("{n} frames declared, not verified"),
        ImageCount::Unknown => println!("frame count not obtained"),
    }
}
# Ok(())
# }
```

Byte-slice helpers are `image_info` and `image_info_with_limits`. Seekable
`Read+Seek` helpers are `image_info_from_reader` and
`image_info_from_reader_with_limits`. The reader route starts at offset zero,
never copies the entire file, and attempts to restore the original position on
success/error. A prior parse error takes precedence over a restoration error.
The `image_info` example accepts file paths and prints typed results.

## Explicit contracts

`ImageDimensions` separates:

- `coded`: the encoded canvas or first TIFF image IFD dimensions.
- `output`: the existing ordinary full-resolution decoder canvas if obtained,
  before caller scaling. `None` is not inferred from `coded`.
- `intrinsic_display`: a separately specified intrinsic/display size if obtained.
  All initial routes return `None`; this does **not** assert absence, and must
  not be filled by copying coded/output dimensions. No resampling is performed.

For JPEG/PNG/GIF/PSD, the ordinary decoder canvas, if decoding succeeds, matches
the header canvas. These routes do not automatically apply EXIF orientation.
TIFF returns `output=None`: its ordinary decoder can skip an initial thumbnail
or mask and select a later primary IFD. The first-directory query does not make
that selection. TIFF's obtained Orientation 1–8 is reported separately and does
not swap coded dimensions. JPEG/PNG EXIF orientation is not parsed (`None`).
These distinctions provide the common dimension contract for #45; JPEG XL
intrinsic-size acceptance and oriented output remain separate work.

Color, channels, bits and alpha describe **encoded samples**, not expanded
RGBA callback storage or pixel opacity. Indexed PNG/GIF depths describe indices.
JPEG 3/4-component color transforms are not guessed from component count: the
model is `Components`, not an asserted RGB/YCbCr/CMYK interpretation. A declared
alpha/transparency mechanism may be completely opaque in actual pixels.
Unknown fields use `None` or explicit `Unknown` variants; no image count defaults
to one when the relevant structure was not traversed. A PSD `Layers` organization
means a layer-capable document, not proof of nonempty layers.

## Initial support and scanning

| Format/feature | Information obtained | Scan / intentional unknowns |
| --- | --- | --- |
| JPEG / `jpeg` | SOF0/1/2 canvas, 8/12-bit precision, 1–4 components; no standard alpha | Marker lengths to first supported SOF. APP/COM/table payloads skipped. Other JPEG processes/deferred dimensions unsupported. Color transform/EXIF unknown. |
| PNG/APNG / `png` | IHDR canvas/depth/model, tRNS/alpha, acTL declared frame count | Preamble to first IDAT header. IHDR and selected tRNS/acTL CRC checked. Indexed palette dimensions checked; opaque metadata/palette data skipped. Frames not reconstructed/counted. |
| GIF87a/89a / `gif` | Logical canvas, indexed color, transparency declarations, actual image-block count | Entire block structure through trailer. LZW sub-block bytes skipped. Palette depth reported for one image; multiframe depth unknown. Plain-text graphics make presentation count/alpha unknown; text-only GIF unsupported. GCE consumption and fixed12/11-byte text/application headers checked. |
| TIFF/BigTIFF / `tiff` | First main IFD canvas/depth/model/sample kind/alpha/orientation; 1–64 samples | Existing bounded IFD parser projected to basic tags. Other payload ranges checked but bytes not read. No strips/tiles, EXIF/GPS, SubIFDs or later pages visited. Main-chain count known 1 only if next pointer zero; otherwise unknown. Output selection unknown. |
| PSD v1 / `psd` | Existing supported 8/16-bit Gray/indexed/RGB/CMYK header canvas/channels | Existing 26-byte parser only. Resources/layers/composite not read. Alpha and layer count unknown; PSB/unsupported modes/depths rejected by existing header policy. |

A `Still` result describes the examined conforming image stream; it is not a
check for appended concatenated images or optional multi-picture metadata.
`Pages`/`Layers` describe container organization even when the count is one or
unknown. `InfoScan` distinguishes a header-prefix walk, container scan, and
first-directory query. A prefix walk is not a fixed-size read: a JPEG SOF or PNG
IDAT can follow large metadata. GIF may require many seeks through the file.
`bytes_read` counts bytes delivered by the supplied `Read` implementation,
including repeated signature reads, not seek distance or physical disk I/O.
Caller-supplied readers may have their own buffering.

**Success is not image validation.** Header information can succeed on bad or
absent pixel streams. It does not validate later frames/pages, JPEG tables or
entropy, PNG IDAT/zlib and later chunks, TIFF compression/sample payloads, or
PSD sections after the fixed header. Unknown TIFF tags are not semantically
validated, although known field-width/count arithmetic and payload ranges are
bounded. Malformed/truncated queried structure is an error; no partial info is
returned. Unsupported or feature-disabled routes return an error. Format
recognition otherwise follows existing `format_check` feature behavior.

ICC/EXIF/XMP/text are intentionally separate from basic info. Their opaque
payloads are neither returned nor decompressed/validated. Malformed detailed
metadata inside an otherwise bounded envelope does not become a basic-info
failure. Existing detailed-metadata workflows remain separate; no universal
metadata extraction API is added here.

## Bounds and errors

The existing `DecodeLimits` vocabulary is reused:

- `input_bytes`: length of the entire seekable input, checked before parsing.
- `metadata_bytes`: cumulative actual header reads; existing TIFF directory and
  selected-field accounting also applies. Skipped payload bytes are not read or
  materialized. File/range framing still has to fit within input length.
- `expanded_bytes`: conservative known header workspace (minimum 4 KiB, plus
  TIFF projected-directory/field estimates), not a dimension-derived pixel
  allocation. It is not total RSS, allocator overhead or caller-owned input.
- `frames`: traversed or declared image counts. An unvisited TIFF chain and PSD
  layer count stay unknown; they are not claimed to fit this count budget.
- `pixels`, `animation_bytes`: intentionally unused; the API allocates no
  decoded canvases or animation frame buffers. Format dimension rules still apply.

`ImageInfoError` distinguishes unknown format, unavailable information route,
invalid queried header, explicit budget failures, I/O, and existing header-parser
errors. `Header` retains the underlying error as its source, including the
parser's own truncation/resource/unsupported errors. `None`/`Unknown` values are
successful incomplete information, not swallowed parse failures.

## Verification

Local Rust 1.91.0/Linux x86_64 validation: 150 passing tests across library,
information API, PNG/APNG, GIF, TIFF/JPEG-TIFF/EX/metadata regression, PSD and
callback targets; one manual benchmark normally ignored and passed separately.
Standalone PNG/TIFF/GIF/PSD/JPEG information feature builds and the feature-off
case pass. Clippy completes with existing library warnings and no diagnostics in
the new API/test/example files. Formatting and diff checks pass.

The information tests use real bundled files, valid generated animation/page/PSD
streams, malformed/truncated queried structures and unknown fields. Synthetic
huge canvases succeed with `pixels=0`, `animation_bytes=0`, 8 KiB header workspace
and a 4 KiB metadata-read budget. The test reader forbids reads from compressed
raster and opaque ICC payloads, and a test allocator checks the largest individual
allocation stays within 8 KiB. TIFF basic field counts are rejected before payload
materialization if outside scalar/64-sample scope. Invalid raster streams are
accepted only where the documented scan ends before raster validation. This is
not an allocation/RSS proof for caller-supplied buffering or an entire process.

```sh
cargo test -p wml2 --locked --no-default-features \
  --features png,tiff-jpeg,idct_llm,gif,psd,image-buffer-ex,high-bit-depth \
  --lib --test image_info --test png_encode --test tiff_encode \
  --test tiff_review_regressions --test tiff_ex_sample_format \
  --test tiff_ex_fill_order --test psd_decode --test gif_encode \
  --test jpeg_png_icc_metadata --test image_buffer_ex_callback_contracts
cargo test --release -p wml2 --locked --no-default-features \
  --features png,jpeg,idct_llm,tiff --test image_info benchmark_header_queries \
  -- --ignored --nocapture
cargo run -p wml2 --example image_info -- samples/WML2Viewer.png
```

Resident-slice information query medians (1000 timed calls +3 warmups, one local
process; test allocation recording disabled) were PNG 0.651µs /81 bytes read from
276778 input bytes, JPEG 0.261µs /35 from 5962, and TIFF 0.931µs /130 from 2118.
They measure header traversal, not disk I/O or universal format throughput.

The existing TIFF parser is specialized at compile time for full/basic modes;
the existing decoder does not carry a runtime information-mode switch. Shared
PNG color/depth validation is unchanged and inline. A separate existing-decode
release comparison against main `6a48db9` used three alternating process pairs
on CPU 0, with 3 warmups and 40/100/1000 timed decodes for PNG/JPEG/TIFF:

| Existing `image_load` fixture | Main median of medians | Candidate |
| --- | ---: | ---: |
| `samples/WML2Viewer.png` | 16904.652µs | 16527.173µs |
| `restart-p420-i1.jpg` | 276.046µs | 256.176µs |
| `ex-gray16-lzw.tif` | 103.346µs | 102.805µs |

Per-process main→candidate medians (PNG/JPEG/TIFF in µs):
`17391.026/293.803/103.346`→`17003.101/256.176/102.184`;
`16904.652/276.046/103.486`→`16282.053/251.729/103.646`;
`16653.553/275.245/99.801`→`16527.173/256.887/102.805`.
No concurrent local builds/tests ran during those measurements. They show local
variation and do not establish a general speedup or zero-overhead guarantee.
Decoded full-buffer FNV-1a64 hashes match in every run:
PNG `d9bb686b04c8b83a`, JPEG `183d6f2bf4b228e1`, TIFF `b999970149ed0ff5`.
The PNG output was exported and visually inspected. Other-OS/WASM runtime and
process RSS were not measured locally; CI results are reported on the PR.
