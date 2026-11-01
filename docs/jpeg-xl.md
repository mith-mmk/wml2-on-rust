# JPEG XL still-image decoding

WML2 0.0.38 adds opt-in JPEG XL still decoding, together with shared native
Gray↔RGB ICC layouts and explicit sRGB display conversion. Version0.0.37 does
not contain this feature. Historical development checkpoints and measurements
are retained in [continuation validation](jpeg-xl-continuation-20261019.md) and
[internal-layer validation](jpeg-xl-layers.md).

The development adapter decodes with **published `jxl =0.8.0`** and retains
**published `jxl =0.7.4` as a parser-only preflight dependency**. This is the
[unreleased 0.8 migration](jpeg-xl-080-migration.md), not an all-0.8 dependency
graph or a claim that published WML2 0.0.41 already contains this change.
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
  alpha channel, with independently supported8/9/10/12/16-bit integer depth.
  Orientation is applied once by
  the decoder. RGB becomes RGBA with opaque alpha; GrayAlpha becomes RGBA.
- The EX source is native Gray8/12/16 or Rgba8/12/16, packed, with12/16-bit
  samples in little-endian u16 slots. Coded9/10-bit inputs are normalized into
  Gray16/Rgba16, with coded depth retained separately. Promotion is within one
  u16 unit of the independent reference and preserves recoverable source codes.
  Mixed colour/alpha depths use normalized Rgba16 for every channel, retaining
  the original colour and alpha depths separately; same-depth layouts stay as
  before. The common16-bit format prevents silently narrowing a deeper alpha.
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
animations (no silent first-frame fallback),
associated alpha outside the [qualified integer subset](jpeg-xl-associated-depths.md),
subsampled alpha outside the [declared integer subset](jpeg-xl-associated-subsampling.md),
float or other alpha depths, multiple/other extra channels,
forbidden compressed control/nested boxes and unsupported depths.
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
| `coded_width` / `coded_height` | Source canvas axes (`DataMap::UInt`), before native orientation |
| `output_width` / `output_height` | Delivered canvas axes after native orientation; no intrinsic resizing |
| `intrinsic_display_width` / `intrinsic_display_height` | Explicit intrinsic display axes after native orientation, present only if declared; display units, not allocated pixels |
| `JPEG XL Source Intrinsic Width` / `JPEG XL Source Intrinsic Height` | Original intrinsic axes (`DataMap::UInt`), present only if declared |
| `JPEG XL Source Header` | Original bit-aligned codestream-header prefix; last byte may also contain following bits; not a standalone file |
| `JPEG XL Source Box <index> <hex-type>` | Exact outer auxiliary payload bytes, including unknown types and duplicate types with distinct indexes; a `brob` value includes its underlying type and compressed stream |
| `JPEG XL Decoded Box <index> <hex-type>` | Bounded decompressed raw payload for a `brob` box, indexed by its underlying type; no recursive box parsing |
| `JPEG XL Source Alpha Bit Depth` | Original integer alpha depth, present for mixed-depth input (`DataMap::UInt`); source colour depth remains separate |
| `JPEG XL Source Bit Depth` / `JPEG XL Output Bit Depth` | Coded colour depth / declared delivered sample depth |

Gray replicated into RGBA gets a matching neutral-axis RGB profile for simple
Gray signaling. Ordinary RGBA decoding of an arbitrary Gray ICC is explicitly
rejected because assigning a matching RGB profile would require a CMS; native
EX Gray callback output can retain it. Subsequent EX storage conversion copies
samples without transforming the ICC; its source format still identifies Gray.
Metadata is delivered after pixels, matching existing static adapters. Source
orientation remains in the raw header; output dimensions/pixels are oriented.
Raw EXIF/XMP boxes are preserved as source bytes. The first valid supported
payload also supplies common `EXIF` (`exif` feature) / `XMP` metadata through
[bounded interpretation](jpeg-xl-metadata.md). Common EXIF Orientation is 1;
codestream orientation is authoritative and is never applied a second time.
ICC headers, tag extents and Gray/RGB channel identity are validated; ICC CICP
PQ/HLG is rejected. Arbitrary opaque ICC transfer curves are preserved, not
interpreted as an HDR rendering promise. There is no HDR tone mapping or
semantic classification of arbitrary ICC LUTs.

## Cooperative display cancellation

`image_from_srgb_with_limits_and_cancel(input, limits, &probe)` adds cancellation
without changing the existing sRGB, ordinary or EX entry points. The synchronous
`Fn() -> bool` probe runs on the decoding thread; return true to cancel. A UI can
capture a shared atomic flag. Keep the probe cheap and nonblocking. It need not
be `Send` or `Sync`; WML2 does not move it to another thread. Panics from the probe
are not caught.

```rust,ignore
let probe = || cancelled.load(std::sync::atomic::Ordering::Relaxed);
match wml2::jpegxl::image_from_srgb_with_limits_and_cancel(bytes, limits, &probe) {
    Ok(image) => publish(image),
    Err(error) if error.is::<wml2::jpegxl::DecodeCancelled>() => {},
    Err(error) => return Err(error),
}
```

The boxed public `DecodeCancelled` error distinguishes cancellation from decode
and resource-limit errors. Cancellation is latched once observed. A cancelled
call returns no `ImageBuffer`; all partial buffers and metadata remain private
and are dropped. There is a final check after private image population and before
return. As with any cooperative API, a caller publishing later must also check
its request generation/flag to handle cancellation after this final check.

The adapter checks before work, after bounded preflight, around codec stages,
between sample-copy/rounding chunks, before private image initialization, and
after population. The pinned **published `jxl 0.8.0`** provides a fallible
`JxlParallelRunner`, rather than libjxl's cancellation API. The cancellable path
uses a sequential runner, overriding both `run` and `run_ordered` (the latter's
default implementation bypasses `run` for one thread). Checks occur before each
LF-group/smoothing/render task and after each task batch. Its own cancellation
error is translated back to `DecodeCancelled`; genuine codec errors are retained.
The legacy routes still select the original codec runner.

ICC processing checks before parsing, before each **256-pixel** conversion batch,
and after the last batch. Alpha is excluded exactly as in the existing route;
conversion still uses the actual reconstruction ICC profile, followed by the
same RGBA8 rounding, and retains source profiles and orientation metadata.

**Non-interruptible sections:** container assembly/preflight, allocations and
individual memory copies, serial codec parsing/global/entropy work, an individual
codec runner task, and ICC profile parsing/transform compilation. No bounded
cancellation latency or ability to interrupt every codec operation is promised.
The existing input/pixel/expanded-workspace/metadata/frame/animation limits and
JPEG XL hard caps still apply. An already-true probe wins before validating input;
otherwise an error encountered before the next checkpoint wins. Cancellation
does not relax or reclassify a previously encountered limit/decode error.

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
Metadata `brob` boxes now have a bounded strict-Brotli path; see
[compressed metadata](jpeg-xl-brob.md). A `brob`-wrapped `jbrd` is forbidden and
remains rejected: this does not add JPEG reconstruction.
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

Published sources are pinned separately: [0.8 event decoder](https://github.com/libjxl/jxl-rs/tree/9a08ca643fd2d01f803dd0be6a5bebc9cc11e597/jxl)
and [0.7.4 compatibility preflight](https://github.com/libjxl/jxl-rs/tree/624ce908afcf8eb0dc2585a671535eaabb5d88cc/jxl).
The adapter uses 0.8's Event API, explicit orientation, straight AlphaMode and
Main5 policy. The latter preserves the former force_level5 defaults; container
Level10 labels do not activate Main10-only complexity. Removing the old parser
requires equivalent pre-allocation admission, not deletion of its checks.

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
Broader associated-alpha support, HDR rendering, animation and encoding require a separate scope proposal before
implementation. No PR, main merge or release is part of this branch work.

## Output orientation contract

`orientation_applied` is a format-independent metadata key whose value is `DataMap::Bool`. `true` means the output pixels already include the source orientation; consumers must not apply source EXIF or other source orientation again. `false` means this marker does not suppress the consumer's existing orientation handling. An absent key likewise retains the legacy behavior. A value of another type is invalid; consumers should treat it as absent rather than infer a boolean. This affects automatic orientation only, not user-requested rotation.

JPEG XL sets the marker to `true` and stores the original coded 1–8 value separately as `JPEG XL Source Orientation` (`DataMap::UInt`), alongside the raw source header. That original value describes the input, not a remaining transform on the delivered pixels. The 0.8 decoder explicitly sets `adjust_orientation = true`; its oriented BasicInfo size and source orientation are checked against preflight. Historical 0.7.4 applied the transform while ignoring that option. The eight fixture tests check pixels against independent references so a change to that behavior is detected. Other formats do not acquire this marker merely because it is absent; their existing metadata and consumer behavior remain unchanged.


## Intrinsic display dimensions and information query

An explicit intrinsic size is retained as metadata and does not resample pixels.
All eight native orientations keep the existing ordinary/EX/CMS behavior.
`wml2::image_info` can obtain coded, oriented output and optional intrinsic
geometry without pixel/profile expansion. Its JPEG XL route intentionally leaves
encoded sample/color/alpha fields unknown and stops before those descriptions.
See [the dimension contract and bounded-prefix limitations](jpeg-xl-intrinsic.md).

## Mixed-depth straight alpha

One full-resolution straight alpha channel can independently use integer
8/9/10/12/16-bit depth with the existing supported Gray/RGB colour depths.
Mixed-depth native EX output is normalized Rgba16; Exact requests for narrower
formats reject, while Round applies the existing conversion to every channel.
Ordinary/CMS output remains deterministic RGBA8, with alpha excluded from ICC
conversion. `animation_bytes` admits the common8-byte-per-pixel native working
canvas before codec work, even for ordinary output. Existing same-depth/native
Gray layouts and refusal of associated/subsampled/general extra planes remain.
See [the bounded subset, independent oracles and upstream evaluation](jpeg-xl-alpha.md).

## Unreleased explicit resource configuration

The additive `image_from_with_resource_limits` and sRGB companion APIs accept
`JxlResourceLimits` for finite, caller-tightened JPEG XL admission. Existing APIs
retain their current caps and behavior. The new paths additionally charge live
adapter output, rounding, collector and CMS/profile storage cumulatively.
This is the first runtime phase of #37, not larger-canvas support or a global
allocation ledger: 2560x1440 still fails the fixed padded-frame screen.
See [configuration, exact scope and validation](jpeg-xl-resource-contract.md).

### Development: preview geometry query

`jpegxl::preview_dimensions_with_resource_limits(input, limits, resources)` returns
`Some(ImageDimensions)` for an admitted embedded preview and `None` for an admitted
still without one. `coded` is preview geometry, `output` applies the shared image
orientation, and `intrinsic_display` is absent (the main image intrinsic size is
not a preview size). Errors are distinct from absence. The full existing container,
ICC size, TOC and layer admission runs under finite caller-tightened limits before
returning; this query initializes no pixel decoder and does not validate pixel
entropy. It is intentionally stricter than a prefix-only metadata probe.

This is the geometry-query substep of #44, stacked on #75 → #73 → #72. Preview
pixel extraction, its color/alpha output and dedicated preview/combined allocation
contract remain unimplemented; #44 stays open. Animation and HDR/float are unchanged.
The parser-only 0.7.4 dependency and fixed safety caps remain; #74 is not applied.

Validation of this substep: six existing independent libjxl preview-header fixtures
(including shared ICC and transposed orientation), no-preview input, every truncated
prefix of the ICC preview fixture, oversized preview, invalid signature, input/
metadata/workspace and individual ICC/TOC/layer limits. Ordinary preview main-image
and layer regressions remain covered. Local extended runs passed 41 JPEG XL, three
layer, 11 resource and two query tests; the query tests also passed without EX/CMS.
A release microbenchmark (Rust 1.91, no defaults, jpeg-xl/image-buffer-ex/color-management,
20 warmups, three processes each doing 1,000 queries on jxl-preview-rgba8.jxl) took
2.101/1.968/1.865 ms per process. This measures the new small-input query, not a
before/after decoder speed claim. The main image was decoded and visually inspected;
preview pixels were not decoded by this API. CI includes the query test target in
the existing JPEG XL platform and Windows 32-bit commands.

### Development: independent embedded preview pixels

`jpegxl::preview_from_with_resource_limits(input, limits, resources)` returns
`Option<JxlPreview>`. An admitted still without preview returns `None`; malformed
structure, invalid preview entropy and budget failures return errors with no result.
The result contains normalized native-endian `Vec<u16>` RGBA samples in **coded
preview coordinates**, width/height, source orientation (1..=8), and its actual
output ICC. Orientation is intentionally not applied; callers apply it when
displaying. `color_channels == 1` identifies Gray ICC coordinates replicated into
R/G/B; otherwise the ICC describes RGB. Alpha remains straight and is never passed
through CMS. This API performs no sRGB conversion or HDR rendering. Integer source
precision is normalized to 16 bits; it does not expose float/HDR or general extra
channels.

This API is separate from main-image APIs: it stops at the preview FrameComplete
event, expects a following main frame, and does not decode main pixel entropy. Full
existing container/ICC-size/TOC/internal-layer admission still runs first, including
for absence. Terminal-visible preview restrictions and fixed caps remain. Caller
canvas pixels and animation_bytes additionally bound preview output; workspace
reserves the existing main admission estimate plus padded preview codec/retained
planes, aligned output, generated profile allowance and retained ICC copy. This
conservative combined budget is not a global codec allocator/RSS guarantee.

Stack: #76 → #75 → #73 → #72; #74 is not applied. #44 now has geometry and native
preview pixels in development, while automatic orientation, managed sRGB preview,
cancellable acquisition and broader preview frame forms remain outside this small
step. Published main is unchanged and #44 remains open pending complete acceptance.

Validation: independent C libjxl 0.11 preview references were rerun against the
existing manifest hashes. Compressed preview vectors are committed, including
RGB/ICC/GrayAlpha, orientation6 and preview larger than main. Integer previews
match exactly; lossy RGB uses <=514/65535 tolerance and alpha is exact. The coded
orientation6 vector matches the nonoriented ICC fixture; applying orientation6
matches the independent C oriented hash. Every truncated ICC-preview prefix and
output/combined-workspace refusal is checked. Corrupt preview entropy rejects
extraction while existing main skip pixels remain identical. Native and minimal
query tests pass; existing main/layer/resource regressions pass. The actual preview
was visually inspected. Rust1.91 release, no defaults, jpeg-xl/image-buffer-ex/
color-management, 20 warmups and three processes ×1000 tiny rgba8 preview decodes
took 48.623/50.370/49.519 ms. This measures the new API, not a decoder-wide speedup.

### Development: oriented sRGB preview

With `jpeg-xl,color-management`,
`jpegxl::preview_from_srgb_with_resource_limits(input, limits, resources)` returns
`Option<ImageBuffer>` with oriented straight-alpha sRGB RGBA8. It reuses the main
image's own-library ICC transform, converts normalized16 samples in place, then
applies orientation and the same final16-to8 rounding in one output pass. The
`ICC Profile` describes sRGB, `JPEG XL Reconstruction ICC Profile` retains the raw
preview's output profile, and `orientation_applied` is true alongside the source
orientation. The existing raw `JxlPreview` API still returns coded native16 samples
and an unapplied orientation. Neither endpoint silently returns main pixels.

The managed endpoint keeps the entire conservative raw reservation while CMS
runs. Native16 samples, future RGBA8 output, metadata, both profile parse estimates,
fixed CMS batch scratch and remaining compiled-transform budget are accounted
together before conversion. Final output storage is reserved before compiling the
transform. The native buffer is reused rather than cloned; profile parse and batch
reservations are released after the transform drops. This is conservative admission
and owned-buffer accounting, not a codec-wide allocation or RSS guarantee.

Independent references combine C libjxl0.11.2 preview output with LittleCMS2.16
relative-colorimetric conversion. All eight orientations are tested on an ICC16
preview, with additional RGB8, GrayAlpha, larger-than-main and lossy previews. RGB
agrees within one8bit unit (two for lossy); alpha agrees exactly, including fully
transparent/opaque and intermediate alpha. The samples cover dark/high/saturated
coordinates and gamma2 versus sRGB. General wide-gamut/LUT profiles are not claimed
as newly qualified by this small fixture set. Raw output is rechecked unchanged.
Absence, all truncated ICC prefixes, damaged preview entropy, metadata/output limits
and exact combined workspace boundaries reject without partial output. The existing
main-image skip regression remains enabled. Eight extended preview tests and six
minimal tests pass, plus 44 library, 41 main, three layer and 11 resource tests.
The oriented managed preview was visually inspected. Rust1.91 release, no defaults,
jpeg-xl/color-management/image-buffer-ex, 20 warmups and three processes ×1000
oriented ICC16 preview calls took 79.428/81.488/79.678 ms. This is a new-API
microbenchmark, not a universal performance claim.

Stack: #77 → #76 → #75 → #73 → #72; #74 remains unapplied. #44 now has native and
managed preview pixels in development, but stays open: caller cancellation, broader
preview frame forms and full acceptance across the documented format envelope are
not completed. No dependency/limit relaxation or published-release claim is made.

### Development: cancellable preview and #44 acceptance

`preview_from_srgb_with_resource_limits_and_cancel` adds the same cooperative
`Fn() -> bool` probe and downcastable `DecodeCancelled` as main-image sRGB output.
The raw and noncancellable managed signatures are preserved. Checks cover entry,
bounded admission boundaries, codec runner work groups, CMS batches, each source
orientation row, metadata/output allocation boundaries and final publication.
Private preflight/ICC/TOC parsing and profile compilation remain bounded operations
with checks around them, not an interruptible hard-deadline mechanism. A cancelled
preview returns no ImageBuffer; partially converted native samples remain private.

The 32×32 preview exercises55 checkpoints locally. Every checkpoint was cancelled
individually and returned DecodeCancelled. After warming library state, a thread-local
System allocator wrapper measured zero net owned heap bytes after dropping each
error and after successful-result destruction. This verifies scoped storage cleanup,
not global process RSS or arbitrary parallel allocator behavior. Immediate abort
precedes malformed-input and absence handling; a false probe preserves pixels and
normal errors. Existing sequential CancellableRunner and CMS checks are reused.

Profile coverage now represents the actual SDR Gray/RGB device families: simple
RGB and Gray, embedded RGB gamma2 and embedded Gray gamma2. The added Gray ICC
preview is an independently C-validated fingerprint-specific bit splice of the
existing Gray16 fixture, with normalized16 C pixels and LittleCMS sRGB expectations.
It does not add a general serializer or loosen profile admission. HDR/float,
associated/subsampled alpha and spot channels remain explicit UnsupportedFeature
errors. Supported CMS profile parsing/transform errors remain errors. Gray ICC main
RGBA expansion still requires explicit CMS; existing managed main output with and
without preview remains identical.

The original #44 acceptance maps to the development stack as follows:

| Original condition | Evidence |
| --- | --- |
| Independent preview/main size, colour and alpha samples | #77 C preview vectors, including preview larger than main; main skip oracles |
| Absence, damage and truncation | #76/#77/#78 query, all-prefix and entropy refusal/skip regressions |
| Orientation and ICC | #78 all8 C+LittleCMS references; RGB/Gray ICC representative tests here |
| Separate/combined memory limits | #77/#78 output limits, simultaneous native/output/CMS reservations and exact workspace refusal |
| Main output oracle and existing users | Existing main/layer/resource oracles; raw and managed preview contracts remain explicit |

JPEG XL preview and main share the global colour/alpha description; the independent
fixtures differ in actual colour/alpha samples and geometry. The contract intentionally
admits only one terminal-visible preview and requires a main frame. Arbitrary frame
forms, all possible ICC constructions and removal of the old parser are not added as
new #44 completion conditions. Cancellation is consistency with referenced #31.
The original conditions are represented for this documented bounded envelope, but
all new retrieval APIs remain unmerged draft work; #44 remains open, not advertised
as released/main functionality. Stack: #78 → #77 → #76 → #75 → #73 → #72.

Local validation:11 extended preview tests,6 minimal,44 library,41 main,3 layer and
11 resource tests. Visual output for an oriented ICC16 preview was checked. Matched
Rust1.91 release microbenchmarks use20 warmups and1000 calls per process. Alternating
three processes each: #78 ordinary79.134/80.159/80.379ms versus current ordinary
79.591/79.026/87.163ms (median essentially unchanged; visible run variability). False
probe cancellation calls81.529/87.720/84.971ms versus paired prior79.907/80.275/
83.530ms show callback-check overhead; no universal speedup claim.

### Development: native original-profile F32 companion (#46)

`jpegxl::image_f32_from_with_resource_limits_and_cancel` returns owned `JxlF32Image`
with oriented RGBA `Vec<f32>`, coded/output/intrinsic dimensions, source orientation,
Gray/RGB coordinate count and separate source/output ICC. Gray coordinates are
replicated into R/G/B; straight alpha is separate. Only original-profile Modular
32-bit samples with8 exponent bits and matching full-resolution straight float
alpha are admitted in this first product step. RGB may be negative/above1 but must
be finite; alpha must be finite in0..=1. There is no integer normalization, clipping,
CMS or implicit conversion to EX/RGBA8. Storage F32 does not imply linear light: the
ICC describes the sample coordinates. Auxiliary container metadata is validated and
budgeted but this companion exports geometry/profiles, not an EXIF/metadata map.

PQ/HLG, nondefault tone metadata, F16/other float layouts, XYB/VarDCT, animation and
unhandled extras remain explicitly unsupported. Integer still/EX/sRGB entry points
continue to refuse float sources. This is native preservation, not completed HDR,
tone mapping, desired-intensity rendering or animation. It follows #68's positive
original-profile F32 finding; it does not repeat or merge that evaluation or pretend
the public core provides a qualified tone operator. #43/#67 require multi-frame
admission, timing/publication and collector accounting and were not the smallest
product step here.

A separately instantiated preflight mode retains input/ICC/TOC/padded-layer/reference
checks and caps; the integer mode is unchanged. The raw input remains borrowed for
assembly. Source/output profiles and16-byte-per-pixel F32 output are claimed before
allocation on top of conservative codec admission. Complete event sequence and
codestream length are checked, public basic info must match bounded preflight, and
output ICC is observed again after completion. No image is returned until every
sample passes the finite/alpha policy. Cancellation reuses the existing runner,
profile/admission boundaries and256-pixel validation batches. Owned storage is dropped
on cancellation or invalid samples; this is not a global codec allocator/RSS bound.

Seven tiny C libjxl0.11.2 cases cover RGB/RGBA/Gray/GrayAlpha, embedded RGB/Gray gamma2
ICC, orientation6, negative/above1, signed zero and minimum F32 subnormals. The
independent C decoder first proves byte-exact lossless source preservation without
orientation, then supplies oriented F32 expectations. Product results are byte-exact.
ORIGINAL and DATA ICC are queried separately; both embedded ICC pairs match the
provided profile bytes. Structured canonical profiles are generated by each decoder,
not claimed byte-identical across profile factories; product source/output identities
and Gray/RGB envelopes are checked. Five other C-generated inputs exercise F16,
PQ/HLG, NaN/Infinity and out-of-range alpha refusals. The latter two reach the explicit
finite-sample guard. All truncated ICC-input prefixes, output-byte exact boundary,
metadata/ICC/internal/TOC budgets, exact cumulative workspace and every one of18 local
cancellation checkpoints are tested. A shared warmed thread-local allocator tracker
shows zero scoped net owned bytes after errors, invalid samples and successful
destruction. Existing integer and preview regression tests remain enabled.

Reproduce fixtures with the #68-derived `jpegxl_f32_reference.c` (two test-only
adaptations: orientation6 generation and optional keep/oriented decode), compiled
against official libjxl0.11.2 headers/runtime, then
`python3 wml2-test/scripts/generate_jpegxl_native_f32.py --reference .test-reference --work .test-f32-fixtures`.
[Fixture hashes and independent header/profile records](jpeg-xl-native-f32-validation.json).
The diagnostic display compares independently decoded and product F32 pixels using
an explicitly test-only clamp; it is not a product SDR rendering API. Three Rust1.91
release processes ×1000 ICC F32 calls after20 warmups took60.783/63.945/60.945ms.
An unchanged preview workload alternated prior82.600/80.904/80.336ms and current
88.901/80.151/78.774ms; no consistent regression is inferred from these noisy small
measurements and no universal performance claim is made.

Stack: #79 → #78 → #77 → #76 → #75 → #73 → #72 (no functional preview dependency;
stacked on the latest shared0.8 admission/cancellation infrastructure). #74 is not
applied. #46 stays open for the remaining HDR/float/tone-map contract; this draft
is not merged/main/released support. No caps or product dependencies are raised.

### Development: F16 input with exact F32 output (#46)

`jpegxl::image_f16_as_f32_from_with_resource_limits_and_cancel` accepts original-profile
Modular16/5 float stills and returns the existing `JxlF32Image`. The storage is
**F32**, not F16: every finite IEEE half sample is exactly representable, including
±2^-24, signed zero and ±65504. The backend F16 output route is never requested;
#68's recorded half-subnormal loss in that output route is not claimed fixed.
The original F32-input API retains its32/8-only acceptance and same return fields.
No new pixel container is introduced. Source precision is explicit in the selected
entrypoint; common float decoding/preflight is privately specialized for16 or32.

Orientation/profiles/Gray-coordinate replication, unnormalized finite RGB, straight
alpha in0..=1, complete-input publication, cumulative resource/cancellation contracts
are the same as the native F32 companion. Matching16/5 alpha is required. Output
budgets still reserve16 bytes/pixel (not the8 bytes/pixel encoded half RGBA width).
PQ/HLG, tone metadata, VarDCT/XYB, animation and unsupported extras remain rejected.
Ordinary/EX/sRGB entry points do not begin accepting float sources. This is precise
source preservation through promoted storage, not HDR display or tone mapping.

Seven independent C libjxl0.11.2 fixtures cover Gray/RGB with/without alpha, RGB/Gray
ICC and orientation6. C F32 output with orientation kept must equal Python's exact
IEEE-half-to-F32 promotion of original source bytes. Oriented C output supplies
byte-exact product expectations. Both signs of the minimum half subnormal, signed
zero, maximum finite half and minimum-alpha samples are checked. ORIGINAL/DATA ICC
are queried separately; embedded profiles match the supplied bytes. Five refusal
cases include a32-bit source (still accepted by the existing F32 endpoint), PQ/HLG,
NaN/Infinity and out-of-range alpha. Finite-policy failures, every truncated ICC
prefix, output-byte/metadata/ICC/TOC/internal/workspace boundaries, all17 local
cancellation checkpoints and zero scoped owned-storage residue are tested.
Four F16 tests, four original F32 tests,11 preview tests and existing44 library/
41 main/3 layer/11 resource regressions pass in available native feature builds.

The existing WASM verifier now accepts `--f16`; a committed harness uses the same
independent vectors,5 refusals, both F32 compatibility directions, all cancellation
checkpoints and recovery. Local Node24.19.0 executes39 checks, no imports, and an
enforced32MiB ceiling; observed memory5,701,632 bytes. The original verifier's229
checks still pass. CI executes the F16 verifier as a separate step; compile-only
and execution results are distinguished in the PR.

Reproduction: use the existing `jpegxl_f32_reference.c` C helper with official
libjxl0.11.2 headers/runtime, then
`python3 wml2-test/scripts/generate_jpegxl_native_f16.py --reference .test-reference --work .test-f16-fixtures`.
Run `python3 wml2-test/scripts/verify_jpegxl_wasm.py --f16 --output .test-f16-wasm`.
[Source/fixture/profile hashes](jpeg-xl-native-f16-validation.json) record precision
and independently decoded headers. Diagnostic visualization explicitly clamps for
inspection only and matches the C reference; no product SDR rendering is added.

Release tiny ICC F16→F32 measurements (Rust1.91,20 warmups,1000 calls,3 processes)
were65.015/62.765/65.084ms. A pinned-CPU0 F32 compatibility benchmark rebuilt #80
and current source with identical release features,6 balanced alternating process
blocks; medians60.472/60.453ms (ratio0.99967). This is a small workload check, not
a universal performance guarantee. Stack: #80 → #79 → #78 → #77 → #76 → #75 →
#73 → #72. Product dependencies/caps are unchanged; #74 remains unapplied. #46
stays open for remaining float/HDR/tone-map work and unmerged implementation.

### Development: encoded PQ/HLG preservation (#46)

`image_hdr_encoded_from_with_resource_limits_and_cancel` returns `JxlEncodedHdrImage`
with oriented normalized F32 **code values** in `rgba_encoded` and `JxlHdrSignal`.
This step accepts structured original-profile Modular integer16 RGB, Rec.2020/D65,
PQ or HLG, optionally matching full-resolution straight integer16 alpha. Input
integer code values use full-range normalization q/65535. RGB is not linear light
or luminance; alpha is linear opacity in0..=1 and never receives a transfer/gamut
operation. Primaries/white, transfer, rendering intent and JPEG XL source fields
intensity_target/min_nits/relative_to_max_display/linear_below are preserved.
Public source/output descriptors must match bounded preflight and are observed
again after decoding; source tone metadata is cross-checked against BasicInfo.

PQ and HLG are distinct enum values. PQ's EOTF is absolute-luminance signaling
(code1 corresponds to10000cd/m² for its equivalent neutral component); HLG's
scene-relative signal needs declared display peak/black/system-gamma conditions
for a display EOTF/OOTF. This endpoint applies neither, chooses no HLG viewing
condition, and does not use intensity_target as a new normalization divisor.
No linearization, tone mapping, gamut conversion or SDR display result is implied.
See [Khronos Data Format §13.5–13.6](https://registry.khronos.org/DataFormat/specs/1.4/dataformat.1.4.inline.html)
and [ITU-R BT.2100](https://www.itu.int/rec/R-REC-BT.2100). A later renderer would
need a separate, explicit interpretation/condition contract; this preservation
step needs no speculative user display choice.

Structured JXL source encoding has no literal embedded ICC bytes. The pinned0.8
HDR ICC factory can synthesize tone-mapped LUT profiles (api/color.rs HDR factory
and tests); these are not fabricated as unprocessed original-profile bytes. The
endpoint returns the actual structured signal instead and never calls that factory.
Embedded-ICC HDR, P3/custom primaries/white, float HDR, associated/unhandled alpha
and other existing unqualified forms return errors. SDR, F32/F16 and main/EX/sRGB
entrypoint contracts remain unchanged; no alternate silent success is introduced.

The integer admission mode is separately specialized for HDR, preserving all input,
box/metadata, pixel, padded/reference/layer/TOC and byte caps. Structured signal and
16-byte-per-pixel output are cumulatively admitted without ICC-generation storage.
RGB/alpha must be finite normalized coordinates; no clamping is used to repair
invalid samples. Complete event sequence/length, final signal descriptor, typed
cooperative cancellation and private-until-success publication remain. The scoped
allocation tracker reports zero owned-storage residue at all12 local checkpoints.
These are conservative admission/adapter guarantees, not global codec heap/RSS.

Six independent C libjxl0.11.2 references cover PQ/HLG RGB/RGBA,0/1 and fractional
codes/alpha, default and nondefault source luminance fields and orientation6. C
queries ORIGINAL and DATA structured descriptions separately, and its kept code
coordinates agree with input uint16/65535 within2^-24. Product F32 arrays
are byte-exact to C. Identical code arrays tagged PQ versus HLG remain identical
numbers but have distinct signals. P3, embedded HDR ICC and associated-alpha inputs
are C-decodable refusal fixtures. Fingerprinted zero/infinite intensity-header
mutations fail both C and product; no valid malformed source is claimed. Every
truncated prefix, output/metadata/workspace boundaries and source-type refusals
are tested. Existing native/preview/main oracle tests retain their behavior.

Reproduce with the #68-derived `jpegxl_hdr_encoded_reference.c` (test-only tone/
structured-DATA reporting additions) and
`python3 wml2-test/scripts/generate_jpegxl_encoded_hdr.py --reference .test-reference --work .test-hdr-fixtures`.
[Independent hashes and metadata](jpeg-xl-encoded-hdr-validation.json). The committed
`verify_jpegxl_wasm.py --hdr-encoded --output .test-hdr-wasm` harness executes47
checks/12 cancellation positions locally on Node24.19.0, no imports,1,507,328 bytes
of module memory and an enforced32MiB ceiling. Existing39 F16 and229 standard WASM
checks also pass. Native3 HDR,4 F16,4 F32,11 preview,44lib,41 main,3 layer and11
resource tests pass in available feature builds. Diagnostic display compares code
values only and is explicitly not an HDR/gamut renderer.

Rust1.91 release tiny HDR preservation (20 warmups,3 processes ×1000 calls) takes
46.784/46.325/45.151ms. Existing F32 pinned-CPU0 six balanced process blocks compare
#81/current medians60.959/61.415ms (ratio1.00749, within measured process variability);
no universal performance claim follows. Stack: #81 → #80 → #79 → #78 → #77 → #76
→ #75 → #73 → #72; #74 remains unapplied. Dependencies/caps are unchanged. #46
stays open for remaining HDR/float/linearization/display work; this is unmerged
implementation, not released/main support.

### Explicit HDR transfer linearization (unmerged follow-up to #82)

`JxlEncodedHdrImage::into_linear_with_resource_limits_and_cancel` consumes the
encoded image, transforms RGB in place and returns `JxlLinearHdrImage`. Callers
must choose `JxlHdrLinearization::PqDisplayNits` or `HlgSceneRelative`; a source
transfer mismatch is an error. Geometry is already oriented; this step does not
reorient it. Straight alpha bits, including negative zero, are preserved.

PQ uses the ST2084 EOTF, with code1 yielding10000 nits. HLG uses the normalized
inverse reference OETF, with code0.5 yielding1/12 in scene-relative units.
These are distinct domains. The HLG constants are the published rounded
`a=0.17883277`, `b=0.28466892`, `c=0.55991073`. Calculations use f64 intermediates
and round to F32; extremely small caller-supplied values can underflow. Source
luminance hints do not rescale these functions. See the [Khronos Data Format
specification sections13.5.2 and13.6.1](https://registry.khronos.org/DataFormat/specs/1.4/dataformat.1.4.inline.html).

Output RGB retains Rec.2020/D65 coordinates. `source_signal` is the encoded
source descriptor, not an output ICC. No source ICC is fabricated or attached to
linear values. This operator does not reconstruct camera exposure or perform an
HLG display OOTF, gamut mapping, exposure selection, SDR encoding or tone mapping.

The public, mutable input carrier is checked again: positive/consistent oriented
geometry, sample length, supported source signal, finite normalized RGB and
alpha. Invalid samples are refused, never repaired by clipping. Storage is
consumed on error or cancellation, so no partial result can escape. Checks occur
at entry, after admission, before each256-pixel batch and before publication.

Conversion admission counts actual vector capacity ×4, the larger input/output
inline object, and128 bytes scalar allowance against the tightened workspace.
Logical output bytes, pixels, one frame and source-signal metadata have independent
limits. There is no second image or CMS scratch allocation. Decoding has already
finished: input bytes, internal padding, TOC and reference-layer limits remain
part of the separate decode operation. This ledger bounds accounted owned storage,
not all process heap/stack/RSS, codec allocation or a caller's other live images.
Existing decode caps and all encoded/native/ordinary APIs remain unchanged.

The [70-digit Decimal oracle](jpeg-xl-hdr-linear-oracle.json) is generated from
independent C libjxl encoded F32 vectors, never candidate linear output. Regenerate
with `python3 wml2-test/scripts/generate_jpegxl_hdr_linear.py`. Tests cover six
PQ/HLG RGB/RGBA/oriented vectors within2 F32 ULP, anchors, the HLG breakpoint,
unchanged alpha bits, metadata hints, bad public carriers, capacity slack and exact
one-byte workspace boundaries. Cancellation is injected at all six checkpoints of
a600-pixel conversion, with no scoped net heap left after consumed storage drops.

Issue#46 remains open. Native F32, F16-to-F32, structured integer16 HDR codes and
transfer linearization are implemented only on the unmerged stack. SDR display
rendering still needs an explicit policy for reference white/exposure, tone curve
and gamut handling; HLG display rendering additionally needs peak/black luminance
and system-gamma conditions. Neither the source intensity hint nor an inverse OETF
selects those conditions. Embedded-ICC/custom-primary/float HDR support remains
outside the current input envelope. No new completion conditions are added.

[Local validation](jpeg-xl-hdr-linear-validation.json) records actual Node24 WASM
execution:69 checks,18 combined decode/conversion probes, no host imports and an
enforced32 MiB memory ceiling. Existing ordinary WASM229 checks pass. Native
minimal/expanded/disabled feature targets pass; CI also runs the new target on
macOS, Windows and actual Windows32. A diagnostic-only normalized tile display
matched the Decimal reference; it does not qualify an SDR renderer.

Matched CPU0 release benchmarks use six alternating pairs of2000 encoded decodes
with20 warmups: before median91.774ms, after89.707ms, ratio0.9775. Ranges overlap
(89.858–95.013ms before,87.532–95.752ms after); this establishes no measured
regression in this test, not a speedup claim. Three runs of1000 in-place256×256
conversions take PQ6.322–6.642s and HLG0.333–0.340s, excluding decode/input
allocation and including output drop. The different transfer costs are explicit.


Associated8/9/10/12/16-bit and mixed-depth original-profile SDR alpha now has an
[explicitly bounded straight-output subset](jpeg-xl-associated-depths.md) on the unmerged
stack. This partially addresses #42; [declared straight/associated subsampled alpha](jpeg-xl-associated-subsampling.md)
also has a bounded subset. An explicit [separate integer Depth-plane API](jpeg-xl-depth-plane.md)
now preserves one qualified non-alpha extra alongside original-profile color on this draft stack.
Other kinds, multiple/combined extras and name-based selection remain unsupported.
It does not change the deferred HDR display policy in #46.
