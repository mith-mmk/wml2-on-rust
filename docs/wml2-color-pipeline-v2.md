# WML2 color pipeline v2 API specification

Status: design only. This document reserves the API shape and behavioral
contract for a future WML2 release. It does not change the existing RGBA8
callbacks in WML2 0.0.29.

## Goals

The v2 API transports decoded and encoded samples without silently reducing
precision, color channels, profiles, extra channels, or animation semantics.
It supports U8, U16, and F32 samples; Gray, RGB, and CMYK color models; ICC and
CICP descriptions; planar and interleaved storage; explicit row and plane
strides; alpha and arbitrary extra channels; and animated frame rectangles.

The existing `DrawCallback`, `PickCallback`, and `ImageBuffer` remain source
compatible. They are defined as an explicit sRGB, straight-alpha, interleaved
RGBA8 compatibility path. A decoder may use that path only when it can produce
that representation through a declared color conversion. It must return an
unsupported-representation error instead of truncating HDR, wide-gamut,
high-precision, CMYK, spot-color, or unknown-profile data.

## Proposed public module

The API will live under `wml2::color_pipeline` and will initially be gated by a
default-off `color-pipeline-v2` feature. Codec integrations may expose a second
feature that forwards to it. Public types are non-exhaustive unless explicitly
stated otherwise.

```rust
#[non_exhaustive]
pub enum SampleType {
    U8,
    U16,
    F32,
}

#[non_exhaustive]
pub enum ColorModel {
    Gray,
    Rgb,
    Cmyk,
}

#[non_exhaustive]
pub enum ColorEncoding {
    Icc(Arc<[u8]>),
    Cicp(CicpColorEncoding),
    Unspecified,
}

pub struct CicpColorEncoding {
    pub color_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
    pub video_full_range_flag: bool,
}

#[non_exhaustive]
pub enum AlphaMode {
    Straight,
    Premultiplied,
}

#[non_exhaustive]
pub enum ChannelRole {
    Color { index: u8 },
    Alpha {
        mode: AlphaMode,
        associated_color_channels: Vec<u8>,
        primary: bool,
    },
    Depth,
    Spot { name: Option<String> },
    SelectionMask { name: Option<String> },
    Cfa { index: u8 },
    Thermal,
    Optional { name: Option<String> },
}

pub struct ChannelDescription {
    pub role: ChannelRole,
    pub sample_type: SampleType,
    pub bits_per_sample: u8,
    pub exponent_bits_per_sample: u8,
    pub dim_shift: u8,
}

pub struct ImageDescription {
    pub width: usize,
    pub height: usize,
    pub color_model: ColorModel,
    pub color_encoding: ColorEncoding,
    pub channels: Vec<ChannelDescription>,
    pub orientation: u8,
    pub animation: Option<AnimationDescription>,
}
```

`bits_per_sample` is the number of meaningful integer bits or float mantissa
precision reported by the format. `exponent_bits_per_sample` is zero for
integer samples. Storage remains one of U8, U16, or F32 even when the encoded
precision is smaller. The API never infers a color space from channel count.

ICC byte strings are retained byte-for-byte. Their transfer curves are defined
only by the profile. CICP values use the numeric values defined by ISO/IEC
23091-2, including `transfer_characteristics`; there is no second transfer
field that can override them. `Unspecified` means exactly that and does not
imply sRGB. Unsupported or internally inconsistent CICP tuples are rejected.

Gray, RGB, and CMYK require exactly one, three, and four unique core
`Color { index }` channels respectively, numbered from zero. CMYK index 3 is K;
black is not represented by a second role. One alpha channel may be marked
`primary`; any additional alpha channels are extra channels and must state
their associated core color-channel indices. A premultiplied alpha channel
must name a non-empty associated set. An absent alpha role means the image has
no alpha; that state is not stored as a channel mode.

## Pixel storage

```rust
pub enum TypedSamples<'a> {
    U8(&'a [u8]),
    U16(&'a [u16]),
    F32(&'a [f32]),
}

pub struct Plane<'a> {
    pub channel: usize,
    pub offset_bytes: usize,
    pub row_stride_bytes: usize,
    pub width: usize,
    pub height: usize,
    pub data: TypedSamples<'a>,
}

pub enum PixelStorage<'a> {
    Interleaved {
        channel_order: Vec<usize>,
        pixel_stride_bytes: usize,
        row_stride_bytes: usize,
        data: TypedSamples<'a>,
    },
    Planar {
        planes: Vec<Plane<'a>>,
    },
}

pub struct PixelRegion<'a> {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub storage: PixelStorage<'a>,
}
```

All offsets and strides are in bytes, including U16 and F32 buffers. U16 and
F32 offsets and strides must be aligned to `size_of::<T>()`; a typed slice has
exactly `len * size_of::<T>()` addressable bytes under checked multiplication.
Interleaved channel offsets are `channel_position * size_of::<T>()`, and the
pixel stride must cover every listed channel. Each callback validates every
multiplication and addition used to address a row or plane. A buffer must cover
the last addressed sample, but may contain padding. Negative strides are not
supported in v2.

Channels with `dim_shift > 0` use their declared plane dimensions and are not
implicitly upsampled. Region coordinates are frame-local for animation and
canvas-local for still images. Subsample mapping is always anchored at the
canvas origin: for animation, add the frame origin before applying floor
division to the start and ceiling division to the exclusive end by
`2^dim_shift`. The resulting plane dimensions must match `Plane`.

An interleaved region contains channels with the same sample type and spatial
dimensions. Mixed sample types or subsampled extra channels use planar layout,
where each plane owns its sample type. Channel order is explicit and is never
assumed to be RGBA. Owned storage mirrors these forms with `Vec<u8>`,
`Vec<u16>`, or `Vec<f32>` per interleaved buffer or plane.

Integer color and alpha samples use the full normalized range of their declared
meaningful bits. F32 color samples may be negative or greater than one for HDR
and scene-referred data, but NaN and infinity are invalid. Straight and
premultiplied alpha samples must be finite and in `[0, 1]`.

## Animation and callback control

```rust
pub struct AnimationDescription {
    pub timebase_numerator: u32,
    pub timebase_denominator: u32,
    pub loop_count: u32,
}

pub struct FrameDescription {
    pub frame_index: usize,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub duration_ticks: u32,
    pub blend: FrameBlend,
    pub dispose: FrameDispose,
    pub is_keyframe: bool,
}

pub struct PassDescription {
    pub frame_index: usize,
    pub pass_index: u32,
    pub pass_count: u32,
    pub is_last_pass: bool,
}

pub enum CallbackControl {
    Continue,
    Abort,
}

pub type CallbackResult = Result<CallbackControl, Error>;
```

The timebase numerator and denominator are both non-zero. `loop_count == 0`
means infinite looping; positive values are the total number of plays. Frame
rectangles must fit the canvas. `duration_ticks == 0` means immediate
advancement after the frame is complete. `pass_count` is non-zero,
`pass_index < pass_count`, and
`is_last_pass` is true exactly for the final pass. Blend and disposal are
applied by the consumer; regions use frame-local coordinates and must fit the
frame rectangle. A codec does not call a callback concurrently or reentrantly.

## Decode callback

```rust
pub trait DrawCallbackV2: Send {
    fn init_v2(
        &mut self,
        image: &ImageDescription,
        options: Option<&InitOptionsV2>,
    ) -> CallbackResult;

    fn next_v2(&mut self, frame: &FrameDescription) -> CallbackResult;

    fn begin_pass_v2(&mut self, pass: &PassDescription) -> CallbackResult;

    fn draw_v2(&mut self, region: PixelRegion<'_>) -> CallbackResult;

    fn set_metadata_v2(&mut self, key: &str, value: DataMap) -> CallbackResult;

    fn terminate_v2(&mut self, result: &DecodeResultV2) -> CallbackResult;
}
```

For a non-empty still image the required order is
`init_v2 -> begin_pass_v2 -> draw_v2+ -> terminate_v2`; its logical
`frame_index` is zero. Each additional progressive pass begins with another
`begin_pass_v2` call using the same frame index. For animation it is
`init_v2 -> next_v2 -> begin_pass_v2 -> draw_v2*` for each frame, followed by
exactly one `terminate_v2`. Additional passes repeat only
`begin_pass_v2 -> draw_v2*`; they do not repeat `next_v2`. `next_v2` carries the
frame rectangle, duration, blend mode, disposal mode, and key-frame flag. A
codec must not emit pixels for a later frame before the current frame boundary.

Metadata calls are allowed only after a successful `init_v2` and before
`terminate_v2`. If `init_v2` returns `Abort` or an error, no termination call is
made because initialization never completed. After any later `Abort`, the codec
makes no more `draw_v2`, `next_v2`, `begin_pass_v2`, or metadata calls and
attempts exactly one `terminate_v2` call marked as aborted. Failure of that
best-effort termination is attached as secondary context and does not replace
the original abort or decode error. Any callback error after successful
initialization follows the same stop-and-terminate rule. An abort is never
reported as a successful partial decode.

Regions for a pass may arrive in any order and may overlap; callbacks are
serial. Later data for the same pass replaces the overlapping samples. A codec
must complete or terminate one frame before starting another.

Progressive passes for one frame use monotonically increasing pass indices in
`PassDescription`. The consumer may replace the affected rectangle, but the
codec must not represent a progressive pass as a new animation frame.

## Encode source

```rust
pub trait PickCallbackV2: Send {
    fn encode_start_v2(
        &mut self,
        options: Option<&EncoderOptionsV2>,
    ) -> Result<ImageDescription, Error>;

    fn frame_v2(&mut self, index: usize) -> Result<Option<FrameDescription>, Error>;

    fn encode_pick_v2(
        &mut self,
        request: &PixelRequest,
    ) -> Result<OwnedPixelRegion, Error>;

    fn metadata_v2(&mut self) -> Result<Option<Metadata>, Error>;

    fn encode_end_v2(&mut self, result: &EncodeResultV2) -> Result<(), Error>;
}
```

`PixelRequest` declares the requested rectangle, channel indices, sample type,
and acceptable layouts. The returned region must exactly describe its own
layout and may use a different allowed stride. Encoders must reject missing,
duplicated, or semantically incompatible channels. They may request conversion
only through an explicit `ColorConversion` option.

For a still image the encoder calls `encode_start_v2`, does not call
`frame_v2`, performs one or more `encode_pick_v2` calls, then calls
`encode_end_v2` exactly once. For animation it requests consecutive frame
indices starting at zero; `Ok(None)` ends the frame list. All pixel requests
for one frame finish before the next index. After a post-start error or abort,
the encoder stops requesting data and makes one best-effort `encode_end_v2`
call containing the primary outcome. An end-callback error is secondary unless
encoding itself had succeeded.

## Explicit conversion policy

The compatibility adapter to the existing callbacks is available only when all
of these are true:

1. The color data is Gray, RGB, or CMYK and has a valid ICC/CICP description
   that the selected color-management implementation can explicitly transform
   to sRGB.
2. All color and alpha samples can be converted to finite straight-alpha U8.
3. Extra channels are absent or the caller explicitly chooses to discard named
   channels.
4. HDR-to-SDR tone mapping, gamut mapping, premultiplication changes, and alpha
   compositing are each explicitly selected rather than inferred.

The default policy rejects precision loss, HDR tone mapping, CMYK conversion,
unknown ICC features, unknown CICP values, and extra-channel loss. Conversion
options record the source and destination encodings, rendering intent,
black-point compensation choice, alpha operation, tone mapper, and dither.
Every unsupported option returns `UnsupportedOption`; it is never ignored. A
successful `DecodeResultV2` records the conversions that were actually applied.

CMYK polarity is part of the codec adapter contract. CMS implementations receive
the convention they document; a JXL adapter performs any required inversion at
its boundary. WML2 does not attach an implicit CMYK convention to raw channel
values.

## Owned buffer

`ImageBufferV2` will store the unchanged `ImageDescription`, one or more owned
typed plane buffers, animation frames, and metadata. Construction validates
dimensions, channel descriptions, strides, and allocation limits before
allocating. It will not expose one `Vec<u8>` as an ambiguous universal backing
store.

The allocation policy is supplied through `ResourceLimitsV2`, including maximum
canvas pixels, frame count, progressive pass count, region count, channels,
bytes per plane, bytes per frame, total decoded bytes, ICC profile bytes, and
metadata bytes. Declared dimensions and immediately knowable limits are checked
before `init_v2`. A limit or truncation discovered while streaming may occur
after draw calls, but the final result must be an error rather than partial
success.

## Errors

The v2 API distinguishes at least:

- `UnsupportedRepresentation`: the codec can decode the source but the selected
  callback or compatibility adapter cannot represent it without forbidden loss.
- `UnsupportedColorEncoding`: the ICC/CICP description cannot be evaluated.
- `InvalidPixelLayout`: dimensions, order, type, stride, offset, or buffer length
  is inconsistent.
- `InvalidChannelDescription`: channel roles or precision are inconsistent.
- `ResourceLimit`: checked dimensions or configured limits reject the operation.
- `Aborted`: the callback requested termination.
- `UnsupportedOption`: a requested conversion or codec option is not implemented.

No one of these errors may be replaced by a successful RGBA8 result.

## Compatibility and rollout

1. Add the v2 types and conformance tests without changing the old traits.
2. Add opt-in adapters for codecs that already preserve the required source
   metadata and precision.
3. Implement JXL against v2 first; its legacy WML2 path uses only the explicit
   sRGB8 adapter.
4. Migrate other codecs independently. The old API remains available until a
   separately approved major compatibility change.

Validation also requires orientation in `1..=8`, positive finite gamma values,
unique channel roles and indices, and CICP tuples consistent with the color
model (for example RGB uses identity matrix coefficients when required by its
declared representation).

Required conformance tests cover U8/U16/F32 round trips, Gray/RGB/CMYK, ICC and
CICP retention, straight and premultiplied alpha, mixed planar extra channels,
padded strides, subsampled planes, overflow/resource rejection, animation
ordering, progressive-pass semantics, abort ordering, and refusal of implicit
precision or channel loss.
