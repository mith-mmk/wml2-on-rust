# WML2 high-bit-depth colour pipeline

This is the implementation contract for typed high-bit-depth buffers, native
AVIF still images, and explicit Gray/RGB ICC conversion. It is not a release
announcement.

## Compatibility and boundaries

The historical `draw::ImageBuffer`, `DrawCallback`, and `PickCallback` remain
RGBA8 APIs. New typed APIs are additive, opt-in, and never selected implicitly
by an old decode or draw call. Existing codec behavior and callback/Abort
contracts remain unchanged.

The initial codec consumer is AVIF still-image native decode. The initial CMS
consumer is Gray/RGB. Generic HDR presentation, PQ/HLG processing, tone
mapping, display headroom, a complete colour framework, ownership ledgers, and
high-precision animated-image transactions are outside this contract.

## Typed representation

The public vocabulary is:

`ImageFrame`, `ImageDescriptor`, `PixelBuffer::{U8,U16,F32}`, `Plane<T>`,
`PlaneDescriptor`, `PlaneLayout`, `ChannelModel::{Gray,RGB,YCbCr}`,
`ChannelRole`, `AlphaAssociation`, and `Subsampling`.

U8 stores 8-bit samples. U16 stores 10-, 12-, and 16-bit integer samples;
`meaningful_bits` records effective precision independently of storage width.
F32 is processing precision for explicit conversions and is not an HDR-only
type. Strides and channel offsets are sample counts, never byte counts.

Planar and interleaved layouts are checked independently. Dimensions,
subsampled ceiling sizes, addressed samples, padding, channel roles, alpha
association, meaningful-bit ranges, and finite F32 values are validated before
processing. Native YCbCr remains YCbCr; no matrix, range, transfer, or
subsampling operation is inferred by constructing a frame.

## Feature matrix

| Feature set | Contract |
| --- | --- |
| no optional feature | Existing WML2 API only |
| `high-bit-depth` | Typed buffers and native metadata, no CMS dependency |
| `color-management` | `high-bit-depth` plus explicit ICC adapter |
| `avif,high-bit-depth` | Native AVIF still-image adapter |
| `avifenc,high-bit-depth` | Typed AVIF encoding adapter at checkpoint 5 |

The old combined high-resolution feature is not used. AVIF and its independent
codec crates do not gain an ICC dependency merely because WML2 has typed
buffers.

## AVIF native data

Existing ordinary native AVIF plane decode is mapped directly into `ImageFrame`:

- 8-bit data may use U8 or U16; 10/12-bit data uses U16.
- Sample Transform output at 16 bits or below uses the bounded AVIF entry
  point; direct AV1 inputs stay on the bounded native path and unsupported
  derived graphs fail explicitly. No unbounded fallback is used by WML2.
- Integer sample values are copied without requantization.
- Y, Cb, and Cr planes retain range, matrix, subsampling, and related decode
  signalling as metadata.
- Alpha is a separate channel and is not an ICC input.
- ICC, nclx/CICP, and AV1 colour descriptions coexist as input metadata.
- `clap`, `irot`, and `imir` are retained as metadata and are not applied to
  native pixels.

Legacy RGBA8 decoding remains the compatibility path. High-precision AVIS
sequence reading is a later thin reuse of the still-frame conversion function,
not part of the initial completion gate.

## Explicit ICC conversion

WML2 does not reimplement ICC parsing. The independent `icc-profile` crate
owns the CMS and exposes an F32-centered transform. The supported initial path
is:

`ImageFrame -> RGB/Gray F32 -> icc-profile::Transform -> ImageFrame`

The caller supplies both source and destination profiles. Coexisting ICC and
nclx data never causes an automatic ICC/CICP/ICC route. Images without ICC do
not receive an ICC synthesized from CICP in this phase.

The CMS covers ICC v2/v4 Gray/RGB profiles, D50 PCS, XYZ/Lab, chad, TRCs,
`curv`, `para` types 0-4, `mft1`, `mft2`, `mAB`, `mBA`, A2B/B2A, four rendering
intents, optional stages, 1D interpolation, and tetrahedral 3D interpolation.
U8/U16 are quantization wrappers around the F32 core. Alpha is excluded.
Unsupported CMYK, N-color, MPE, BPC, and unsupported float-profile operations
return an explicit unsupported-feature error. No approximate fallback is
permitted.

## Encoding boundary

Typed AVIF encoding accepts U8/U16 input only for explicit coded depths 8, 10,
and 12. It never infers output depth from the largest input sample and never
encodes a normal AV1 16-bit stream. Reducing U16/F32 requires an explicit
quantization request. Pixel conversion and ICC/nclx metadata replacement are
separate operations. New Sample Transform 16-bit encoding is excluded.

## Safety and development

External AVIF and ICC input is checked for integer overflow, invalid
dimensions, sample/plane sizes, strides, offsets, tag ranges, curve counts,
CLUT sizes, and unsupported representations before use. Checked arithmetic,
fallible reservation, Miri, and fuzzing cover the relevant boundaries.

This work does not require byte-accurate tracking of every internal allocation
or proof of peak live allocation. Repository commits remain separated by
typed WML2 buffers, AVIF decoder integration, independent ICC CMS, WML2 ICC
adapter, and AVIF encoder integration. Publication and main integration are
separate approval-gated work.
