# highres implementation and acceptance checklist

Status: implementation started; no unchecked item is a completion claim.
Contract: [WML2 highres color pipeline](wml2-color-pipeline-v2.md).
Coding is assigned to luna; design, independent review and acceptance are
assigned to sol. Review each checkpoint before its separate repository commit.
No version bump, GitHub push, PR, tag, release or registry publication is
authorized by this checklist. JXL remains stopped and out of staged changes.

## H0: boundaries and reproducible baseline

- [x] Record the approved `highres` namespace, typed API and compatibility scope.
- [x] Capture existing WML2/AVIF/AVIF encoder/ICC baselines independently.
- [ ] Verify only intended branches/files changed; preserve unrelated changes.
- [ ] Verify `.test*` and `test_data` ignore rules before creating local data.
- [x] Record public ICC checkpoint and local patch configuration separately.
- [ ] Prove default/feature-off fresh checkout works without local patch paths.

Baseline evidence: existing WML2 workspace tests and 12 default doctests pass;
the AVIF adapter has 6 decode and 8 encode passes, and feature-off has 1 pass.
The independent encoder baseline has 23 library, 28 encode and 14 FFmpeg passes.
The ICC legacy suites have 128 passes and 1 ignored test; these are not LUT CMS
acceptance tests. The public ICC revision is pinned in the dependency contract;
new local CMS behavior still requires an ignored, explicit configuration patch.
Branch/staging isolation and patch-free fresh-checkout acceptance remain open.

## H1: typed buffers and feature contract

- [x] Add `ImageFrame`, `ImageDescriptor`, `PixelBuffer::{U8,U16,F32}` with
  checked construction, native planar storage and explicit supported views.
- [x] Validate dimensions, meaningful bits, roles, plane lengths, padding,
  strides, channel offsets, independent X/Y subsampling, and arithmetic limits.
- [x] Reject NaN/Inf and invalid alpha at frame validation; retain finite
  negative/over-one color samples without implicit clipping.
- [ ] Describe the active F32 sample domain explicitly, including relative
  scene-linear versus absolute nit-valued RGB, before conversion integration.
- [ ] Preserve ICC+nclx+AV1 signaling, provenance, geometry and timing together.
- [x] Gate the current new WML2 module/dependency with default-off `highres`; no automatic
  AVIF enablement and no legacy/aggregate feature change.
- [x] Keep independent codecs free of new CMS dependencies and MSRV increases.

The typed/feature checkpoint is independently approved. Its 9 typed tests and
1 safety test pass on stable, Rust 1.91 (x86_64 and executed i686), and Miri;
stable Wasm compilation also passes. Six independent temporary tests cover
layout arithmetic extremes, oversized channels, mutation revalidation, integer
precision, invalid HDR domains, and numeric references. PQ/HLG round trips pass
the tracked 10/12-bit grids and an independent 16-bit grid, including endpoints
and HLG branch-adjacent values. New-file Clippy and formatting checks pass.

Current typed metadata stores ICC+nclx+AV1 fields together and separates source
metadata, but complete codec signaling/geometry preservation and the AVIF bridge
remain open. ResourceLimits, fallible bulk allocation, and raw-layout offset
count/work bounds remain required before untrusted codec integration. Scalar
HDR checks do not complete ICC/CICP frame conversion or H4, and this checkpoint
does not close the complete consumer/build matrix in H6.

## H2: native AVIF and stateful AVIS reading

- [x] Add rich codec wrappers without new required fields on existing structs.
- [ ] Read native Gray/RGB/YCbCr 8/10/12-bit samples without display conversion.
- [ ] Preserve existing `sato` 16-bit behavior with regression vectors; validate
  derived-item relationships separately from `av01` master/alpha constraints.
- [ ] Test odd dimensions, 400/420/422/444, chroma locations, full/limited range,
  ICC+nclx coexistence, orientation/crop/pixel aspect, and transparent samples.
- [ ] Reject malformed `av01` master/alpha depth mismatch and invalid auxiliary
  monochrome/range; ignore alpha `colr` in color processing.
- [ ] Read AVIS sequentially with unchanged timing/repetition, synchronized
  color/alpha state and no state advancement after a failed frame.
- [ ] Match native sample values with independent decoder/vector expectations;
  do not use legacy RGBA8 output as the high-precision reference.

Decoder checkpoint evidence: 486 library tests pass with 6 ignored tests, and
3 public rich-metadata parser tests pass; all-target checks, formatting and
diff checks pass. Existing 16-bit `sato` sample decoding has one real-sample
regression pass. ICC/nclx coexistence, ICC property type and unknown `colr`
retention are additive; legacy metadata projection and public fields remain.

This checkpoint is not yet approved: the strict raw-grid path still performs
legacy mixed-cell normalization, and the new parser fixture needs valid item
offsets plus a conflicting other-item color property to prove isolation.
The new derived-alpha rejection branches also need regression coverage.
Strict derived grid/`sato` alpha is intentionally unsupported at this stage;
that rejection does not satisfy derived-alpha support or the full H2 gate.
Native oracle comparisons, complete layout/geometry cases and the new
high-precision stateful AVIS path remain open.

## H3: shared ICC Gray/RGB CMS

- [ ] Preserve existing reader APIs and all existing regression tests.
- [ ] Check header/tag offsets, lengths, channel counts, CLUT sizes and allocation
  limits; fail closed on malformed or unsupported profiles.
- [ ] Test `curv`, `para` 0-4, Gray/RGB matrix/TRC, XYZ/Lab D50 and `chad`.
- [ ] Test `mft1`/`mft2` and `mAB`/`mBA`, optional stages and LUT endpoints.
- [ ] Test A2B/B2A direction, four intent selections and missing-tag rules;
  forward-only curves must not fail solely because inversion is unsupported.
- [ ] Reject BPC, CMYK/N-color/MPE and unsupported float-profile operations.
- [ ] Test immutable transform reuse, worker isolation and concurrent results.
- [ ] Test U8/U16 quantization wrappers against the F32 core.
- [ ] Independently compare matrix/TRC to analytic values (absolute error
  `<= 1e-5`); compare LUTs to LittleCMS on RGB `17^3` and Gray 4096 points,
  including endpoints (DeltaE00 median `<= 0.1`, p95 `<= 0.25`, max `<= 1.0`).
- [ ] Validate the color-difference metric independently; required oracle/data
  absence is an incomplete gate, not a successful skipped comparison.
- [ ] Keep legacy AVIF ICC behavior unchanged; do not substitute an incomplete
  shared CMS for its existing LUT implementation.

## H4: explicit U16/F32 and PQ/HLG conversion

- [ ] Separate range/chroma/matrix work from ICC-or-CICP color interpretation.
- [ ] Verify ICC/CICP transfer is applied exactly once; preserve source metadata
  separately from the active output descriptor after conversion.
- [ ] Test full/limited endpoints, chroma position, gray/RGB and matrix choices.
- [ ] Test PQ absolute F32 (`1.0 = 1 nit`) against independent numeric vectors.
- [ ] Test HLG scene-linear separately; require explicit conditions for any
  requested display-referred conversion.
- [ ] Reject implicit HDR-to-SDR, tone mapping, clipping, invalid domains and
  unsupported conversion options; gain-map composition remains out of scope.
- [ ] Test alpha association/domain/zero-alpha policies without changing native
  hidden samples or running ICC over alpha.
- [ ] Verify F32 HDR paths do not pass through clamped RGBA16 or normalized ICC
  helpers before preserving their luminance and gamut.

## H5: high-precision AVIF still encoding

- [ ] Require explicit coded 8/10/12-bit depth and 400/420/422/444 native layout.
- [ ] Add native alpha input with AVIF-legal matching depth and full range.
- [ ] Generate RGB input's requested target matrix/range/chroma planes; reject
  unsupported combinations instead of relabeling fixed BT601/full-range data.
- [ ] Preserve ICC+nclx and semantic metadata without fixed-sRGB overwrites.
- [ ] Prove native lossless sample/alpha/colorimetry/geometry identity with
  self-decoding and independent external decoding.
- [ ] Reject source-lossless requests that need requantization, extra
  subsampling, ICC conversion, alpha normalization or metadata loss.
- [ ] Test explicitly quantized U16/F32 targets separately from source-lossless
  claims; no 16-bit AV1 or F32 file-encoding claim.

## H6: independent regression and safety gates

- [ ] Separate consumer runs: default; no-default; highres-only; avif with and
  without highres; avifenc with and without highres; relevant multithread cases.
- [ ] Compile unchanged legacy consumer examples and compare old output pixels,
  metadata behavior, callback order and Abort behavior with highres off/on.
- [ ] Verify highres-off WML2 normal dependency graph has no new CMS dependency.
  Do not inspect the whole workspace graph or fail merely because Cargo.lock or
  optional dependency declarations mention ICC.
- [ ] Verify highres-only does not activate either AVIF codec.
- [ ] Run standalone codec and ICC suites, WML2 workspace regression, doctests,
  rustfmt, Clippy, rustdoc, and relevant feature combinations.
- [ ] Run WML2 MSRV 1.91, independent codec MSRVs, 32-bit and Wasm checks/tests
  as appropriate; record execution versus compilation-only coverage explicitly.
- [ ] Exercise truncated/malformed/oversized input, checked allocation failures,
  fuzz targets and applicable Miri tests; no panic/OOM/partial-success claims.
- [ ] Record performance/peak-memory baselines and regressions without inventing
  an unapproved libjxl/JXL performance gate for this AVIF task.
- [ ] Independently review exact diffs and test provenance before each commit.
- [ ] Inspect staged parent versus nested changes; exclude samples, local
  configs/reports, unrelated WebP changes and retained untracked JXL files.

## H7: dependency synchronization and publication (separate authorization)

- [ ] Authorize and publish/synchronize a tested upstream source before claiming
  patch-free fresh highres consumer support for newly added CMS APIs.
- [ ] Verify patch-free highres consumer against the intended upstream revision.
- [ ] Approve versions, release order and registry switch separately; downstream
  publication waits for upstream registry consumer validation.
- [ ] Verify packaged dependency resolution and contents with no local patch or
  machine paths; package/dry-run is not a successful publication.
- [ ] Never mark this gate complete using only a local patch-enabled test.

Local harness configuration, oracle binaries, generated fixtures and reports
belong to ignored `.test*` locations. Pass oracle/corpus/report paths at runtime
and record exact versions/hashes in ignored reports. Clean temporary artifacts
after use; do not delete permanent user samples.
