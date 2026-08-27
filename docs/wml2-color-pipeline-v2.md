# WML2 highres color pipeline

Status: approved implementation contract; implementation and verification are
tracked separately in [highres-checklist.md](highres-checklist.md). This is not a
release announcement. It replaces the earlier JXL-oriented color-pipeline draft.
JXL work remains stopped.

## Scope and compatibility

Add opt-in native/high-precision AVIF still reading and writing, stateful AVIS
reading, and explicit U16/F32 color conversion. Reuse the independent AVIF and
AVIF encoder crates. Initial CMS support is Gray/RGB. PQ and HLG conversion to
F32 is included; gain-map composition, tone mapping, viewer/OS HDR presentation,
AVIS encoding, CMYK/N-color/MPE processing, and other codec integration are not.
Existing 16-bit `sato` decoding is preserved and regression-tested, not expanded
into a new 16-bit AV1 or F32 file-encoding claim.

All existing public types, required fields, functions, `DrawCallback`,
`PickCallback`, output pixels, callback ordering, and abort behavior remain
unchanged with `highres` either disabled or enabled. In particular, this work
does not redefine the historical RGBA8 conversion as a new strict conversion.
New checked wrappers and additive codec entry points carry richer information;
do not add required fields to existing public structs. Do not add new drawing
or picking callback traits in this phase.

## Feature contract

| WML2 features | Additional API |
| --- | --- |
| no `highres` | Existing API only; no new CMS normal dependency |
| `highres` | `wml2::highres`: typed buffers and explicit ICC/CICP conversion |
| `avif,highres` | Native AVIF still decode and stateful AVIS read |
| `avifenc,highres` | Native/typed AVIF still encode; existing `avifenc` also enables `avif` |

`highres` is default-off and is not inserted into any existing aggregate
feature. It enables only the new optional WML2 ICC dependency, not AVIF by
itself. Gate the entire new public namespace with `cfg(feature = "highres")`
and codec entry points additionally with their existing codec features. The
existing meanings of `avif`, `avifenc`, and `multithread` are unchanged.
Independent `avif-rust`/`avifenc-rust` gain no CMS dependency or MSRV increase.
`--all-features` is an explicit opt-in, not a feature-off check.

## Typed API and native data

The public vocabulary is `ImageFrame`, `ImageDescriptor`, and
`PixelBuffer::{U8, U16, F32}`, under `wml2::highres`. Do not replace these with a
second competing `ImageBuffer` or untyped byte-buffer API. New structs use
checked constructors and accessors; validation is repeated at processing
boundaries if mutable samples can invalidate a previously valid frame.

- `ImageDescriptor` describes dimensions, explicit Gray/RGB/YCbCr channel roles,
  color information, alpha association, plane layout, and applicable geometry.
- `PixelBuffer` stores typed samples, not raw bytes with a reinterpreted format.
  Integer meaningful precision is independent of U8/U16 storage width. F32
  denotes processing precision and never an AV1 coded bit depth.
- `ImageFrame` owns its descriptor, one or more typed pixel buffers, metadata,
  and optional sequence timing. Planar storage is canonical for native AVIF;
  interleaved views require homogeneous sample storage and spatial dimensions.
- Each plane records width, height, row/pixel stride, channel offsets and
  independent horizontal/vertical subsampling. Use explicitly named sample-unit
  strides/offsets; do not mix byte counts and sample counts. If any byte-based
  view is exposed, its units and alignment checks must be separate and explicit.
- Validate the last addressed sample with checked arithmetic, not only
  `width * height`. Permit positive padding, reject negative strides, duplicate
  or missing roles, overlapping channel offsets, invalid dimensions, and
  unsupported layouts. Odd-sized subsampled planes use ceiling dimensions.
- Validate every integer sample against its meaningful-bit range. Reject NaN
  and infinity. Linear F32 RGB may be negative or greater than one; normalized
  alpha remains in `[0, 1]`. Unknown colorimetry is not silently sRGB.

Native decode is the default of the new API: no color conversion, display
mapping, precision reduction, unpremultiplication, or implicit geometry change.
Keep coded and display geometry distinct, including applicable crop, rotation,
mirror and pixel aspect metadata. An explicit geometry operation must declare
its subsampling/resampling consequences. Do not implement native decoding by
converting the existing RGBA8/RGBA16 display output back to wider storage.

## Color metadata and alpha

Rich color metadata retains embedded ICC bytes, container `nclx`, and AV1 color
signaling simultaneously, with their provenance. They are not mutually
exclusive enum alternatives. Keep unknown signaling available for preservation;
an unsupported conversion fails explicitly. Reject contradictory or malformed
signaling according to the applicable format rules. The old single
`color_information` field retains its historical projection.

Separate original/source metadata from the active interpretation of converted
pixels: an explicit conversion updates the output descriptor and records what
was applied. Never attach an original ICC profile to newly converted sRGB as
though it described those output samples. ICC byte preservation alone does not
imply that a profile is supported by the CMS.

The generic typed representation can describe independent plane precision, but
AVIF has stricter rules. For `av01` master/alpha items and image sequences,
require equal coded bit depth. Auxiliary AV1 data must be monochrome and
full-range. Ignore alpha `colr` for color processing. A new strict decoder
rejects a mismatched master/alpha depth rather than normalizing it. New encoding
requires matching depth; any preceding alpha quantization is explicit and is
not source-lossless. Derived `sato`/grid arrangements need their own validated
rules; do not apply the `av01` master rule blindly to reconstructed output depth.
See [AVIF 1.2 section 4.1](https://aomediacodec.github.io/av1-avif/v1.2.0.html#auxiliary-image-items-and-sequences).

Alpha is never an ICC color channel. Preserve straight/premultiplied association
and zero-alpha hidden samples in native storage. A requested association change
must specify the sample domain and zero-alpha policy; reject cases that cannot
meet the requested preservation contract. Do not introduce an implicit alpha
operation while performing a nonlinear color conversion.

## Explicit color conversion

Keep the following stages separate:

1. Native range handling, chroma reconstruction, and YCbCr matrix conversion to
   source RGB. Preserve chroma position and matrix/range semantics.
2. Choose exactly one source color interpretation: supported ICC, or CICP
   transfer/primaries. The presence of ICC does not suppress the YCbCr matrix
   stage, and a CICP transfer must not be applied before the ICC TRC.
3. Convert to an explicitly described destination representation.
4. Perform only requested, validated quantization or alpha/layout operations.

Options declare source/destination interpretation, rendering intent, output
precision and quantization policy. Unsupported options are errors, not no-ops.
Native preservation does not invoke the CMS. ICC transforms operate on their
specified normalized/device/PCS domains; do not feed nit-valued HDR samples
directly into a normalized ICC transform or call a clamping U16 helper first.
Out-of-domain and out-of-gamut behavior is explicit; the default refuses a
conversion that needs unrequested clipping. No HDR-to-SDR conversion or tone
mapping is inferred from an integer output request.

PQ output is absolute linear F32, where `1.0` means one nit (`cd/m2`), with
declared RGB primaries. HLG scene-linear output is a distinct relative
representation, not nit-valued RGB. A requested HLG display-referred conversion
requires its display peak/black/system-gamma conditions. Reject missing or
non-finite parameters. Keep the transform reversible within its declared
mathematical domain; a display shoulder or fixed sRGB clamp is outside scope.
Use [ITU-R BT.2100](https://www.itu.int/rec/R-REC-BT.2100) for these conventions.

## ICC responsibility

`icc-profile` remains an independent crate and preserves its reader API. WML2
uses its shared CMS only inside `highres`; legacy AVIF's existing ICC evaluator
is not replaced in this phase. A future replacement requires feature parity,
output-regression evidence, and a separately reviewed compatibility checkpoint.

The initial shared CMS covers Gray/RGB matrix/TRC, `curv`, `para` types 0-4,
`mft1`/`mft2`, `mAB`/`mBA`, XYZ/Lab D50 PCS and `chad`, plus four rendering
intents with correct profile direction and tag selection. Apply the normative
optional-stage ordering and endpoint interpolation. Unsupported BPC, CMYK,
N-color, MPE and float-profile paths return an explicit unsupported-feature
error. No RGB/CMYK approximation substitutes for an unsupported profile.
Source-to-PCS and PCS-to-destination paths are distinct; a reverse-only LUT is
not a forward source transform. Forward curve evaluation must not require a
mathematically invalid inverse. Reference: [ICC.1:2022](https://www.color.org/specifications/ICC.1-2022-05.pdf).

## Encoding contract

New AVIF encoding takes explicit 8/10/12-bit precision, never a depth inferred
from the largest sample. Native 400/420/422/444 input retains its channel
semantics. Native alpha is passed separately to the backend as needed, but its
AVIF bit-depth constraints still apply. RGB input requires a declared target
matrix, range and chroma layout; generate those native planes instead of
relabeling the existing fixed-matrix RGBA16 converter with new metadata.

Lossless means decoded native source samples, alpha, and supported semantic
metadata are unchanged. This includes color interpretation and applicable
geometry, not byte-identical container layout or identical AV1 entropy syntax.
ICC bytes remain byte-identical when retained. Requantization, ICC conversion,
new chroma subsampling, clipping and alpha normalization cannot be reported as
source-lossless. Existing native 420/422 samples can be losslessly recoded
without further subsampling. Reject an encode request if required metadata or
item semantics cannot be preserved; do not silently discard unsupported
gain-map/derived-item content under a preservation claim.

F32 color processing does not promise F32 AVIF storage. Explicitly quantized
8/10/12-bit output is a distinct operation and is tested against the quantized
target separately from source-lossless tests.

## Sequence, safety and errors

The new stateful AVIS reader reuses forward decoder state and returns native
`ImageFrame` values with exact rational timing and repetition information.
Keep color/alpha synchronization transactional: failure must not advance one
track or return successful partial pixels. End-of-sequence is distinct from
truncation. Do not reinterpret frame timing as a new callback protocol.

Resource limits cover dimensions, channels, plane/frame bytes, total live
decoded bytes, reference frames, frame count, ICC bytes, metadata bytes, and CMS
CLUT allocations. Check before allocation where possible and use checked
arithmetic/fallible allocation. Count sequence reference state, not just the
returned frame. Errors distinguish malformed input, invalid layout, resource
limit, unsupported representation/profile/option, and failed conversion.
Untrusted input must not panic, OOM through unchecked sizes, or succeed after a
truncation/unsupported-stage failure. Tests live outside product source files.

## Development dependencies and local validation

Do not change versions or push/publish as part of this implementation scope.
During development WML2 may use this optional, immutable public checkpoint:

```toml
icc_profile = { package = "icc-profile", version = "0.0.4", git = "https://github.com/mith-mmk/icc-profile.git", rev = "c96f6e3899e884eb874d39357f601074e3888277", optional = true }
# In [features]:
highres = ["dep:icc_profile"]
```

Local CMS development overrides that exact Git source with a `[patch]` in an
ignored `.test*` configuration file, supplied explicitly with Cargo `--config`.
The local manifest version must satisfy the dependency requirement. Actual
worktree paths belong only in that ignored file, never in tracked manifests,
docs, example commands or lockfiles. No tracked dependency points at an absent
ignored directory. Cargo supports local configuration patches; see
[Overriding Dependencies](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section).

Check `cargo metadata` to prove that patch-enabled tests used the intended local
CMS. Separately test a fresh default/feature-off consumer without any local
configuration. Local-only CMS additions cannot be called fully integrated in a
fresh `highres` consumer until the dependency has been synchronized to an
authorized, reachable tested source. That synchronization and registry/package
publication are separate pending gates, not authorization to push or publish.
The existing `wml2-test` ICC dependency is not silently redirected to the CMS.

All validation targets, generated consumers, oracle outputs and reports are
under ignored `.test*` directories outside OneDrive when possible. Harnesses
accept runtime paths (for example `--ffmpeg`, `--lcms-library`, `--corpus`,
`--report-dir`) and never hard-code a machine path. Missing required oracle or
corpus inputs fail the relevant acceptance run rather than producing a passing
skip. Reports record tool version/hash, source revision and dirty status,
feature set, target/toolchain, input hashes and comparison metrics. Those local
reports are ignored; permanent user samples are not deleted during cleanup.
