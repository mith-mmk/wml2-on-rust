# JPEG XL still-image decoding

WML2 0.0.38 adds opt-in JPEG XL still decoding, together with shared native
Gray↔RGB ICC layouts and explicit sRGB display conversion. Version0.0.37 does
not contain this feature. Historical development checkpoints and measurements
are retained in [continuation validation](jpeg-xl-continuation-20261019.md) and
[internal-layer validation](jpeg-xl-layers.md).

The opt-in `jpeg-xl` feature uses **published `jxl =0.7.4`**, not upstream main.
Defaults are unchanged; JPEG XL encoding is unavailable. Add `image-buffer-ex`
for high-precision callback output. No C library, image-rs, CMS, SIMD or thread
pool is pulled into the runtime by this feature.

```toml
wml2 = { version = "0.0.38", features = ["jpeg-xl", "image-buffer-ex"] }
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
- Integer **8,9,10,12,16-bit** Gray/RGB, optionally one full-resolution straight
  alpha channel with the same depth as colour. Orientation is applied once by
  the decoder. RGB becomes RGBA with opaque alpha; GrayAlpha becomes RGBA.
- The EX source is native Gray8/12/16 or Rgba8/12/16, packed, with12/16-bit
  samples in little-endian u16 slots. Coded9/10-bit inputs are normalized into
  Gray16/Rgba16, with coded depth retained separately. Promotion is within one
  u16 unit of the independent reference and preserves recoverable source codes.
  Transparent RGB and low bits survive
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
animations (no silent first-frame fallback), intrinsic display sizes,
premultiplied or mixed-depth/subsampled alpha, multiple/other extra channels,
compressed `brob` metadata and unsupported depths.
Supported embedded previews are [bounded and skipped](jpeg-xl-preview.md) while
returning the main image. No preview extraction API or progressive-output mode is
added; preview entropy is not validated by main-only decoding.
Explicit Level5 and Level10 `jxll` boxes are accepted; dimensions, workspace and
the supported sample profile remain independently bounded. This does not promise
the entire Level10 profile. Native JPEG bitstream reconstruction is not implemented.

## Colour and source metadata

JPEG XL's own XYB→RGB reconstruction stays inside `jxl`. The existing ordinary
and EX APIs retain their source-coordinate colour contract. With
`jpeg-xl,color-management`, `jpegxl::image_from_srgb` and
`image_from_srgb_with_limits` explicitly produce sRGB RGBA8 through the own
ICC engine, after16-bit reconstruction and before final rounding. They support
Gray→RGB with separate channel spans and exclude alpha. No tone mapping is added. **Output samples need not be in the source
profile**: XYB may use an sRGB fallback when the embedded profile cannot be
used directly. The adapter queries the output profile after setting the pixel
format, not before.

| Metadata key | Meaning |
| --- | --- |
| `ICC Profile` | Profile for the adapter's delivered callback samples, generated from simple signaling when needed; explicit EX storage conversion does not transform this profile |
| `JPEG XL Source ICC Profile` | Original embedded ICC bytes, or equivalent generated ICC for original simple signaling |
| `JPEG XL Reconstruction ICC Profile` | Actual pre-CMS reconstruction profile, retained by the explicit sRGB display API; may differ from the original embedded profile |
| `JPEG XL Source Header` | Original bit-aligned codestream-header prefix; last byte may also contain following bits; not a standalone file |
| `JPEG XL Source Box <index> <hex-type>` | Uncompressed auxiliary box bytes, including unknown types and duplicate types with distinct indexes |
| `JPEG XL Source Bit Depth` / `JPEG XL Output Bit Depth` | Coded colour depth / declared delivered sample depth |

Gray replicated into RGBA gets a matching neutral-axis RGB profile for simple
Gray signaling. Ordinary RGBA decoding of an arbitrary Gray ICC is explicitly
rejected because assigning a matching RGB profile would require a CMS; native
EX Gray callback output can retain it. Subsequent EX storage conversion copies
samples without transforming the ICC; its source format still identifies Gray.
Metadata is delivered after pixels, matching existing static adapters. Source
orientation remains in the raw header; output dimensions/pixels are oriented.
Raw EXIF/XMP boxes are preserved as source bytes, not parsed or reapplied.
ICC headers, tag extents and Gray/RGB channel identity are validated; ICC CICP
PQ/HLG is rejected. Arbitrary opaque ICC transfer curves are preserved, not
interpreted as an HDR rendering promise. There is no HDR tone mapping or
semantic classification of arbitrary ICC LUTs.

## Still images with JPEG reconstruction side data

An uncompressed outer `jbrd` box no longer prevents pixel decoding. Its payload
is retained under the existing indexed `JPEG XL Source Box <index> 6a627264`
Raw key, with the same aggregate metadata, box-count and workspace checks as
other auxiliary data. Before/between/after-codestream placement, duplicate boxes,
extended sizes and final zero-size boxes follow the existing container rules.
Duplicate payloads remain separately indexed rather than overwriting each other.

**Opaque retention is not JPEG reconstruction or payload validation.** The
payload may itself contain compressed reconstruction information; WML2 never
parses or decompresses it, uses it for pixels/ICC, or promises it can reproduce
a JPEG file. An empty or invalid opaque payload can accompany valid pixels.
The outer container envelope and pixel codestream must still be valid.
All `brob` boxes remain explicitly unsupported, including wrapped `jbrd`.
Unknown auxiliary data is also retained as bounded Raw bytes without execution
or semantic interpretation. Native JPEG reconstruction APIs are not added.

This follows pixel-only behavior in [libjxl0.11.2](https://github.com/libjxl/libjxl/blob/v0.11.2/lib/jxl/decode.cc#L1842)
and the auxiliary-data path of the published `jxl0.7.4` parser.
The independent fixtures use official [conformance data pinned to b1d0f990](https://github.com/libjxl/conformance/tree/b1d0f990b03e57bf6d137c365cd5dc8b470b9191/testcases).

| Official case | Ordinary RGBA8 | Native EX | ICC and orientation |
| --- | --- | --- | --- |
| `bench_oriented_brg` | Supported | Rgba8 | Exact official RGB ICC; coded orientation5 applied once,606x500 output |
| `grayscale_jpeg` | Explicit Gray ICC/CMS refusal remains | Gray8 | Exact official Gray ICC;200x200 output |

Request native `PixelFormatEx::Gray8` for the Gray sample. Explicit EX storage
conversions do not perform CMS transforms; a request to copy Gray samples into
RGBA storage does not establish a matching RGB profile. The ordinary decoder's
existing CMS refusal is preserved rather than changing or discarding the ICC.

`generate_jpegxl_jbrd_fixtures.py` pins input hashes, fetches official ICC/float
reference objects by SHA-256, and obtains a second oracle from `djxl0.11.2`.
Native/ordinary tests compare every supported output pixel with both oracles
within2/255, plus exact ICC/Raw side data. Float references are quantized to the
declared integer8 domain; the unquantized float RMS bound is not an integer8
output requirement. The actual cloud maximum difference was1/255 for both cases.
Fixture oracles are zlib-compressed test data, unrelated to runtime `brob` support.
The WASM harness checks25 spatial probes per official image, full ICC/dimensions,
limit failures and the ordinary Gray refusal. Provenance, hashes and testing
permissions are in `wml2/tests/fixtures/review/jxl-jbrd-fixtures.json`.

## Limits and cancellation

Both entry points use `DecodeLimits` and the shared callback guard. Input,
canvas pixels/bytes, frame count, source/output metadata and callback forwarding
are checked independently. Abort stops later draws/metadata and terminates
once, including the ordinary RGBA8 path. Malformed/truncated container envelopes
and pixel codestreams are rejected before initialization; opaque auxiliary payloads
are retained without semantic validation. There is no partial-image success.

Additional fixed profile caps:64MiB input,64KiB header prefix,16384 per image
dimension,4Mi effective padded frame pixels,auxiliary metadata insertion before box index1025. The conservative
workspace estimate includes input assembly,128bytes per maximum admitted internal
frame pixel, cumulative retained-layer f32 planes,1MiB header workspace, bounded
TOC permutation/ICC entropy work and ICC storage. All internal frame headers
and TOCs are scanned before pixel decoding; the canvas quota does not cap layer
dimensions. See [internal layers](jpeg-xl-layers.md). It is charged against `expanded_bytes`;
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
alpha0/1/mid/max and nonzero RGB under alpha0. Twelve Gray/GrayAlpha/RGB/RGBA depth
combinations also have `djxl` oracle output. A LittleCMS2 Gray gamma2 profile
checks native Gray ICC retention and explicit RGBA-expansion refusal. Fixture
regeneration requires Pillow and a system LittleCMS2 library. Hashes/provenance are in
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

## Validation snapshot

See [machine-readable results](jpeg-xl-validation.json) for commands, source
hashes, local results, measured allocations and the actual WASM artifact hash.
The portable helper runs55 cases in Node without host imports:

```console
python3 wml2-test/scripts/verify_jpegxl_wasm.py --output .test-jxl-wasm
```

Independent read-only review findings were fixed before the final checks.
Clippy is run in the same warning mode as repository CI: existing unrelated
warnings remain; the JPEG XL implementation/tests introduce none. Windows and
macOS execution, browser UI integration, general ICC/HDR rendering, SIMD and
threaded decoding are not covered by this local validation.

## Branch CI coverage

The shared `checks` workflow also runs dedicated JPEG XL jobs on Windows and
macOS, with ordinary, EX and disabled-feature tests. Windows additionally
executes those contracts as32-bit MSVC binaries. These dedicated jobs do not
wait for the slower existing JPEG2000/TIFF platform regressions.

The WASM compile job executes the55-case Node harness and uploads its report.
A feature-contract job checks the expected mutually exclusive IDCT failure for
`--all-features`, then checks all targets and executes JPEG XL tests for both
all-codec and `noretoro` profiles with only one IDCT enabled. This also exposed
and repaired the existing `SJIS` feature's missing optional `encoding_rs`
dependency; its locked version and default features are unchanged.

These checks run on branch pushes; a PR is not required. For the result of an
individual revision, use its GitHub Actions run and exact commit SHA rather
than treating the older local validation snapshot as a cross-platform result.

## Remaining validation and scope decisions

The October18 independent still-image review found and fixed rejection of a
valid explicit Level5 container. Its regression fails on revision
`4467931cceb30071d4d77891aed10e808650b7da` and passes with the fix. Both levels
are checked with `jxlc`/`jxlp`, ordinary/native pixels, ordering and limits.
Seven additional independently generated RGB/Gray/RGBA and thin-canvas inputs
matched original samples;2,100 deterministic corruption/truncation probes did
not panic. The subsequent read-only diff review checked fixture hashes, sample
formulas, Gray gamma2 identity and the EX/ordinary regression suites. These
finite probes do not establish general decoder correctness.

The local Windows owner is responsible for real display testing against the
25-file corpus at `D:\data\samples\images\jxl\wml2_static_selection_20261018`
and its `manifest.json`. The earlier local report identified `jbrd` refusals for
the two official cases above at revision1b8007c; a local rerun of this patch is
pending. The local collection/manifest is unavailable in the cloud; official
copies were obtained independently. Hosted CI and cloud image checks are separate.
The [hosted56-job success for revision1b8007c](https://github.com/mith-mmk/wml2-on-rust/actions/runs/39112081722)
is prior evidence, not a result for this patch. Normal pushes run existing CI automatically;
no separate Windows execution or workflow dispatch is needed for this review.

Remaining profile limitations include fixed resource caps, Gray ICC expansion,
compressed auxiliary metadata and native JPEG reconstruction. All eight orientation
values have not been independently tested;90-degree rotation and orientation5
transpose are covered.
Browser UI integration, SIMD and threads are also outside the current evidence.
9/10-bit sources, premultiplied alpha, HDR rendering, animation and encoding require a separate scope proposal before
implementation. No PR, main merge or release is part of this branch work.

## Output orientation contract

`orientation_applied` is a format-independent metadata key whose value is `DataMap::Bool`. `true` means the output pixels already include the source orientation; consumers must not apply source EXIF or other source orientation again. `false` means this marker does not suppress the consumer's existing orientation handling. An absent key likewise retains the legacy behavior. A value of another type is invalid; consumers should treat it as absent rather than infer a boolean. This affects automatic orientation only, not user-requested rotation.

JPEG XL sets the marker to `true` and stores the original coded 1–8 value separately as `JPEG XL Source Orientation` (`DataMap::UInt`), alongside the raw source header. That original value describes the input, not a remaining transform on the delivered pixels. The pinned jxl 0.7.4 applies the coded transform to its output; its `adjust_orientation` option is not consumed by that version. The eight fixture tests check pixels against independent references so a change to that behavior is detected. Other formats do not acquire this marker merely because it is absent; their existing metadata and consumer behavior remain unchanged.
