# WML2 high-bit-depth and ICC checklist

Status: checkpoints 1 through 5 are implemented on the dedicated branch.
Ordinary and bounded Sample Transform AVIF still bridging are included in
checkpoint 2; high-precision AVIS remains later independent work. Each
checkpoint gets its own commit. Registry publication, tag, or main integration
is still outside this work; the release-candidate version bumps are included.

## Scope

- [x] Store 10/12/16-bit integer samples in `PixelBuffer::U16` with
      `meaningful_bits`; retain `U8` and `F32`.
- [x] Keep sample-unit strides and both planar/interleaved layouts.
- [x] Keep `draw::ImageBuffer`, `DrawCallback`, and `PickCallback` unchanged
      as RGBA8 APIs.
- [x] Make high-bit-depth APIs additive and explicitly invoked.
- [x] Preserve ordinary native AVIF YCbCr, alpha, range, subsampling, matrix,
      signalling, and geometry metadata without implicit display conversion.
- [x] Complete Gray/RGB ICC v2/v4 CMS in the independent `icc-profile` crate.
- [x] Connect WML2 through thin, explicit adapters only.
- [x] Keep unrelated legacy codec integration and planning artifacts out of
      this staged work.

## Feature contract

- [x] `high-bit-depth`: typed buffers only; no ICC dependency.
- [x] `color-management`: depends on `high-bit-depth` and `icc-profile`.
- [x] `avif,high-bit-depth`: native still decode adapter for ordinary and
      bounded Sample Transform stills.
- [x] `avifenc,high-bit-depth`: typed AVIF encode adapter at checkpoint 5
      is reached.
- [x] Do not retain a combined `highres = ["dep:icc_profile"]` feature.

## Checkpoints

### 1. Typed WML2 buffers

- [x] Recover only the typed buffer foundation and its tests from `00488f4`.
- [x] Exclude HDR transfer functions and generic conversion planners.
- [x] Validate dimensions, sample-unit strides, plane sizes, roles,
      subsampling, alpha association, meaningful bits, and finite F32 values.
- [x] Commit the checkpoint after feature-off and feature-on regression runs.

### 2. AVIF native still decode

- [x] Map existing native AVIF planes directly to `ImageFrame`.
- [x] Cover ordinary native 8/10/12-bit AV1 samples and bounded
      Sample Transform output up to 16 bits.
- [x] Preserve integer samples; do not RGB-convert YCbCr.
- [x] Preserve ICC, nclx/CICP, AV1 colour description, range, matrix,
      subsampling, alpha, and geometry metadata.
- [x] Keep legacy RGBA8 decode, callback order, and Abort behavior unchanged.
- [x] Leave high-precision AVIS reader/transaction/ownership work out of this
      checkpoint.

### 3. Independent ICC CMS

- [x] Implement Gray/RGB v2/v4 parsing and D50 PCS.
- [x] Implement XYZ/Lab, chad, Gray TRC, RGB matrix/TRC, `curv`, `para` 0-4,
      `mft1`, `mft2`, `mAB`, `mBA`, A2B/B2A, four intents, optional stages,
      1D interpolation, and tetrahedral 3D interpolation.
- [x] Make `transform_f32` the core and U8/U16 wrappers quantize around it.
- [x] Exclude alpha from ICC transforms.
- [x] Return `UnsupportedProfileFeature` for CMYK, N-color, MPE, BPC, and
      unsupported float-profile paths; never use a simplified fallback.
- [x] Keep checked offset, length, tag, curve, CLUT, and allocation limits.
- [x] Compare analytic and LittleCMS vectors, including LUT endpoints.

### 4. WML2 ICC adapter

- [x] Add only `ImageFrame -> RGB/Gray F32 -> Transform -> ImageFrame` glue.
- [x] Require source and destination ICC profiles explicitly.
- [x] Do not infer ICC from CICP or create ICC/CICP/ICC pipelines.
- [x] Keep input signalling distinct from the current output descriptor.
- [x] Do not publish conversion-history metadata in this phase.

### 5. AVIF typed encode

- [x] Accept U8/U16 input with an explicit coded depth of 8, 10, or 12 bits.
- [x] Reject treating 16-bit input as a 16-bit AV1 coded stream.
- [x] Require explicit quantization for U16 reduction; F32 is rejected by the
      initial encoder adapter.
- [x] Keep ICC/nclx metadata replacement separate from pixel conversion.
- [x] Exclude new Sample Transform 16-bit encoding.

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

## Publication handoff (historical, 2026-08-31)

The original release handoff kept these candidates on implementation
branches pending publication approval:

| Crate | Candidate | Required order |
| --- | --- | --- |
| `icc-profile` | `0.0.5` | 1 |
| `avif-rust` | `0.0.7` | 2 |
| `avifenc-rust` | `0.0.7` | 3 |
| `wml2` | `0.0.29` | 4 |

That handoff required locked package and publish dry-runs for every crate,
and replacement of the temporary ICC git dependency with a registry
dependency. The latter is already reflected in the current manifest; see
the dated status below. This historical list is not a statement of current
registry publication or branch status.

## Release-candidate validation record

On 2026-08-31 the following checks passed on the candidate commits:

- `icc-profile`: 45 library tests, locked package, and publish dry-run.
- `avif-rust`: locked package and publish dry-run; native boundary tests and
  the existing decoder regression suite were run.
- `avifenc-rust`: locked package and publish dry-run.
- `wml2`: locked no-default, `high-bit-depth`, `color-management`,
  `avifenc + color-management`, and integration test matrices passed.
- WML2 and AVIFENC formatting checks passed; Clippy completed successfully
  with pre-existing warnings in legacy code.

At that time, WML2 package and publish dry-runs were blocked until the
three child release candidates exist in their required registry/git order.
AVIF and ICC repositories retain unrelated pre-existing formatting debt, so
their full-tree format checks are not release gates for this change.

Miri/fuzz and external fixture-dependent checks remain follow-up validation;
they are not silently represented as passed here.

## Implementation status verified on 2026-09-05

The manifest uses registry `icc-profile 0.0.6`, registry `webp-rust 0.3.1`,
and version `0.0.7` path dependencies for the independent AVIF decoder and
encoder. ICC 0.0.6 was published on 2026-09-05 after main integration and
locked package/dry-run validation. Its registry download was verified before
updating both WML2 and wml2-test. See the current release integration record
in [review-plan-progress.md](review-plan-progress.md). WML2 publication is a
separate step.

The review implementation adds meaningful-bit ICC normalization, explicit
premultiplied-alpha rejection, timing preservation, reusable tiled ICC
transforms, explicit AVIF precision rejection, and preservation of AV1 range
when its color description is absent. Existing APIs remain available.

See [the implementation and validation record](review-plan-progress.md) for
completed tests, reproducible commands, CI configuration, and unverified
platforms. Local Miri boundary tests and a bounded PNG mutation campaign now
pass; these do not mark the entire historical validation checklist as
complete. A new LittleCMS 2.16 RGB gamma reference covers 256 colors at
8/10/12/16-bit precision. Linux/macOS CI execution remains unverified.
