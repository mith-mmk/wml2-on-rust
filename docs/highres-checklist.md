# WML2 high-bit-depth and ICC checklist

Status: implementation is staged from `main`; checkpoint 1 is the active
implementation unit. Each checkpoint gets its own commit. No release, version
bump, registry publication, tag, or main integration is part of this work.

## Scope

- [ ] Store 10/12/16-bit integer samples in `PixelBuffer::U16` with
      `meaningful_bits`; retain `U8` and `F32`.
- [ ] Keep sample-unit strides and both planar/interleaved layouts.
- [ ] Keep `draw::ImageBuffer`, `DrawCallback`, and `PickCallback` unchanged
      as RGBA8 APIs.
- [ ] Make high-bit-depth APIs additive and explicitly invoked.
- [ ] Preserve native AVIF YCbCr, alpha, range, subsampling, matrix, colour
      signalling, and geometry metadata without implicit display conversion.
- [ ] Complete Gray/RGB ICC v2/v4 CMS in the independent `icc-profile` crate.
- [ ] Connect WML2 through thin, explicit adapters only.
- [ ] Remove the unused JPEG XL subtree and references from the staged work.

## Feature contract

- [ ] `high-bit-depth`: typed buffers only; no ICC dependency.
- [ ] `color-management`: depends on `high-bit-depth` and `icc-profile`.
- [ ] `avif,high-bit-depth`: native still decode adapter.
- [ ] `avifenc,high-bit-depth`: typed AVIF encode adapter when checkpoint 5
      is reached.
- [ ] Do not retain a combined `highres = ["dep:icc_profile"]` feature.

## Checkpoints

### 1. Typed WML2 buffers

- [x] Recover only the typed buffer foundation and its tests from `00488f4`.
- [x] Exclude HDR transfer functions and generic conversion planners.
- [x] Validate dimensions, sample-unit strides, plane sizes, roles,
      subsampling, alpha association, meaningful bits, and finite F32 values.
- [ ] Commit the checkpoint after feature-off and feature-on regression runs.

### 2. AVIF native still decode

- [ ] Map existing native AVIF planes directly to `ImageFrame`.
- [ ] Cover native 8/10/12-bit AV1 samples and derived 16-bit-or-less output.
- [ ] Preserve integer samples; do not RGB-convert YCbCr.
- [ ] Preserve ICC, nclx/CICP, AV1 colour description, range, matrix,
      subsampling, alpha, and geometry metadata.
- [ ] Keep legacy RGBA8 decode, callback order, and Abort behavior unchanged.
- [ ] Leave high-precision AVIS reader/transaction/ownership work out of this
      checkpoint.

### 3. Independent ICC CMS

- [ ] Implement Gray/RGB v2/v4 parsing and D50 PCS.
- [ ] Implement XYZ/Lab, chad, Gray TRC, RGB matrix/TRC, `curv`, `para` 0-4,
      `mft1`, `mft2`, `mAB`, `mBA`, A2B/B2A, four intents, optional stages,
      1D interpolation, and tetrahedral 3D interpolation.
- [ ] Make `transform_f32` the core and U8/U16 wrappers quantize around it.
- [ ] Exclude alpha from ICC transforms.
- [ ] Return `UnsupportedProfileFeature` for CMYK, N-color, MPE, BPC, and
      unsupported float-profile paths; never use a simplified fallback.
- [ ] Keep checked offset, length, tag, curve, CLUT, and allocation limits.
- [ ] Compare analytic and LittleCMS vectors, including LUT endpoints.

### 4. WML2 ICC adapter

- [ ] Add only `ImageFrame -> RGB/Gray F32 -> Transform -> ImageFrame` glue.
- [ ] Require source and destination ICC profiles explicitly.
- [ ] Do not infer ICC from CICP or create ICC/CICP/ICC pipelines.
- [ ] Keep input signalling distinct from the current output descriptor.
- [ ] Do not publish conversion-history metadata in this phase.

### 5. AVIF typed encode

- [ ] Accept U8/U16 input with an explicit coded depth of 8, 10, or 12 bits.
- [ ] Reject treating 16-bit input as a 16-bit AV1 coded stream.
- [ ] Require explicit quantization for U16/F32 reduction.
- [ ] Keep ICC/nclx metadata replacement separate from pixel conversion.
- [ ] Exclude new Sample Transform 16-bit encoding.

## Required validation

- [ ] U8, U16 10/12/16-bit, F32, planar, interleaved, padding, and
      sample-unit stride tests.
- [ ] Gray/RGB/YCbCr, alpha, and 4:2:0/4:2:2/4:4:4 tests.
- [ ] AVIF native sample/vector, range, alpha, ICC+nclx, derived 16-bit,
      geometry, and legacy RGBA8 tests.
- [ ] ICC parser, transform, intent, endpoint, interpolation, and LittleCMS
      comparisons.
- [ ] Overflow, malformed input, invalid stride/depth, non-finite F32, and
      alpha consistency tests.
- [ ] Miri/fuzz for boundaries and unsafe code where applicable.
- [ ] Full WML2 regression, feature matrix, formatting, Clippy, MSRV, 32-bit,
      and Wasm checks as applicable.

Allocation accounting is limited to checked arithmetic and practical safety
limits. Exact byte-by-byte peak-live-allocation proof is not an acceptance
condition.
