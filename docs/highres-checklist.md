# highres implementation and acceptance checklist

Status: implementation started; no unchecked item is a completion claim.
Contract: [WML2 highres color pipeline](wml2-color-pipeline-v2.md).
Coding is assigned to luna; design, independent review and acceptance are
assigned to sol. Review each checkpoint before its separate repository commit.
No version bump, GitHub push, PR, tag, release or registry publication is
authorized by this checklist. JXL remains stopped and out of staged changes.

## Latest resume snapshot

- Encoder native/CfL checkpoint 9c93f1c and parent gitlink-only checkpoint
  a00a12d are saved; independent native lossless checks cover 37 cases/55 streams.
- C1 parser slices and the prefix grammar seam have limited acceptance. Native
  show-frame validation and alpha prefix retention/reparse removal remain open.
- ICC S1-S4 are accepted only within their selected-route/allocation slices.
  S4 plan-bound ownership and tracked real-allocation/drop/same-ledger retry are
  verified. Broader ICC WIP lint cleanup and overall product acceptance remain open.
- Full H3 intent/domain/oracle coverage and H4 explicit frame conversion remain
  unfinished. Historical evidence below is slice-specific, not a blanket gate.
  No publishing, version bump or wider checkbox completion is authorized.

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

Decoder checkpoint evidence: 488 library tests pass with 6 ignored tests, and
3 public rich-metadata parser tests pass; all-target checks, formatting and
diff checks pass. Existing 16-bit `sato` sample decoding has one real-sample
regression pass. ICC/nclx coexistence, ICC property type and unknown `colr`
retention are additive; legacy metadata projection and public fields remain.

The functional review findings are resolved: strict raw-grid skips legacy
mixed-cell normalization and retains the existing configuration mismatch
rejection; the rich-parser fixture has valid item offsets and a conflicting
other-item color property. Separate tests cover raw/legacy preparation and
the derived-alpha rejection policy. The limited standalone decoder checkpoint
is independently approved; all-target Clippy passes with warnings denied.
An earlier Rust 1.91 i686 run had three legacy container error-classification
failures (`iinf`, `iloc`, `ipma` oversized counts), also confirmed at baseline
`1203c44`. The newer frozen i686 library run passes all 488 tests with 6 ignored;
native-boundary 5, native-limits 5 and rich-color 3 also pass. This resolves those
three failures in the tested library scope, not examples or every integration
target, and does not establish the bounded native/C1 contract.
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

## Next checkpoints: native bridge, domains and bounded decoding

These are implementation instructions, not completed acceptance items. Keep the
approved still/AVIS/derived-image scope; an interim unsupported path remains
unfinished. WML2 stages A/B can proceed before standalone AVIF stages C1-C3.
Do not expose a new unbounded byte-decoding API while those stages are pending.

### A: WML2 domain, metadata and limit vocabulary

- Extend existing private-field highres types additively; keep their current
  constructors/accessors and the legacy WML2 API unchanged. Put domain and limit
  logic in `wml2/src/highres/domain.rs` and `limits.rs`, not in codec modules.
- Add `SampleDomain` with `Unknown`, `Encoded`, `LinearRelative`,
  `LinearAbsoluteNits`, `HlgSceneLinear` and `HlgDisplayLinear` interpretations.
  Store checked primaries/white-point information separately. The HLG display
  interpretation includes `HlgDisplayConditions`; absolute units are explicitly
  nits. Do not infer linearity, primaries or HDR interpretation from F32 storage.
  Existing descriptor construction defaults to `Unknown` and remains valid;
  conversion with an unspecified required domain fails explicitly. Domain-
  specific range checks never silently clamp samples or act on alpha as color.
- Extend `FrameMetadata`/`ColorInformationSet` with checked raw clean aperture
  rationals, separate coded/render geometry, exact pixel aspect, and ordered
  geometry operations. Keep integer `Rect` as an explicitly resolved crop, not
  a replacement for fractional `clap`; preserve wire numerator/denominator bits
  and validate denominators before any arithmetic or signed interpretation.
  Retain ICC `prof` versus `rICC`, unknown `colr` type/payload and provenance.
  Keep AV1 description presence, monochrome/range/subsampling/chroma-position
  flags and extended `pixi` location information without inventing defaults.
- Add `SequenceInformation` and `RepetitionCount` for exact rational duration,
  frame count and finite/infinite/unknown repetition. Continue using
  `FrameTiming` for each frame; milliseconds are not the authoritative timing.
- Add checked `ResourceLimits` with explicit byte/count units: input bytes,
  width/height/pixels, channels/planes, plane/frame bytes, total live decoded
  bytes, references, frame count, metadata/ICC/CLUT bytes, parser entries/depth,
  grid cells and derived-image work/depth. Use checked conversion to `usize`.
  Start with explicit caller-provided limits; do not add an undocumented
  unlimited default. `ImageFrame::validate_with_limits` checks an already-owned
  frame only and must not claim to limit a preceding decode/allocation.
- Add a separate non-exhaustive `ProcessingError` for new processing APIs,
  wrapping `HighresError` and distinguishing malformed/truncated input,
  resource limit, allocation failure, unsupported operation and conversion
  failure. Do not break exhaustive matches on the existing `HighresError`.

Stage A tests: old constructors remain valid; Unknown fails conversion;
encoded/relative/nit/HLG domains stay distinct; invalid conditions and rational
denominators fail; ICC variants/unknown properties and ordered geometry retain
their exact source values. Source metadata and active interpretation remain
separate. This stage does not implement ICC/CICP frame conversion.

### B: WML2 native ownership adapter, without byte decoding

Add `wml2/src/highres/avif/{mod,mapping,metadata}.rs`, gated by both existing
features. Implement one internal `consume_native_frame` shared by still and
sequence adapters. Its inputs are an owned standalone `DecodedFrame`, rich
native metadata and checked limits; its output is the existing `ImageFrame`.

- Move `Vec<u16>` planes; retaining U16 storage for 8-bit AV1 is valid and avoids
  an unnecessary copy. Set meaningful precision to the actual 8/10/12-bit
  source, or validated reconstructed 16-bit `sato` precision. No `to_rgba8`,
  `to_rgba16`, CMS, resampling, unpremultiplication or geometry application.
- Map monochrome to Gray; native identity-matrix RGB plane order is G/B/R, so
  assign those explicit roles rather than assuming R/G/B. Otherwise preserve
  Y/Cb/Cr roles and independent subsampling. Locate alpha by native plane ID 3,
  not vector position; preserve association and zero-alpha hidden samples.
- Validate native dimensions, IDs, layouts, bit ranges, alpha and metadata
  before publishing the frame. Preserve raw/render geometry even when they
  differ, without relabeling coded samples as display-transformed samples.
- Use budgeted fallible allocation for metadata/descriptor copies and account
  for simultaneous native and destination allocations. Do not expose standalone
  codec types from the highres-only namespace.

Stage B tests use generated native-plane fixtures, not legacy RGBA as an oracle:
odd 400/420/422/444, GBR identity order, 8/10/12/16-bit precision, alpha/hidden
colors, malformed plane IDs/layouts, and complete A-stage metadata forwarding.
Add `wml2/tests/highres_native_mapping.rs` with explicit feature requirements.

### C1: standalone AVIF input/header/per-allocation bounds

Own these additions in `avif/src/limits.rs`, `container/` parser helpers and
`av1/decode.rs`; keep the codec CMS-free and at its existing MSRV. Add private-
field `NativeDecodeLimits` and `NativeAvifInformation`/`NativeDecodedFrame`
wrappers instead of new required fields on `AvifInfo`, `RichAvifInfo` or
`DecodedFrame`. Reuse `parse_rich_info` semantics through shared parser helpers;
do not repeat an unrestricted parse to obtain metadata.

- Check input length before parsing/copying. Validate box nesting, entry counts
  and minimum remaining bytes before `iinf`/`iloc`/`ipma`/sample-table reserve;
  bound item extents, property payloads, ICC and aggregate metadata copies.
  Add `pasp` and ordered property/track metadata retention in the standalone
  parser; WML2 must not maintain a second BMFF parser.
- Inspect AV1 sequence maximum dimensions and actual coded/upscaled/render
  dimensions before frame planning/allocation. Include coded padding, each
  master/alpha track, hidden frames, sequence-header changes and every grid or
  `sato` input. Container `ispe` alone is not a trustworthy allocation bound.
- Introduce shared checked/fallible allocation helpers for each strict path.
  The current fixed plane-sample constant is not a caller-controlled budget;
  reject malicious counts before reserve, and do not implement limits solely
  by inspecting a successfully returned frame.

C1 tests use tiny malformed containers claiming huge counts/dimensions and
allocation failpoints. A per-input/per-plane bound alone does not complete
ResourceLimits or justify a general no-OOM claim.

Selected frozen C1 regression evidence covers native-boundary 5, native-limits 5
and rich-color 3 tests: all 13 pass under Miri, actual i686 execution and
wasm32-wasip1 execution with Node WASI.
Shared-parser pre-allocation/fallible accounting and C1 acceptance remain open;
these selected passes do not establish a complete memory bound.

### C2: standalone live-state and derived-image budget

Thread one internal `DecodeBudget` through strict parsing, decode, filters,
super-resolution, sequence references, grid composition and `sato` evaluation.
Reserve before allocation and release on drop/rollback; do not rely on ambient
global allocator state. Count actual owned allocations, deduplicating shared
reference buffers while charging real deep clones, CDF/motion/tile/filter
scratch, old-plus-candidate transactional state, color plus alpha, decoded grid
cells plus output and all live sample-transform inputs. Bound derived recursion,
cycles and total work as well as output dimensions. Audit every strict-path
`vec!`, `with_capacity`, `collect`, `to_vec`, deep clone and reference refresh.

Total-live limits cover decoder-owned state, staging, bridge allocations and
output creation. Ownership transfer at successful return ends this accounting;
caller-retained/cloned/detached frames are outside the decoder budget. This is
not a process-wide cap and does not introduce a managed-frame wrapper. The
bridge must reserve its peak while the decoder still owns candidate/reference
state; independent per-layer checks against the same full limit are inadequate.

C2 tests force failure with a limit that permits one frame but not simultaneous
references/candidates, both alpha tracks, filters, repeated grid cells or
`sato` inputs. Check budget rollback/release and source samples after failures.
Temporary rejection of derived alpha does not complete derived-alpha support.

### C3: strict transactional stateful AVIS

Refactor `avif/src/decoder/frame.rs` and `decoder/sequence.rs` around their
existing stateful core. Add `StrictAvifSequenceDecoder` rather than changing
legacy `AvifSequenceDecoder` behavior. Reuse strict alpha validation for every
animated/static alpha frame before attachment: matching coded depth, dimensions,
monochrome/full-range auxiliary data, and no alpha color-profile processing.
Never call the legacy depth-normalizing attachment path without these checks.
Validate exact color/alpha timing relationships, not only sample counts.

Expose a `PreparedSequenceFrame` guard: `prepare_next_frame` stages both tracks;
the bridge takes the owned native frame, constructs/validates its `ImageFrame`,
then calls an infallible `commit`. Dropping an uncommitted guard rolls back cursor,
references, CDF/motion state, static-alpha cache and budget. Clean EOS is `None`;
missing/corrupt expected samples are errors. No replay from frame zero, new
drawing/picking callbacks, or changes to legacy callback/Abort behavior.

C3 tests cover Nth-frame alpha depth/range mismatch, truncated alpha, hidden and
show-existing frames, timing mismatch, bridge/allocation failure after native
decode, retry with unchanged state, exact repetition/timing and clean EOS.

### D: connect public WML2 APIs only after C1-C3 gates

Add `NativeDecodeOptions::new(ResourceLimits)`,
`highres::decode_avif_bytes(&[u8], &NativeDecodeOptions) -> Result<ImageFrame,
ProcessingError>` and `highres::AvifSequenceReader::{new, information,
next_frame}`. The reader uses the strict prepared/commit path, and both adapters
share B's ownership mapping. Standalone bounded APIs are
`decode_frame_bytes_strict_with_limits` and the strict sequence constructor;
unbounded legacy helpers are not a fallback on unsupported limits/metadata.

Validate independent native samples, all declared semantic metadata, strict
alpha, resource failures, feature-off dependency isolation and unchanged legacy
outputs/callbacks. Commit A/B, C1, C2, C3 and D as separately reviewed checkpoints
in their owning repositories. Keep full H2/H4/H5/H6 acceptance open until the
remaining derived cases, conversions, encoding and oracle gates are completed.

## Independent review: open ICC and C1 blockers

- [x] Validate ICC permitted stages: require B, pair A with CLUT and M with
  matrix, and require equal channels whenever CLUT is absent. Independent
  RGB/Gray tests cover all stage-presence combinations in both directions.
  The six clamp/gamma, Gray mBA and curve-limit regressions pass; this limited
  closure does not complete H3 or establish unrestricted unclamped behavior.
- [ ] Complete ICC intent selection and absolute PCS handling, immutable shared
  compiled storage, checked allocation and the required LittleCMS comparisons.
- [ ] Separate ICC structural profile parsing from direction-specific transform
  and inverse-curve compilation; unused forward curves must not require inversion.
- [ ] Close C1's hidden-frame helper bypass and reduced-still/show-existing
  classification error; unsupported sequence/derived paths must stop before
  unbounded parsing or reconstruction, without changing legacy entry points.
- [ ] Check AV1 sequence maxima and every actual coded/upscaled master/alpha
  allocation before reserve. Super-resolution and other unbudgeted strict
  paths must fail explicitly until their pre-allocation checks exist.
- [ ] Charge retained metadata copies and aggregate associations across ipma
  boxes; thread fallible bounds through the shared parser instead of relying
  only on encoded box sizes and post-parse checks.

ICC and C1 remain NO-GO as complete checkpoints. C2/C3, full native decoding,
derived-image support and H6 remain open; limited consumer, MSRV, Wasm or Miri
passes do not establish these broader contracts.

## Independent review: open Stage A/B blockers

- [ ] Charge metadata vector/string capacity and actual geometry element sizes;
  validate budgets before bridge clones and use fallible descriptor/metadata
  allocation while accounting for simultaneous source and destination ownership.
- [x] Reject native precision/configuration and chroma-layout inconsistency,
  mismatched extended pixi channel counts, and unusable RGB white-point values.
  Actual decoded monochrome signalling maps to full-resolution luma, and
  ordinary 16-bit frames without derived-sample provenance are rejected.
- [ ] Also cross-check retained pixi precision/channel declarations against the
  actual native colour planes; auxiliary alpha is a separate item.
- [x] Preserve unknown ICC type information or reject it explicitly; do not
  silently replace an unknown type with an unspecified type.
- [ ] Connect authoritative ordered native property metadata before claiming
  geometry preservation. Rich metadata's separate rotation/mirror fields cannot
  recover source order or pasp; raw fractional aperture needs an ordered form.
  Do not synthesize fixed rotation-then-mirror or identical coded/render lists.

Stage A/B remain unaccepted. Native pixel-vector ownership transfer is verified,
but does not establish complete semantic preservation or allocation bounds.
Keep the mapper internal and public byte/sequence APIs deferred behind C1-C3.

## H3 next implementation design: compatibility, intent and oracle boundaries

This is an implementation plan, not H3 acceptance. The closed stage-presence
and six regression cases remain closed; the other H3 gates above remain open.
No CMYK/N-color, MPE execution, BPC, iccMAX, publication or version change is
included. Keep the existing AVIF ICC path separate until shared-CMS acceptance.

### Primary references and fixed profile facts

Use [ICC.1:2022](https://www.color.org/specifications/ICC.1-2022-05.pdf), notably
6.2, 6.3.2, 6.3.4, 8.10.2, Table 25, 9.2.15, 9.2.36 and Annex F. Annex E.4 is
informative guidance on `chad`, not a requirement to apply an additional CAT.
The [ICC profile registry](https://registry.color.org/rgb-registry/srgbprofiles)
and [ICC v2 guidance](https://www.color.org/v2profiles/) explain the supplied
v2 profile's D50 media-white convention; do not infer all v2 semantics from a
v4 specification. Profile names are not substitutes for their actual bytes.

| Corpus profile | Header version / class / PCS | Available transform data |
| --- | --- | --- |
| `sRGB2014.icc` | 2.0.0 / `mntr` / XYZ | RGB matrix, sampled TRCs; no A2B/B2A |
| `sRGB_v4_ICC_preference.icc` | 4.2.0 / `spac` / Lab | A2B0/A2B1, B2A0/B2A1 |
| `sRGB_ICC_v4_Appearance.icc` | 4.3.0 / `mntr` / Lab | A2B0/A2B1, B2A0/B2A1 |

All three supplied files have header intent 0, D50 header illuminant and `wtpt`
approximately `(0.964202881, 1, 0.824905396)`, plus an invertible `chad`. Neither
v4 file supplies A2B2/B2A2. Preference is a ColorSpace profile, not a display
profile. Preserve version, class, raw white and CAT rather than rewriting them
from the filename or RGB label. Preserve the profile's embedded black behavior;
turning CMM BPC off does not remove black scaling already encoded in a profile.

`inverse(chad) * wtpt` is a diagnostic reconstruction of media white in the
original measurement conditions, not the default ICC-absolute scaling white.
For these bytes it is approximately `(0.950398, 0.999996, 1.088919)` for sRGB2014
and Appearance, and `(0.950473, 1.000100, 1.094741)` for Preference. Do not replace
these values with nominal D65 or use their difference to modify stored profiles.
The actual adopted white is separately reconstructed from `inverse(chad) * D50`.

### Additive API and shared ownership

The proposed minimal additions are:

```rust
Profile::parse(bytes) -> Result<Profile, TransformError>
Profile::parse_with_limits(bytes, parse_limits) -> Result<Profile, TransformError>
Profile::compile(direction, intent, transform_limits)
    -> Result<CompiledProfile, TransformError>
Transform::new_with_limits(source, destination, options, transform_limits)
    -> Result<Transform, TransformError>
```

`TransformDirection` has DeviceToPcs/PcsToDevice; `CompiledProfile` is opaque and
cheaply cloneable. Its PCS boundary is physical D50 XYZ with Y=1, not normalized
LUT codes or unlabelled Lab triples. Direction-specific float-slice evaluation
may be exposed on this type for reuse and one-way verification. A private
assembler links the two compiled directions; existing `Transform::new` remains
the convenience entry point and delegates with documented default limits.

Keep `new`, `from_bytes`, `from_bytes_with_limits`, existing transform/worker
signatures, public options/limits struct fields and public error variants intact.
In particular, do not add fields to existing structs used with struct literals
or variants to the existing exhaustive error enum. New limits use a separate
type with private fields and checked builders. If old constructors must preserve
eager validation/error timing, call a separate compatibility validator after the
common structural reader; the new `parse*` path must not run it. Do not duplicate
the byte parser or make old constructors the implementation of structural parse.

Structural parsing owns bounded bytes and checked tag ranges, plus raw version,
class, PCS and intent metadata. It is not proof that every possible transform is
executable. Permit exact shared payload ranges, as used by the official RGB TRCs;
validate arithmetic, table/range boundaries and the chosen overlap policy.
Unused tags do not require curve inversion or allocating decoded LUTs.

Direction compilation selects its route first, then validates/decodes that route.
DeviceToPcs accepts constant and non-monotonic forward curves when otherwise
valid, without matrix inversion. PcsToDevice executes B2A curves in their stored
forward direction; it does not numerically invert A2B. Only a selected inverse
matrix/TRC route requires an invertible matrix and non-constant monotone curves.
Apply Annex F plateau choices to increasing and decreasing inverse curves.
Support Gray TRC's XYZ and Lab achromatic interpretation separately from RGB's
XYZ-only matrix/TRC model. Validate parametric functions over their actual
piecewise domains, not just a few sampled points.

Use `Profile(Arc<ProfileStorage>)`, `CompiledProfile(Arc<CompiledDirection>)` and
`Transform(Arc<CompiledTransform>)`. Keep large validated Vec payloads inside
these immutable objects; worker construction clones Arc handles and starts with
empty per-worker buffers. Do not clone LUT/curve Vecs or use mutable global caches.
Avoid converting an already owned Vec into an Arc slice when that duplicates its
payload allocation. Identity/shared tag data may be reused by validated ranges.

Count decoded float CLUT/curve bytes, descriptors and simultaneous compile
temporaries, not only encoded tag sizes. Limits cover cumulative compiled bytes,
curve/CLUT entries, pixels, output bytes and worker scratch. Every input-derived
allocation uses checked arithmetic and fallible reserve before materialization.
The fixed-size standard Arc control allocation is distinct from payload budgets;
do not promise recovery from all process OOM conditions or introduce unsafe Arc
construction to make that claim.

Validate input/output lengths and limits before any wrapper reserve. The F32
slice core needs no whole-image copy; U8/U16 wrappers can convert pixel/chunk
scratch rather than collect the entire input. The allocating F32 convenience
wrapper checks pixel count, output samples and byte count before reserving.
Retained worker capacity counts toward its budget. Test allocator-failure paths,
zero allocation for warmed slice evaluation and constant-cost worker creation.

### Route selection and the single PCS bridge

| Requested intent | Selected A2B / B2A suffix | PCS bridge |
| --- | --- | --- |
| Perceptual | 0 | Preserve the selected profile's perceptual PCS semantics |
| Relative colorimetric | 1 | Physical XYZ, no media-white rescaling |
| Saturation | 2 | Preserve the selected profile's saturation PCS semantics |
| Absolute colorimetric | 1 | Relative-to-absolute-to-relative white scaling |

For supported Input/Display/Output/ColorSpace models use 8.10.2 precedence:
designated A/B tag, then same-direction 0 tag if missing, then the applicable
matrix/TRC model. Record requested intent, selected tag/model and fallback in the
compiled route. Unsupported MPE execution is not silently claimed: its presence
may be bypassed as allowed by 8.10.2(a) when a supported legacy route exists;
MPE-only profiles return Unsupported. A selected malformed A/B tag is an error,
not a reason to hide corruption by falling back. Never look for A2B3/B2A3.
Do not blanket-reject matrix/LUT mixed routes or all non-relative intents.
Reject unsupported profile classes/models explicitly at compilation, while
retaining structurally readable metadata. Do not use ColorSpace RGB matrix/TRC
as though Table 25 defined it.

Use one internal path for matrix/matrix, matrix/LUT, LUT/matrix and LUT/LUT:

```text
device -> selected source stage -> decode tag PCS -> physical XYZ
       -> intent PCS bridge -> encode destination PCS -> selected destination stage
```

For Absolute, compose equations (1)-(6) of 6.3.2.2:
`absolute_xyz = source_relative_xyz * source_wtpt / D50`, then
`destination_relative_xyz = absolute_xyz * D50 / destination_wtpt`.
Thus the combined scale is source-white / destination-white, component-wise.
Apply it once, in physical XYZ, never directly to Lab or normalized LUT codes.
A source white of 0.8 D50 and destination white D50 must yield destination
relative white 0.8, not 1.25. Validate finite, positive usable white components.

Normal profile transforms already contain their chromatic adaptation. Neither
`chad` nor its inverse is applied again in this bridge. Distinguish PCS D50,
adapted `wtpt`, and reconstructed measurement white. Do not assume every v2
display's white is D65 or silently repair arbitrary legacy profiles. The supplied
sRGB2014 follows the documented D50 convention; other v2 conventions need
explicit fixtures and a documented policy, not filename heuristics.

Maintain separate legacy mft Lab/XYZ and modern mAB/mBA PCS adapters. Tag format,
not version alone, selects the legacy Lab encoding. Add non-neutral one-way
checks and exact legal white endpoints; a clamped white round trip cannot validate
scales. Perceptual reference-medium and mixed-v2/v4 linking policy must be tested
separately from numeric PCS encoding. Do not introduce undocumented BPC or CAT
steps to force agreement with an oracle's default behavior.

### Explicit clamp and domain contract

`clamp=true` preserves the legacy normalized-device behavior: clamp finite device
inputs before forward curves, handle out-of-range inverse inputs according to the
bounded ICC model, and quantize only at U8/U16 boundaries. Include Gray as well as
RGB inverse paths. Reject NaN/Infinity; do not turn non-finite results into black.

`clamp=false` is a float carrier with a strict supported-domain contract, not an
HDR/negative-value extension to every ICC curve or LUT. Do not silently clamp,
linearly extrapolate table ends, square negative gamma inputs into positive
values, or return a binary-search endpoint for an unreachable inverse value.
Reject device inputs outside the supported normalized domain and reject a stage
input that would require implicit range clipping. Check inverse-curve image
ranges, including plateau behavior, before evaluation. Profile-defined constant
branches remain part of the curve, not an invented extension. Physical XYZ and
Lab values must not be mistakenly validated as normalized device channels.

Map a domain failure to an explicitly documented existing error category/message
unless a separate additive error-returning API is introduced; do not expand the
old exhaustive enum. The output slice is unspecified after an execution error
unless a separately budgeted transactional API is added. Genuine unbounded HDR
processing belongs to an explicit future domain-aware API, not this boolean.

### LittleCMS black-box acceptance plan

Keep LittleCMS outside product dependencies. Execute the external utility only;
do not inspect/copy its implementation. Store fixture provenance, permitted-use
information, complete SHA-256, tool version/options, units and expected point
counts in the ignored oracle corpus. Track the runner and synthetic generators,
not environment-specific paths or external profile binaries.

1. Calibrate transport first with black/white/neutral/primary sentinels. This
   utility's ordinary RGB and Gray values use 0..255, XYZ uses Y=100, and Lab
   uses physical L*/a*/b*. Do not assume Gray percentages. Use explicit
   `-n -c0 -d1` and one of `-t0`, `-t1`, `-t2`, `-t3`, with the same
   source/destination bytes and no `-b`.
   Do not enable `-q`, encoded integer mode or global `-s` for physical Lab/XYZ;
   bounded mode in this utility can clip Lab L*=100 to 1. Convert device units
   explicitly and apply only the agreed device-boundary policy.
2. Fix complete observer adaptation (`-d1`) for the ICC.1:2022 normal-PCS gate.
   `-d0` produces a different measurement-white result and is a separate
   diagnostic, not the expected default. Use dissimilar-white synthetic profiles
   to test Absolute; the three official D50 `wtpt` values alone cannot detect a
   missing bridge. Never infer intent correctness from self-roundtrips alone.
3. Run RGB `r,g,b = k/16` (4913 points) and Gray `k/4095` (4096 points), endpoints
   included, for each declared route/intent. Cover Gray<->RGB, all four matrix/LUT
   pairings, XYZ/Lab, mft1/mft2/mAB/mBA, and both stored directions. Add independently
   generated Gray profiles and synthetic tags with distinct 0/1/2 transforms.
   The official v4 files exercise saturation fallback, not a genuine 2-tag route.
4. Compare full device-to-device outputs, then convert both result sets through
   the same independently fixed destination-to-physical-Lab measurement path.
   For one-way compiled-stage tests, use a verified PCS bridge; do not assume
   transicc's built-in Lab profile is a raw tag evaluator. Check black/reference-
   medium handling explicitly, especially for perceptual and mixed-v2/v4 paths.
   Keep direct channel/analytic errors alongside DeltaE00 so a lossy measurement
   path cannot hide a channel error. Out-of-domain failures are reported and must
   not be dropped from a required bounded-mode grid.
5. Implement the metric independently and verify the 34 published
   [Sharma/Wu/Dalal test pairs](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/)
   to their printed precision, plus identity/symmetry and zero-chroma cases.
   Do not copy their program or LittleCMS internals. Keep downloaded validation
   data in ignored test_data with its provenance/terms recorded.
6. Enforce the existing H3 thresholds separately per pair/direction/intent:
   analytic matrix/TRC absolute error <=1e-5; LUT DeltaE00 median <=0.1,
   p95 <=0.25, maximum <=1.0. Specify percentile calculation and rounding once.
   Record the worst input and exact number of compared values. Missing oracle,
   malformed output, missing points or unsupported required routes keep the gate
   incomplete. Investigate interpolation/PRM differences rather than relaxing
   limits, enabling BPC, or replacing profile values to obtain a pass.

The sentinel probes establish transport/settings only. The complete 4913/4096
oracle grids and the above error thresholds have not yet been accepted.

### Luna implementation order and review checkpoints

1. Add structural `parse*`, immutable raw metadata and compatibility adapters;
   prove that a forward-only curve is readable without inverse compilation.
2. Add direction-specific compilation and route diagnostics. Test unused invalid
   inverse curves, constant/non-monotone forward curves, inverse plateaus, Gray
   XYZ/Lab, singular destination matrices and malformed selected tags.
3. Move compiled payloads behind Arc; fix wrapper arithmetic/reserves and worker
   isolation. Add allocation-count/limit/failure and multi-worker equality tests.
4. Implement 0/1/2 precedence, real four-intent behavior and the shared physical
   PCS white bridge. Cover unequal whites, missing tags, class/version rules,
   all matrix/LUT pairings, and no double CAT. Replace tests that currently assert
   nonexistent A2B3/B2A3 semantics; do not count blanket Unsupported as support.
5. Enforce the explicit clamp/domain policy, preserving the six closed regressions
   and adding Gray, descending/flat inverse, negative/high finite and non-finite
   boundary cases. Keep integer wrappers consistent with the F32 core.
6. Connect and validate the black-box runner and metric, run complete grids and
   retained regressions, then request independent H3 review. Native/Wasm/Miri,
   API compatibility and oracle accuracy are separate acceptance evidence.

## C1 shared-parser refactor: first bounded still slice

This plan closes the parser allocation boundary first, without redesigning the
whole codec. Ordinary still is an explicit temporary support boundary, not a
claim that C1-C3 are complete. Existing public structs and legacy entry points
remain unchanged. Do not add another BMFF parser or ambient/global budgets.

The independent metadata regression now accepts the correct low-budget error and
checks that a sufficient budget preserves all three ICC projections and their
capacities. That regression passes. A separate owned-storage check still finds
under-accounting of vector/record storage. Multiplying ICC payload size by three
does not account for the shared parser, intermediate copies or typed containers.

### Shared callgraph and minimal signatures

Keep `parse_avif_with_metadata(data)` as a compatibility wrapper. Introduce one
context-bearing core and use it from both legacy and native entry points:

```text
parse_avif / parse_rich_info / old decode
  -> parse_avif_with_metadata -> ParseContext::legacy
parse_native_info / bounded decode
  -> ParseContext::native_still(limits)
both -> parse_avif_with_context
          1. parse_container_state: checked boxes -> bounded tables/properties
          2. validate/classify: primary, associations, selected alpha, extents
          3. materialize: payloads + legacy/rich/ordered metadata projections
```

Suggested private signatures (return types abbreviated):

```rust
parse_avif_with_context(data: &[u8], ctx: &mut ParseContext<'_>) -> Result<ParsedAvif>
parse_ftyp(data: &[u8], ctx: &mut ParseContext<'_>) -> Result<Brands>
parse_meta(data: &[u8], header: BoxHeader, state: &mut MetaState,
           ctx: &mut ParseContext<'_>) -> Result<()>
parse_iinf(payload: &[u8], ctx: &mut ParseContext<'_>) -> Result<Vec<ItemInfo>>
parse_iloc_with_indexes(payload: &[u8], ctx: &mut ParseContext<'_>) -> Result<Locations>
parse_ipma(payload: &[u8], ctx: &mut ParseContext<'_>) -> Result<Associations>
merge_ipma(target: &mut Associations, incoming: Associations,
           ctx: &mut ParseContext<'_>) -> Result<()>
property_record(property: &ItemProperty, ctx: &mut ParseContext<'_>)
    -> Result<NativePropertyRecord>
item_payload(data: &[u8], state: &MetaState, id: u32,
             ctx: &mut ParseContext<'_>) -> Result<Vec<u8>>
```

Propagate the same context through `parse_meta_children`, `parse_iprp/ipco`,
`parse_infe/read_c_string`, `parse_iref/grpl/altr`, `parse_pixi/auxc/colr`,
`item_metadata`, `primary_property_records`, `collect_color_information`,
`alpha_auxiliary_items_for`, and every nested reserve/copy they perform. Small
scalar readers stay shared and allocation-free. Private legacy helper wrappers
may construct Legacy context for existing tests/callers; a native call must never
reach a wrapper that resets its context or creates a fresh allowance.

The core must finish structural classification before materialization. NativeStill
rejects `moov`/sequence, selected non-`av01` primary/alpha, derived composition and
unsupported `iloc` construction method 2 before `parse_sequence_tracks`,
`effective_primary_item_id`'s derived fallback, grid construction or recursive
payload assembly. Recognize a late `moov` before any sequence payload allocation.
Legacy mode keeps existing AVIS/derived behavior and error classifications.
Method 0/1 still extents may be supported after complete range checks. Parse-only
still metadata continues to have zero movie samples, including at max_frames=0.

Replace `scan_native_limits -> parse_avif_with_metadata` with this shared flow.
A non-owning box/count prepass using the same readers can remain as an early
optimization, but is not the authority for allocation safety or property semantics.

### Context, allocation helpers and ownership accounting

`ParseContext` owns `ParseMode::{Legacy, NativeStill(&NativeDecodeLimits)}`,
cumulative structural counters, current/peak byte ledgers and test-only failpoints.
Pass `&mut ParseContext`; no global allocator state, thread locals or locks are
needed. Legacy mode applies no new user limits. Native mode checks input length
before even allocating the compatible-brand vector.

Use separate ledger classes: Metadata, ICC (a subset of Metadata), and encoded
Payload. Metadata includes parser tables, Vec element storage, names, properties,
projection buffers and temporary metadata; payload includes owned idat/master/
alpha copies. Do not substitute encoded box length for any of these capacities.
Keep live and peak accounting separate from final retained bytes.

Keep the existing 12-argument limits constructor source-compatible. Add private
parser-limit fields through an additive builder if separate payload/live/box/
extent/reference limits are required. The initial finite defaults can derive
payload allowance from max_input_bytes and total parser allowance from the
checked sum of payload and metadata allowances; validate overflow before parsing.
Document these defaults and expose getters. Never silently invent an unlimited
parser budget or reuse max_plane_bytes for encoded payloads.

Implement shared fallible helpers with explicit allocation class and context:

- `try_vec_exact<T>(count, class, ctx)`: check count and count*size_of::<T>(),
  pre-charge the requested layout, then try_reserve_exact; reconcile reported
  capacity immediately. Roll back the reservation on failure. No push may grow
  the vector outside a checked helper. Account Rust-visible capacity, while
  explicitly excluding OS allocator overhead from the memory contract.
- `try_grow_vec<T>`: pre-charge simultaneous old and replacement capacity before
  growth; a fresh fallible buffer plus moves gives a clear peak bound. Release
  old capacity only after its allocation is dropped. For known counts reserve
  once, not once per element; any geometric growth target is itself budgeted.
- `try_copy_bytes`, `try_copy_string`, `try_clone_pixi`, `try_clone_color` and
  `try_property_record`: reserve then copy every nested owned member. Preserve
  existing UTF-8/lossy-string semantics with a fallible builder. Never call
  infallible Clone/to_vec/collect inside these helpers and charge afterward.
- Transfer a reservation with an ownership move; release only on actual drop,
  not clear(). Retain source charges during destination construction. Failed
  parse scopes drop partial results and release their reservations; no partially
  populated native result escapes. Fixed-size error reporting/allocator internals
  are not grounds for a process-wide no-OOM claim.

Required injection sites and checks:

| Existing helper group | Check before allocation |
| --- | --- |
| `parse_ftyp`, `parse_infe/read_c_string`, `parse_auxc` | Brand/name bytes and capacity; bound actual encoded string output |
| `parse_iinf`, `parse_iloc_with_indexes` | Existing minimum-payload checks, cumulative records, three iloc tables, nested extent/index capacities; zero-width fields still need count limits |
| `parse_iref/grpl/altr` | Reference/group records, target-ID arrays and total entries across boxes |
| `parse_ipma/merge_ipma` | Cumulative record/association counts before reserve/merge; preserve source order and duplicate rules |
| `parse_ipco`, `parse_pixi/colr`, av1C and idat branches | Property vector slots plus nested typed/payload capacities and ICC subset bytes |
| `validate_primary_item_metadata` | Use fixed flags for finite property kinds; budget unknown-color seen sets or compare borrowed entries without allocating |
| `item_payload/append_item_extent` | Validate every checked start/end and total output length against source/limit before reserve; append only after one authorized reservation |
| Projection builders and alpha collection | Every copied Vec/String, outer record array and simultaneous parser/result ownership |

Maintain counters per table kind: iinf and iloc may each describe the same N
items, so do not incorrectly add them together as 2N unique items. Within a kind,
counts are cumulative across boxes, including replaced tables. Property definition
and association limits are separate counters even if the first policy uses the
same numeric cap. Metadata byte budgets cover all their simultaneous storage.

### Eliminate duplicate copies without losing API data

Change `item_color_information` to a borrowed iterator over validated associated
properties. Feed it directly into `collect_color_information`; remove the
temporary `Vec<ColorInformation>` and its deep clones. Choose the legacy nclx-
preferred property before copying, preserving the exact legacy selection rule.
Construct legacy, rich and ordered projections from the same parsed property set,
using fallible copy helpers. An ICC-only result must still preserve the entire
ICC in all three existing projections. Do not clear a compatibility field or
replace owned data with an empty Vec to pass the budget test.

With the existing owned MetaState, its original ICC buffer remains live while
the three result copies are built: account that fourth allocation unless an
explicit final ownership move removes it. A successful move may reduce copies,
but must not invalidate other item associations. Nested pixi/config/unknown-colr
arrays and record capacities are included, not only ICC bytes. Drop MetaState
only after all projections are complete; final `metadata_bytes()` reports the
retained allocation total, while an additive statistic may expose parser peak.

For payloads, first validate all extents without copying, then reserve the total
and append within capacity. Count idat plus assembled payload when both are owned.
A later shared borrowed-range representation can remove idat copying, but is not
required for the first safe implementation. Unsupported recursive method 2 must
stop before the current recursive allocator. Selecting alpha items must precede
their payload creation; do not eagerly copy every auxiliary payload then reject.

### Ordered implementation and acceptance slices

1. Move the new machinery into `container/budget.rs`, `container/boxes.rs`,
   `container/items.rs`, `container/properties.rs`, `container/projection.rs` and
   `container/payload.rs` by responsibility. Keep public types/wrappers in the
   existing facade. Thread context and split phases without changing legacy
   results. Existing sequence/grid code can remain in place, unreachable from
   NativeStill; do not grow another large all-purpose module.
2. Replace all structural Vec/String reserves with shared helpers. Test limits
   before allocator failpoints for iinf/iloc/ipma, many properties, extended pixi,
   strings, repeated boxes, zero-width extent fields and invalid extent ranges.
3. Replace projection/auxiliary copies and remove the temporary color vector.
   Test low-budget rejection, a budget between encoded and retained size, full
   high-budget data preservation, and actual nested capacities. Include an exact
   boundary below the retained total and a budget sufficient for output but not
   simultaneous MetaState+output. A post-return sum alone is not this test.
4. Verify the NativeStill callgraph cannot reach unrestricted sequence/grid/
   recursive payload helpers. Keep tracked 13 regressions, the corrected external
   cases, legacy library tests, Miri/i686/WASI, and allocation-failure checks as
   separate evidence. Review the parser slice before claiming parser bounds.
5. Parser acceptance alone does not accept bounded decoding: split AV1 header/
   dimension validation from tile/entropy materialization, replace allocating
   OBU collection with a shared iterator where only inspection is needed, and
   check actual show_frame/show_existing and master/alpha plans before reserve.
   The current header parser copies tile payloads before the caller checks its
   decode plan; alpha-info creation also clones payloads before validation and
   is repeated. Use borrowed parts or budgeted copies and reuse validated alpha
   headers. Until this boundary is complete, keep C1 NO-GO. C2 still owns the full
   decoder live-state/reference/filter budget; C3 owns transactional AVIS.

## Stage A/B re-review: allocation and limited-checkpoint contract

The independently reviewed parent snapshot, linked to standalone AVIF checkpoint
`a55753e`, passes 20 of 25 external/embedded boundary tests. Five remain open:
an ICC with length 1 and capacity 4096 bypasses an ICC-byte limit of 64; a
264-byte descriptor allocation succeeds under frame/live limits of 128;
three 4096-byte ICC copies precede rejection of a frame-byte limit of 1;
pixi precision 10 conflicts with native AV1 precision 8 without rejection; and
valid but unavailable ordered geometry is classified as malformed rather than
unsupported. The ICC-capacity failure also reproduces without the AVIF feature.
These results do not supersede the already-approved typed foundation.

Real decoded 64x64 monochrome, native vector-pointer identity, ordinary 16-bit
rejection and legacy-projection retained capacity checks pass. Highres-only
typed/safety/Stage-A tests pass 9+1+6 locally; the same 16 have separately passed
Miri, i686 and WASI. Default and default+highres library tests each pass 30.
Legacy constructor signatures and existing Eq traits remain intact. These are
scope-limited compatibility results, not bounded-allocation or full-B evidence.

The next repair is one shared construction boundary, not extra final counters:

1. Add checked, non-allocating owned-size helpers for ICC capacity, descriptors,
   plane vectors, both copies of layout offsets, channel roles, sample vectors,
   colour metadata and tags. Apply the separate ICC limit to retained source and
   active storage. Preserve existing public constructors and metadata variants.
2. Before any bridge copy, inspect borrowed native frame/metadata into a small
   fixed-size plan. Check dimensions, channel IDs/counts, layout, precision,
   pixi agreement and all relevant byte limits. Count the simultaneous native
   outer vector and samples, borrowed rich projections, destination descriptors,
   metadata and active colour; include temporary copies until actually dropped.
3. Build through private fallible, budgeted helpers for vectors, layout/role
   storage, colour provenance and metadata copies. Share validation with legacy
   constructors; use fixed-size role validation instead of a temporary Vec.
   A private frame constructor taking already-built metadata can avoid the
   redundant default metadata clone. Allocation failure must retain the new
   processing boundary's allocation classification, not become malformed input.
4. Remove EXIF display-string formatting from owned-byte validation. Walk owned
   EXIF storage without copies, or explicitly reject that unsupported variant
   only in the new limited-validation API until an exact walk is available.
   Existing EXIF/tag storage and formatting APIs remain unchanged.

Byte accounting covers Rust-visible retained capacities and their construction
peaks, with checked element-size multiplication. It must not claim to bound OS
RSS, allocator headers/rounding, or opaque HashMap bucket/control overhead merely
from `capacity * size_of::<entry>()`; these exclusions need explicit API wording.
No allocation attempt may knowingly exceed a declared visible-storage budget.
Reconcile actual returned capacities before filling/copying when they exceed the
planned reservation, and test failpoints as well as successful final totals.

A geometry-free ownership subset may become a limited checkpoint after these
repairs, while full B/D remain open. The RichAvifInfo adapter cannot prove absence
of lost pasp or recover original clap/irot/imir order. Reject known unavailable
geometry as Unsupported and do not call its empty ordered lists authoritative.
Proof of a geometry-free input, or complete raw ordered forwarding, requires an
internal view from NativeAvifInformation's authoritative ordered properties and
exact pasp presence. Do not infer original order from rotation/mirror fields or
apply geometry to the transferred samples. Public byte/sequence APIs remain
deferred behind C1-C3; the current allocation-bound A/B checkpoint is NO-GO.

## C1 context-wiring review: foundation only

The shared-context wiring is accepted as a refactor foundation, not a completed
parse/classification phase or C1 allocation boundary. NativeStill uses the shared
core without an identified call resetting it to Legacy. The late-movie and
derived-primary checks reject before the master payload copy. Library tests pass
488 with 6 ignored; native-boundary 6, native-limits 5 and rich-color 3 pass.
The flat private `container_budget.rs` module with an explicit path attribute is
a valid small responsibility boundary; it does not waive further module splitting.

The extended external harness passes 6 of 9 tests. Remaining failures are:
reported metadata 12324 below a retained-capacity lower bound of 12498; one
4096-byte master copy before unsupported derived-alpha classification; and one
65536-byte payload reserve before rejecting an out-of-file extent. The latter
two are allocation observations, not conclusions inferred only from an error.

Proceed with the existing substeps 2/3, addressing these concrete gaps:

- Replace the mode-only Copy context with one mutable Metadata/ICC/Payload
  ledger and cumulative table counters. Current context reserve helpers provide
  fallibility only; they do not debit limits. Reserve known arrays once rather
  than repeatedly requesting exact growth for each property/association.
- Classify selected master/alpha items and validate all extent ranges before
  master payload creation. Keep legacy ordering/error behavior in Legacy mode.
- Remove the unchecked colour clone/collect in `item_color_information`, pixi
  Clone in `property_record_with_context`, unknown-colour push, auxiliary ID/
  string collection, validation scratch Vecs and payload-stack push. Reuse the
  context-aware clone helpers or borrowed/fixed-size views, with live ownership
  charged until actual drop. Do not reset allowance in nested wrappers.
- Correct the tracked native-boundary fixture's zero item offset. Its reduced-
  still test must reach the intended AV1 payload and assert the expected decode
  path; merely accepting any error other than show-existing is insufficient.

The known retained-capacity regression and these phase/allocation failures remain
open. No full C1, deep decoder C2, transactional AVIS C3, or Abort execution gate
is closed by this foundation review.

## ICC structural-parse review: stage 1 still open

Independent structural-only tests confirm that non-monotone forward curves and
unusable matrix tags no longer force inverse/transform compilation in `parse*`.
Encoded profile/tag limits and out-of-range tables are checked, exact shared
tag ranges are accepted, raw header bytes survive, and Profile clones share
ownership across threads. These tests deliberately do not use the new compile
scaffold as evidence for structural parsing.

Five of seven new external checks pass. Two prevent stage-1 acceptance:

- A valid stored `chad` yields Some through the eager constructor but None
  through `parse*` followed by the same existing chromatic-adaptation accessor.
  `wtpt` has the same uncompiled-versus-absent representation problem internally.
- A 4096-byte profile is copied into both a Vec and an Arc slice; the second
  full-size allocation bypasses fallible reserve. The observer sees the 4112-byte
  Arc allocation in addition to the original payload, not just a fixed control
  allocation. Preserve a Vec directly inside the outer shared Profile storage.

Keep structural parsing independent of unused semantic errors. Read chad/wtpt
lazily from checked raw ranges with stack-only checked helpers returning
`Result<Option<_>>`; absent tags and malformed tags must remain distinguishable
to new checked accessors/compilation. Existing accessors must still return valid
stored metadata, and old constructors retain eager validation/error behavior.
Do not eagerly compile transforms merely to populate these metadata values.
Version/class bytes are retained, but their explicit interpretation and selected
route metadata remain required before direction-specific acceptance.

The current all-target suite passes 149 tests with 1 ignored, including 21 LUT
tests; the existing external seven-regression suite also passes. The optional
official v4 fixture was present in this run. These passes do not close the two
new structural/ownership failures or provide LittleCMS accuracy acceptance.

Keep the added CompiledProfile/TransformLimits surface classified as stage-2/3
scaffolding: LUT parsing precedes its compiled-byte check; decoded curve/CLUT
storage and aggregate peaks are not correctly charged; the new entry-count
limits are not wired; and matrix parsing still runs after selecting a LUT route.
Old Transform/worker sharing and wrapper pre-allocation checks remain unfinished.
Select exactly one supported route before compiling it, pass limits into each
allocation, and keep the intent/Absolute/domain/oracle gates open. Stage 1 and
full H3 remain NO-GO; this review changes no product acceptance checkbox.

## Stage A/B follow-up: original five closed, budget checkpoint still open

The frozen repair passes the previous 30 external/embedded checks, including
the five failures recorded above. It removes the temporary frame-metadata clone,
checks pixi/native precision and sample range, preserves real monochrome mapping,
and rejects unavailable ordered geometry as Unsupported. EXIF owned-size checking
no longer allocates a display string; the new limited boundary rejects that
unsupported variant without changing legacy metadata representation.

Four additional checks within the same allocation contract fail (30 pass, 4 fail):

- Width/pixel/channel/plane/plane-byte limit failures still allocate respectively
  1/2/2/1/2 copies of a 4096-byte ICC payload before rejection; all must be zero.
- An incoming native outer Vec retains 229376 bytes while a 65536-byte live
  budget succeeds. Its capacity remains live during destination construction.
- A legacy ICC projection of length 1 and capacity 4096 bypasses max_icc_bytes 64.
- A frame with 2243 unique owned bytes is rejected at that exact frame/live limit
  because descriptor-owned active colour is also added through metadata totals.

Implement the remaining repair in three separately verified substeps:

1. **Accounting and ordering.** Introduce private, non-allocating
   `NativeMapPlan::inspect(&DecodedFrame, &RichAvifInfo, &ResourceLimits)
   -> Result<NativeMapPlan, ProcessingError>` with fixed-size plane decisions.
   Validate all applicable limits, layouts, precision/pixi, ICC kinds and known
   unsupported geometry before `frame_metadata` or destination allocation.
   Check every retained ICC projection by payload capacity. Separate final frame
   ownership from construction peak: final bytes are descriptor-owned bytes
   (already including active colour) + pixel-owned bytes + source metadata;
   the metadata sublimit still includes source and active colour. The live plan
   additionally includes native outer capacity, borrowed rich projections and
   temporary destination owners; moved sample storage is counted only once.
   Expand the parameterized limit-observer test across all bridge-relevant
   limits, not only the five currently failing cases, and require rejection
   before any payload copy. Preserve exact-limit acceptance for unique owners.
2. **Fallible construction.** Execute the validated plan through one private
   construction ledger. Add budget-aware planar-layout and owned-layout-copy
   helpers, reserve colour provenance before setters, and route metadata copies
   through typed `Result<_, ProcessingError>` helpers. Reserve fallibly, reconcile
   actual capacities against the same ledger before filling, and retain owners
   in live accounting until drop. Remove infallible `vec![0]`, layout Clone and
   provenance growth from this bridge; do not change public legacy constructors.
   Allocation failure remains `ProcessingError::Allocation`, not a String later
   mapped to Malformed. Retain `ImageFrame::from_parts` to avoid a third colour
   copy. Add allocation failpoints and near-exact peak tests alongside all four
   regressions; a successful final-size check is not proof of the peak bound.
3. **Feature gating.** Gate AVIF-only private constructors/helpers appropriately.
   Parent highres all-target checking passes, but new AVIF-off dead-code warnings
   for `PlaneDescriptor::try_planar` and `ImageFrame::from_parts` remain mandatory
   fixes; unrelated baseline Clippy failures do not excuse these new warnings.

Each substep must retain the previous 30 checks. This is still NO-GO for the
allocation-bound A/B checkpoint; the approved typed foundation is unchanged.
Full B/D still require authoritative native ordered properties/pasp presence and
the C1-C3 gates. Legacy callback tests pass seven with real fixtures, but their
drawer always continues: Abort execution and full H6 remain unverified.

## H4 implementation slices: explicit frame conversion

This narrows the approved contract into implementation steps; it neither replaces
that contract nor closes H4. Parent A/B ownership/accounting repairs and shared ICC
direction/limit acceptance remain dependencies. No product changes or new oracle
passes are claimed here. Use only `highres`; no AVIF dependency is needed to convert
an already-owned typed frame. Byte decoding, geometry application and encoding
are separate. Never call the legacy RGBA8/RGBA16/display conversion path.

### Additive surface and internal ownership

Add these highres entry points, using existing frame/error/limit types:

```rust
convert_frame(source: &ImageFrame, options: &ColorConvertOptions<'_>,
              limits: &ResourceLimits) -> Result<ImageFrame, ProcessingError>
quantize_frame(source: &ImageFrame, options: &QuantizeOptions,
               limits: &ResourceLimits) -> Result<ImageFrame, ProcessingError>
```

`ColorConvertOptions` is a checked private-field type with an explicit destination;
its initial output is planar F32 Gray/RGB, with alpha when present. The destination
is checked ICC profile bytes (Gray/RGB), encoded CICP RGB, or RGB with declared
primaries and one existing linear SampleDomain. Borrow ICC bytes in options and
parse/compile them under limits, rather than exposing unfinished ICC internals.
Source selection is `ActiveMetadata` by default, or an explicit ICC/CICP
interpretation. Keep native sample encoding (range/matrix/chroma location), source
domain, rendering intent, alpha policy and any white-adaptation policy separate
from source ICC/CICP selection. Unsupported option combinations fail explicitly.
Use checked builders for `SourceInterpretation::{ActiveMetadata,Icc,Cicp}`,
`NativeSampleEncoding`, `ChromaLocation`, `WhiteAdaptation::{RequireSameWhite,
Bradford}` and `AlphaPolicy::{PreserveAssociation,ChangeAssociation}`. The change
policy carries input/output association, multiplication domain and zero-alpha
policy; do not make it executable until its dedicated slice passes. CICP-only
routes support the explicitly documented relative-colorimetric policy first;
other requested intents are Unsupported, not silently ignored. ICC route
selection keeps all four intents delegated to the reviewed shared CMS.
There is no unlimited ResourceLimits default and no implicit integer output.

Put orchestration/options in small `highres/convert/{mod,options,plan}.rs` modules,
sample access/range/chroma/matrix in `native.rs`, CICP/primary math in `cicp.rs`,
the shared-CMS boundary in `icc.rs`, alpha operations in `alpha.rs`, and explicit
integer output in `quantize.rs`. Reuse `domain.rs`, `hdr.rs` and the repaired
ownership ledger; do not create another PixelBuffer or competing CMS. Split a
module further by responsibility when needed instead of growing a giant file.

`ConversionPlan::inspect(&ImageFrame, &ColorConvertOptions, &ResourceLimits)` first validates
borrowed layouts/samples, resolves interpretation and estimates live ownership.
`compile_color_route` then compiles exactly one route with the remaining budget.
`execute` reads checked sample-unit strides into fixed-size pixel/row workspaces
and fills fallibly allocated output; no full-size intermediate RGB frame is
necessary. Include caller-retained source, options/profile owners, compiled CMS,
worker scratch, destination planes/layouts and metadata in the construction peak.
Reconcile returned capacities before filling. Share fallible constructors with
A/B; do not call public infallible clone/layout/provenance helpers internally.
Failure leaves the borrowed source unchanged and returns no partial output.

### Interpretation and state transitions

The internal plan distinguishes native code values, normalized encoded Gray/RGB,
relative linear RGB, absolute-nit RGB and HLG scene/display linear RGB. F32 is
storage, not a domain. Unknown required domain/primaries/transfer/range/location
is an explicit error unless the caller supplies that missing interpretation.

| Input state | Required stages | Next state |
| --- | --- | --- |
| Integer native Gray/RGB | meaningful-bit range expansion and role addressing | encoded Gray/RGB |
| Native YCbCr | range expansion, located chroma reconstruction, matrix | encoded source RGB |
| Encoded Gray/RGB with selected ICC | DeviceToPcs only, then explicit destination route | physical D50 PCS, then destination |
| Encoded RGB with selected CICP | one selected inverse transfer, then primary/white conversion | declared linear domain |
| Already-linear F32 | domain/primary conversion only | declared linear domain |
| Destination F32 encoded Gray/RGB | separately requested quantization | U8/U16 with explicit meaningful bits |

For Gray ICC input, keep one CMS channel; do not duplicate it into an RGB profile.
CICP Gray can become an explicitly declared neutral RGB triplet after its scalar
transfer. Alpha is never included in the CMS channel count. ICC is preferred only
from the active descriptor; an unsupported active ICC is an error, not an
automatic CICP fallback. An explicit CICP override records that decision but does
not bypass structural validation or choose different YCbCr range/matrix values.
ICC selection bypasses the CICP transfer/primary stage, not native reconstruction.
Already-linear samples must not be fed through an ICC device TRC again.

Preserve original ICC+nclx+AV1 bytes, geometry and timing in FrameMetadata.
Build a fresh active descriptor for the destination: only destination ICC/CICP
describes encoded output; linear output has its explicit domain/primaries and no
stale source ICC or native YCbCr matrix/range. Add a checked last-conversion record
with selected source/destination, intent, adaptation, alpha and quantization policy
using private fields/accessors; preserve existing Eq/public enum compatibility.
On a second conversion, consult the active descriptor/record, never rediscover the
original source profile from preserved metadata. Preserved source AV1/pixi flags
do not describe newly full-resolution RGB planes.

### Native precedence and reconstruction

Resolve each field independently: actual descriptor roles/layout/subsampling are
authoritative storage; explicit native options fill missing interpretation;
otherwise use validated active/container signaling, then available AV1 signaling.
Check two present known descriptions for agreement before selection. Reject
unresolved ambiguity; classify it as malformed only when a cited format rule
requires equality. Explicit CICP override is not permission to repair malformed
AVIF. ICC cannot supply missing YCbCr matrix/range or chroma siting. Source-only
metadata must not override an already-converted active description.

Use [H.273 (07/2024), 8.3 and 8.7](https://www.itu.int/rec/T-REC-H.273-202407-I/en)
for range/matrix and location meanings. For precision b, full-range luma/RGB uses
`code/(2^b-1)`; colour differences use `(code-2^(b-1))/(2^b-1)`. Limited range uses
offset/span `(16,219)*2^(b-8)` for luma/RGB and `(128,224)*2^(b-8)` for chroma.
Initially require b>=8 for limited-range coding. Never confuse the asymmetric
full-range chroma endpoints with a 256-based denominator. Preserve excursions
through reconstruction; reject a later unsupported transfer domain, not silently
clip. F32 encoded input must declare normalized component semantics; arbitrary
float code values are not inferred from meaningful_bits=32.

Start with identity/GBR and NCL matrices 1, 5/6 and 9. Identity uses explicit RGB
roles, not a YCbCr subtraction. BT.2020 constant-luminance, YCgCo, ICtCp and other
unimplemented matrices return Unsupported, never an NCL approximation. Initial
CICP primaries are 709, BT.2020 and P3-D65; transfers are linear, 709/2020, sRGB,
PQ and HLG. Implement each code's published domain/branch behavior, including
matrix-dependent sRGB/sYCC distinctions, rather than one generic gamma curve.
Other codes remain explicit unsupported cases, not full H.273 acceptance.

For 444 no chroma filter is needed; 420/422 use explicit phase and a documented
separable bilinear/edge-extension policy, with ceiling dimensions for odd sizes.
Do not claim a universal normative interpolation kernel. Known location metadata
must agree after translation to luma-centre coordinates; unknown siting requires
caller choice. Extended pixi numbers are not AV1 numbers: implement a proven
mapping or reject, without guessing. In [AV1 6.4.2](https://aomediacodec.github.io/av1-spec/av1-spec.pdf),
position 1 means (0,0.5), position 2 means (0,0); position 0 is unknown. These
correspond to different H.273 location code numbers. Subsampling factors come
from checked plane layouts, not chroma position or the AV1 monochrome flags.

### Domain, HDR, alpha and quantization boundaries

Reuse the existing scalar HDR functions, with independent frame-level vectors.
The implementation reference is [BT.2100-3 Tables 4/5](https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.2100-3-202502-I!!PDF-E.pdf).
PQ produces LinearAbsoluteNits (1.0 means one nit); HLG inverse OETF produces
HlgSceneLinear. HLG display conversion requires explicit peak/black/system-gamma
conditions and BT.2020 luminance-coupled OOTF, not per-channel powers. Apply the
specified black lift at the signal stage and retain conditions in the result.
`HlgDisplayLinear(conditions)` stores nit-valued components, not scene-relative
values relabeled with a display peak.
Check invertibility/domain of the derived lift as well as finite constructor
fields; handle zero luminance without NaN. Initial supported signal domains are
explicitly bounded, not a claim of all production-headroom behavior.

Keep relative/absolute/scene/display transitions explicit. No assumed SDR white
nits, HLG display peak or HDR-to-SDR mapping. The initial route table rejects
cross-domain normalization lacking a defined caller policy; it does not feed
nits into normalized ICC. RGB-primary changes use checked f64 matrices; different
whites need a named adaptation policy. An ICC PCS-to-CICP bridge is a separate
D50-to-destination-white operation, not a second application of profile chad.
Preserve finite negative/over-one linear results when the destination permits
them; destination encoding or ICC domain failure is not permission to clamp.

Default alpha policy preserves association. Straight alpha is copied/normalized
only as required by declared storage; transform hidden colour at alpha=0 normally
instead of zeroing or skipping it. Native decode still preserves original hidden
samples exactly. A bit-exact hidden-colour request across a changed colour space
is unsupported unless it can actually be satisfied. Premultiplied nonlinear
conversion is rejected in the first slice; the later explicit association slice
must name the multiplication domain and zero-alpha policy, then test reversible
nonzero cases. Homogeneous linear transforms may preserve association without
division. Do not silently unpremultiply, remultiply or colour-transform alpha;
[AVIF 1.2, 4.1](https://aomediacodec.github.io/av1-avif/v1.2.0.html#auxiliary-image-items-and-sequences)
also excludes auxiliary alpha colr from colour interpretation.

`QuantizeOptions` requires U8/U16, meaningful bits and a rounding policy. Initially
accept only full-resolution normalized encoded Gray/RGB plus alpha, not nit-valued
or HLG-scene integers. Use explicit nearest-ties-away for the first policy;
test half-way values and exact endpoints independently. Default out-of-range
policy is Reject; a separately requested ClipToUnit affects only this quantizer,
never earlier transfers, and is recorded as lossy. No hidden dithering, bit-depth
inference, cast saturation, tone map or automatic association change is allowed.

### LUNA order and independent gates

1. Add checked options, the private borrowed resolver/plan and route-table tests.
   Exercise ICC preference/explicit CICP override, missing/contradictory fields,
   repeated conversion, all domains and option errors without allocating output.
2. Implement native reconstruction and CICP F32 conversion with bounded synthetic
   U8/U16/F32 Gray/RGB/YCbCr planes. Cover padding/strides, GBR roles, 400/420/422/444,
   odd sizes, phase impulses, full/limited endpoints and encoded excursions.
3. Add PQ and HLG scene/display routes, then explicit alpha/quantization slices.
   Independent f64 formula tests cover each branch, primaries, inverses, finite
   negative/over-one linear values, PQ nits, HLG black/peak and chromatic OOTF.
   Require relative/SDR error <=1e-5 and HDR error <=max(1e-4 nit, 2e-5*reference)
   on declared domains; quantized codes follow the exact chosen rounding rule.
4. Wire ICC only after H3 directional/physical-PCS/no-clamp/limit APIs pass their
   own review. Keep that dependency behind `convert/icc.rs`; retain the pinned
   public source and ignored local-patch practice. A mock route proves dispatch,
   not real CMS support or patch-free consumer acceptance.
5. Use LittleCMS only as a black-box ICC oracle on RGB 17^3/Gray 4096, with H3's
   validated DeltaE00 thresholds; no LCMS claim for chroma or HDR transfer math.
   Use FFmpeg as a separate black-box reconstruction check with explicit range,
   matrix, location, filter and high-precision output. Compare exact geometry and
   phase first; freeze code-value tolerance per matching kernel before the run,
   rather than relaxing it after failures. Different kernels are diagnostic, not
   an ICC or exact-pixel oracle. No existing CMS/codec implementation is copied.
6. Run all-limit-before-copy tables, allocation failpoints/peak observations,
   source immutability and no-partial-result tests. Preserve source metadata bytes
   and test destination relabeling/re-conversion and alpha=0 separately. Execute
   highres-only with neither AVIF codec, legacy feature-off/on regressions, MSRV,
   i686/WASI and applicable Miri. Missing required oracle data leaves its gate open.

Review each slice before committing it. CMYK/MPE/BPC, gain maps, JXL, tone mapping
and animation encoding remain out of scope; no H4 checkbox is closed by this plan.

## C1 parser substeps 2/3 re-review: NO-GO

The previous nine external regressions now pass independently. The non-Copy
context is threaded through the shared core and nested colour/pixi copies; no
NativeStill-to-Legacy reset was identified on the inspected parse route. Independent
library execution passes 488 with 6 ignored, plus native-boundary 10, native-limits
5 and rich-color 3. The same 18 integration tests separately pass Miri, Rust 1.91
i686 and executed WASI; i686 also passes the 488-test library scope. These results
do not establish the parser allocation contract.

Nine added adversarial checks fail, giving external totals of 9 pass / 9 fail:

| Priority | Reproduced parser-boundary defect |
| --- | --- |
| P1 | Growth from 8 to 16 bytes is attempted under a 16-byte limit without allowing for simultaneous old/replacement storage; the required worst-case peak is 24. |
| P2 | Reserving one element into empty length with capacity 8 incorrectly requires budget 9. A public 4096-byte payload in a 4301-byte input likewise fails an 8192-byte precheck although append needs no new allocation. |
| P1 | An unsupported a1op, an invalid selected master extent with valid alpha, and a derived primary with idat each trigger one 4096-byte payload copy before the relevant rejection. |
| P1 | Two one-item iloc tables pass max_items=1 instead of sharing the cumulative per-kind limit. |
| P2 | Identical returned metadata reports 525 versus 12324 bytes when an unselected ICC is added; dropped parser owners and encoded-copy estimates contaminate retained accounting. |
| P2 | An idat-backed image with sufficient encoded-payload allowance fails metadata limit 2048 because encoded meta size 4283 is used as metadata; idat is also copied through the Metadata class. |

Close this same repair bundle in small steps; do not substitute extra post-return
tests for the allocation order:

1. In the shared reserve helper compute `needed = len + additional`; if it fits
   existing capacity, return without a reservation or charge. On growth keep the
   old owner charged while reserving the whole replacement capacity. A fresh
   fallible buffer plus moves makes the peak explicit. Reconcile actual capacity,
   fill/move only after approval, drop the old buffer, then release its charge.
   Allocation/check failures must leave the old vector and ledger consistent.
   Preserve Legacy behavior through its compatibility branch.
2. Add release/ownership-transfer accounting, rather than an ever-increasing
   live counter. Distinguish current, peak and returned retained totals. Finish
   projections while MetaState is live, then drop/release parser-only owners and
   report a checked retained walk. Treat idat as Payload, and remove encoded-meta
   size/three-copy guesses as the authority for retained or live metadata limits.
   Keep an encoded-byte scan only as a separately named bound/optimization.
3. Build one borrowed selected-item plan before materialization: validate primary
   associations and unsupported selectors, classify every selected alpha item,
   then validate every selected master/alpha extent and total size. Defer idat
   copying until that plan succeeds (a checked borrowed input range is enough
   during classification). Only then reserve/copy any selected payload. Keep
   legacy primary/sequence/metadata/alpha ordering unchanged; current shared-core
   alpha reordering needs restoration or paired legacy error-path verification.
4. Move table-kind counters into the same context. Accumulate repeated iloc/ipma
   entries as well as iinf/properties/associations; do not add different table
   kinds together as duplicate unique items. Charge/count before every reserve,
   including replaced tables. Retain all nine previous regressions and require
   all nine new checks to pass, with allocator failpoints and exact-budget cases.

Open static allocation audit, not additional independently reproduced failures:
the payload recursion stack still uses an unchecked push, and alpha ID sorting
uses an allocating stable sort without context accounting. Remove/budget these
native paths and add observers/failpoints; Legacy behavior remains unchanged.
Responsibility-based extraction of the structural/projection/payload helpers from
the enlarged container facade also remains required. The small flat budget module
alone does not complete that separation.

Substeps 2/3 therefore remain NO-GO. AV1 header/tile materialization (substep 5),
deep decoder C2 and transactional AVIS C3 are separately open, not reasons to
dismiss these parser-only failures. No general C1 or public bounded-decode
acceptance is granted by the passing library/portability/legacy callback suites.

## ICC stage-1 repair accepted; stage-2/3 re-review remains NO-GO

This review supersedes the two stage-1 failures recorded above, not the remaining
H3 gates. Profile now keeps its fallibly copied Vec inside the shared owner;
structural parsing no longer creates a second full-size Arc-slice copy. Valid
chad remains visible through both accessors, absent and malformed chad are
distinct through the checked accessor, and the lazy checked read allocates
nothing. The corresponding internal wtpt helpers retain the checked raw range
and use the same stack-only pattern. Structural parsing remains independent of
unused inverse/transform semantics, and legacy constructors retain eager curve
and optional-tag validation. Stage 1 is accepted for this limited repair scope;
class/version interpretation and selected route execution are not covered by it.

Independent execution passes all 153 existing all-target tests with 1 ignored,
including LUT 23, parse 2 and transform 9, plus the previous external boundary 7
and structural 7. Two added checks pass: forward-only Gray curve evaluation,
cheap CompiledProfile/Transform clones and worker creation, warmed worker/core
zero-allocation evaluation; and strict lazy chad absent/Some/error assertions.
The tracked valid-chad test still needs separate `Some(expected_matrix)` asserts
for both getters: comparing two Options after discarding the getter's value can
pass when both are None or the returned matrix is wrong.

Eleven added test cases fail in the following fixed repair groups (external
total: 16 pass / 11 fail). Allocation observations are requested allocation sizes,
not RSS; the retained Vec omissions below exist even excluding fixed Arc controls.

| Priority | Reproduced stage-2/3 defect |
| --- | --- |
| P1 | Both selected LUT directions still parse unused invalid matrix tags and return an XYZ-tag error. Separately, Transform construction rejects structurally parsed Gray profiles even when each direction compiles successfully. |
| P1 | Matrix compilation accepts a 1024-entry/4096-byte table under either a 128-byte compiled limit or a 2-entry limit. Transform construction clones that eager table twice before any matching check and also succeeds. |
| P1 | Two individually valid 49200-byte decoded LUT payloads succeed under one 50000-byte Transform budget. A single LUT at its exact float-payload limit also succeeds without charging outer curve/table Vecs or grid storage. |
| P1 | mAB and mBA with a truncated selected matrix allocate three 4096-byte curves before rejection. A Gray-header/RGB-LUT mismatch allocates its 49152-byte CLUT before rejection. |
| P1 | Invalid RGB input length 4096 still causes 32768-byte allocations in each integer wrapper and integer worker, or 16384 bytes in the F32 worker, before returning its length error. |
| P2 | A singular RGB matrix is correctly usable forward, but inverse compilation reports success and postpones its inevitable failure until evaluation. |

Implement this same bundle in small, independently verified slices:

1. Build a borrowed direction/route plan first. Validate supported device/PCS
   channels and every selected encoded range (including the 48-byte mAB/mBA
   matrix) before materialization. A selected LUT excludes matrix parsing and
   inverse validation entirely. Only a selected inverse matrix requires a
   nonsingular matrix and inverse-compatible curves, checked during compilation.
2. Make both Profile compilation and Transform assembly use the same selected
   compiler. Structural and eager Profile inputs must behave equivalently for
   their selected routes; do not clone the eager matrix cache before planning.
   Keep the old eager constructor's validation behavior without imposing it on
   the new structural parser.
3. Carry one non-resettable checked compile budget through both directions.
   Preflight matrix curves as well as mft/mAB/mBA data; count decoded Table/Para
   payloads, outer typed Vec capacities, grid storage, selected owned structures
   and live temporaries. Apply cumulative curve/CLUT limits, reconcile actual
   capacity and release temporary owners. A post-parse byte count or one full
   independent allowance per LUT is not sufficient. Keep fixed allocator/Arc
   bookkeeping exclusions explicit rather than excluding input-sized storage.
4. Share a checked buffer-shape/byte preflight across core, allocating wrappers
   and workers; reject invalid lengths before clearing/reserving/copying scratch.
   Add the planned pixel/output-byte/scratch limits (currently absent), checked
   output multiplication, fallible integer conversion buffers, and old-plus-new
   scratch growth accounting. Remove input-sized infallible collect/vec paths;
   also eliminate/budget the parametric-validation copy and LUT grid vec helper.
5. Retain all prior tests and all 13 new tests, add allocation-failure and exact
   cumulative-budget assertions, and fix the weak tracked chad assertion. Split
   the enlarged compile facade by direction compilation, assembly/evaluation,
   PCS helpers and wrappers instead of duplicating their planning logic.

Separate portability evidence remains valid: Miri and Rust 1.91 i686 execute
33 selected tests each; WASI executes 32. The official-file test is intentionally
excluded there, and WASI also excludes thread spawning. The host official v4
fixture was verified present and its finite-output test passes; this is not a
colour-accuracy oracle. Documentation generation also passes without warnings.

Stage 2/3 remains NO-GO independently of the already-open intent routing,
Absolute/media-white/chad semantics, Gray Lab coverage, unclamped domain policy
and LittleCMS RGB 17-cubed / Gray 4096 comparisons. Cheap Arc handles and passing
legacy/portability suites do not close the selected-compilation or allocation
failures. No full H3 or public high-resolution pipeline gate is closed here.

## C1 reserve repair substep 1: pre-allocation gate still open

Independent execution passes the four new tracked reserve tests. Of five scoped
external checks, spare-capacity reuse and the public payload append now pass;
three allocation observations still fail:

- P1: replacing capacity 8 with 16 under limit 16 allocates the 16-byte candidate
  before returning the old-plus-new budget error. The earlier observer therefore
  remains red; checking only the returned error and unchanged Vec is insufficient.
- P1: an 8-byte metadata allowance admits an 8-byte owner plus a 96-byte dynamic
  owner registry, while metadata_live reports only 8. The registry reserve is
  fallible but unbudgeted; it is input-sized Rust-visible storage, not allocator
  bookkeeping that may be excluded from the contract.
- P2: checkpointing 64 populated owners infallibly clones a 1536-byte registry
  outside the limits. This is a direct helper reproduction. The inspected shared
  parse currently calls checkpoint once at its normally empty entry, not at
  every reserve; no repeated-clone runtime claim is made for that public route.

Fix the reserve foundation before proceeding with the still-open retention,
classification and cumulative-count substeps. Do not add another uncharged
dynamic index to repair the first one:

1. Keep the context ledger scalar and non-Copy. Replace the pointer registry with
   a private move-only allocation token `{ class, charged_capacity_bytes }`
   attached to each parser-owned Vec (a private owned-buffer wrapper or embedded
   token in the private parsed owner). New empty owners start with zero charge;
   importing a preallocated owner requires an explicit checked adoption operation.
   Tokens move with the owner, are never inferred from reused allocation addresses,
   and are explicitly consumed on drop or transfer. Account token storage when it
   resides in an input-sized outer Vec. Public result types and Legacy APIs remain
   unchanged; this does not introduce a second parser or ambient state.
2. `reserve_owned(context, owner, additional)` checks `len + additional` first;
   spare capacity needs no new charge. Keep the old token live, precheck/reserve
   the full requested replacement bytes before calling `try_reserve_exact`, then
   reconcile actual capacity. Only after all checks succeed move the elements,
   drop the old Vec and atomically replace its token/charge. Allocation or budget
   failure drops the candidate and cancels only its provisional debit; the old
   Vec/token and existing ledger remain valid. Compute all class-counter updates
   before committing them, including Metadata+ICC, to avoid partial mutation.
3. Checkpoints contain only fixed-size scalar counters. Whole-parse rollback is
   allowed after the inner operation has returned and dropped all newly created
   owners; it must not pretend to undo mutations of live preexisting owners.
   Use token-specific cancellation for local allocation failure, not an owners
   clone. Preserve actual peak evidence separately from restored live counters.
4. Require all five scoped external checks plus the tracked tests to pass. Add
   allocator-failure, repeated growth and token drop/transfer tests; returning Err
   after the forbidden allocation does not satisfy the acceptance condition.

The raw pointer values in the current registry are used as accounting identities,
not dereferenced; this review does not claim a reproduced memory use-after-free.
Address reuse and missing owner-release coupling nevertheless make them unsuitable
as lifetime authority for the next retention substep. Substep 1 and general C1
remain NO-GO; the known substeps 2-4 and AV1-header/deep-decoder gates are unchanged.

## Parent A/B construction review: accounting repairs closed, B remains NO-GO

Independent execution passes the previous 34 external/embedded checks, including
all four earlier accounting/order failures, actual decoder Gray mapping and native
sample-pointer preservation. Highres-only typed 9, safety 1 and Stage-A 6 also pass
independently. The two AVIF-off private-helper warnings are gone; the two observed
draw-module warnings are pre-existing. Separate Miri/i686/executed-WASI evidence
covers those same 16 non-AVIF tests, not the AVIF construction ledger.
After the three tracked test additions, a final independent rerun gives 37 pass /
4 fail; all four failures are the construction observations below.

Four minimal added observations fail on the frozen product implementation:

| Priority | Fixed construction-boundary reproduction |
| --- | --- |
| P1 | The private ledger reserves 16 bytes under an 8-byte allowance before returning ResourceLimit. |
| P1 | The real consume-native-frame entrypoint passes its borrowed plan, allocates a 264-byte descriptor Vec under max_frame_bytes=887, then rejects that already-known overrun. |
| P1 | The metadata-taking entrypoint with a 4096-byte source ICC and frame allowance 6000 allocates another 4096-byte active ICC before its final frame validation rejects. |
| P2 | An actual allocator denial for the first AV1 provenance reserve becomes Invalid(InvalidMetadata), not ProcessingError::Allocation. |

The ledger currently charges only the two outer destination Vecs and selected
scalar increments. Initial metadata projection occurs before that ledger; layout
offsets, layout Clone, role storage and active-colour clone are not wired through
it. Source versus active metadata, borrowed rich storage, native outer storage and
moved samples must use one consistent ownership plan. Fixed header-size additions
are not substitutes for missing nested allocations; final accounting must agree
with the established unique-owned walk rather than double-count inline fields.

Close this fixed bundle in two slices, without reopening full B/D:

### B-1: shared allocation primitive, test first

Introduce a small private construction-allocation helper beside the AVIF mapper.
Keep the context explicit; no global production budget, pointer-owner registry or
input-sized bookkeeping clone is needed. Its first supported operation is a fresh
typed Vec allocation, matching both current outer-Vec callsites:

```rust
fn try_new_vec<T>(ledger: &mut ConstructionLedger, count: usize,
                  class: ConstructionClass) -> Result<Vec<T>, ProcessingError>;
fn try_copy<T: Copy>(ledger: &mut ConstructionLedger, source: &[T],
                     class: ConstructionClass) -> Result<Vec<T>, ProcessingError>;
```

Use named classes for frame-owned data, frame metadata, borrowed metadata and
temporary live-only storage; apply each sublimit only to its documented owners.
The required operation order is:

1. Checked `count * size_of::<T>()`, validated shape/count and all applicable
   frame/live/metadata/ICC limits, without modifying committed counters.
2. Reserve the complete requested debit while existing live owners remain
   charged. Only then call the fallible allocator. Allocation failure cancels
   that debit and returns Allocation, not malformed-input or metadata errors.
3. Reconcile the returned capacity with the same ledger before any fill/copy;
   handle a failed reconciliation by dropping the candidate and cancelling its
   debit. Commit all affected counters atomically; no partially modified ledger.
4. Fill within reserved capacity and transfer the returned owner/charge. A copy
   keeps its source live; a move transfers ownership without charging samples
   again. Empty/reused capacity performs no allocation. Do not silently reuse a
   fresh-Vec helper for growth: growth needs old-plus-full-replacement preflight,
   explicit old-owner release and failure-atomic behavior.

Replace the two outer descriptor/plane reserves with this primitive in B-1. Require
the private 16/8 and real 887/264 reproductions to pass, plus exact-capacity,
one-byte-under, zero-size/overflow and allocator-denial counter-rollback tests.
Review B-1 independently; the remaining metadata/error cases are still open until
B-2, so this helper checkpoint is not general construction acceptance.

### B-2: complete the existing callsite inventory

| Current callsite | Required wiring |
| --- | --- |
| Both consume entrypoints | Construct/check the same ownership plan and ledger before projection; the metadata-taking form must plan its active-colour clone and all nested output owners too. |
| frame_metadata / color_information / local try_copy | Pass that ledger through ICC, unknown-colour outer/payload, pixi/extended channels and provenance copies; include borrowed source owners until their actual lifetime ends. |
| AV1 provenance reserve/setter | Preflight capacity and return typed Allocation; setters may only append within an approved reserve. |
| PlaneLayout::planar and high_layout.clone | Use private budget-aware layout creation/copy; preserve public legacy constructors and eliminate infallible Clone on this bridge. |
| PlaneDescriptor::try_planar | Reserve role storage through the same helper and preserve typed errors. |
| try_clone_owned_bridge | Preflight and charge every active-colour nested owner, including provenance, unknown-colour arrays and payloads, before copying. |
| from_parts / final validation | Preserve single ownership of moved samples and avoid the removed temporary source-colour clone; final validation verifies the ledger result rather than discovering predictable overruns afterwards. |

The initially shared global Atomic failpoint has been replaced by a thread-local
hook and RAII reset during this review, removing the identified cross-test race.
It still only covers ledger reserves, not actual layout or colour allocations;
use an actual allocator observer as well. The newly added exact-peak and expanded-
limit tests inspect only the plan: they must also execute real construction with
allocation observers.
Keep the four fixed external reproductions; add no unrelated implementation scope.

The original accounting/order substep and AVIF-only cfg cleanup are accepted at
their narrow boundaries. B construction remains NO-GO. The authoritative native
metadata connection, ordered geometry/pasp, deferred Exif/derived-16-bit handling,
full B/D and H6 Abort/oracle execution gates remain separately open.

## C1 reserve token repair: old failures closed, three local defects remain

Independent execution confirms the previous 20-check result: 14 pass and six
known retention/classification/cumulative-count failures belonging to later
substeps. All five reserve/owner/checkpoint checks now pass. The raw-pointer
registry and dynamic checkpoint clone are gone; the context/checkpoint are scalar.

One new positive test covers Metadata, ICC and Payload tokens: moving the Vec and
its token together preserves ownership; growth from 8 to 16 gives live 16/peak 24;
allocator denial leaves the tracked Vec, token and complete accounting unchanged;
and spare-capacity reuse allocates nothing. Three other narrowly scoped checks
still fail, so reserve substep 1 is not yet accepted:

- P2: tokenless repeated reserve creates a fresh zero token each time. Already
  charged capacity 8 is adopted again, so a replacement of 16 incorrectly requires
  32 instead of 24 bytes. This pattern remains in unknown-colour accumulation,
  alternate groups and merged IPMA inner associations. ipco/iprp tokens are also
  local to parser calls while their target vectors live in MetaState across calls.
- P2: class validation occurs after the spare-capacity early return, allowing a
  Metadata token to report success for a Payload-class request without validation.
- P2: rejected growth of an untracked preallocation from 8 to 16 under limit 16
  now allocates nothing, but leaves metadata_peak changed from 0 to 8. Temporary
  adoption was committed before the combined preflight and not fully rolled back.

Keep the next repair within substep 1:

1. Persist each move-only token for the entire lifetime of the growing private
   owner, including state-backed vectors and nested IPMA associations; move it
   with the Vec across helpers. Complete the remaining repeated-call inventory.
   Make tokenless helpers explicitly fresh-only or eliminate them from growth
   routes; never reconstruct a zero token to resume an already-accounted owner.
   Regression fixtures must still accept actual old8+new16 at budget24, using the
   same token-bearing route as the repaired parser, not loosen the expected bound.
2. Validate token class and nonzero capacity consistency before the spare shortcut.
   Keep legitimate borrowed/preallocated adoption explicit rather than treating
   a mismatched or stale token as an empty owner.
3. Compute hypothetical adoption plus replacement and all class-counter updates
   before modifying committed accounting. A budget/allocation failure before a
   new owner exists must leave the original full ledger and token unchanged.
   Continue to preserve actual successful construction peaks when owners release.
4. Fix the new Clippy extend-with-drain finding with the equivalent pre-reserved
   move operation. Retain the previous five passing checks and the four added
   token checks, including all three allocation classes and allocator failure.

The six later-substep failures are not reclassified as substep-1 regressions.
Conversely, their deferred status does not waive these token wiring/atomicity
defects. No general C1 or decoder-bound acceptance follows from this review.

## Parent B-1 review: fresh callsites repaired; helper contract needs one closure

Independent execution reproduces 46 pass / 2 known B-2 failures from the existing
48 checks. Both B-1 failures are closed: the private 16-byte request under limit 8
and the real mapper's 264-byte descriptor under frame limit 887 now reject before
the observed allocation. Both production outer-Vec callsites start with Vec::new.

Two additional tests pass: fresh exact limits, real allocator denial with an empty
candidate and complete counter rollback, zero-count/overflow boundaries; and TLS
failpoint RAII restoration plus isolation from another thread. One new helper-only
test fails: an existing capacity-8 Vec is accepted by this nominally fresh helper,
and growing length/capacity 8 to 16 succeeds under live limit 16 after a 16-byte
reallocation request. General growth requires the old-plus-replacement peak 24,
not a capacity delta of 8. This is not an observed misuse at the two current
production callsites, but the shared primitive must not expose that false contract
to B-2 callers.

Keep the remaining B-1 repair local: either make the helper create/return its own
fresh Vec, or explicitly reject a caller Vec with nonzero length/capacity before
budget mutation. Allocate into a local candidate and publish it only after actual
capacity reconciliation succeeds. The current reconciliation-error branch restores
ledger counters but leaves the already-modified caller Vec allocated; that branch
is a static finding, not an allocator-capacity-overshoot reproduction on this host.
Do not add general growth support in B-1. Require the new fresh-contract test and
existing rollback/limit tests to pass before declaring the shared helper accepted.

B-2 remains unchanged: active ICC is copied before a predictable limit error and
real provenance allocation failure is misclassified. A tracked provenance-named
test currently injects only the first ledger failure, while the active-colour test
only checks Err; neither proves the actual allocation boundary. Inspect-only
exact-peak/limit tests also need real construction observers in B-2. No full B or
pipeline gate is closed by the two repaired fresh callsites.

## Parent B-1 fresh-only closure: limited GO

The helper now creates its local candidate and returns `Result<Vec<T>>` only
after checked preflight and capacity reconciliation. There is no caller-Vec
argument or growth/reuse API, so the earlier nonempty-owner misuse is excluded
structurally. Error returns drop the local candidate, and the inspected failure
branches restore the ledger. Both production outer allocations use this helper.

Independent final execution passes 49 of 51 external/embedded tests; only the two
previously assigned B-2 observations remain red. The private 16/8 and real 264/887
pre-allocation observers, actual allocator-denial rollback, exact/empty/overflow
boundaries and TLS/RAII isolation remain intact after the private-API test changes.
The new 8/9-byte boundary observer was strengthened to watch the requested size
in each case, so an accidental 9-byte allocation cannot escape that assertion.

B-1 is accepted for this fresh allocation helper and its two outer-Vec callsites.
Proceed only with the recorded B-2 wiring inventory. Active ICC pre-copy accounting,
provenance typed allocation errors, nested layout/metadata ownership and real
construction-peak observers are not accepted by this checkpoint. Full B/D and
the public high-resolution pipeline remain open.

## C1 token minimum-repair review: local contract still open

Fresh-only ownership with capacity zero and MetaState owner-lifetime tokens are
implemented. The four tracked token-budget regressions pass, including wrong
class rejection, complete scalar rollback after failed adoption, moved-owner
growth, and actual allocator failure. The external token-boundary fixture still
requires an equivalent update from the old tokenless repetition form to explicit
tokens; it has not been updated or rerun for the final fixes. The final
independent sol review has not run yet. Formatting and the tracked budget checks
pass for the repair set.

The reported 492-library-test and Clippy results predate the final two fixes and
must not be treated as final-diff evidence. The six known later-phase failures,
AV1-header allocation work, C2, and C3 remain outside this limited review and
remain open.

## Pause checkpoint

The user requested a pause at a clear checkpoint. Product code is frozen.

- B-1 has limited GO from sol review.
- B-2 has a luna report of 51/51 external checks, 53 product library tests,
  6 Stage A tests, and 11 highres-only tests passing. Independent sol review
  of B-2 has not run; full ownership inventory and actual near-exact
  construction observation remain pending.
- The final two C1 owner/token fixes are implemented; external fixture migration
  and independent sol review remain pending. C1 substeps 2–4, AV1 header work,
  C2, and C3 remain open.
- ICC stage 1 has limited GO. ICC stages 2–3 retain 11 independent failures.
- H4, encoder work, public integration, and oracle gates are incomplete.
- No new commit, version change, or push was made. Existing temporary
  harnesses remain available for resumption; saved samples and unrelated
  worktrees were left untouched.

Resume order: sol B-2 review, then C1 substep 1. Do not treat the reported
product/test counts as a full B acceptance gate until that review and the
remaining construction-ownership checks are complete.

## Resumed B-2 independent review: fixed inventory remains NO-GO

The existing 51 external/embedded tests independently pass with the fixed rich-
metadata AVIF baseline. Four additional tests in the previously approved ownership
inventory fail; the complete set is 51 pass / 4 fail. No product code was changed.

- P1, construction peak: a TLS/RAII real-allocator observer measures dynamic
  peak 747 and final dynamic ownership 579 on the plain three-plane fixture.
  The final owned walk is 595 because its two inline dimension pairs add 16;
  subtracting those non-heap values exactly reconciles the observer. Including
  the already specified fixed construction headers of 584 gives peak 1331.
  Actual construction succeeds at that exact limit, but also succeeds at 1330.
  Nested layout offsets, their clone and roles remain outside the ledger. This
  is an allocation/deallocation observation, not merely a plan-size comparison.
- P1, pre-copy limit parity: both entrypoints are exercised across width, height,
  pixels, channels, planes, plane bytes, frame bytes, live bytes, metadata bytes
  and ICC bytes. Nineteen cases reject before the watched ICC allocation. The
  metadata-taking entrypoint with ICC limit 1 copies a 4096-byte active profile
  before its final validation rejects it.
- P2, exact unique ownership: a retained source ICC with length 1/capacity 4096
  produces an active clone of capacity 1 under generous limits. Actual final
  source-plus-active metadata accounting is 4139, yet that exact metadata limit
  rejects the same construction because the future clone is predicted using
  source capacity again.
- P2, typed allocator errors: real allocator denial at the 8-byte layout offset
  and 1-byte role allocations returns InvalidLayout, not Allocation. The AV1
  provenance case fixed earlier passes, but does not cover these allocations.

Static inventory confirms that frame_metadata/colour/local copy helpers and the
active-colour clone still receive no ledger. AV1 provenance is reserved/set before
its delta is charged. PlaneLayout::planar and try_planar allocate independently,
and high_layout.clone is still infallible. Public colour setters used during rich
projection can also grow provenance without a checked bridge reserve. The latter
infallible paths were inspected, not deliberately driven into process abort.

### B-2 repair structure: wire owners, not another aggregate estimate

Move the accepted fresh allocation primitive and its explicit ledger into a small
private bridge allocation module, shared by mapping and metadata. Keep all public
constructors and compatibility APIs unchanged. Extend the ledger with explicit
frame-owned, metadata-owned and live-only classifications; validate per-profile
ICC capacity separately. Preserve the B-1 provisional debit, fallible allocation,
capacity reconciliation before copying, and complete rollback contract.

1. Both entrypoints perform the same allocation-free shape/configuration and
   complete limit preflight. Separate borrowed-source bytes, retained output
   bytes and planned fresh-copy lengths. The rich entrypoint creates the ledger
   before frame_metadata and passes that same ledger into construction; do not
   discard borrowed ownership by creating a new ledger inside the second entry.
2. Borrowed RichAvifInfo nested owners, including its independent legacy colour
   projection, remain live-only until the enclosing borrow ends. They are not
   final frame owners. Incoming native outer storage is likewise live-only until
   its actual drop. Samples are retained once and moved, never charged again as
   a destination copy. Owned input FrameMetadata is retained once as source.
3. Route ICC, unknown-colour outer/payload, pixi/extended-channel and provenance
   copies through checked fresh allocation/copy helpers taking that ledger.
   Fresh requests use length; actual returned capacity is reconciled. Retained
   input capacity remains fully charged. Prepare all source provenance before
   active cloning; setters may only fill already approved capacity. If an
   existing provenance owner needs replacement, precheck old plus the whole new
   owner and release the old charge only after dropping it.
4. Create both layout-offset owners and the role owner through the same helpers;
   use existing validated constructors with these preallocated parts instead of
   the infallible layout Clone. Pass the ledger into the bridge-only colour clone
   and every nested allocation it performs. Allocate the active colour as a fresh
   owner, never as a second reservation of source capacity. All fallible bridge
   allocation failures preserve ProcessingError::Allocation.
5. Transfer metadata, descriptor, planes and moved samples through from_parts.
   Reconcile final ownership against the ledger and keep the actual construction
   peak; final validation must not be the first predictable limit rejection.
   The real exact/one-under observer and both-entrypoint limit table must pass
   alongside the existing tests. Neither the fixed counts above nor an added
   aggregate allowance may substitute for wiring each listed allocation.

B-1 remains accepted only for its fresh helper and two outer allocations. B-2,
full B/D, authoritative native metadata/ordered geometry integration and public
high-resolution pipeline gates remain open. Separately, root reports current
seven-configuration consumer checks, legacy AVIF callbacks with highres off/on
(7 each), and non-AVIF typed/safety/Stage A tests (9/1/6) passing. Those results
do not cover this construction ledger, native functional completion or H6 Abort;
the recorded callback drawer always continues.

## C1 substep 1 final owner-contract review: three local checks remain

The scalar ownership heuristic is removed from the unclassified tokenless helper.
Property and outer-association tokens now live in MetaState. An independent real
input split across two top-level meta boxes matches the consolidated input's
ordered properties and metadata accounting, and preserves the legacy payload and
dimensions. The original five token-specific checks pass, including the foreign-
owner rejection, moved-owner growth, wrong-class check and failure atomicity.
The two older growth/spare harness calls were migrated to explicit tokens without
changing their budget, zero-allocation or success/failure assertions.

The complete independent external set is 20 pass / 9 fail: six failures are the
already assigned later-phase work; three belong to this same reserve contract:

- The class-qualified tokenless convenience still creates a temporary zero token
  and accepts nonempty storage. Metadata, ICC and Payload each allocate a new
  16-byte owner from an existing capacity-8 Vec instead of requiring an explicit
  owner token. The ordinary tokenless helper alone is fresh-only.
- Explicit adoption with a zero token and capacity 8 returns success under limit
  7 when the requested append fits spare capacity, leaving the ledger at zero.
- A nonzero token for capacity 8 accepts a Vec changed to capacity 16 whenever
  the request fits spare capacity; token capacity validation is after the return.

Keep the repair local and shared: make every native tokenless wrapper delegate to
one fresh-only capacity-zero guard. Route all existing-owner uses, including
payload extent append, through a token retained by the actual Vec owner. In the
token-bearing primitive, check class, checked length/capacity and nonzero token
consistency before any spare shortcut. For explicit adoption, validate and charge
the existing owner even if no allocation is needed. Calculate hypothetical full
adoption/replacement counter changes before committing; failed checks preserve
the Vec, token and all scalar counters. Already tracked spare reuse remains a
zero-allocation, zero-extra-charge success. Preserve Legacy behavior.

Independent host execution also passes the library suite (493 pass, 6 ignored)
and native/rich integrations (10/5/3). All-targets Clippy with warnings denied and
diff whitespace checks pass for this snapshot. Root's corresponding Rust 1.88
result is separate portability evidence, not proof that these owner checks are complete.
Substep 1 remains NO-GO until the three local checks pass. No new later-phase
findings were added; substeps 2–4, AV1 allocation boundaries and C2/C3 remain open.

## ICC selected-route slice: four repairs verified, checkpoint still incomplete

The four assigned route repairs are independently verified: selected LUTs no
longer validate unused matrix/inverse tags; header-channel mismatch is rejected
before LUT materialization; Transform assembly accepts structural profiles through
the same directional compiler; a selected singular inverse fails compilation.
The wrong-direction mAB/mBA error precedence is retained by checking direction
before channel shape. The original compile harness is now 6 pass / 7 known fail.
Product all-targets tests independently pass 153 with 1 ignored, and documentation
tests pass 1 with 8 explicitly ignored. Whitespace checking passes.

Two additional semantic checks pass. Gray/RGB XYZ matrix routes give equivalent
eager/structural results and retain their channel counts. RGB matrix Lab remains
unsupported, and Gray Lab remains explicitly unimplemented rather than being
silently interpreted as XYZ. A nonidentity chad is returned as the actual Some
matrix by both lazy accessors and is not applied a second time during relative
conversion. Structural profiles with unused malformed chad/wtpt remain usable
for that relative route, while the compatibility eager reader still rejects
malformed optional tags. Do not reinstate eager validation on unused tags to fix
the selected-white issue below.

One added P2 selected-white check fails. The assembler uses the lossy optional
media-white getter: malformed selected Absolute wtpt becomes None/Unsupported,
while zero or negative selected white assembles successfully and is rejected only
during evaluation. This is a local validation gap exposed by the newly usable
structural assembly route, not evidence that all Absolute semantics were already
implemented. Read required white with the checked accessor, preserve malformed
versus absent, and check finite positive components before compiling/materializing
selected stages. Only validate tags required by that route/bridge. No additional
CAT or reconstructed measurement-white substitution belongs in this repair.

Four new dead-code warnings also remain and must be resolved without dropping
validation:

- validate_matrix_channels is now redundant with the selected private parser's
  header-derived channel construction; remove or integrate the invariant once.
- MatrixProfile.pcs is redundant while matrix_for_direction and its parser both
  enforce XYZ; retain that guard if removing the unused stored field.
- ProfileInner.matrix is an unused eager cache. Compatibility semantic validation
  must still run even if its decoded result is no longer retained for assembly.
- media_white_checked is not redundant: wire it into selected-white validation
  rather than deleting it or suppressing the warning.

With the three new semantic checks, the compile harness is 8 pass / 8 fail;
including the earlier boundary/parse harnesses gives 22 pass / 8 fail. Seven
failures are the unchanged allocation/limits bundle and one is selected white.
The four route fixes may be marked closed, but this product checkpoint is not
GO pending the local validation/cleanup work. Stage 1's earlier limited acceptance
is unchanged. Cumulative compilation and wrapper budgets, full intent routing,
unclamped-domain behavior, Gray Lab and LittleCMS accuracy gates remain open.
No product code, version or publication state was changed during this review.

## Legacy callback test review: characterize unchanged behavior

Independent execution of the new callback tests gives 4 pass / 1 ignored with
highres disabled and enabled. The metadata-emission implementation matches the
earlier codec baseline, and the WML2 callback adapter is unchanged. Metadata
callback responses are ignored by that historical emitter; this is not a new
highres regression and the approved compatibility contract does not authorize
changing it here.

The tests-only checkpoint needs two local changes before acceptance:

1. Replace the ignored future expectation that metadata Abort stops decoding with
   an active legacy characterization: explicit metadata Abort and Continue both
   succeed and produce identical events and RGBA buffers. Assert metadata, draw
   and termination actually occur. Do not describe this as strict metadata-Abort
   support, and do not change production callback behavior to satisfy that future
   expectation.
2. Classify the AVIS Next test as an explicitly ignored external-fixture gate.
   Its required star fixture is not in a clean checkout. Document the existing
   download/bootstrap command and explicit ignored-test execution with avif and
   avif+highres. Keep the fixture existence assertion: missing fixtures must fail
   the explicit gate, not silently pass. The default ignored result does not
   count as acceptance of Next Abort.

The independent metadata characterization matches full metadata event strings,
canvas events and RGBA bytes on the saved still fixture. The current normal RGBA
test compares two entrypoints of the same implementation; separate feature-on/off
test runs are not a persisted cross-feature byte-for-byte oracle. This is a
bounded legacy regression checkpoint, not complete H6, native decoding acceptance
or authorization to repair historical metadata response handling.

## C1 substep 1 owner-contract closure: limited GO

Every native tokenless convenience now shares the fresh-only guard. Explicit
adoption and nonzero-token capacity validation precede the spare-capacity return;
successful adoption is charged once, and failed checks preserve all counters and
the Vec/token pair. MetaState retains its growing property/association tokens,
and the payload owner now retains the same token across its extent appends.
The legacy wrapper still selects Legacy behavior. The assigned owner-contract
defects are closed at this limited boundary.

Independent execution initially reproduced 22 pass / 7 fail in the external set.
The additional apparent reserve failure was an observer mismatch: its TOTAL=78
was exactly the capacity of the returned InvalidParam diagnostic String, while
the watched 8-byte data-owner allocation count was zero. Fixed-size error
reporting is already outside any process-wide no-allocation/no-OOM claim.
The corrected assertion permits exactly that returned diagnostic capacity, not
an arbitrary allowance: total requested bytes must equal it, data allocations
must remain zero, and Vec pointer/length/capacity, token and full ledger must
remain unchanged. Successful spare reuse still requires total allocation zero.
The test is active; no failure was ignored or removed.

Final independent external execution is 23 pass / 6 known later-phase failures,
including all nine token tests passing. Library tests pass 496 with 6 ignored,
and native/rich integrations pass 10/5/3. All-targets Clippy with warnings denied
passes. The multi-meta preservation and real allocator-denial cases remain green.

Proceed with the already assigned substeps 2–4 repair bundle. This accepts only
the fresh/adopt/growth/spare ownership primitive and its reviewed token wiring;
the six classification/retention/cumulative-count failures, AV1 header allocation
work, C2/C3 and full C1 remain unaccepted. No product code was edited by review.

## Legacy callback test follow-up: final acceptance pending

The external Next test is now explicitly ignored by default, with the existing
bootstrap command and both feature configurations documented. Its required
fixture exists and matches the bootstrap manifest hash; explicit missing-fixture
execution still fails. Independent normal execution passes four tests with one
ignored under both avif and avif+highres. The explicit external Next test passes
under avif and under the same already-built avif+highres test binary. A subsequent
Cargo rebuild of the latter was interrupted by an unrelated in-progress native
parser helper, so this is evidence for the earlier compiled snapshot only.

The metadata test is now an active comparison of legacy Abort and Continue
events and RGBA buffers, and the historical emitter still matches the earlier
codec baseline. One agreed test-only condition remains: assert that metadata,
draw, termination and nonempty draw buffers actually occur, so the equality
checks cannot pass vacuously. Re-run both configurations after that addition
and a buildable dependency checkpoint before accepting the test-only commit.
Formatting passes. Full H6 and a persisted cross-feature pixel oracle remain
outside this bounded callback checkpoint; no production callback behavior was
changed by the review.

## H5 native encoder WIP: bounded next repair bundle

This is a read-only source review and implementation plan, not encoder acceptance.
The native API has no new regression tests yet; no encoder build, generated
assets, dependency replacement or version change was performed by this review.
Keep the existing entrypoints and Rust 1.88 contract unchanged. Before generating
fixtures, add the missing encoder ignore rules for temporary work and test data.

Implement the following local slices in order, with tests preceding each repair:

1. **P1 property associations.** In `avifenc/src/container.rs`, emitting ICC+nclx
   inserts two `colr` properties, but primary and alpha `ipma` indices remain
   fixed for one. The primary loses its own `av1C`; alpha references the wrong
   properties and loses `auxC`. Build indices while appending properties and use
   those returned indices for both items. Test ICC-only, nclx-only and both,
   with/without alpha, plus the explicitly chosen missing-color policy. Inspect
   every association, essential flag and item reference independently of pixels.
2. **P1 native plane validation.** `encode_native_bytes` currently applies the
   chroma exponents to plane zero too: legal 420/422 luma with exponents 0/0 is
   rejected. Luma and alpha must use full dimensions and 0/0; only chroma uses
   the selected exponents and ceiling dimensions. Test odd dimensions, IDs,
   sample counts and explicit 8/10/12 limits. This is a validation defect, not
   evidence that this function actually downsamples luma. Preserve supplied
   samples without RGBA conversion or inferred bit depth.
3. **P1 one native color plan.** Resolve a borrowed, checked color plan before
   either backend runs, then use it for AV1 headers and container properties.
   The WIP overwrites nested `EncoderOptions.color_information`; absent native
   nclx gives lossless headers implicit identity/sRGB but the container implicit
   BT.601/sRGB. Do not inherit those legacy defaults into the new contract or
   infer coded matrix/range from opaque ICC bytes. Document the authority of the
   native color field and reject conflicting explicitly supplied metadata.
   Preserve supplied ICC bytes/type and reject inconsistent ICC fields before
   encoding. Validate representability of all CICP codes, range and layout:
   the lossy mapper otherwise substitutes Unspecified, while the lossless
   header writer emits only eight bits per CICP value. Its identity shortcut
   tests matrix alone, although the AV1 syntax shortcut depends on the complete
   BT.709/sRGB/identity tuple. Reject unsupported native tuples before encoding
   until the shared header supports them; do not relabel samples to fit it.
   This temporary rejection does not complete H5 support for legal requested
   tuples: the final target is correct conditional header syntax and tested
   support for those tuples, not permanent blanket rejection.
4. **P2 lossless controls and semantic metadata.** The new lossless route passes
   only speed and bypasses rejection of jobs, tiling and advanced controls.
   Split reusable backend-capability checks from old color restrictions and
   validate primary and alpha plans before starting either encode. Extended
   pixi is accepted but the writer emits only depth fields: preserve it through
   a supported writer or reject it explicitly. The current plane/color-only
   API does not prove source geometry or other required item metadata survived;
   the eventual frame bridge must reject unsupported preservation requirements.
   This is still an open H5 gate, not an instruction to expand this repair into
   gain-map, derived-item or animation encoding.

Alpha routing currently selects lossless exactly when `lossless` is true or
`quality_alpha` is 100; primary quality alone does not make alpha lossless.
The code passes native alpha separately at matching depth, full resolution and
full range, without ICC conversion, premultiplication or hidden-color clearing.
Verify rather than assume those properties survive encoding: use alpha values
0/1/mid/max and distinct hidden colors, all 8/10/12 depths and 400/420/422/444,
including mixed color-lossless/alpha-lossy and the reverse. A source-lossless
claim requires both items and supported interpretation metadata to remain exact.
These auxiliary constraints follow [AVIF 1.2 section 4.1](https://aomediacodec.github.io/av1-avif/v1.2.0.html#auxiliary-image-items-and-sequences);
header signaling follows [AV1 section 5.5.2, color_config](https://aomediacodec.github.io/av1-spec/av1-spec.pdf).

Keep the new `encode_native_bytes`/`NativeEncodeOptions` names within existing
`encode`, `*_bytes` and `*Information` vocabulary; do not rename old exports.
Document native plane order, identity GBR order and optional alpha ID 3 instead
of leaving `FrameBuffers` documented as only one or three YUV planes. Put native
preflight/property planning in small private responsibility-based modules rather
than adding another large block to the existing encoder implementation.

Acceptance is layered: pure validation and box/header tests first; generated
native sample roundtrips through this decoder and an independent FFmpeg/libavif
decoder next; then existing API regressions, MSRV, formatting and Clippy. Lossless
tests compare raw native samples, alpha and metadata, not an RGB conversion.
RGB target conversion and explicitly quantized F32/U16 targets remain separate
H4/H5 work, not proof supplied by this native-plane slice.

## C1 substeps 2/3 follow-up: six regressions closed, four local failures remain

Independent execution confirms the original external 29 tests pass, including
all nine accepted owner-token checks. The earlier six classification/idat/report/
repeated-iloc regressions are locally closed. Four additional tests in the same
selected-plan/ownership/compatibility scope fail, making the external result
29 pass / 4 fail. Substeps 2/3 therefore remain NO-GO.

| Priority | Reproduced boundary failure | Required local correction |
| --- | --- | --- |
| P1 | Primary and alpha each need 4096-byte output payloads, but their combined ownership exceeds the input-derived payload allowance. One alpha payload is allocated before rejection. | The borrowed selected plan must check the total for primary plus the actual unique selected alpha items before any selected payload allocation. Do not compute and discard each extent sum. Use the same selection/fallback/dedup rules as materialization. |
| P2 | Alpha output reports 218 metadata bytes although observable owned capacities require at least 261. The missing 43 bytes are the retained auxiliary-type string. | Walk every retained nested owner, including each alpha auxiliary string. Pass the actual ordered-record vector capacity to the walker rather than recovering it from slice length. Encoded alpha payload remains in Payload, not Metadata. |
| P2 | Twenty replacement iinf tables have measured whole-parser heap peak 1407 and retained metadata 162, yet metadata budget 8192 fails at an accumulated 8559. | Release parser-only table/string owners on actual drop/replacement, with rollback and old/new coexistence accounted. A final returned-owner sum does not repair monotonically accumulating live counters. Preserve separate current, peak and retained values. |
| P2 | Legacy parsing now reports the later invalid alpha extent as NotEnoughData instead of the earlier primary payload-length Bitstream error. | Restore the old Legacy order: primary payload, sequence, primary metadata validation, then alpha materialization. Native preflight may run earlier without reordering Legacy. |

The Legacy error-priority test was executed against the earlier codec baseline
using the identical generated input and passes there. Library regression tests
pass 496 with 6 ignored, the native/rich integrations pass 10/5/3, and all-targets
Clippy with warnings denied passes. These passing suites do not waive the four
independent failures. No product code was changed by review.

Substep 1 remains accepted only within its previously reviewed primitive/wiring
scope. Repeated iloc counting is now checked in the early scan, but the remaining
shared-context table-kind counters, unchecked recursion-stack allocation and
alpha stable-sort allocation remain open substep-4 work. AV1 header/tile
allocation, C2/C3, and general bounded-decoding acceptance remain separate gates.

## Legacy callback test-only checkpoint: accepted

The active metadata characterization now asserts nonempty metadata/draw/terminate
events and draw buffers as well as Abort/Continue equality. The external Next
test remains explicitly ignored by default, with bootstrap and off/on execution
instructions and a hard missing-fixture failure. Independent fresh Cargo runs
with highres disabled and enabled each pass four normal tests with one ignored;
explicit ignored-test runs each pass the one external Next test. The required
fixture hash matches the bootstrap manifest, and formatting passes.

The single callback test file is accepted for an isolated tests-only checkpoint.
This does not approve any product-code or gitlink change, resolve the separate
B-2 warnings/ownership gates, prove strict metadata Abort support, or complete
H6. The normal RGBA comparison remains two entrypoints of the same implementation,
not a persisted cross-feature byte-for-byte oracle.

## B-2 shared-ledger follow-up: four ownership/ordering failures remain

Independent execution against the fixed codec baseline confirms the prior 55
tests pass. Four minimal additions within the existing ownership inventory fail:
55 pass / 4 fail overall. B-2 remains NO-GO; the accepted B-1 fresh primitive is
not an acceptance of these metadata and lifetime paths.

| Priority | Reproduced failure |
| --- | --- |
| P2 | A borrowed legacy ICC owner with length 1/capacity 4096 is wrongly included in retained metadata/frame budgets. Correct output budgets of metadata 16 or frame 1179 are rejected although the independent live allowance is sufficient. |
| P1 | Final metadata needs 8217 bytes, but budget 8216 triggers one 4096-byte active ICC copy before rejection. Output dimension pairs and all final logical metadata must be included in preflight. |
| P2 | Native outer storage is gone before the late active ICC clone, but a live limit of 9380 is rejected. The measured heap peak is 8780; 584 fixed header bytes and even the separately counted 16 inline metadata bytes are included in that allowance. |
| P1 | Provenance capacity 1 grows to 8 before either whole-replacement live or final-metadata rejection. On failed reconciliation, the vector keeps capacity 8 while the ledger rolls back to its original values: owner and accounting are no longer consistent. |

The source now routes nested metadata copies and active-color copies through
metadata-aware fresh helpers, and both layout offset owners plus roles/outer
vectors use the shared frame helpers. Those are real improvements. Remaining
gaps are the borrowed-source seed, phase-aware release, in-place provenance/
unknown growth wrappers, predictable final metadata, and per-ICC capacity checks.
The active/source ICC fresh-length regression and original allocator-denial tests
remain passing; they do not cover these additional boundaries.

Use three small implementation/review slices, not another aggregate allowance:

1. **Ownership and actual drop.** Seed explicit frame-retained, metadata-retained
   and live-only amounts, using fields or equivalent named constructors. Borrowed
   RichAvifInfo nested/legacy projection owners are live-only until return;
   incoming owned FrameMetadata is retained frame+metadata. Seed samples and
   fixed headers once. Split NativeMapPlan's final totals from temporary live
   ownership; it currently adds borrowed data to final budgets. Release only the
   native outer live charge after its iterator/allocation is actually dropped,
   before active cloning. Freeze after both borrowed-budget checks and the late
   active-clone peak case pass, with the previous tests unchanged.
2. **Transactional metadata growth.** Reuse the accepted local-candidate fresh
   allocation structure for metadata and provenance/unknown replacement. Keep
   the old owner intact; precheck old+whole candidate live peak and the resulting
   final metadata; reserve fallibly; reconcile capacity; copy/fill; then replace,
   drop old storage and release it. A failed operation must preserve old owner
   and all frame/metadata/live counters. Wire both metadata projection helpers
   and the AV1-provenance update through this transaction. Do not roll back only
   scalars after mutating the vector. Freeze against the real allocator observer
   and failure-owner checks before continuing.
3. **Final metadata and ICC.** Compute the complete final metadata state before
   copying, including known dimension/scalar changes and source provenance before
   active cloning. Fresh copies use lengths, retained owners use capacities.
   Add a shared ICC-copy boundary that checks requested length, allocates through
   the metadata helper, checks actual capacity against the per-owner ICC limit,
   and only then copies. The current generic metadata helper has no ICC capacity
   limit; this is a static missing check, not an independently reproduced
   allocator-overcapacity failure. Freeze with the one-under metadata copy
   observer, exact budgets and original limit table all passing.

Independent product all-target checks pass for highres-only and avif+highres.
Highres-only has the two previously recorded draw warnings. Avif+highres still
has four new unused items: consume_native_frame_with_metadata, ledger try_copy,
try_clone_owned_bridge and try_planar. Remove obsolete private duplication or
apply the correct test/feature boundary without discarding required validation;
these are not baseline warnings. No product code was edited by review. Full B/D,
authoritative NativeAvifInformation/ordered geometry integration and the broader
H6 gates remain separate from these local repairs.

## ICC wrapper-length and selected-white repairs: limited acceptance

Independent review accepts these two local repairs only. All fixed-buffer F32,
U8 and U16 Transform/worker entrypoints now share checked length validation before
scratch/output allocation. Invalid lengths preserve caller output; empty buffers
remain valid. The original large invalid-length allocation regression passes.
A further input/output-length table observes zero allocation requests and unchanged
sentinels for direct and warmed-worker calls; nonfinite F32 inputs still fail.
This does not make successful integer-wrapper allocation fallible or bounded.

Absolute selected media white now uses the checked raw accessor in the common
directional compiler and assembler. Missing white is Unsupported; malformed or
nonpositive XYZ is Invalid/Malformed before the selected direction's sampled-curve
copy. Independent checks cover both directions, both assembler sides and all
three nonpositive components. Relative conversion still permits unused malformed
wtpt/chad; actual nonidentity chad remains visible without a second adaptation.
These checks do not certify Absolute routing or physical-white color accuracy.

The original two external regressions each run and pass individually. All earlier
external tests give 24 pass / 6 known budget failures; two additional same-scope
checks pass, giving 26 pass / 6 fail overall. Product transform 11 and LUT 23 pass,
with the official fixture's presence and hash independently verified rather than
relying on its missing-file early return. The previously reported Miri 34 result
was after the wrapper repair but before the white repair, not validation of both.

The six budget failures remain open: matrix decoded bytes/entries (standalone
and assembler), independent table limits, two-direction cumulative LUT budget,
retained outer/grid accounting and truncated selected-matrix preflight. Successful
wrapper allocation, full intents/domain behavior and LittleCMS gates remain open.
Three new unused items still require cleanup: validate_matrix_channels,
MatrixProfile.pcs and ProfileInner.matrix; media_white_checked is now used.
Compiler responsibility separation is also pending. No full H3/product checkpoint
GO is granted, and no product source was changed by this review.

## C1 substeps 2/3 repair follow-up: shared selection and owner release still open

Independent execution confirms all previous 33 external checks pass. The four
latest local repairs are real: the ordinary explicit primary/alpha payload sum
is checked before copying, retained alpha auxiliary strings are included, iinf
replacement releases its table/name accounting, and Legacy primary-error
precedence is restored. Substep 1 remains accepted within its earlier scope.

Three added tests in the same agreed selection/ownership inventory fail; the
external result is 33 pass / 3 fail. Substeps 2/3 remain NO-GO.

| Priority | Reproduced remaining failure |
| --- | --- |
| P1 | An auxl reference with zero targets suppresses auxC fallback in the preflight but not in materialization. One 4096-byte alpha payload is copied before the combined payload limit is rejected. |
| P2 | Two references to the same alpha item are counted twice by preflight although materialization deduplicates them. One 2048-byte primary plus one 2048-byte alpha is rejected at budget 4096 as a supposed 6144-byte selection. |
| P2 | Twenty-four replacement iref/grpl tables have independently observed whole-parser heap peaks 671/663 and retained metadata 162, yet metadata budget 4096 fails at accumulated 4111/4198. Releasing only iinf leaves the same dropped-owner defect elsewhere. |

Apply the remaining work in small, separately frozen slices:

1. **One native selection.** Share a borrowed, repeatable selection or one
   context-accounted selected-item plan between preflight and materialization.
   Fallback depends on no selected auxl targets, not merely no auxl box/reference.
   Deduplicate item IDs using the materializer's semantics before payload totals;
   primary and alpha output buffers remain distinct owners even if their encoded
   source extents overlap. Validate every selected type/extent and the complete
   unique-output total before any payload allocation. Preserve Legacy selection
   and order. Freeze with both new selection tests and earlier extent/alpha tests.
2. **Complete actual-drop inventory.** Reuse checked capacity walks plus explicit
   class/token debit instead of resetting the context or estimating totals.
   Cover replaced iloc locations/methods/index tables and nested extents, replaced
   iref target vectors and grpl entity vectors, consumed IPMA incoming outer/inner
   storage, and alpha selection/dedup temporary storage. Moved nested owners keep
   their charge; only actually dropped owners release it. Retained MetaState
   properties stay live until the final projections are built and their source
   storage is actually dropped. Precompute checked release amounts, preserve
   old/new coexistence during replacement, and keep scalar rollback consistent
   with owner state. The new iref/grpl failures are concrete; the other listed
   callsites are the same unfinished static inventory, not extra reproduced cases.
3. **Actual retained capacity.** The retained walker still receives ordered
   records as a slice and charges its length. Pass the private Vec capacity (or
   a checked capacity byte count) alongside the borrowed records before moving
   the Vec into the result. Current ordinary fixtures do not demonstrate spare
   ordered capacity; this is a static contract gap, not a claimed allocator
   overcapacity reproduction. Keep encoded payload outside Metadata and retain
   the newly fixed alpha string capacity.

Independent tracked integrations pass native_boundary 10, native_limits 5,
native_phase_boundary 4 and rich_color 3. The reported Miri 12 covers limits 5,
phase 4 and rich 3, not the three new failures or the full owner inventory.
Shared cross-kind counters, recursion-stack and stable-sort allocation remain
separate substep-4 gates; AV1 header/tile allocation, C2/C3 and general bounded
decode acceptance remain open. No product source was changed by review.

## ICC remaining six compile-budget regressions: small implementation slices

This is a sequencing refinement of H3, not additional scope or acceptance. Keep
the accepted route, wrapper-length and selected-white behavior unchanged. The
following six external failures are the fixed target; do not combine their repair
with intent/domain/oracle work or successful image-wrapper scratch budgeting.

Use small private responsibility modules rather than extending compile.rs:

- compile_plan: borrowed direction/route plans and checked cost composition;
- compile_budget: scalar compile ledger and transactional fresh Vec allocation;
- curve_plan / lut_plan: format-specific borrowed ranges, counts and stage shape,
  shared by planning and the corresponding materializer, not duplicate parsers;
- keep public types/reexports, assembler and evaluator compatibility in the
  existing facade initially. Move planning out first; do not add another giant
  facade or expose the private ledger publicly.

The internal seam should be equivalent to:

```rust
plan_direction(profile, direction, intent) -> Result<DirectionPlan<'_>, Error>
plan_curve(encoded, usage) -> Result<CurvePlan<'_>, Error>
plan_lut(encoded, pcs, direction) -> Result<LutPlan<'_>, Error>
budget.admit(plans) -> Result<(), Error>
budget.try_new_vec::<T>(count) -> Result<Vec<T>, Error>
materialize_direction(plan, &mut budget) -> Result<CompiledProfile, Error>
```

Plans borrow checked bytes and use bounded stack descriptors for the already
supported channel/stage counts. They hold no decoded Vec or eager-cache clone.
Separate logical curve/CLUT entries from storage: an inline identity/gamma still
counts according to the existing entry policy but does not own a separate float
allocation. Table and parametric Vecs, typed outer Vecs, grid Vecs and selected
owned headers each count once. Shared encoded tag ranges are not proof that two
materialized curve Vecs share ownership. ParseLimits continue bounding source
bytes; caller-owned raw bytes are not a second compiled allocation.

Admission checks the complete known planned requirement before materialization;
do not debit that estimate and then debit the same allocations again. During
materialization the single ledger reconciles real capacities, including pending
planned storage and temporary coexistence. The fresh helper creates its candidate
internally: checked requested bytes, precheck/provisional accounting, fallible
reserve, actual-capacity reconciliation, then return/commit. On error drop the
candidate before restoring scalar accounting. Retained owners stay charged until
actual drop; no pointer registry, cloned ledger or implicit reset. Fixed Arc
control bookkeeping keeps its documented exclusion, not an exemption for its
input-sized payload or owned descriptor storage.

| Slice | Narrow implementation and acceptance |
| --- | --- |
| 1. Matrix planning and ledger | Read the selected Gray/RGB TRC counts, encoded ranges, parameters and matrix fields before decoding. Apply TransformLimits bytes and entries independently, including curve outer/payload storage. Route structural and eager inputs through this plan rather than cloning ProfileInner.matrix. Standalone compile creates one ledger; assembler uses the internal seam. Close matrix_compile_limits_precede_decoded_table_allocation, matrix_table_limits_are_independent and matrix_transform_limits_precede_eager_cache_clone: each must reject before a 4096-byte table allocation. |
| 2. Selected LUT shape | Move the current mft/mAB/mBA preflight into the borrowed LUT plan. Preserve direction-before-channel error precedence and legal stage-pair rules. Validate every selected offset/range, especially the full 48-byte matrix, before any A/B/M curve allocation; materialization consumes this same validated shape. Close truncated_selected_matrix_stage_rejected_before_any_curve_copy in both directions. |
| 3. LUT owner inventory | Wire mft input/output Table outer Vecs, their float tables, CLUT values and grid; mAB/mBA A/M/B Curve outer Vecs and Table/Para payloads, CLUT/grid, plus selected owned headers through the same ledger. Replace/budget input-sized temporary parameter clones and the grid vec allocation. Keep actual-capacity reconciliation, not a final len-only walk. Close lut_compiled_budget_includes_outer_vectors_and_grid_storage: float-only limits 49200/12288 must fail before the observed CLUT/curve copies. |
| 4. Two-direction admission | Plan both selected directions before either materializes; admit combined bytes, curve entries and CLUT entries once. Transform must not call the public fresh-budget Profile::compile twice. Materialize both with one mutable ledger, retaining the source while building the destination; if either fails, drop candidates before rollback. Close two_selected_luts_share_one_compiled_budget: two 49200-byte payloads cannot fit a shared 50000 limit. |

Freeze/review after each slice. Retain every existing assertion and both newly
accepted repair probes; strengthen the same six fixtures with exact-success /
one-under limits and real allocator-failure observations where appropriate.
Parametric validation must preserve the existing forward/inverse semantics while
borrowing parameters or explicitly budgeting its temporary owner; no unrelated
curve-domain rewrite belongs here. Do not report all six closed while any selected
materializer still bypasses the ledger. New unused private items are cleaned up
without deleting their required channel/PCS or compatibility validation.

## C1 substeps 2/3: selection repair accepted, remaining drop inventory fails

All previous 36 external checks independently pass. Native preflight and
materialization now use the same borrowed unique-alpha selector, including empty
auxl fallback and duplicate-target handling. The retained walker takes the actual
ordered Vec capacity, and replaced iref/grpl outer/nested owners are released.
These local repairs are accepted; they do not finish the previously listed
parser-owner inventory.

One additional test contains two failures from that existing inventory. Twenty-four
iloc replacements with eight zero-length, valid extents have whole-parser heap
peak 791 and retained metadata 162, yet metadata budget 4096 fails at 4143.
Twenty-four empty IPMA merges into an existing item have peak 511 and retained
162, yet budget 1024 fails at 1071. The high-budget fixtures parse successfully;
their counts fit the existing configured limits. These are P2 dropped-owner
overcount failures, not new cross-kind-counter or payload-format requirements.
The external result is 36 pass / 1 fail, with both cases reported by that test.

Keep the next repair narrowly on actual ownership:

- For iloc replacement, account and release the old locations/methods/index
  outer Vecs plus each location's extent Vec and nested extent-index Vec after
  actual replacement/drop. Keep old/new coexistence charged while parsing the
  candidate. Use checked capacity walks or persistent move-only tokens, not a
  context reset or a guessed total.
- For IPMA merge, retain charges for whole incoming item/inner Vecs moved into
  the target. Release an incoming inner Vec only after its entries have moved
  into an existing target and its storage drops; release the incoming outer Vec
  after its iterator/storage drops. Preserve the parse transaction's rollback
  behavior on error, with no token/owner lifetime mismatch.
- The alpha-selection temporary outer Vec and final parser-only MetaState drop
  remain part of the same static inventory: moved auxiliary strings stay owned,
  whereas destroyed temporary containers release their own storage. This review
  adds no extra reproduced failure for those static callsites.

Substeps 2/3 remain NO-GO until the inventory is wired and the unchanged 36 plus
this two-case regression pass. Substep 1 acceptance is unchanged. Substep 4,
AV1 header/tile materialization and C2/C3 remain separately unaccepted. No product
source was changed by review.

## Parent ownership Slice A: old test assumptions corrected, one phase gap remains

Independent execution of the frozen source initially gives 55 pass / 4 fail
across the existing 59 harness tests. Three failures are obsolete test premises,
not three new product regressions: two require an incomplete borrowed preflight
to succeed before descriptor construction fails, and one charges a borrowed
legacy projection against the retained metadata limit. The agreed contract
requires earlier preflight and treats that projection as live-only.

The two external tests were corrected without weakening their observations.
Descriptor preflight now must return ResourceLimit, while the actual mapper still
must allocate zero descriptor buffers. Borrowed legacy metadata capacity 4096
must succeed with retained metadata limit 100 and sufficient live allowance, but
a live allowance below that borrowed owner must reject with zero ICC copies.
The equivalent tracked descriptor test still needs the same early-ResourceLimit
expectation; review did not edit product tests.

The earlier exact borrowed metadata/frame budgets and late-active-clone peak
checks now pass. A new scalar test confirms frame-retained, metadata-retained
and live-only seeds stay distinct through live release and subsequent charges.
Samples and headers are charged once on successful paths; native outer storage
is released after its consuming loop drops the iterator/allocation, before the
active clone. Borrowed RichAvifInfo stays charged through the mapper return.

However, a new P1 same-scope phase test fails. With native outer capacity 128,
the observed coexistence peak is 16535 including the agreed logical borrowed
seed and fixed headers. At live limit 16534, the mapper copies one 4096-byte
source ICC before returning ResourceLimit. This is late predictable-limit
rejection, not a claim that it returns an over-budget successful frame.
The first borrowed plan includes final ownership plus borrowed data but omits
the native-outer coexistence phase. The later pre-release calculation runs after
metadata copying and also omits the borrowed live-only amount.

The minimal A repair is one allocation-free ownership plan shared by both
entrances. Name the initial, pre-release and post-drop owners explicitly; check
their maximum before metadata copying, with borrowed data included in every
phase where the caller still owns it. Seed known sample/header storage once at
the entrance, not after the first metadata copy. Retain the actual native-outer
drop/release ordering and never release the borrowed Rich object early. Do not
replace the missing phase with a blanket multiplier or padding allowance.

After external test corrections and two additions, the harness is 58 pass /
3 fail: the new A phase failure, the unchanged provenance-growth failure for
Slice B, and the tracked obsolete descriptor expectation. Slice A remains
NO-GO. The final-metadata one-under regression now passes, but Slice C's remaining
ICC actual-capacity/inventory work is not thereby accepted. Newly unused private
inspection/metadata/ledger helpers remain cleanup requirements, not baseline
warnings. Full B/D and H6 remain separate gates; product source was not edited.

## C1 substeps 2/3: iloc and IPMA release accepted, alpha temporary release remains

Independent execution confirms the existing 37 external tests pass. The three
iloc outer tokens and their nested storage now release on replacement; IPMA
preserves whole incoming owners moved into the target and releases consumed
inner/outer storage after its iterator drops. Shared alpha selection, fallback,
deduplication, retained ordered Vec capacity and substep 1 token tests remain
passing. The tracked native limits, phase boundaries and rich metadata tests
also pass independently: 5 + 8 + 3 tests.

One added P2 regression from the already listed temporary-owner inventory fails:
`consumed_alpha_selection_outer_is_released_before_final_icc_projection`.
With one selected alpha and a 4096-byte ICC, the real whole-parser heap peak is
17332 and returned metadata ownership is 12597. A metadata limit of 17332 still
rejects the final ICC projection with a ledger total of 17360. This allowance
already includes the tiny payload owners in the observed heap peak; it is not
an artificially smaller metadata-only estimate.

The alpha selector's `item_ids` outer Vec is consumed and dropped before the
final projection, but its `item_ids_token` remains charged. Release that token
after the consuming loop and before returning `auxiliary_items`. Keep the moved
auxiliary strings and output Vec charged: only the destroyed temporary outer
storage is released. Keep the regression's real-allocation peak observation and
the existing tests unchanged. Parser state and its context end together after
the final projection; that terminal drop introduces no further allocating
continuation requiring an early release.

The complete external harness is therefore 37 pass / 1 fail across 38 tests.
Substeps 2/3 remain NO-GO for this single same-inventory release omission, not for
the now-closed iloc/IPMA cases. Substep 1 acceptance is unchanged; substep 4,
AV1 header/tile allocation and C2/C3 remain separately unaccepted. Review changed
only the ignored harness and this record, not product source.

Final static checklist for this same owner/drop inventory:

| Owner or transition | Frozen-source finding |
| --- | --- |
| iinf outer and item-name strings | Old capacities released during replacement; candidate/old coexistence was charged first. |
| iloc locations, methods, indexes and nested Vecs | All three outer tokens and nested capacities replaced together; no allocating continuation between old release and replacement. |
| iref and grpl outer/nested Vecs | Replaced owners released; repeated-table regressions pass. |
| ipco properties and nested payloads | Appended/moved into MetaState, not discarded; keep charges until the state ends. |
| IPMA incoming/target outer and inner Vecs | Whole moved entries retain inner charges; merged-away inner and consumed incoming outer release after consumption. |
| Alpha selection IDs versus returned auxiliary items | Temporary IDs outer release is missing (the single P2 above); moved strings/output owners must remain charged. |
| idat and selected payloads | Native idat remains borrowed; materialized selected payload owners stay charged as Payload, with shared deduplicated selection. |
| Legacy, rich and ordered metadata projections | Distinct copied owners remain charged and returned capacities are walked; ICC duplicates are rejected before collection rather than overwritten on a valid native path. |
| Parser-only final MetaState and helper-growth replacement | State/context end together after projection; growth helper drops the old Vec before releasing its charge. |

No further reproduced failure or missing release was found in these listed
transitions. This statement does not extend to the separately deferred count,
stack, sort or AV1 allocation work.

## Parent ownership Slice A: early rejection repaired, exact phase still overcounted

Independent rerun of the frozen source reproduces 60 pass / 1 known Slice B
provenance-growth failure across the existing 61 external tests. The tracked
mapping suite also passes independently, 23 tests on Rust 1.91. The approved
early-ResourceLimit test correction keeps both the preflight and real mapper
assertions; the external allocation observations were not weakened.

The three ledger seeds remain distinct, sample/header charges occur once on
successful paths, borrowed Rich metadata remains live until return, and the
native outer Vec drops before its live debit is released and active colour is
copied. The previous live-limit 16534 failure now rejects before the 4096-byte
source ICC copy. However, one additional exact-side assertion on that same
native-outer-capacity-128 fixture fails: the observed pre-release phase is
16535, final ownership plus borrowed data is 13480, but the plan computes
pre/post totals 16544/13473 and rejects a live allowance of 16535.

This P2 is an inconsistent ownership-phase calculation, not a weakened limit
test. `NativeOwnershipPlan::for_rich` retains the two late dimension pairs
(16 logical bytes) in its pre-release total; the metadata entrance's separate
formula excludes them. Meanwhile the source provenance allocation has actual
capacity 8 but the rich prediction counts length 1, offsetting seven bytes of
that overcount. Merely subtracting 16 would reintroduce the original early-
rejection gap and is not an acceptable repair.

Use one phase constructor from both entrances, fed by the same source/active
ownership construction plan. Keep source copies, active fresh copies, late
inline metadata, native outer storage and borrowed data explicit. Align the
provenance fresh allocation with the shared fallible exact helper/plan rather
than adding a fixture-specific padding constant; its transactional growth
remains Slice B. Seed known headers before the first rich metadata copy (they
are currently charged afterwards), retaining the single sample charge and the
actual native-outer drop/release ordering. Correct the ledger comment so only
native outer storage is released there; borrowed Rich data stays live.

The unchanged 61 tests plus
`a_native_outer_spare_exact_observed_phase_budget_succeeds` now give 60 pass /
2 fail: this Slice A exact-phase case and the known Slice B growth case.
Slice A therefore remains NO-GO. The final-metadata one-under case passes, but
Slice C ICC actual-capacity reconciliation and new unused helper cleanup remain
unaccepted; full B/D and H6 are not implied. Product source was not edited.

## C1 substeps 1/2/3: limited parser ownership checkpoint accepted

The final frozen alpha repair releases `item_ids_token` only after the
consuming loop ends and its IntoIter has dropped the temporary outer Vec.
Returned auxiliary strings and payloads remain moved owners with their charges
intact. An earlier placement before the output reserve was rejected by static
review and is not the accepted snapshot. The fixed owner/drop inventory above
was rechecked: replacement tables, IPMA moved-versus-consumed storage, shared
selection/fallback/deduplication, actual retained capacities and terminal parser
state ownership introduce no further finding in these substeps.

Independent final execution:

- External boundary harness: 39 pass, no failures or ignored tests. The original
  38 assertions remain intact. The extra zero-payload, seven-alpha boundary
  requires metadata-limit rejection below observed coexistence after excluding
  the separately deferred traversal-stack storage; it is a positive regression
  on the repaired snapshot, not a claimed pre-repair runtime reproduction.
- Product library: 496 pass, 6 existing ignored diagnostics.
- Native boundary / limits / phase / rich metadata integrations: 10 / 5 / 9 / 3
  pass respectively.
- Clippy all targets with warnings denied: pass. The legacy primary-before-alpha
  error regression also passes; the release is a no-op for uncharged Legacy
  tokens, with compatibility entrypoints and materialization order preserved.

Supplemental coordinator verification of this same C1 snapshot: Rust 1.88 runs
the library (496 pass / 6 ignored) and the same 27 integrations successfully;
wasm32-unknown-unknown check passes. Miri passes the single repaired alpha-drop
case. This is not a claim that all 39 external tests ran under Miri or WASM.

Substeps 1/2/3 receive limited GO for this parser ownership checkpoint. This
supersedes their prior same-inventory NO-GO findings only. Substep 4's complete
cross-kind counts, stack and sorting work, AV1 header/tile allocation checks,
C2/C3 and full C1 remain unaccepted. No public highres byte API or overall
pipeline completion is authorized by this result. Review edited no product
source and performed no commit or release operation.

## Parent Slice A: runtime boundaries pass, three agreed structural items remain

Independent execution of the next frozen source passes all 62 external tests
and all 23 tracked mapping tests. The unchanged real-allocation exact and
one-under native-outer tests both pass. The earlier provenance-growth test also
passes legitimately: its existing owner still grows from capacity 1 to 2, and
the test already allowed an exact two-byte replacement instead of the forbidden
eight-byte request. That result does not establish Slice B's general whole-
candidate admission or transactional failure contract.

The Slice A repair remains limited to these three previously required items:

1. In `consume_native_frame`, move the known header charge before
   `frame_metadata_with_ledger`, next to the single sample charge. Preserve the
   precharged flags so the shared consumer does not charge either owner twice.
2. Make the same `NativeOwnershipPlan` phase constructor serve both entrances.
   Its checked inputs are final frame ownership, active fresh colour ownership,
   late inline dimensions, borrowed live-only bytes and native outer capacity.
   Pre-release is final minus active minus late dimensions plus borrowed plus
   native outer; post-release is final plus borrowed. The rich wrapper and the
   owned-metadata consumer must call this constructor instead of retaining the
   latter's separate formula. Check before each entrance's first copying or
   destination-allocation step, preserving actual native-outer drop/release.
3. Correct `ConstructionLedger::new_with_ownership` documentation: only the
   destroyed native outer allocation is released before active colour copying;
   caller-owned borrowed Rich metadata remains live until return.

These are source-confirmed contract/maintenance gaps, not three newly failing
runtime tests. Keep the existing allocator observers, exact/one-under success
and rejection assertions, borrowed final-budget tests, three-seed test and both
entrance limit table unchanged. A small scalar phase-constructor assertion may
cover the shared borrowed-versus-owned input mapping; no broader fixture or
pipeline redesign is required. No other item is added to this fixed A bundle.

Slice A remains NO-GO until these agreed structural requirements are wired.
Slice B's owned growth, Slice C's ICC actual-capacity accounting and unused
helper cleanup remain separate unfinished work. Passing these tests is not
full B/D/H6 acceptance. Review changed no product code.

## Parent Slice A: limited ownership-phase checkpoint accepted

The final three structural repairs are verified: both entrances use the same
checked `NativeOwnershipPlan::new`; known sample/header storage is charged once
before the rich metadata copy; and ledger documentation distinguishes the
native outer drop from borrowed Rich data that remains live until return.
The actual native outer consuming loop still drops its storage before release
and active colour cloning. No additional Slice A finding remains in the fixed
inventory.

Independent execution passes all previous 62 external tests and all 23 tracked
mapping tests. The real-allocation exact/one-under observations and the approved
earlier-preflight assertions remain unchanged. With the three new Slice B tests
below, the complete external harness is 64 pass / 1 fail across 65 tests; that
one failure contains the two Slice B growth-helper cases and is not a Slice A
regression. Slice A receives limited GO. Slice B/C, unused-helper cleanup,
authoritative native metadata wiring, full B/D and H6 remain unaccepted.

## Parent Slice B: fixed transactional-growth implementation and tests

The original exact-two-byte growth case remains valid and passing. New boundary
tests keep it unchanged and distinguish final ownership from coexistence:

| Test | Independent result |
| --- | --- |
| `b_existing_metadata_growth_prechecks_old_plus_whole_exact_candidate` | Fails for both helpers: provenance old ownership 9 plus candidate 2 needs live 11, yet limit 10 allocates and succeeds; unknown metadata old ownership 35 plus candidate 64 needs live 99, yet limit 98 allocates and succeeds. |
| `b_existing_metadata_growth_exact_peak_and_real_allocator_failure` | Passes for both helpers at exact live limits; real allocator denial of 2/64 bytes preserves original outer pointer/capacity, nested payload pointer/content and all three ledger counters. |
| `b_av1_provenance_real_allocation_failure_restores_caller_ledger` | Passes: the actual AV1 provenance growth request is denied, typed Allocation is returned, and its provisional debit is undone. The consumed API cannot expose its former metadata owner after failure; an extracted update helper needs its own owner-preservation assertion. |

Keep this implementation confined to the shared metadata allocation boundary,
the provenance/unknown wrappers and the AV1 provenance update:

1. Add a private metadata replacement primitive in the allocation module, for
   example `try_grow_metadata_vec<T>(owner: &mut Vec<T>, additional: usize,
   inline_delta: usize)`. Use a scalar snapshot of frame/live/metadata counters;
   do not create an owner registry or clone nested payloads. Validate checked
   `len + additional` and byte arithmetic first. A spare-capacity request with
   no inline delta allocates nothing and changes no counters.
2. Before the allocator, check resulting frame/metadata ownership as old total
   minus old outer capacity plus requested new outer capacity plus any known
   inline delta. Separately check coexistence live usage as old live plus the
   whole candidate, as well as the post-commit live total. This preserves the
   legitimate exact-two-byte success case; final metadata is not charged as
   though both outer owners were retained permanently.
3. Allocate a fresh local candidate fallibly, keeping the old Vec untouched.
   Recheck both final ownership and live coexistence with its actual capacity.
   Only after all fallible checks succeed, move old elements into the candidate,
   replace the owner, drop the empty old storage, and commit actual counters.
   Unknown payload Vecs move with their elements; their pointers and charges
   remain unchanged. Do not use in-place reserve followed by scalar-only
   rollback. Predictable overflow/budget failures are ResourceLimit; allocator
   denial is Allocation.
4. Route `reserve_provenance_with_ledger`, `reserve_unknown_with_ledger` and the
   AV1 update through that primitive. Private, feature-scoped Vec accessors or a
   small internal closure adapter may expose the exact owner without changing
   public metadata APIs. Extract a private AV1 update operation taking mutable
   metadata and the ledger: precompute missing provenance and inline AV1 bytes,
   reserve/check first, then set fields without another allocation. Assert both
   owner contents and counters on failed updates, not merely a consumed-frame
   error category.
5. Share candidate admission/reconciliation with metadata fresh copies. On any
   post-allocation rejection, explicitly drop the local candidate before
   restoring the scalar snapshot. Existing metadata fresh error branches restore
   counters before implicit drop; fix that ordering in this B metadata work.
   This is not a reopened B-1 frame-helper runtime failure: no escaping candidate
   was observed there.

Actual-capacity rejection still needs a controlled test seam. The System-backed
Vec in these tests reports its requested exact capacity; pretending that an
allocator's hidden usable size changes Vec capacity would not test this path.
Keep a private shared implementation accepting a candidate-maker closure, with
production supplying the ordinary fallible empty-Vec allocation. A test closure
can allocate an empty candidate with capacity greater than the admitted request
after initial admission; the same production reconciliation must then reject,
drop it and preserve the old owner plus every counter. No global failpoint,
production bypass, or copied transaction algorithm is needed. Add explicit
actual-overcapacity rejection and candidate-drop-before-restore assertions for
both fresh metadata and replacement, including overflow and allocation failure.
These tests are required but not yet executed; current passing allocation-denial
tests are not evidence for actual-capacity failure.

The fixed B inventory is metadata fresh candidate cleanup, provenance growth,
unknown outer growth with nested owners preserved, and the AV1 update. Keep the
old 62 tests and the three additions unchanged. Per-ICC capacity checks remain
Slice C; this bundle adds no new colour-processing or codec scope.

## ICC S1 matrix-budget review: three regressions closed, slice still NO-GO

Independent execution confirms the three original S1 matrix-limit regressions
now pass, along with the unchanged route, wrapper-length and selected-white
cases. The external suites total 30 pass / 4 fail: boundary 7, structural parse 7,
and compile 16 pass / 4 fail. Three failures are the already assigned S2-S4 LUT
shape, owner-inventory and cumulative-direction cases. The fourth is the new S1
header-boundary case below; it is not a LUT or full-intent requirement.

| Fixed S1 boundary | Independent result |
| --- | --- |
| `s1_selected_matrix_headers_are_included_before_materialization` | Fails. On this host a Gray identity compiled stage retains 208 allocated bytes. Subtracting both calibrated fixed Arc control blocks leaves a 176-byte owned-storage lower bound. Exact 176 succeeds and evaluates finite output; one-under 175 also succeeds and allocates 208 bytes instead of returning ResourceLimit before allocation. |
| `s1_matrix_table_allocator_failure_drops_partial_owners` | Passes. Denying the actual 4096-byte decoded-table allocation returns the existing ResourceLimit variant, leaves no partial compiled owner live, and permits a later compile from the same Profile. |

Independent product execution passes library 18, compile-limits 4, LUT synthetic
23, structural parse 2 and transform 11 tests. Root additionally reports the
broader all-targets run as 159 pass / 1 ignored and Miri compile-limits 4 pass.
Those results do not exercise the newly failing external header test under Miri
and do not establish a complete compile allocation boundary.

The fixed S1 repair is the following four-part implementation, confined to the
matrix/curve planning and materialization boundary:

1. **Make the borrowed plan authoritative.** `CurvePlan` currently contains only
   counts; `MatrixPlan` records those counts but discards checked ranges and
   matrix values. Compilation then calls `matrix_for_direction` to parse raw
   tags again and walks the constructed owners afterward. Replace this with
   small lifetime-bearing plans containing validated encoded table/parameter
   slices, inline identity/gamma or bounded parameter values, curve usage and
   checked matrix/inverse fields. A private `materialize_matrix(plan, &mut
   budget)` and `materialize_curve(plan, &mut budget)` must consume that same
   plan, not look up/reparse profile tags. Keep bounded stack arrays for one or
   three curves. Structural and eager inputs already avoid cloning the eager
   matrix cache; preserve that success and existing facade semantics. Share the
   checked curve primitives with compatibility parsing rather than duplicating
   validation or extending the compile facade further.
2. **Admit every selected owner exactly once.** Plan `size_of::<MatrixProfile>()`
   and `size_of::<CompiledDirection>()`, the Curve outer capacity and each
   Table/Para float capacity. The current two-matrix float constant is not a
   substitute for these headers. Embedded matrices, Vec headers and enum fields
   are already within their containing owner's size and must not be added twice.
   Count logical entries separately; preserve current Identity/Gamma entry
   policy without inventing a float allocation. Raw Profile bytes remain under
   ParseLimits rather than being charged again as compiled ownership. Only the
   documented fixed Arc control bookkeeping is excluded. Charge owned Arc
   payload headers before construction; this does not make Arc::new a fallible
   API or establish general process-OOM recovery.
3. **Remove or account for parameter temporaries.** The inverse parametric path
   still creates `values.to_vec()` during monotonicity validation and another
   `values.clone()` during domain validation. Prefer shared borrowed evaluation
   and validation over the original slice or a fixed seven-value stack buffer,
   then allocate only the retained parameter owner. Preserve forward-only,
   inverse, non-finite and singular-matrix behavior. If any heap temporary must
   remain, it needs the same admission and an explicit lifetime through actual
   drop; a retained-only final walk cannot account for it. These two unchecked
   copies are source-confirmed gaps, not a newly claimed measured peak overrun.
4. **Use one transactional fresh-allocation boundary.** Make the live ledger
   non-Clone/non-Copy; a separate scalar checkpoint may be Copy. Complete planned
   admission is atomic: compute and validate next counters before mutation.
   Keep remaining planned storage separate from actual materialized ownership
   and temporary live storage. For each fresh owner, substitute its actual
   capacity for its reserved planned contribution, checking actual plus pending
   plus live temporaries without charging the original estimate a second time.
   The helper creates the empty Vec internally, checks arithmetic and complete
   known requirements before reserve, reserves fallibly, reconciles actual
   capacity before fill or any next allocation, then commits. On rejection,
   explicitly drop the candidate before restoring the scalar checkpoint; on
   success retain its charge until the owner actually drops. No pointer registry,
   global budget or independent reset per curve is needed. Standalone compile
   creates the ledger and calls the private plan/materialize seam; the assembler
   must be able to use that seam. Combined two-direction/LUT admission remains
   S4 and is not certified by this S1 repair. Use the existing ICC ResourceLimit
   error for budget/overflow/allocation failures, not a new public error variant.

Keep the two new external tests and the three original S1 assertions unchanged.
Actual-capacity rejection still needs a controlled private helper test: the
current System-backed Vec reports exact requested capacity, so a real allocator
denial is not evidence for overcapacity reconciliation. Share the production
candidate-admission implementation with a private candidate-maker closure. A
test candidate with larger capacity must consume reserved headroom for pending
curve/header owners, be rejected before a subsequent allocation, and drop before
all counters are restored. Check fresh success, empty/overflow, allocator denial,
actual-overcapacity failure and retry without a duplicated transaction algorithm
or global failpoint. This actual-capacity branch is a required unexecuted gate,
not another failing public fixture or an expansion into LUT work.

New responsibility modules are the right boundary, but their presence alone is
not S1 acceptance. Remove obsolete private validation/cache fields only after
preserving the channel/PCS and eager-facade checks they represented; three unused
private-item warnings remain in this snapshot. S1 remains NO-GO pending this
finite repair. S2-S4, all intents/domain policy, LCMS accuracy and full H3 remain
unfinished. Review changed no product code.

## Parent Slice B: candidate transactions verified, AV1 update still open

Independent execution of the frozen B repair passes the existing main harness
68/68, including all previous 65 assertions and three new tracked cases. The
original harness source and allocator observer are unchanged. Product mapping
tests pass 26/26 on Rust 1.91; coordinator Miri evidence covers the same 26.

A separate candidate-observer target passes four of five independent tests:

- Fresh metadata candidates with actual capacity above the request reject under
  each independently constrained frame/live/metadata limit. The actual allocator
  request and exact candidate deallocation are observed; all counters stay
  unchanged and an exact-capacity retry succeeds.
- Provenance and unknown outer replacements pass the corresponding three-limit
  table, including the known inline AV1 cost and borrowed live-only seed. Failed
  actual-capacity reconciliation preserves old pointer/length/capacity, nested
  payload pointer/content and all counters. Exact replacements move the nested
  owner, retain the new outer until real drop, and preserve the payload pointer.
- Predictable inline-budget errors reject before the candidate-maker. Real
  fresh allocator denial returns Allocation without creating a candidate owner.
  Additional fresh precheck/empty/overflow assertions are prepared, but their
  rerun encountered the subsequently resumed AV1-helper work in progress; they
  need execution against its next frozen snapshot.
- A preexisting identical AV1 description still succeeds at the complete final
  metadata budget. This positive test is not a proof of internal ledger equality.

The fresh and replacement helpers leave committed counters untouched until the
successful commit. Their error path explicitly drops its local candidate; no
scalar restoration is needed in this version. Real deallocation is observed,
and source inspection verifies the absence of an early counter mutation. The
observer never dereferences candidate addresses or reads a mutably borrowed
ledger. This is not a process-wide no-allocation claim: diagnostic Strings are
outside the watched data-owner request. The tests use the production candidate
admission/reconciliation implementation, not a copy of its transaction algorithm.

One fixed-inventory P2 remains in the inline AV1 update. Adding AV1 to metadata
without it and overwriting the same existing AV1/provenance produce identical
metadata, descriptors and owned-byte totals, but their final ledger tuples are
respectively (1183,1183,20) and (1191,1191,28) on this host. The latter charges the
already present eight-byte inline owner again. The failing test is
`b_candidate_existing_av1_update_does_not_charge_same_inline_owner_twice`.
It demonstrates counter inconsistency, not a newly claimed public low-budget
rejection. The separate exact final-metadata test above succeeds.

Finish only the previously required AV1 update seam: extract a private operation
over mutable metadata and the same ledger; calculate missing provenance and
missing AV1 inline ownership separately; pass only the actual inline delta to
the accepted growth primitive; then set fields within admitted capacity. Updating
an already present AV1 value must not debit its inline owner again. Test this
operation directly for zero-delta/repeated updates and for allocation, budget and
actual-capacity failure, preserving metadata values, outer/nested pointers and
all counters. The earlier consumed-frame error test cannot observe its former
metadata owner and does not replace this agreed mutable-owner assertion.

The candidate primitive and provenance/unknown growth repairs receive local
acceptance; Slice B remains NO-GO for the AV1 update item. Per-ICC actual capacity
is still Slice C, and unused private helper cleanup remains required separately
(five warnings in this independent product-test configuration). Full B/D/H6 are
not accepted. Review changed only ignored tests and this record, not product code.

## ICC S1 follow-up: borrowed materialization accepted, two ledger checks remain

The matrix compiler now consumes lifetime-bearing curve plans and checked matrix
fields rather than reparsing raw tags. Selected MatrixProfile/CompiledDirection
headers, Curve outer storage and payload capacities are included once. The new
matrix parametric path uses borrowed validation without the earlier temporary
parameter clones; remaining legacy-only copies are not reopened by this slice.
The header exact/one-under observer now passes, as do all earlier S1, route,
wrapper-length and selected-white regressions. External public suites total
31 pass / 3 previously assigned S2-S4 LUT failures. Independent product library
21, compile-limits 4, LUT 23, structural parse 2 and transform 11 all pass.

Three added private-seam tests isolate the remaining ledger contract. One passes:
with actual ownership 4, candidate capacity 13 and another pending owner 8, limit
24 rejects the combined 25 even though the candidate alone fits. Its real
allocation and deallocation are observed, the prior owner and checkpoint remain
unchanged, and exact retry plus the remaining owner succeeds. Empty and overflow
requests also preserve that checkpoint. This extends the tracked three private
tests; their earlier overcapacity fixture alone exceeded its limit and did not
prove the other-pending-owner case. Root's reported Miri 3 covers those tracked
tests, not these new external checks.

Two fixed-contract checks fail:

- P2, admission atomicity: entry-limit, byte-limit, arithmetic-overflow and matrix
  storage rejection mutate scalar counters. A three-curve MatrixPlan rejected by
  a two-entry limit leaves entries 3/pending 247 instead of its preceding
  entries 0/pending 7. No allocation is involved. Make each scalar admission
  compute/check before commit; make complete plan admission atomic too, using
  aggregate checked costs or restoring a scalar checkpoint on any error before
  materialization. Do not clone the live ledger or preserve a rejected partial
  plan. Keep a retry assertion after the failed admission.
- P2, known fresh request preflight: `try_new_vec::<u8>(16, planned=8)` under limit
  8 requests the 16-byte allocation before rejecting it. Compute checked
  count-times-element-size and validate its equality to this owner's planned
  contribution, pending allowance and current total before invoking the allocator.
  Retain actual-capacity reconciliation for genuinely larger returned capacity.
  Current matrix callsites supply matching values: this is a private-helper
  contract reproduction, not a demonstrated public-input bypass.

The new private suite is 1 pass / 2 fail. Fix only these admission/fresh-helper
checks; borrowed-plan consumption, complete selected headers, the new parametric
path and measured pending-capacity rollback remain locally accepted. S1 stays
NO-GO until both checks pass. S2-S4, unused-helper cleanup, full intent/domain
semantics, LittleCMS accuracy and H3 completion remain separate unfinished gates.

## Parent Slice B AV1 update: repeated-value repair accepted, one presence branch open

The extracted mutable AV1 update fixes the previous duplicate inline charge.
Independent main-harness execution passes 71/71; the separate candidate target
passes 6/7. Fresh/replacement actual-capacity rejection, pending-inline preflight,
empty/overflow checks, allocator denial and exact retry all pass. Direct mutable
AV1 budget and real allocator-failure tests preserve every metadata value,
provenance/unknown outer pointers, nested payload pointer and ledger counter.

The fixed four-way AV1-value/provenance table leaves one P1 private-boundary case:
Some AV1 with no AV1 provenance. The helper checks provenance only inside its
AV1-absent branch, then the setter grows provenance infallibly. With expected
metadata ownership and limit 44, the test observes one eight-byte allocation,
actual ownership 51 and success while retained metadata accounting stays 43.
The private owned-parts constructor represents this state and the helper does
not validate an invariant excluding it. Normal public setters add provenance,
so this is not a claim of a demonstrated malformed public-input route.

Compute the two missing-owner decisions independently. Reserve missing
provenance even when AV1 already exists, passing zero inline delta in that case;
when only AV1 is missing, debit only its established logical size. Use the same
size definition in prediction/update/tests instead of another hard-coded eight.
Keep setters after all fallible checks. The other three combinations and the
earlier repeated-update/candidate observers remain unchanged. Slice B remains
NO-GO for this one fixed-item branch; Slice C and unused-helper cleanup remain
separate, and no product code was edited by review.

## ICC S1 matrix compilation: limited checkpoint accepted

The final two ledger repairs pass independent review. Scalar admission checks
all next values before mutation; whole MatrixPlan admission restores its scalar
checkpoint on failure before any materialization. The same budget accepts a
fitting retry afterward. Fresh Vec allocation checks count-times-element-size
and equality with the admitted owner contribution before invoking the allocator.
The known 16-byte request under an eight-byte plan now makes zero requests.

All three private-seam tests pass, including actual ownership plus a larger
candidate plus a separate pending owner, real candidate destruction, unchanged
prior owner/checkpoint, exact retry and empty/overflow handling. The earlier
borrowed-plan materialization, complete selected header/outer/payload inventory
and new matrix parametric validation remain unchanged and accepted. No additional
failure remains in this fixed S1 inventory.

Independent external results are 34 pass / 3 known S2-S4 failures across 37
checks: boundary 7, structural parse 7, public compile 17 pass / 3 fail, and
private ledger 3 pass. Product all-targets execution passes 164 with one existing
ignored test. Coordinator Miri 5 covers the tracked private-budget tests, not the
three external observer tests. These scope differences remain explicit.

S1 receives limited GO for selected matrix planning/materialization and its
compile ledger. The three LUT shape/ownership/two-direction budget regressions,
unused private-item cleanup, full intent/domain semantics, LittleCMS accuracy
and full H3 remain open. Successful integer-wrapper budgeting is also separate.
This result does not authorize a broad product commit or publication; review
edited no product source and left version/staging state unchanged.

## Parent Slice B transactional metadata: limited checkpoint accepted

The final AV1 update now decides missing inline value and missing provenance
independently. All four presence combinations pass, including an existing AV1
value with no provenance through the private owned-parts seam. Repeated equal or
different AV1 values do not charge the existing inline owner again. Both missing
and existing AV1 cases preserve metadata values, outer and nested payload
pointers, capacities and all ledger counters on a budget error or real allocator
denial. The setter runs only after successful fallible admission.

Independent results are main harness 72/72, separate candidate observer 7/7 and
product mapping tests 30/30. The candidate target also checks fresh/replacement
actual-capacity rejection against each frame/metadata/live bound, the whole
replacement live peak, pending inline cost, empty/overflow requests, destruction
of rejected candidates and exact retry. These are real System-backed TLS/RAII
allocator observations, not only a synthetic failpoint. Original main-harness
assertions and observer sources are unchanged.

Slice B receives limited GO for this fixed transactional allocation inventory.
Common AV1 logical-size definitions remain a small DRY cleanup: prediction still
uses literal eight where the update uses the current eight-byte type size. The
values currently agree; this is not another demonstrated allocation failure.
Per-ICC actual-capacity checks and unused private-item cleanup remain Slice C.
Full B/D, authoritative native metadata/ordered geometry and H6 are not accepted.
No product source, version or staging state was changed by review.

## Parent Slice C: fixed ICC-copy and cleanup implementation bundle

Keep this slice to the already assigned per-ICC boundary, final metadata
observers and cleanup. Reuse Slice B's candidate transaction; do not add another
aggregate allowance, owner registry or second copy implementation.

1. Store the per-owner ICC byte limit in ConstructionLedger. Extend the common
   fresh-metadata admission core with an owner byte ceiling. Check the requested
   count-times-element-size against that ceiling and all existing budgets before
   calling the candidate maker. Check the candidate's actual capacity against
   the same ceiling and all three ledger budgets before committing any counters.
   Ordinary metadata callers retain their existing unconstrained per-owner
   ceiling. Add `try_copy_icc_with(source, make_candidate)` for the same private
   maker seam, plus `try_copy_icc(source)` using the production maker. Copy only
   after successful admission. Failure destroys the empty candidate and leaves
   all counters and existing owners unchanged; do not commit generic metadata
   first and only then reject its ICC capacity.
2. Route both the Rich source ICC projection in `color_information_impl` and
   the active ICC clone in `clone_color_information_with_ledger` through that
   one helper. Preserve separate checks on existing retained/source capacities.
   A fresh destination uses requested length, then actual destination capacity;
   it must not inherit the source's spare capacity. Keep predictable final
   dimensions, scalar values and provenance before copying, and preserve both
   mapper entrypoint limit tables and exact/one-under real-copy observers.
3. Use one established logical AV1 size definition in the owned walk, fresh
   prediction, additional-byte calculation, projection, update and their tests.
   Do not conflate AV1 bytes with unrelated nclx or geometry fields. Clean only
   the identified unused private inventory: ledger `new`/`try_copy`; old
   NativeMapPlan inspect wrappers; `consume_native_frame_with_metadata`;
   `metadata_owned_bytes`; `unknown_colr_capacity`/`try_clone_owned_bridge`;
   and `PlaneDescriptor::try_planar`. Keep genuinely used test seams under the
   correct test/feature gate, remove obsolete duplication, and retain all
   required validation and public/legacy entrypoints. Normal library and unit
   configurations expose different subsets of this same inventory.

Three independent ICC-copy fixtures are prepared against the agreed production
seam. Requested overflow of the per-owner limit must call neither maker nor
allocator. Actual overcapacity necessarily allocates a candidate: a fully
initialized sentinel byte buffer is cleared to length zero, and its bytes are
observed immediately before real deallocation. Rejection must leave those bytes
unchanged, destroy the candidate once and preserve source/ledger state. Exact
retry both succeeds and changes the sentinel as a positive copy control. A real
allocator-denial case preserves the same state and returns Allocation. No
uninitialized memory is read, and allocation count is not used as a substitute
for copy observation. Before implementation these fixtures cannot compile
because the agreed helper is absent; they are prepared acceptance checks, not
passing evidence or a new product regression.

Freeze this slice only after those checks plus unchanged B main 72/candidate 7,
mapping tests, final metadata exact/one-under and both-entrypoint limit observers
pass. Verify ordinary library and highres-only configurations after cleanup;
do not mask new warnings as baseline. No wider color conversion, codec or public
API scope is added by this bundle.

## ICC S2 selected LUT shape: matrix repair accepted, shared shape still open

The full 48-byte selected matrix range is now checked before A/B/M curve
materialization in both directions. The original allocation observer passes,
and an added private check observes zero requested allocation bytes for both
matrix truncation and wrong-direction-before-invalid-channel rejection. Existing
legal/illegal stage-pair tests remain passing. This closes that original matrix
regression, not the whole S2 validated-shape contract.

Two minimal checks in the same S2 inventory fail:

- P2, mft plan completeness: mft1 and mft2 tags missing their last table byte
  are accepted by the new plan in either direction. LutShape::Mft contains no
  ranges/counts; materialization falls back to the old parser, which finally
  rejects the truncated table. That old parser still checks the total range
  before decoding allocations, so this is an incomplete authoritative-plan
  contract, not a demonstrated public allocation-before-range-check bypass.
- P1, grid validation phase: with three selected grid dimensions equal to two
  and the next grid byte equal to one, both mAB/mBA plans succeed. Their existing
  materializer then rejects after allocating A/B Curve outers and the grid:
  320 requested bytes, including one 128-byte grid allocation on the tested
  64-bit target. The original parser already rejects this fixture; the test asks
  that its validation happen in the plan, not for a new interpretation of unused
  grid fields. Plan checks only selected dimensions while materialization walks
  up to sixteen entries, exposing the duplicated shape interpretation.

Complete these together as one S2-only repair:

1. Make mft shape retain checked input/CLUT/output ranges, entry counts, element
   width, dimensions and fixed matrix values. Validate all existing structural
   rules before returning it. Materialization must consume those descriptors,
   not dispatch back through the old raw-header parser.
2. Make mAB/mBA shape retain bounded stack curve descriptors with checked ranges
   and kinds, the complete selected matrix and one checked CLUT/grid descriptor.
   Consume these in every materializer. Move shared structural interpretation
   into lut_plan/curve_plan responsibilities instead of keeping duplicate
   encoded_curve_info/curve_size and independent grid scans in lut.rs. Reuse
   existing curve planning where compatible while preserving LUT entry policy,
   forward-curve semantics and current direction/stage error precedence.
3. Keep any compatibility parser as a thin plan-then-materialize entrypoint, or
   share the same structural primitive; do not create a second validated path.
   Preserve current legacy acceptance/rejection while making the two grid walks
   one decision. Leave decoded-owner ledger wiring and two-direction aggregate
   admission to S3/S4; no new intent/domain or curve-semantic work belongs here.

Independent new private tests are 1 pass / 2 fail. Existing external suites are
boundary 7, structural parse 7 and S1 private ledger 3 all passing; public compile
18 pass / 2 known S3/S4 failures. Product library 23, compile-limits 4, shape 2,
LUT 23 and transform 11 all pass. The official LUT fixture was present, not an
empty optional skip. Only the new lut_plan module declaration was added to the
old private S1 harness; all its assertions were preserved. Review changed no
product code. S1 limited acceptance stands; S2 remains NO-GO for these fixed
shape gaps, with S3/S4 and the wider H3 gates separately unfinished.

## Parent Slice C review: ICC-copy boundary accepted, one size cleanup remains

The shared metadata fresh-candidate core now checks both requested and actual
per-owner ICC capacity before committing any counters. Source ICC projection and
active ICC cloning both use that core through the same ICC-copy helper. The
independent sentinel fixture passes: requested excess calls no maker; actual
excess allocates then destroys one candidate without copying source bytes or
changing source/ledger state. Exact retry succeeds, with changed sentinel bytes
as its positive copy control. Real allocator denial also preserves state.

Independent runs pass ICC-copy target 36 (three new observer checks plus included
product tests), unchanged main 74, B candidate 7 and product mapping 32. The main
suite retains final metadata exact/one-under, both-entrypoint limits-before-copy
and source spare-capacity/fresh-clone checks. Normal avif+highres library checking
has no warnings; highres-only typed 9/safety 1/Stage A 6 pass with only the two
previously recorded draw warnings. Obsolete private helpers were removed and
remaining test seams gated without widening feature dependencies.

One fixed cleanup item remains before complete Slice C acceptance: the consumer's
`av1_metadata_delta` prediction still starts with literal `8usize` instead of
`AV1_COLOR_INFORMATION_BYTES`. All other assigned AV1 byte calculations now use
the shared definition. Replace this last literal and its stale size wording;
retain independent type-size test expectations. The current value is equal, so
this is a DRY/maintenance requirement, not another reproduced runtime-budget
failure. ICC-copy behavior receives limited acceptance; final Slice C acceptance
awaits this small cleanup. Full B/D/H6 and public bounded decode/color conversion
remain outside the accepted slices. No product source was edited by review.

## Parent Slice C final behavior and checkpoint dependency boundary

The last AV1 prediction literal now uses the shared size definition and its
comment agrees with that contract. Independent final reruns pass main 74,
candidate 7 and all three ICC-copy observer cases. The earlier complete ICC-copy
target 36 includes product tests as well as those three independent cases.
Slice C receives limited GO for the assigned per-ICC requested/actual capacity,
copy ordering, failure ownership and size/unused-helper cleanup behavior.
Coordinator verification additionally reports mapping Miri 32, MSRV all-target
checking and wasm checking passing. Full B/D/H6 and conversion/decode completion
remain excluded. Three separately found strict-Clippy issues are being repaired;
ordinary warning-free checking is not a replacement for that gate.

The parent-only product checkpoint has a separate dependency blocker. Its
recorded AVIF gitlink is still 1203c44, which has no RichAvifInfo. The accepted
internal ownership adapter imports that type from a55753e. The ignored review
baseline and current workspace use a55753e, so their successful tests do not
prove compatibility with a clean checkout of the recorded gitlink. Committing
the complete parent bridge while excluding the gitlink would break the
avif+highres build in that checkout. No gitlink change is authorized here.

Therefore do not stage the complete parent foundation bundle yet. Either obtain
a separately approved dependency-checkpoint synchronization, or prepare and
validate an explicitly split Stage-A-only snapshot without the bridge module
and its bridge-only helpers/reexports. The latter is not simply omission of the
five bridge files: dependent module declarations and feature-specific helper
warnings need matching partial changes and fresh staged-snapshot tests.

Outside that dependency boundary, read-only inspection found the proposed
parent changes limited to the Stage A vocabulary, the accepted A/B/C internal
adapter, their tests and the test-target manifest registration. HLG inverse
arithmetic keeps the division in f64 until the final cast; the added Eq trait
is supported by private fields and finite checked construction. Domain defaults
remain Unknown; explicit interpretation is separate from storage. ResourceLimits
validation covers already-owned frames and does not bound a preceding decoder.
The manifest diff changes no dependency, feature default or version. Codec,
encoder, JXL, gitlinks and version files stay excluded from this review's staging
authority. No files were staged or committed by review.

The coordinator subsequently approved a separate synchronization of the already
reviewed AVIF checkpoint. Independent inspection confirms cbdd77a changes only
the avif gitlink from 1203c44 to exact a55753e; no dirty codec implementation is
included. The local dependency blocker above is therefore resolved. The clean
review worktree is exactly a55753e, and the current parent bridge compiles and
passes its external cases against that baseline. This local checkpoint does
not establish remote availability or registry-version API compatibility.

After the final strict-Clippy repair, the proposed parent product checkpoint is
limited to fourteen files: wml2/Cargo.toml; highres hdr, metadata, mod, types,
domain, limits and processing; highres/avif mod, mapping, metadata, allocation
and mapping_tests; and tests/highres_stage_a.rs. This checklist may be recorded
separately. No encoder, JXL, ICC working-tree code, further gitlink, lockfile or
version change belongs in that parent checkpoint. Its acceptance remains a
foundation/internal-adapter checkpoint, not completion of the public pipeline.

Final freeze review accepts those fourteen parent files for staging, followed by
an exact-index check before commit. The former eight-argument internal helper
is replaced by six arguments plus a three-field scalar state; both test-harness
calls preserve their prior booleans/live-byte values and every observer/assertion.
The unused eight-argument test shim is removed, and the two bool assertions are
equivalent. Independent final main 74, candidate 7 and ICC-copy 3 all pass.
Installed Clippy 0.1.95 reports zero highres diagnostics; the full command still
fails on 24 diagnostics outside this scope, so it is not reported as globally
clean or as an MSRV-Clippy run. Coordinator's final mapping Miri 32 and MSRV
all-target checks also pass. Slice C and the scoped foundation/internal adapter
are accepted; public byte decoding, complete Stage B/D and H3-H6 remain unfinished.

Exact-index review then confirms all fourteen staged files match the reviewed
working-tree contents, have ordinary file modes, and contain no extra path,
gitlink, dependency/version or environment-specific data change. The staged
diff passes whitespace checks. That exact parent foundation/internal-adapter
checkpoint is accepted for commit, with the previously stated scope exclusions.

## ICC S2 follow-up: early rejection repaired, materializer sharing still pending

The three independent S2 checks now pass, as do all three S1 private-ledger
checks. Truncated mft ranges and the mismatched grid-validation phase are fixed;
the full matrix and error-precedence checks remain passing. These positive
results do not complete the remaining implementation contract.

Read-only inspection still finds two authoritative-shape gaps: mft's planned
materializer checks its descriptor then dispatches to the old raw-header parser;
mAB/mBA CurveSetShape still holds only offset/count and reconstructs each curve
through curve_size/parse_curve_forward. Curve planning is not shared with
curve_plan. Planning/materialization duplication has also grown lut.rs beyond
1500 lines, with obsolete parsing paths retained under dead-code allowances.
These are the same already assigned S2 requirements, not new reproduced runtime
failures or added S3/S4 scope. S2 remains NO-GO until they are implemented.

Finish the fixed bundle directly:

- Decode mft input tables, CLUT and output tables from the planned ranges,
  widths/counts and matrix; remove the fallback to the old raw-header parser.
- Store each selected curve's borrowed CurvePlan in a bounded stack array
  (at most the supported three channels per set), with explicit absent/used
  counts. Materializers decode its kind/count/checked payload and do not reread
  curve signatures/count headers or rediscover sizes.
- Share the decoded curve loop through a fallible allocator callback if needed:
  the S1 wrapper keeps its existing CompileBudget allocator and semantics; the
  LUT wrapper can retain its current fallible allocation until S3 wires the
  ledger. This does not authorize a semantic curve rewrite or claim LUT owner
  budgets complete. Preserve S1 admission/rollback assertions.
- Move shape/range/stage interpretation into lut_plan/curve_plan responsibility,
  and keep compatibility parsing as thin plan-then-materialize wrappers. Delete
  obsolete raw-parser duplicates rather than suppressing their unused warnings.
  Keep the repaired three tests, stage-pair/error-precedence coverage, and the
  existing S3/S4 failures unchanged through this refactor.

## C1 substep 4: fixed counter, stack and sort implementation slices

Substeps 1/2/3 remain accepted in their recorded scope: the original 39 external
assertions pass unchanged. Substep 4 is not accepted. Eight new private-helper
checks produce one positive and seven failures; one additional public-entry
check also fails. These are the previously identified counter/stack/sort scope,
not a reopening of ownership/drop accounting or AV1/deep-decoder work.

- Public repeated IPMA with two entries for one item and no extra associations
  succeeds under max_items=1 and copies the selected payload. The exact bound 2
  and Legacy path both succeed. The scan checks each IPMA entry count separately.
- Shared structural parsing accepts a second iinf, iloc, IPMA entry, property,
  or association past its same-kind bound, after allocating that new owner.
  Property counting must precede nested av1C/colr/string materialization, not
  just the outer property Vec reserve. Different kinds at their own exact bound
  pass together; do not sum them into an invented unique-item count.
- Direct Native method-0 payload copying requests an unaccounted 16-byte
  recursion-stack Vec despite requiring no recursion. A 1024-alpha unique-ID
  permutation requests two 32768-byte allocations: charged ID-vector growth
  plus unaccounted stable-sort scratch. The sort assertion expects the former
  allocation, not a zero-allocation complete parser.

Implement and freeze in these three small slices:

1. Add fixed scalar ParseCounts to the existing non-Copy ParseContext. Provide
   checked admission for IinfEntries, IlocEntries and IpmaEntries against
   max_items, and Properties and Associations against max_properties. Check
   addition/limit before mutation. Replaced tables count as parsing work, so
   dropping owners does not release counts. Ownership rollback must not silently
   erase consumed work. Legacy admission is a no-op.
   Wire iinf/iloc after validated count headers and before their first reserve;
   IPMA uses one borrowed count walk to admit entries plus associations before
   materialization, with atomic multi-counter admission and no loop double count;
   ipco admits each property before parsing its payload. One context survives
   repeated meta/iprp calls. The pre-scan may reuse the same checked arithmetic
   independently, but must not seed totals and then charge the structural walk
   again. It is not a substitute for shared-parser enforcement. Add exact,
   one-over, overflow/no-mutation and repeated-meta coverage, preserving all
   five fixed data-owner allocation observers and the mixed-kind positive.
2. Extract borrowed direct-item extent planning/copying for Native methods 0/1
   from the recursive compatibility routine. Reuse checked source ranges and
   cumulative length before the existing Payload-token reserve. Native method 2
   remains Unsupported before allocation; Legacy retains its recursive path,
   cycle detection and error precedence. Remove the Native stack allocation,
   rather than inventing an uncharged local Vec or enabling new recursion.
3. Keep Native gathering on the existing unique-selection helper, then use an
   allocation-free ordering step for those unique numeric IDs. Native-only
   sort_unstable_by_key is sufficient when uniqueness is established; keep
   Legacy stable ordering and its first-duplicate choice unchanged. Retain
   string/payload ownership and release the temporary ID token only after its
   consuming iterator drops. Preserve sorted output, duplicate/fallback and
   exact returned-record assertions. Add allocation-failure tests only around
   fallible owners; do not inject allocator failure into unchecked stable sort.

The fixed external probes are count_stack_boundary's c1_ cases and
count_public_boundary::c1_public_repeated_ipma_entry_limit_precedes_payload_copy.
No product edits were made for these observations. Existing C1 tests, Legacy
compatibility, MSRV and portable tests remain required after each implementation
slice. AV1 header/tile preallocation, C2 and C3 remain separately open.

The parent foundation checkpoint also has isolated clean-checkout evidence:
parent 6830906 with reviewed AVIF a55753e and encoder 5b00acd builds without
dirty dependencies. Root reports lib43, typed9, stageA6 and safety1 on host and
i686, WASI lib43 execution, plus callback4 and explicit external-Next1 in both
feature configurations. This is foundation/compatibility evidence, not H4,
complete decoder bounds, encoder quality or ICC oracle acceptance.

## H5 slice 1: property-index behavior verified; tracked graph assertions pending

The writer now captures each appended property's index and uses those indices
for both primary and alpha IPMA entries. Independent execution passes lib23 and
the two tracked container tests. Two additional independent writer tests pass:
all eight color-presence/alpha combinations have exact IPMA entry counts,
nonzero in-range indices, required essential flags, distinct primary/alpha av1C
payloads, existing-item reference endpoints and correct iloc payload ranges;
the old writer path is byte-identical to clean checkpoint 5b00acd for default,
nclx, prof and rICC options with/without alpha. This is a container-writer
comparison with fixed encoded payloads, not an independent AV1 decoding oracle.

No further property-index product defect was found. Before calling the tracked
slice complete, strengthen container_properties.rs with the exact IPMA count
and end cursor, iref presence/absence, one auxl record and its endpoint IDs
matched against iinf/iloc. Find meta by top-level box traversal instead of a
fixed byte offset. Keep every existing association/essential assertion. The
existing 1-to-2 auxl direction is characterized as unchanged Legacy output;
these tests do not establish new HEIF directional conformance. AVIF specifies
auxiliary relationships through HEIF, separately from property indexing
([AVIF container hierarchy](https://aomediacodec.github.io/av1-avif/v1.2.0.html#avif-box-structure)).

The metadata helper-only second test is not item-reference coverage. ICC byte
and type authority, absent-color signaling policy, AV1 header consistency,
native luma/chroma validation and source-lossless/alpha fidelity remain the
previously listed later H5 slices. No version, dependency or Legacy API changed
in this review; temporary tests remain ignored. Full encoder acceptance and
publication are not granted.

## ICC S2 latest review: descriptor consumption accepted; two fixed repairs remain

Mft materialization now decodes the planned table/CLUT ranges directly instead
of returning to the old raw parser. mAB/mBA curve sets now contain a bounded
three-element array of borrowed CurvePlan values consumed by the shared curve
materializer. The previous three shape/early-rejection tests pass. These parts
of S2 are closed; S1 acceptance and its independent three ledger tests remain
unchanged. S2 as a whole remains NO-GO for the following finite repairs.

1. **P1 ParseLimits regression in shared LUT curves.** The selected mAB/mBA
   curve path creates a local TransformLimits budget but no longer applies the
   Profile's ParseLimits.max_curve_entries, formerly enforced by
   parse_curve_forward. A 32-entry curv under a parse bound of 16 now compiles
   in both directions, allocating its 128-byte table; Transform assembly also
   succeeds and allocates both tables. The exact parse bound 32 passes. Both a
   public Profile/Transform probe and a private shared-plan probe reproduce this
   one cause, with the old curve helper's ResourceLimit result as a control.
   Restore an allocation-free check over all selected borrowed curve plans
   before any curve-set outer or payload reserve. Keep per-curve ParseLimits
   separate from cumulative TransformLimits. Preserve the former curv
   identity/gamma/table count rule; do not silently impose that count rule on
   para functions that the old parser handled differently. The check can live
   at the plan/materialize boundary where both limits are available; reintroducing
   the raw curve parser is not a fix. Retain exact-positive and allocator-zero
   rejection assertions for both directions and assembler sides.
2. **P2 unfinished responsibility split and duplicate parsing.** Shape/range
   construction and check_encoded_limits still occupy lut.rs, while lut_plan
   delegates back into it; lut.rs remains over 1300 lines. Move that structural
   interpretation to lut_plan and reuse its descriptors for encoded-count/cost
   checks. Keep lut.rs focused on materialization/evaluation. Remove the old
   unused parse_clut duplicate rather than hiding it with allow(dead_code).
   Keep compatibility entrypoints only as thin shared-plan wrappers, test-only
   where appropriate. Preserve direction-before-channel errors, stage pairing,
   both-direction invalid-range observers and the S1 shared curve semantics.

Independent execution: boundary7, parse7 and S1-private3 pass; compile21 is
18 pass/3 fail, and S2-shape4 is 3 pass/1 fail. Two failures are the same new
ParseLimits regression; the other two are unchanged S3 retained outer/grid
storage and S4 combined-direction budgets. Selected product suites total 65
passes (lib23, compile4, shape2, LUT23, parse2, transform11), with the official
LUT fixture confirmed present. Root separately reports all-targets166 pass and
one ignored. Passing existing suites does not dismiss the independent failures.

The temporary local CompileBudget used by the LUT curve materializer is not
acceptance of full pending/retained LUT ownership or shared-direction budgets.
Those remain S3/S4, not additional S2 repair work. The three pre-existing unused
items, all-intent/domain behavior and LCMS precision gates also remain open.

## C1 substep 4 counter slice: bounds verified; Legacy guard repair pending

Independent execution passes the original 39 tests plus the public repeated-IPMA
case (40/40), all six fixed counter/mixed-kind cases, and four added checks for
atomic IPMA retry, arithmetic overflow without mutation, counts surviving
ownership rollback/separate MetaState instances, and complete Legacy admission
no-op. The new checks observe the work counters, not only ParseAccounting.
The five structural admissions precede table reserve and nested property copies;
the independent pre-scan does not seed and double-charge the context counters.
The scan's per-box IPMA check is not the authoritative cumulative enforcement.

One small compatibility repair remains before counter-slice GO: the new
ipma_counts walk is called unconditionally, even for Legacy. Against exact
checkpoint a55753e, a truncated one-entry IPMA now produces a different
NotEnoughData diagnostic. Guard the borrowed count walk and admit_ipma together
with is_native_still, retaining the old Legacy parser path. The additional
legacy-error assertion fails until that repair. Move the four passing counter
proofs into tracked tests without weakening the existing allocation observers.
This is a P2 compatibility/coverage repair, not a counter-budget bypass.

The Native recursion-stack and stable-sort scratch assertions still fail and
remain the next two substep-4 slices. They are not part of the counter repair.
Root separately reports lib499/6 ignored, native10/limits5/phase9/rich3 on Rust
1.88, strict Clippy success, and Miri counter3; these do not replace the pending
Legacy guard or the stack/sort work.

## H5 property-index slice: limited acceptance after tracked graph coverage

The final test-only addition supplies top-level meta discovery, exact IPMA entry
count/end cursor, iinf IDs, iref/auxl presence and endpoints, and iloc-to-mdat
extent checks across all eight combinations. Existing association/essential
assertions remain intact. Independent final execution is lib23 plus container3
passes, not 65 separate container tests. The earlier independent writer graph
and eight-option Legacy byte comparisons remain valid because product source
did not change during this test-only repair. No before-fix failing execution is
claimed: that was not run for the initial index implementation.

H5 slice 1 is accepted only for property indices and the characterized container
references/Legacy writer compatibility. Remaining H5 color authority, signaling,
native plane validation and lossless quality gates stay open. This does not
authorize publishing or committing unrelated encoder WIP.

## LCMS oracle preparation: transport only, full-grid comparison pending

Black-box transicc reports calculator 5.1 / LittleCMS 2.19. Eight sentinels for
sRGB2014-to-itself and v4-Preference-to-sRGB2014 run with -n -c0 -d1 -t1 and
without BPC, encoded, quantized or bounded-mode flags. RGB transport uses 0..255
and this pipe output prints four fractional digits. Self-transform examples
include 127.5 becoming 127.5019 and 1 becoming 0.9961; they are not a precision
pass or proof of internal arithmetic precision. The mixed-profile relative
black becomes approximately 29.3/255; do not enable BPC or change profile data
to erase that observation.

Work paused at transport calibration for the higher-priority frozen review.
RGB4913 and Gray4096 comparisons, fixed destination-Lab measurement, DeltaE00
self-check/Sharma validation and all acceptance thresholds remain unexecuted.
No LittleCMS implementation source was inspected or product dependency added.

## C1 substep 4 counter slice: limited acceptance

The Native-only IPMA count guard preserves the Legacy parser and its existing
truncation diagnostic. The final tracked overflow proof executes each admission
separately and checks the complete context plus ownership accounting before and
after failure; eager evaluation and the ownership-only assertion are removed.
Independent execution passes the eight tracked counter tests, eleven filtered
external counter proofs, and forty old/public boundary tests. Included product
tests are not counted again as independent cases. Root also reports final Miri8
and strict Clippy success.

The five independent count kinds, atomic IPMA admission, failed-admission retry,
overflow, Legacy no-op and count-work survival across owner rollback/replacement
are accepted for this counter slice. Native direct-payload recursion-stack and
alpha stable-sort scratch owners remain the next two fixed substep-4 slices.
This does not accept all of C1, AV1 pre-allocation, C2 or C3.

## LCMS first diagnostic pair: validated metric and complete RGB grid

The independent f64 DeltaE00 implementation passes all 34 published
[Sharma/Wu/Dalal reference pairs](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/)
to their four-decimal printed precision (maximum absolute discrepancy
0.000049498977), plus identity, symmetry and zero-chroma checks. It was derived
from the paper's equations, not the authors' software or LittleCMS source.
Reference data and provenance remain in ignored test_data.

One diagnostic pair, official sRGB v4 Preference to sRGB2014 with Relative
colorimetric intent and BPC off, completes all 4913 RGB grid points (14739 channel
values), including endpoints, without errors or omitted points. Product F32
evaluation uses clamp=true. Black-box transicc uses -n -c0 -d1 -t1, RGB transport
0..255, and no encoded, quantized or bounded-mode flags. Both output sets are
measured by the same destination-to-Lab transform with physical Lab output;
this does not treat the built-in Lab profile as a raw tag evaluator.

DeltaE00 median is 0.0017950127, nearest-rank p95 is 0.0063801949 and maximum is
0.0459857599; maximum normalized channel absolute error is 0.0003992994. These
meet the existing 0.1/0.25/1.0 thresholds for this one pair without relaxing them.
The worst DeltaE00 input is (0, 0, 0.125). Pipe output has four fractional digits;
this result does not establish the oracle's internal arithmetic precision.

Executable and profile SHA256, command options, raw outputs, all point results,
and metric checks are retained in the ignored harness. The executable was built
during S2 work: its binary hash is fixed, but no exact pre-build source-hash
snapshot was captured. Later Cargo metadata is explicitly not evidence of that
binary's exact source. Repeat this same pair on the final frozen implementation
with a pre-build source snapshot. Other intents, Gray4096, remaining routes and
full H3 acceptance stay open; this diagnostic is not an S2/S3/S4 budget approval.

## ICC S2 final shape review: two repairs closed; degenerate mft grid remains

The selected curv ParseLimits regression is closed in both directions and public
assembly, with exact-32 success and bound-16 rejection before table allocation.
The private probe was adapted to the new plan_lut ParseLimits argument and
observes planning plus materialization; none of its rejection or allocation
assertions was weakened. Additional controls confirm identity at count zero,
gamma rejection at count zero, and the legacy fixed-parametric-shape exception.
The PCS-neutral check_encoded_limits wrapper does not bypass the subsequent
PCS-aware mft plan: Lab nonidentity matrices reject before allocation, while
valid Lab and XYZ cases retain their existing behavior.

Structural planning now lives in lut_plan and materialization consumes its
descriptors. The old duplicate parse_clut is removed; the remaining parse/mAB
wrappers are thin shared-plan adapters. This closes the previous responsibility
split repair without accepting the temporary per-LUT ledger as S3/S4 complete.

One P1 within the selected-shape inventory prevents final S2 acceptance:
checked_mft_shape accepts grid counts zero and one. Public mft2 compilation then
succeeds in both directions, and transform_f32 panics in Clut::eval at grid-1 or
grid-2. Private mft1/mft2 checks reproduce accepted malformed shapes and owner
allocations for both directions and both grid counts. These are current-source
observations, not a claim about when the missing guard was introduced.

The finite repair is to reject grid < 2 with InvalidProfile in the shared mft
planner before grid-size arithmetic or any materialization. Preserve grid-two
positive controls and all existing Lab, range, stage-pair and direction-order
assertions. Add tracked coverage for both formats/directions with public
compile/evaluation and allocation-free rejection; no LUT budget redesign is
part of this fix.

Independent external totals are 42 pass / 4 fail: boundary7, parse7 and
S1-private3 pass; shape7 has six passes plus the degenerate-grid failure;
compile22 has nineteen passes plus the public grid failure and the two known
S3/S4 budget failures. Selected product suites pass 66 tests, including the
present official LUT fixture. Root separately reports all-targets167 pass with
one ignored. The three older unused items remain separate cleanup. S2 is NO-GO
only for this new fixed grid-shape repair; S3/S4 and full H3 remain unaccepted.

## H5 slice 2: limited native-plane validation acceptance

The new private native_plan helper checks plane ID, full expected geometry,
subsampling exponents, checked sample count and per-sample range. The entrypoint
first admits only explicit 8/10/12 depths, so the helper's range shift cannot
receive an arbitrary depth. Luma/gray and alpha are full-resolution with 0/0;
only U/V use the selected ceiling-divided chroma dimensions. All color and alpha
planes are validated before the first backend encode. Borrowed supplied samples
are passed directly to native PlaneInput without RGBA conversion or depth
inference. Existing entrypoints are unchanged by this slice.

Independent external validation passes three tests: 24 combinations of odd
400/420/422/444 geometry, explicit depth and optional alpha; twenty invalid
layout mutations; and plane-count, dimension-overflow, invalid-depth and
per-plane sample-range boundaries. The dark-value-one cases retain explicit
depth in av1C/pixi, with alpha configuration and source buffers unchanged.
The av1C observations use the
[AV1-ISOBMFF configuration syntax](https://aomediacodec.github.io/av1-isobmff/v1.3.0.html#av1codecconfigurationbox-syntax),
not a copied decoder. These header checks are not decoded native-sample fidelity
or full color-signaling conformance proofs.

Independent product execution passes lib23, container3, native_validation5 and
existing encode28, plus the two prior external property/Legacy-byte tests.
Strict Clippy all-targets with warnings denied and diff check pass. There are
five native_validation test functions with multiple cases, not fourteen new
tests. Root separately reports Rust1.88 total73 including fourteen existing
FFmpeg tests with the executable present, i686 Rust1.91 container3/native5,
and both wasm target checks. The implementation author reports three initial
failing cases before the repair; that red-first result was not independently
re-executed on the earlier snapshot.

H5 slice 2 is accepted only for this native-plane validation and explicit-depth
boundary. Shared color authority/header planning, lossless controls, semantic
metadata and source-lossless sample/alpha fidelity remain slices 3/4 and later
gates. The surrounding uncommitted encoder WIP is not accepted for release or
an isolated encoder commit.

## ICC S2 selected-shape slice: limited acceptance after grid guard

The shared mft planner now rejects grid counts below two with InvalidProfile
before count/offset arithmetic or owner allocation. Independent mft1/mft2 and
both-direction grid-zero/one checks pass, as do grid-two positives and the
public compile/evaluation panic regression. All seven external shape tests,
the prior S1-private3, boundary7 and parse7 pass; compile22 is twenty passes and
only the two unchanged S3/S4 budget failures. Tracked shape4 and LUT23 pass.

S2 is accepted for authoritative selected descriptors, complete selected ranges,
ParseLimits preservation and the repaired grid/evaluator precondition. No
additional product change was made by review. Retained LUT outer/grid/header
accounting, shared two-direction budgets, the older unused items and full H3
intent/domain/oracle gates remain open.

## C1 substep 4 direct-payload slice: runtime boundary closed, shared-range repair pending

Native methods zero and one now leave the compatibility recursion resolver
before its stack is allocated. Every extent and cumulative payload length is
checked before the Payload-token reserve; method two remains Unsupported.
Independent execution passes the original stack-allocation assertion, eleven
counter proofs and forty old/public boundaries. The latter includes exact
4096-byte payload preservation through the public borrowed-idat path.

Four new external tests pass: borrowed idat with a nonzero range/base and
multiple/empty extents; late-invalid ranges and cumulative-budget rejection
before the payload owner reserve; real payload allocation failure with unchanged
context and successful retry; and finite Legacy method-zero/one bytes, errors
and item-offset cycle diagnostics against exact a55753e. A huge-length Legacy
case was not made an equality requirement: that old checkpoint itself panics
in its infallible capacity reserve, a pre-existing hardening difference rather
than this extraction's regression. Native overflow checks remain asserted.
The four tracked direct-payload tests and strict all-target Clippy also pass.
Root reports Rust1.88 lib508/6 ignored plus native10/limits5/phase9/rich3 and
Miri direct-payload4; included product tests are not counted as independent
external assertions.

One P2 from the existing shared-range/copy requirement remains. Native's
direct_extent_bounds calls validate_item_extent and then repeats the same
checked start/end arithmetic; Legacy append repeats it again. Extract a single
private item_extent_bounds returning the checked start/end pair from the
existing validator. Keep validate_item_extent as a thin discard-result wrapper
where needed, use the returned pair in Legacy append, and call that same helper
from both read-only Native walks. Delete the redundant Native helper. Preserve
the current validation/error order, method-two recursion/cycle behavior,
payload-token lifetime and reserve-before-copy sequence. No extent Vec or
additional parser is required. Keep all allocator, exact-byte and retry
assertions, including borrowed-idat coverage.

The direct-payload runtime repair is accepted, but the slice stays NO-GO until
this finite responsibility-sharing repair is completed. The separately fixed
Native stable-sort scratch test still fails and belongs to the next slice.
No new AV1-header, C2/C3 or general bounded-decoder acceptance is implied.

## H5 slice 3: native color authority clarification

This is an implementation-policy clarification, not acceptance of the changing
encoder. Require explicit native nclx for this API, including ICC-only or absent
color input; reject missing coded matrix/range before either backend. This is
an API restriction, not a claim that every ICC-only AVIF is invalid. Preserve
legacy defaults. Require ICC bytes and an explicit prof/rICC type together,
preserve both unchanged, and reject partial pairs. Native color is authoritative;
an explicitly supplied nested color value must match its canonical nclx payload
or its ICC type and bytes exactly. Do not silently discard conflicts, trailing
nclx data or noncanonical bits through the permissive legacy getter.

[AV1 sections 5.5.2 and 6.4.2](https://aomediacodec.github.io/av1-spec/av1-spec.pdf)
define eight-bit CICP fields. Check both field width and supported meaning:
never truncate or replace unsupported values with Unspecified. Code 2 is a
defined Unspecified value; rejecting it under the native explicit-color policy
must not be described as inability to represent it in eight bits. The syntax
shortcut uses the complete non-monochrome tuple (1,13,0), inferring full range
and 4:4:4. Other identity tuples need the general syntax branch; temporary
Unsupported is permitted but leaves their support open. Independently, identity
requires AV1 subsampling flags 0/0, whereas monochrome infers 1/1, so native
monochrome with matrix 0 must not be accepted. Do not confuse full-resolution
native luma layout with those AV1 flags.

Use a separate alpha plan: monochrome, full range, matching master depth,
without copying the primary identity tuple or ICC into alpha. Alpha colr should
be omitted under [AVIF 1.2 section 4.1](https://aomediacodec.github.io/av1-avif/v1.2.0.html#auxiliary-image-items-and-sequences).
Keep one borrowed checked plan through both writers; ownership/copy wiring and
the finite rejection/metadata-preservation tests are reviewed only after freeze.

## ICC S3 review: outer/grid repair closed; headers and pending admission remain

The frozen S3 change closes the original float-only outer/grid regression.
Independent execution passes boundary7, parse7, S1-private3 and S2-shape7;
compile22 passes 21 with only the known S4 two-direction cumulative failure.
Selected product library23, compile4, shape5, LUT23, parse2 and transform11 all
pass (68 tests). The implementation author reports all-targets 169 pass / 1
ignored; root independently confirms Rust 1.91 compile4, shape5 and transform11
success (20 tests). These are not full H3 proof.

Two new independent S3 tests give one pass and one failure. The passing test
constructs all A/M/B parametric stages in both directions: nine 28-byte parameter
owners are allocated exactly once, without a temporary full parameter clone.
Real allocator denial at the parameter, grid and curve-outer allocations returns
ResourceLimit, destroys partial owners (live bytes return to zero), and permits
retry from the unchanged profile. The S1 private candidate test also still
checks actual capacity against another pending owner, real destruction and
checkpoint restoration. It proves the shared helper, not LUT pending wiring.

**P1: selected fixed headers are missing.** The new
`s3_selected_headers_exact_and_one_under_precede_every_allocation` checks mft1,
mft2 and all-stage parametric mAB/mBA, both directions. Complete logical owned
sizes on the reviewed host are 6656, 49616 and 908 bytes respectively. Each exact
positive succeeds, but each one-under also succeeds and allocates the entire
result. Observed allocator totals add only the documented 32-byte exclusion for
the two Arc control pairs. The missing logical header contribution is
`size_of::<LutTransform>() + size_of::<CompiledDirection>()`, 248 bytes here.
Current `compile.rs` wraps both headers without admitting them; `lut_plan.rs`
counts nested Vec owners but omits the headers. The tracked mft exact test
therefore describes an incomplete inventory and must be corrected, not preserved
as proof that its lower limit is sufficient. Use size_of in tests, not these
host-specific numbers. Whole LutTransform storage already includes its inline
matrix/optional stages; do not also add the mAB matrix's 48 bytes separately.

**P2: complete pending admission is not wired to LUT materialization.**
`validate_shape_limits` compares a planned total, but each materializer creates
a fresh ledger and admits the next stage only immediately before allocating it.
Consequently real capacity is checked without all future planned owners pending.
The helper is transactional; the missing piece is the LUT caller's complete
admission. This is a source-proven design gap, not a claim that the ordinary
allocator in the passing tests returned excess Vec capacity.

Keep the repair finite and within S3:

1. Expose one checked owner/entry inventory from the existing borrowed LutPlan.
   Include Table/Curve outer storage, each table/parameter payload, grid/CLUT,
   and the selected fixed headers exactly once. Reuse it for validation and
   admission instead of independently maintained byte formulas.
2. Add atomic `LutPlan::admit(&mut CompileBudget, owned_headers)` and make the
   materializer consume that admitted ledger. Standalone compile creates the
   ledger, admits the complete selected plan before any allocation, and commits
   its header contribution before returning the owned result. The mft/mAB/mBA
   helpers must not create or reset another ledger.
3. Remove incremental re-admission from table/curve/CLUT helpers. Their existing
   fresh candidates replace their own reserved contribution with actual capacity
   while other pending owners remain charged. Keep the existing drop-before-
   rollback and typed failure paths; do not double-charge known planned bytes.
4. Keep the new exact/one-under and allocator-failure assertions. Add a test
   using an actually admitted LUT plan and the same candidate seam: candidate
   capacity alone fits, but capacity plus another pending owner exceeds the
   limit. Require rejection before payload fill, actual candidate destruction,
   unchanged checkpoint/previous owner and a successful fitting retry. An
   isolated fresh-ledger test is not a substitute for this wiring proof.

S3 stays NO-GO for these two fixed points. S4 combined directions, existing
unused-item cleanup, all-intent/domain behavior and complete CMS oracle gates
remain separate; no new LUT semantics or format support is requested.

## C1 substep 4 direct-payload slice: limited acceptance after shared range repair

The finite P2 is closed: item_extent_bounds now owns the checked start/end
calculation; the validation wrapper, Legacy append and both Native read-only
walks use it. The duplicate Native range helper is removed. Existing error
precedence, diagnostic text, token lifetime and reserve-before-copy order remain
unchanged. Independent direct4, original stack1, counter11 and public boundary40
all pass, as do tracked direct4 and strict all-target Clippy with warnings denied.
This retains borrowed-idat bytes, late-range/total rejection, real allocation
failure/retry and finite exact-checkpoint Legacy comparison.

The direct-payload slice is accepted only for Native method-zero/one stackless
materialization and this shared-range boundary. Stable-sort scratch, broader
AV1 allocation guards, C2/C3 and general bounded-decoder acceptance remain open.

## H5 slice 3 review: authority checks pass; configuration and borrowed writer remain

The frozen native color plan now rejects missing nclx, partial/invalid ICC
pairs, conflicting nested color, noncanonical nested nclx, out-of-width CICP
and the unsupported identity combinations before encoding. Independent negative
coverage exercises nineteen cases and an exact nested-nclx positive. The prior
external plane3 and property/Legacy-byte2 tests still pass; only the plane
fixtures gained the now-required explicit nclx, without weakening assertions.
Independent product library23, encode28, color3, native5 and container3 pass
(62 tests), and strict all-target Clippy/diff check pass. Root separately
confirms Rust1.88 color3/native5/container3/encode28, i686 color3/native5/container3
and wasm32-unknown-unknown check. The author's full76 includes fourteen existing
FFmpeg tests; these are not new color-plan oracle cases.

The three new independent color tests give one pass and two failures:

1. **P1 native lossy monochrome av1C disagrees with the payload.** Thirty real
   encodes cover five selected color/layout tuples, 8/10/12 bits and quality
   80/100, each with alpha. Exact a55753e parser APIs inspect the resulting
   Sequence Headers and associated properties, without copying decoder code.
   Eighteen monochrome items in quality-80 outputs have av1C subsampling 1/0
   but their Sequence Header infers 1/1; this affects alpha and the Gray primary.
   Quality-100 controls agree. The native entry currently retains the backend
   container record without applying its checked native configuration plan.
   [AV1-ISOBMFF section 2.3.4](https://aomediacodec.github.io/av1-isobmff/v1.3.0.html#av1codecconfigurationbox)
   requires these fields to match the Sequence Header, including inferred
   color_config values. Fix the new native record path, preserving the unrelated
   bits and old entrypoint/backend byte behavior. Do not change native luma
   layout 0/0 to the AV1 monochrome flags 1/1.
2. **P2 borrowed ICC does not reach the writer.** A 4093-byte opaque profile
   triggers five standalone profile-sized allocations: one NativeColorPlan
   to_owned copy, then profile.clone and color_box's clone during each metadata
   sizing/final-writing pass. Supplying a matching nested ICC adds a sixth via
   cloning EncoderOptions before overwriting its color field. Exact output ICC
   type/bytes and caller options remain intact, but the prescribed borrowed
   plan/writer seam is not implemented. Pass borrowed checked color through the
   native writer and use a shared type-plus-borrowed-bytes box helper. Build
   native backend options without cloning an ICC field that is immediately
   replaced. Framed output copies are expected; standalone metadata clones are
   not. Keep the old writer wrapper and its byte-equivalence assertions.

In all thirty outputs, the tested primary CICP/range and exact ICC type/bytes
match their inputs; alpha is monochrome/full-range at matching depth, uses no
identity tuple, and has no associated colr. These are header/property checks,
not a proof of decoded native sample or source-lossless fidelity.

Complete the same finite slice with two supporting changes: document mandatory
native nclx, paired ICC fields, authority/conflict rules, identity GBR order and
optional alpha ID 3 in the public API docs; distinguish defined Unspecified
code 2 rejected by API policy from values that exceed the eight-bit field.
EncoderOptions retains its legacy default color, so examples supplying a
different native tuple must explicitly clear that nested field or supply an
exact match. Strengthen tracked color tests from unrelated byte-window searches
to exact associated colr and AV1/av1C comparisons, including matching nested
ICC and both backends. Retain the existing width/reserved-code rejection tests.

H5 slice 3 remains NO-GO for this finite configuration/borrowed-writer/documented
contract bundle. Legal identity tuples still temporarily rejected, slice-4
controls/pixi/semantic metadata and full source-lossless fidelity remain open.
No old API change, encoder publication or broader format work is authorized.

## C1 substep 4 sort slice: limited acceptance

Native alpha IDs are made unique by the existing shared auxl/fallback selector
before in-place unstable sorting. The public Native path always passes its
resolved primary ID, so it reaches that unique-selection branch. Legacy retains
its stable sort and first-duplicate dedup behavior. The temporary ID owner is
still released only after the consuming iterator drops; moved strings and
returned payloads remain live and charged.

Independent execution passes the fixed counter11, stack1, sort1 and direct4
assertions, plus public boundary40. The 1024-item scratch observer sees only the
expected charged ID-vector growth, no second stable-sort scratch owner, and
checks the exact ascending IDs and payload/type preservation. The full include-
based harness passes 170 tests, including product and baseline tests; these are
not 170 independent review cases. Tracked sort1 and strict all-target Clippy
also pass. Root separately confirms Rust1.88 and Miri for the tracked sort test.

The fixed C1 parser counter/direct-payload/sort slices are accepted. This does
not complete AV1-header/tile allocation guards, C2/C3, the full bounded native
byte API or the wider H6 gates. The earlier parser ownership acceptance remains
limited to its reviewed allocation classes and supported ordinary-still subset.

## C1 substep 5 preparation: header inspection before decode materialization

This is a design checkpoint, not acceptance of the bounded decoder. Keep the
reviewed parser slices closed and restrict the next work to ordinary av01 still
items with their selected alpha. Deep decoded/reference/filter ownership (C2)
and AVIS state (C3) remain separate.

The current bounded call sequence checks sequence maxima, then collects an OBU
Vec, calls parse_av1_headers, checks its decode plan and decodes the master.
That header call already allocates tile-layout vectors, tile descriptors and
entropy states and copies frame/tile payloads. Alpha validation runs afterward,
constructs a cloned AvifInfo/payload and parses its headers; alpha decode repeats
both operations, then clones the decoded alpha plane.

Use these small implementation slices, preserving the old public wrappers:

1. Add a private borrowed ObuIter over the existing read_next_obu primitive.
   Keep a single syntax implementation and preserve old collection/first-match
   early-stop behavior. The Native inspection walk must consume the required
   full item without collecting OBUs and distinguish actual show_frame from an
   OBU_FRAME/FRAME_HEADER count. Hidden/show-existing, multiple frames and the
   currently unbudgeted super-resolution route remain explicit Unsupported.
2. Split the existing frame-header parser at the common pre-tile boundary into
   scalar prefix and resumable remainder, including reduced-still and ordinary
   branches. Do not copy the grammar into a second Native parser. A private
   borrowed ItemHeaderSource supplies payload/config/pixi/color/geometry without
   synthesizing AvifInfo. A prefix records display state, sequence maxima,
   coded/upscaled/render dimensions, depth and the parser continuation. Check
   both master and selected alpha prefixes before tile-layout allocation,
   entropy work or decoded planes. Validate real dimension pairs, padded plane
   extents and checked sample-byte arithmetic; do not pair upscaled width with
   render height. Master/alpha geometry, depth, monochrome/full-range constraints
   are part of this preflight, not a post-master-decode test.
3. Resume those checked prefixes into reusable header plans. Tile-range scanning
   and materialization must share the same checked range primitive; borrowed
   payload ranges must not become per-tile temporary byte copies before bounds
   are known. Admit necessary descriptor/encoded-scratch owners before fallible
   reserve and reconcile capacities before filling, without claiming that this
   accounts for C2 entropy/reference/filter state. Keep Legacy wrappers and their
   diagnostic order; any temporarily unsupported Native organization is explicit.
4. Decode both items from the already validated plans. Borrow alpha payload and
   move its final samples owner into the master after strict validation; retain
   alpha ID 3 and hidden-pixel semantics. Remove only the Native repeated
   alpha-info/header/plane-copy route, preserving the old API behavior.

Fixed acceptance fixtures will observe OBU collection elimination, sequence and
actual/padded/render bounds before tile copies, hidden/show-existing rejection,
invalid alpha before master allocation, one alpha header materialization and
sample-pointer identity on attachment. Use real allocator observations scoped
to data owners, not a claim that error formatting allocates nothing. Include
exact/one-under, malformed range and successful still/alpha controls. Two initial
external observer fixtures currently stop at a missing required ispe property;
they are unfinished test setup, not reproduced product failures or acceptance.

Root additionally confirms the current parser-only snapshot with WASI execution
of 21 budget/counter/direct/sort tests and i686 execution of native10, limits5,
phase9 and rich3. These results do not cover the new header/decode work.

## ICC S3 follow-up: selected ownership fixes verified; finite remainder

The prior selected-header and partial-admission defects are closed in the
reviewed snapshot. LutPlan admits its complete owner inventory and selected
LutTransform/CompiledDirection headers atomically. The selected materializer
receives that ledger, consumes pending costs without readmitting stages and
commits headers once. mAB matrix storage is inside the header rather than a
second 48-byte charge. Parameter curves materialize directly into their final
owners through the common curve-plan helper.

Independent execution passes both existing S3 tests: exact/one-under checks for
mft1, mft2, mAB and mBA, plus parameter/grid allocation-failure cleanup. A new
test first admits an actual mAB/mBA plan with all its future owners pending,
then substitutes a 32-byte candidate for one planned 28-byte parameter owner.
The candidate fits alone but exceeds the total including another live 8-byte
owner and pending stages. It is rejected and actually dropped; the old payload
pointer/content and scalar checkpoint are unchanged, and materializing the
same admitted plan afterward succeeds. This is additional evidence, not the
old generic candidate test relabelled as LUT wiring coverage.

Three finite S3 follow-ups remain:

1. **P2 raw encoded length is still charged as compiled storage.** The selected
   compile path retains `tag.len() > max_compiled_bytes`. A checked mAB or mBA
   with an 8192-byte zero gap before its offset-addressed stages has encoded
   length 8676 but the same exact 908-byte owned inventory as the compact tag
   on the tested host. Planning accepts it; public compile incorrectly rejects
   it with ResourceLimit. Remove that raw-length/compiled-budget comparison,
   preserving Profile ParseLimits, structural ranges and all owner admission.
   The new test requires both directions to accept the same exact owned budget
   and reject one-under before any materialization.
2. **P2 tracked ownership regression needs an independent expectation.** The
   public mft test now binary-searches its own successful budget and tests that
   discovered boundary. It cannot detect omission of the private headers and
   is not a replacement for a fixed owner inventory assertion. Keep its public
   monotonicity coverage if useful; add a separate private tracked test using
   size_of for every header/outer/payload/grid owner and an admitted-LutPlan
   excess-capacity/pending-owner test with drop, unchanged state and retry.
3. **New cleanup item, not baseline:** LutTransform::decoded_bytes is now an
   unused fourth warning, in addition to the three previously recorded items.
   Remove the obsolete private float/length-only accounting helper, or retain
   it only under test cfg if a real test requires it; do not suppress the new
   warning or reinstate it as authoritative owned-byte accounting.

The original external set passes 45 of 46 tests; its sole failure remains S4
two-direction cumulative budgeting. The expanded S3 target passes three of
four tests, with the raw-padding test failing in both directions. Independent
selected product execution passes lib23, compile4, shape5, LUT23, parse2 and
transform11. Root separately reports all-targets169 pass/one ignored. S3 stays
NO-GO for the finite remainder above; S4, all intents/domain/LCMS completion and
overall H3 acceptance remain separate and incomplete.

## C1 substep 5: established probes and next prefix/resume slice

The initial external fixture setup is repaired: ispe, pixi and av1C are generated
from the actual synthetic Sequence Header and Frame Header. Every case first
requires successful Native container parsing. Three fixed probes now reach the
intended AV1 path:

- 2048 padding OBUs after a valid sequence cause a 98304-byte OBU-vector
  allocation before rejection of a deliberately missing frame. This failed
  on the pre-iterator implementation; a later run during the iterator work
  passes. That in-progress result is not the iterator freeze review.
- A valid displayed reduced-still header followed by a 4093-byte frame payload
  is copied once before the correctly reported one-byte plane-limit rejection.
- A selected alpha has a valid RGB header, which already violates the required
  alpha monochrome constraint. The deliberately invalid master entropy is
  nevertheless processed first, after one 4093-byte tile-payload copy, and its
  trailing-bit error hides the known alpha-header error. The acceptance result
  is Native alpha Unsupported before either item's tile copy/entropy work.
  This poison-entropy case is a phase-order test, not a valid image/oracle test.

The original forty external parser assertions are unchanged. The latest three-
probe run is one pass/two failures while iterator implementation is in progress.
Error-string allocations are not confused with the observed data owners.

After the iterator-only freeze, implement the following private seam as the
next small slice; names can follow existing conventions, ownership cannot:

```rust
fn parse_frame_prefix<'a>(source: &'a [u8], sequence: &SequenceHeader,
    sequence_metadata: &SequenceHeaderMetadata,
    references: &[Option<ReferenceFrameState>; 8])
    -> Result<FramePrefix<'a>, DecoderError>;
fn finish_frame_header(prefix: FramePrefix<'_>, /* shared checked context */)
    -> Result<FrameHeader, DecoderError>;
fn inspect_native_item<'a>(source: ItemHeaderSource<'a>, limits: &NativeDecodeLimits)
    -> Result<NativeItemPrefix<'a>, DecoderError>;
fn inspect_native_still<'a>(info: &'a AvifInfo, limits: &NativeDecodeLimits)
    -> Result<NativeStillPrefixes<'a>, DecoderError>;
```

FramePrefix owns only scalar/fixed-size state and the borrowed bit-reader
continuation at the existing pre-tile boundary. It is consumed by the remainder
instead of reparsing raw bytes. Public frame-header functions compose these two
steps with Legacy policy, preserving old diagnostics and reference behavior.
Native checks display state, sequence maxima and all actual dimension/plane
limits before resuming either prefix. Factor the per-plane geometry arithmetic
out of build_plane_layouts so the old Vec constructor and Native fixed three-
slot preflight use one implementation. Check coded (width,height), upscaled
(upscaled_width,frame_height) and render (render_width,render_height) separately,
including coded padding and subsampling when counting u16 samples.

ItemHeaderSource borrows payload and available config/pixi/color/geometry; it
does not clone compatible brands or fabricate an alpha AvifInfo. The Native
pair holds one master and an optional selected alpha prefix. Validate alpha
dimensions/depth/monochrome/full-range and supported display behavior against
the master's header before finishing either item. Do not claim validation of
alpha metadata that the present public projection does not retain. Multiple
frames, hidden/show-existing and the current super-resolution route stay
explicitly unsupported; do not call the Legacy hidden-frame search helper.

Acceptance of this prefix slice requires the plane/alpha probes to become green
without weakening allocator observations, plus reduced-still and normal-header
positive controls, checked actual/render/padded bounds and exact old-wrapper
results. It does not by itself close later tile-descriptor/entropy allocations.
Those remain the next slice: checked range/count inspection shared with the
materializer, fallible descriptor reserves, checked actual capacities and a
single necessary encoded-data owner or borrowed payload. Do not repurpose the
container metadata budget as an unexplained decoder-wide budget. C2 total
decoded/reference/filter state and C3 sequence state remain outside this work.
The last slice reuses the validated alpha header and moves its sample owner,
with a pointer-identity assertion, instead of reparsing or cloning the plane.

Root separately confirms S3's current compile4/shape5 tests by WASI execution;
those nine passes are supplementary portability evidence, not S3 acceptance.

## H5 slice 3 repair review: runtime fixes closed; documentation/test remainder

The native monochrome av1C correction now applies only to the new Native
primary/alpha record path and preserves the other configuration bits. The
thirty-encode external matrix passes actual AV1 Sequence Header versus av1C,
associated CICP/range and exact ICC assertions for both selected backends and
8/10/12-bit inputs. NativeColorPlan remains borrowed through NativeColorSource
and the shared container append helper. The 4093-byte ICC observer sees no
standalone profile-sized copies, with or without an exact matching nested ICC;
caller data and output ICC remain unchanged. The old writer's exact baseline
byte comparisons also pass; this is not an exhaustive old encoder pixel oracle.

Independent external color3, plane3 and property2 all pass. Selected product
lib23, encode28, container3, color3, headers1 and validation5 pass, as does strict
all-target Clippy. Root separately confirms Rust1.88 all77 including fourteen
executed FFmpeg tests, i68612 and the wasm32-unknown-unknown build for this
pre-support-fix snapshot.

The runtime defects are closed, but the prescribed slice-3 support contract
still needs two finite corrections before whole-slice acceptance:

- The public prose incorrectly requires a single nested colr to match both
  CICP and ICC. State the actual alternatives: absent nested color, exact
  canonical nclx match, or exact ICC type/bytes match. Explain the legacy
  EncoderOptions default color and how to clear/override it. Document input
  alpha plane ID 3 (distinct from container item ID 2), and explicit identity
  input order 0=G, 1=B, 2=R. Remove the unrelated future item-3 claim.
- The new tracked header test checks only 8-bit quality-80 Cs444/Cs400 av1C
  flags and associated properties. It still does not compare an actual AV1
  Sequence Header, cover both selected backends or supply matching nested ICC.
  Add those fixed regression assertions; the passing independent thirty-encode
  matrix must not be reported as tracked coverage already present in the repo.

Thus runtime repair is accepted narrowly; complete H5 slice 3 remains pending
these support changes. Slice 4 controls/pixi/semantic metadata, temporarily
rejected legal identity tuples and full source-lossless fidelity remain open.

## C1 substep 5 iterator slice: limited acceptance and separable checkpoint

The private ObuIter uses the unchanged read_next_obu syntax primitive, borrows
payloads and yields an error once before becoming fused. Old parse/search/count
wrappers retain their public signatures, collecting result, first-match early
return and independent-part framing. The bounded still inspection now walks
this iterator without an OBU collection; it does not yet inspect actual
show_frame or preflight the master/alpha decode pair.

Four new independent tests pass: successful iteration performs no allocations
while preserving payload pointers/source bytes; ordinary 128-byte size and
extension handling are retained; malformed input errors once; and results from
the old collector, searches, count and zero-target search agree with the exact
baseline. Parts cannot complete a truncated OBU across a boundary, and a
completed first-match search still does not parse a malformed suffix. The
include-based target passes eighteen tests: four independent, nine current and
five baseline tests, not eighteen independent cases. Product focused11,
external parser40 and strict all-target Clippy also pass.

The fixed OBU collection observer passes. Plane-limit-before-tile-copy and
invalid-alpha-before-master-entropy remain the two expected failures of the
next slice. Root separately confirms the four new tracked iterator tests under
Miri, nine OBU tests under actual WASI execution and eleven focused tests under
Rust1.88. Its parent lib62, AVIF decode7 and Stage-A6 regressions also pass.

This iterator-only slice is accepted. The two files `avif/src/obu.rs` and
`avif/src/obu_iter_tests.rs` are a separable checkpoint candidate against the
reviewed baseline: no new Native API/type, dependency or version change is
required. Leave decoder/frame.rs integration and all other uncommitted C1 work
outside that checkpoint. Additional i686 LEB128 overflow investigation was not
retried and remains unverified; the primitive was not altered by this refactor.
Neither this checkpoint nor the parser's earlier acceptance completes bounded
AV1 decode, C2/C3 or the wider H6 gates.

The iterator checkpoint is now saved as nested AVIF a461af7 and parent gitlink
sync c0d7ccb. Root verifies an isolated checkout at that object with Rust1.88
focused OBU11, without the uncommitted Native implementation. Its clean parent
regressions pass lib62, decode7 and Stage-A6 after the required ignored fixture
was supplied with its existing matching hash. This does not commit or accept
the decoder/frame.rs Native integration.

## ICC S4 next slice: shared two-direction admission

Keep Profile::compile's public single-direction API and semantics. Extract a
small private route-plan module rather than further enlarging compile.rs:

```rust
enum SelectedStagePlan<'a> { Matrix(MatrixPlan<'a>), Lut(LutPlan<'a>) }
struct RoutePlan<'a> { /* selected stage, direction, PCS, channels, media white */ }
fn plan_route(profile: &Profile, direction: TransformDirection,
    intent: RenderingIntent, limits: TransformLimits) -> Result<RoutePlan<'_>, TransformError>;
fn admit_pair(input: &RoutePlan<'_>, output: &RoutePlan<'_>, budget: &mut CompileBudget)
    -> Result<(), TransformError>;
fn materialize_route(plan: RoutePlan<'_>, budget: &mut CompileBudget)
    -> Result<CompiledProfile, TransformError>;
```

The standalone wrapper plans one selected route, creates one budget, admits
once and calls the shared materializer. Transform::new_with_limits must not
call that fresh-budget public wrapper twice. Instead: preserve the BPC rejection;
plan input DeviceToPcs and output PcsToDevice without decoding either; check the
existing selected-white/bridge conditions; atomically admit both route costs;
then materialize input and output using the same mutable ledger. No broad tag
walk, unused matrix fallback, opposite-direction compilation or eager raw-cache
clone is added. Preserve direction-before-channel diagnostics, forward-only
curve support, selected inverse/singular rejection and Relative handling of
unused malformed white/chad. This is not an intent/domain behavior rewrite.

CompileBudget already accumulates bytes and curve entries; add CLUT-entry state
to its scalar checkpoint and admission checks. Matrix contributes zero CLUT
entries. LutPlan admits curve entries, CLUT entries and full owned bytes as one
transaction, and pair admission rolls back all counters if either fails. Counts
describe the two compiled routes and are not released while materializing them.
Actual-capacity reconciliation consumes pending bytes, with the complete output
plan still pending while building input, then the actual input owners remaining
live while building output. On error, drop partial candidates and the first
completed stage before restoring the outer checkpoint; retry must be safe.

Reuse S1/S3's exact selected header inventory, including each stage's payload
header and CompiledDirection header once. Keep both CompiledProfile handles
alive through pair assembly so that this accounted construction peak matches
real ownership. Transform can retain its existing Arc fields and worker clone
behavior; cloning handles must not copy payloads or charge the same owner twice.
The temporary direction wrappers can drop after assembly. Do not charge raw
Profile/tag bytes as compiled storage, and do not infer sharing merely because
both calls reference the same Profile: the current two directions construct
distinct owners. No new public dependency, option or worker representation is
needed for this slice.

The existing two_selected_luts_share_one_compiled_budget regression remains
unchanged. Add a narrow tracked table for matrix/matrix, matrix/LUT, LUT/matrix
and LUT/LUT: exact/one-under bytes and independent total curve/CLUT-entry limits,
with known admission failure before the first data-owner allocation. The old
test's fewer-than-two-CLUT-allocations assertion alone is not enough; the new
pair-preflight observer requires zero. Include a valid input plus truncated
selected output, and keep unused malformed routes accepted. A candidate test
must combine input actual-capacity excess with pending output cost, observe
drop/checkpoint/source preservation and retry; a destination allocation failure
must also release the completed input. Retain all S1/S2/S3, route, Absolute,
wrapper-length and worker-sharing assertions. Freeze/review this finite S4
slice before any wider H3 acceptance or oracle expansion.

## H5 slice 3 support follow-up: limited acceptance

The two remaining support items are closed. Public NativeColorInformation
documentation now distinguishes an exact nested nclx match from an exact nested
ICC type/byte match, explains clearing the legacy nested default, states identity
input plane order 0=G/1=B/2=R and separates input alpha plane ID 3 from the
container-owned auxiliary item ID. The obsolete future-item claim is removed.

The strengthened tracked native_headers test passes eight combinations:
lossless false/true, nested nclx/ICC and Cs444/Cs400, with alpha in each. It reads
actual sequence OBU payloads, checks primary CICP/range and alpha mono/full-range,
compares actual sequence monochrome and inferred subsampling with associated
av1C (including the corrected 0x08 x-subsampling mask), and checks exact associated
nclx/ICC bytes plus absence of alpha colr. Independent execution of that one
tracked test and strict all-target Clippy pass. This tracked matrix is 8-bit;
the previously passing independent thirty-encode 8/10/12-bit matrix remains
separate evidence, not additional tracked test functions.

Together with the already verified runtime corrections and old writer byte
comparisons, the fixed H5 slice-3 color authority/header/borrowed-ICC/documented
contract is accepted. This does not complete slice-4 controls/pixi/semantic
metadata, support for temporarily rejected legal identity tuples, full native
source-lossless validation, encoder publication or all of H5. No product files
were edited by this review.

## ICC S3 final review: selected LUT ownership slice accepted

The three finite follow-ups are closed. Public selected compilation no longer
compares raw encoded tag length with the compiled-owner budget; Profile
ParseLimits and checked selected ranges still apply. Both offset-addressed
mAB/mBA gap fixtures now accept their unchanged exact owned budget and reject
one-under before materialization. The obsolete decoded_bytes helper is removed.

Three private tracked tests now use portable size_of-based header/outer/payload/
grid expectations, including fixed public exact/one-under boundaries. The real
admitted mAB plan test keeps all future owners pending alongside a live 8-byte
owner, rejects a 32-byte candidate for a planned 28-byte parameter owner, checks
candidate drop, old pointer/content and checkpoint preservation, then retries
the same plan successfully. Independent mAB/mBA allocation observations also
confirm actual deallocation and pending-owner accounting. The older public
binary-search test remains supplementary monotonicity coverage, not the fixed
header proof.

Independent S3 four pass; the include-based target reports seven because it
also runs the three tracked tests. The original external forty-six pass
forty-five, with only the unchanged S4 two-selected-LUT cumulative-budget
failure remaining. Included copies of the new tracked tests in other private
targets are not counted as independent cases. Selected product lib26,
compile4, shape5, LUT23, parse2 and transform11 all pass (71 total); the optional
official profile used by the LUT fixture is present. Root separately confirms
Rust1.91 all-targets172/one ignored and the three new private tests under i686,
Miri and actual WASI execution.

Focused formatting and diff checks pass after one import-order-only repair.
Normal compilation is back to the three earlier unused items. Clippy is not
globally clean: the existing iccprofile comparison error remains, and the
highres WIP has separately recorded unused/style diagnostics, including the
LutShape large variant, wrapper divisibility checks and RenderingIntent Default
implementation. No new diagnostic is attached to this final S3 repair/test
slice. These WIP diagnostics remain checkpoint-cleanup work, not an unrelated
published-baseline exemption.

S3's fixed selected-LUT ownership/allocation behavior is accepted. S4 shared
two-direction bytes/curve/CLUT admission, cleanup, all-intent/domain behavior,
full oracle coverage and H3/product checkpoint acceptance remain incomplete.
The S4 design above is unchanged; no product source was edited by the reviewer.

## ICC S3 frozen-source diagnostic: existing LCMS pair repeated

The existing RGB17-cubed v4-preference to sRGB2014 Relative comparison was
rebuilt and repeated without changing its oracle or metric. SHA256 manifests
for the ICC source/build inputs and harness/metric agree before and after the
build/run (42 files). The preserved run also records Cargo metadata, executable
and both profile hashes, transicc hash, exact command/settings and full output
rows in a new ignored artifact directory; the older diagnostic is untouched.

The Sharma metric self-check passes all 34 vectors. All 4913 RGB points complete
without failure. DeltaE00 median is 0.0017950127, nearest-rank p95 0.0063801949
and maximum 0.0459857599; maximum normalized channel difference is 0.0003992994.
These reproduce the previous diagnostic values and satisfy this pair's existing
thresholds. Input/output transport remains floating 0..255, Relative without
BPC, with both result sets measured through the same destination-to-physical-Lab
oracle path. The subject retains its explicit clamp=true setting.

Unlike the earlier run, this result has a verified source snapshot attribution.
It is still only one pair/intent: S4 cumulative budgeting, other intents, Gray,
unclamped/domain behavior and overall H3/oracle completion remain open.

## H5 slice 4 review: selected backend controls and pixi boundary accepted

The shared lossless capability check preserves the old control predicate,
diagnostic and its precedence before legacy pixi/color restrictions. Native
preflight resolves actual primary and optional alpha routes and validates both
before entering either backend. Jobs, row/column tiling and advanced controls
are rejected for selected lossless routes, including explicit lossless=true.
An absent alpha with quality_alpha=100 does not restrict a lossy primary.
Extended pixi is explicitly rejected by the Native route; mixed channel depths
were already rejected by common validation and are not a newly fixed defect.

Three independent tests pass. Sixteen control negatives and four extended-pixi
cases observe no backend-sized allocator requests, calibrated against successful
execution of both backends; small preflight/error allocations are not claimed
to be absent. Mixed-quality output comparisons confirm the primary and alpha
payloads follow their independently selected qualities. A no-alpha lossy encode
with jobs=1 succeeds despite quality_alpha=100. Legacy control error text and
precedence remain exact, including intentionally conflicting later metadata.
The previous external color3, plane3 and property2 tests remain unchanged and
pass, including the old writer's baseline byte comparisons.

The strengthened tracked support adds four tests with thread-local, test-only
entry counters. Negative controls observe zero primary/alpha entries, a positive
control verifies primary-then-alpha order, the no-alpha control observes only
primary, and mixed-depth pixi rejects before entry. Counters sit immediately
before the real selected backend branches and introduce no release-build state.
Independent final product execution passes all84: lib27, encode28, container3,
native color3, headers1, validation5, controls3 and fourteen executed FFmpeg
tests. Strict all-target Clippy, formatting and diff checks pass. Root separately
confirms Rust1.88 all84, i686 counter4, wasm32-unknown-unknown check and the
negative backend-entry test under Miri. The author did not run a pre-fix red
test for this slice; no such failure count is claimed.

The fixed slice-4 controls/extended-pixi boundary is accepted. This is not
extended-pixi preservation support, geometry/other frame metadata preservation,
support for temporarily rejected legal identity tuples, full new-native sample
or source-lossless coverage, animation encoding, publication or all-H5 closure.
The existing FFmpeg suite covers old entrypoints and does not substitute for
all those new-native roundtrip gates. Encoder checkpoint scope is the reviewed
native slices and their tests; type-level plane documentation must agree with
the explicit GBR/alpha contract before that checkpoint is saved.

The final encoder candidate includes fourteen reviewed files: the four existing
source integrations, four private helper/test modules, five integration-test
files and ignore rules. The two existing plane-type doc comments now distinguish
the legacy YUV interface from new-native GBR and alpha plane 3. Exact staged
contents match the tested source apart from that documentation-only correction;
there are no dependency, manifest, version or other-repository changes. The
candidate is accepted as a standalone implementation checkpoint, not release
or full H5 completion.

## C1 prefix implementation seam: concrete saved state

The temporary helper returning a fully parsed FrameHeader is not a pre-tile
prefix: parse_tile_info has already allocated its vectors. Split the existing
grammar at reduced-still allow_intrabc completion and at normal-frame
disable_frame_end_update_cdf completion, immediately before each tile-info call.

A move-only FramePrefix carries the existing BitReader borrowing the OBU data,
a copied SequenceHeader and a borrowed fixed eight-slot reference-state array.
Native callers use a stable all-None array; do not borrow a helper-local array
into the returned prefix. SequenceHeaderMetadata is consumed by the existing
buffer-removal-time read before this boundary and need not survive afterward.
No seek/reparse, dummy TileInfo or full FrameHeader allocation is needed.

Its fixed parsed fields are frame type/display/showable/error-resilience flags;
CDF/screen-content/integer-motion flags; size-override, order hint, primary
reference, refresh flags, frame ID, seven reference indices/order hints and
short-signaling state; high-precision/filter/motion-mode/reference-MV state;
coded/upscaled/render dimensions; intrabc and frame-end-CDF flags. Intermediate
frame_order_hints need not survive after reference_order_hints are resolved.
Normalize reduced-still fields to the existing literal defaults without reading
normal-only bits, particularly disable_frame_end_update_cdf.

The consuming finish function uses that same reader for tile info, trailing
parameters, global motion and film grain, then assembles FrameHeader once.
Reduced-still trailing/grain parsing keeps its existing all-None references;
normal parsing keeps the supplied reference state. Existing public wrappers
compose prefix plus finish, preserving bit order, early show-existing rejection,
diagnostics and final offsets. Only the Native caller inserts both-item limit/
alpha validation before either finish. Keep this seam in a small private module
sharing existing syntax helpers; do not clone the grammar into a Native parser.
This is design guidance, not acceptance of the in-progress prefix implementation.

The reviewed encoder checkpoint is saved as 1bcf542, with parent gitlink sync
0995d14. Root's clean temporary checkout confirms Rust1.88 encoder84, parent
encoder integration8 and callback4 plus the explicitly run external Next case.
These checkpoint regressions do not establish full new-native lossless fidelity.

## H5 new-native source-lossless pilot: confirmed chroma mismatch

This is a new encode_native_bytes oracle, not the existing fourteen old-entry-
point FFmpeg tests. The exact clean encoder checkpoint generates native planes
with explicit depth/layout, lossless=true, independent alpha values 0/1/mid/max,
distinct hidden colors and exact ICC+nclx. FFprobe checks native pixel format,
dimensions and full range. FFmpeg maps primary and alpha streams separately,
disables automatic conversion/scaling and requires the matching native output
format; no RGB intermediate is used.

The first 4x4 12-bit 4:4:4 primary, whose samples never exceed one, and its
12-bit alpha are byte-exact. ICC, nclx and caller data are also preserved.
The 5x3 8-bit 4:2:0 case fails: U[1] changes 138 to 137 and V[3] changes 74 to
75, while luma and all fifteen alpha samples are exact. Explicit libdav1d and
libaom decodes of the same encoded file produce identical raw bytes and the
same two errors, with native yuv420p/gray full-range output.

The fixed variations retain the mismatch at speed 0, 6 and 10 and without
alpha. An even 6x4 control also fails, with three U samples increased by one.
Therefore this is not isolated to odd dimensions, alpha attachment or one speed
preset. It is a P1 source-lossless failure on the selected pure-Rust primary
route; the internal prediction/transform cause has not yet been diagnosed.
The all-depth/subsampling matrix is deliberately deferred until this finite
failure is repaired, not reported as passed.

The ignored harness retains exact input/expected/decoded planes, encoded files,
commands, both oracle versions and SHA256 manifests for the clean commit source,
harness, executable, ICC and artifacts. Source hashes were captured after the
run and tied to the exact clean commit verified before/after, not falsely
claimed as pre-build hashes. The finite fixture/options are handed to the
implementation agent for a tracked red/green repair. Preserve the passing dark
12-bit control and do not replace the repair with geometry-only rejection or
weaker sample tolerances. No product source was edited by this review.

## ICC S4 independent review: cumulative gate closed, exact ownership not yet closed

The unchanged independent boundary46 now all pass, including the formerly
failing two-selected-LUT cumulative limit. The four independent S3 owner tests
also pass. Product all-targets execution independently confirms 176 passed and
one explicitly ignored supplementary-metric test. Included product tests in
private harnesses are not counted again as independent cases. Root separately
confirms the four tracked S4 cases on i686, Miri and executed WASI.

Four new independent S4 cases produce three passes and one failure. Fixed
matrix/matrix, matrix/LUT, LUT/matrix and LUT/LUT counts reject cumulative
byte/curve/CLUT one-under budgets before the first allocation. A destination
12-byte curve allocation failure occurs after the input's eight-byte curve
was constructed; all completed-input/partial-output owners are actually
deallocated, and source pointers and contents remain unchanged. A genuinely
admitted two-LUT plan rejects a 28-byte-planned/32-byte-actual candidate,
deallocates it and restores its checkpoint; the same plans and same ledger then
materialize successfully without a fresh Transform or readmission.

P1: exact compiled-owned storage is overcharged in Transform construction.
RoutePlan unconditionally charges a CompiledDirection heap header, whereas
Transform keeps that intermediate on the stack and retains only its stage Arc.
The independent 64-bit fixture's actual owned byte totals are 288, 49720, 49720
and 99152 for the four pair kinds. Allocation observation confirms those totals
plus only the documented two Arc control-pair exclusions. All four exact
budgets incorrectly return ResourceLimit before allocation. Tests derive these
totals from fixed payload counts and size_of owner types, not by asking the
ledger to discover its own passing boundary.

Finite repair, superseding the earlier suggestion to retain extra compiled
handles solely to match their charge:

1. Give the shared route planner an explicit private owner policy for standalone
   CompiledProfile versus Transform stage storage. Charge the selected stage
   header once in both; charge CompiledDirection only where its heap owner
   really exists. Admission and materialization must consume the same policy.
   Keep the standalone S1/S3 exact-header tests unchanged; do not add needless
   Arc allocations to justify a charge for non-existent owners.
2. Strengthen tracked S4 tests with the fixed independent inventory/allocator
   observations above. The current pair_cost derives its expectation from
   admission itself, and the current candidate retry creates a new Transform.
   Use a small shared private pair-materialization seam to expose the admitted
   checkpoint: on failure, drop partial output and completed input before
   restoring it, then retry the same admitted plans and ledger. Preserve source
   pointer/content, output-allocation failure and candidate actual-capacity
   observations; do not replace them with inspect-only or fresh-budget tests.
3. Remove the newly unused Curve storage_bytes and LUT curve_entries/clut_entries
   helpers. The new SelectedStagePlan large-enum Clippy diagnostic also belongs
   to S4 cleanup. Keep planning allocation-free rather than boxing the large
   variant; private checked fixed storage is an available alternative.

Focused formatting passes. Ordinary checking reports five warning groups:
the prior three plus two newly unused-method groups. Clippy is not clean: the
existing reader comparison error and prior high-resolution WIP diagnostics
remain distinct from the newly added route enum diagnostic. These are not
waived by runtime test success. S4 remains NO-GO pending this finite repair;
full intent/domain/Gray/oracle coverage and the overall ICC checkpoint remain
separate unfinished gates. No product source was edited by this review.

## H5 source-lossless repair: fixed native oracle accepted

The finite chroma mismatch is repaired by retaining subsampled luma at Q3
precision before its block average and using the reconstructed coded backing
extent rather than clipping every read to visible source dimensions. The
tracked sixteen-value vector is independently calculated from the official
CfL process; its Q3 block average is 752 and every expected delta agrees.
The sum scaling and signed final prediction follow
[AV1 chroma-from-luma decoding](https://aomediacodec.github.io/av1-spec/#predict-chroma-from-luma-process).

The normative MaxLumaW/H variables describe the last reconstructed luma
transform extent, not an unrestricted whole-frame prediction window. Current
CfL selection is restricted to one 4x4 chroma footprint and luma is emitted
before chroma. Subsampled partitions do not use the finer 4:4:4-only partition
search; the selected 4:2:0 footprint is its complete 8x8 luma leaf. A 4:4:4
CfL block uses its corresponding 4x4 luma footprint. Interior reads therefore
do not activate the coded backing-edge clamp. This is a current callsite
invariant, not a general redefinition of MaxLumaW/H. A final comment-only
clarification is requested before the two-file repair checkpoint is staged.

Independent new-native oracle execution passes 37 cases: the two original
pilots, five speed/alpha/even-size variations, twelve combinations of explicit
8/10/12-bit and 400/420/422/444 at odd dimensions, and eighteen 8-bit tiny/edge
cases. The latter use 1x1, 1x3, 3x1, 2x2, 7x5 and 9x9 in 420/422/444. The exact
same encoded files are decoded through explicit libdav1d and libaom routes.
All 55 primary/alpha streams are byte-identical to each other and to source
native samples, with automatic conversion/scaling disabled. ICC+nclx, depth,
range, alpha values and distinct hidden colors are checked without an RGB
intermediate. This closes the fixed pilot failure; it does not claim arbitrary
image/content, quality, geometry or all-H5 conformance.

The first review attempt accidentally used the original clean encoder path
after the implementation agent restored that external manifest. Cargo metadata
exposed the baseline dependency and the original red result was retained as
baseline evidence, not mislabeled a repair regression. The successful rerun
uses a separate ignored manifest with a canonical current-dependency assertion,
complete encoder source/manifest/lock hashes before and after execution,
executable/tool/profile hashes, and preserved commands/raw artifacts. Both
decoder runs confirm unchanged frozen source. Earlier pilot artifacts remain
untouched.

The final functional snapshot independently passes product all-targets85,
including fourteen executed existing FFmpeg tests, and the eleven unchanged
external color/plane/property/control tests. Strict all-target Clippy,
focused formatting and diff checks pass. Root's supporting host Rust1.88,
i686 Rust1.91 and Miri results remain separately attributed. The two-file
runtime repair is accepted. Final exact staged review confirms only the two
repair files, 52 additions/11 removals and no unstaged change. The final comment
now distinguishes the reconstructed footprint/backing limit from generic AV1
MaxLumaW/H; this is the only change after the oracle snapshot. The isolated
two-file checkpoint candidate is accepted, not publication or all-H5 completion.
No product source was edited by the reviewer.

## ICC S4 owner-policy repair review: public exact boundary closed

The selected Transform routes now charge only their real stage heap owners;
standalone CompiledProfile retains its additional direction header. Independent
allocator-calibrated exact budgets pass for all four matrix/LUT pairings, while
the unchanged standalone S3 header tests still pass.

One old external cumulative test initially failed because its byte threshold
still added two standalone direction headers. That amount is above the repaired
Transform boundary, so successful construction was correct. The reviewer changed
only the expected inventory to the observed stage-only total and used the same
Transform policy for admission/materialization in the pending-candidate probe.
Exact positive controls, typed one-under rejection, zero allocation before
known rejection, curve/CLUT cumulative limits, real candidate deallocation,
source preservation and same-ledger retry assertions remain intact. All four
independent S4 tests, the four independent S3 tests and the original independent
46 now pass. Included tracked tests are not added to those independent counts.

This closes the public exact-budget P1, not all S4 acceptance. Three finite
requirements remain:

1. Bind OwnerPolicy immutably when RoutePlan is created. The current separate
   admit_with_policy/materialize_with_policy arguments and default wrappers
   can disagree; both phases must consume the same plan-bound owner contract.
2. Move pair construction into the small shared production seam described
   above and migrate the real allocator/drop/source/same-admitted-ledger proof
   into tracked tests. The existing candidate test still retries a fresh
   Transform, and pair_cost still derives expectations from admission itself.
   Restore the admitted checkpoint only after partial output and completed
   input have actually dropped, then exercise retry through that same seam.
3. The obsolete Curve/LUT count helpers are removed, but a newly unused default
   admit_pair and SelectedStagePlan's large-enum diagnostic remain. Remove the
   unnecessary wrapper and use checked fixed plan storage without heap boxing.

Current public callsites consistently select the correct policy; the first
remaining item is a fixed design/invariant gap, not a claimed new public runtime
reproduction. Focused Clippy confirms the new route diagnostics separately from
the previous reader error and other WIP cleanup. S4 remains unaccepted until
these finite requirements are complete. No product source was edited.

## C1 substep5 prefix grammar seam: limited acceptance

The final prefix/finish extraction is accepted as a grammar seam only. The
move-only prefix retains the original borrowed BitReader and reference table,
copies the small SequenceHeader state, and finishes on that same reader after
the prefix boundary. Sequence metadata is consumed before the boundary; no
second header parser or reference-vector clone is introduced. The reduced
branch continues to use the legacy NONE reference table during its trailing
stages. Static comparison preserves old trace strings, conditions and order.

The deterministic normal fixture now uses disable_cdf_update=false (0x11) and
monochrome's true/true subsampling flags. Its prefix ends at bit8 and its full
header at bit21/byte3. The former 0x19 fixture exercised an existing legacy
unconditional end-CDF-bit read, not normative syntax: when CDF updates are
disabled, AV1 infers that field instead of reading it. This pre-existing
grammar issue is recorded separately and was not repaired in this extraction.
The applicable conditional and monochrome flags are specified in the
[official AV1 syntax](https://aomediacodec.github.io/av1-spec/#uncompressed-header-syntax).
The fixture proves the selected header branch, not complete-bitstream or
entropy conformance.

Independent external tests compare all header fields and exact error variants
and messages against the committed pre-extraction parser. Normal/reduced byte
truncations, show-existing rejection, and the real normal WML2Viewer header
with every header-byte truncation pass two tests; the real fixture is required,
not silently skipped. A separate production-cfg allocator observer confirms
zero prefix allocations, unchanged source bytes/pointer, the retained reference
pointer and sequence snapshot, with a positive tile-allocation control in
finish. The initial test-cfg observation included four legacy Windows trace
environment-query allocations; it is not misrepresented as a production data
owner or removed by weakening the zero-allocation assertion.

The final product snapshot passes the four tracked prefix tests, two native
prefix tests and twenty-one existing frame tests independently. The three
existing external OBU/plane/alpha early-admission probes also pass. The corrected
tracked finish-truncation assertion compares the complete error to the public
wrapper. Focused formatting and diff checks pass. Root separately confirms
Rust1.88 prefix4/native-prefix2, Miri prefix4 and actual WASI prefix4 on the final
fixture snapshot. Included product tests are not added to external counts.

Native hookup remains unfinished: actual show_frame validation and retaining
the selected alpha prefix through finishing/decoding instead of reparsing are
still required. Tile/decode ownership and later C2/C3 work are not accepted by
this grammar-only result. No product source was edited by the reviewer.

## Encoder CfL repair checkpoint recorded

The independently accepted two-file CfL repair is saved in encoder checkpoint
9c93f1c; parent a00a12d synchronizes only that encoder gitlink. Exact object and
gitlink review confirmed no unrelated product/version change. Root separately
reran all 85 encoder tests with Rust1.88 in the clean checkpoint checkout.
The earlier 37-case/55-stream native source-lossless evidence remains specific
to the reviewed repair and does not imply full H5 completion or publication.

## ICC S4 final fixed bundle: limited acceptance

The owner policy is now private immutable state of each borrowed RoutePlan;
admission and materialization cannot select different policies. Standalone
CompiledProfile still counts its real direction heap header, while Transform
counts only its actual selected stage owners. Fixed checked matrix/LUT plan
storage replaces the large enum without introducing a planning allocation.
Transform uses the common pair-materialization seam; on failure it drops
partial output and completed input before restoring the admitted checkpoint.

The final tracked proof uses actual allocations, not a manually returned error.
A Gray two-entry input curve is created before the allocator denies the
three-entry destination's twelve-byte allocation. Exactly one input payload
and one allocation denial are observed; completed/partial owners are actually
deallocated, and both source pointers and complete tag bytes are unchanged.
A separately admitted real mAB/mBA pair rejects a planned28/actual32 candidate
with typed ResourceLimit, a real 32-byte peak and zero remaining live bytes.
Its checkpoint is unchanged, and the same plans and same budget then complete
through the production pair seam without readmission or a fresh Transform.
The fixed full-owner size_of formula is checked independently of plan inventory.
TLS/RAII test instrumentation has one test-only allocator registration and does
not change the normal-library allocator.

Final independent execution passes the unchanged 54 boundaries: base7, parse7,
compile22, S1 private3, S2 private7, S3 private4 and S4 private4. Included tracked
cases are not counted again. The seven tracked S4 tests pass, as do product
all-targets179 with one ignored supplementary-metric test. A separate doctest
run passes one and leaves eight documented fragments ignored. Focused formatting
and diff checks pass. The S4 large-enum and newly unused route/count-helper
diagnostics are closed. Ordinary checking still reports the prior three WIP
unused groups; full Clippy is not clean, including earlier compile wrapper
modulo diagnostics and other reader/WIP findings. This result does not waive
that cleanup or authorize an ICC product checkpoint.

Windows Miri is not reported as passed. Its System allocator path fails a
Stacked Borrows header read while the test harness drops its CompletedTest
channel. An independent dependency-free empty test passes with the default
allocator, but reproduces the same failure with bare System registration, a
stateless System wrapper and the actual test Probe. Thus ICC parsing/transform
code and TLS instrumentation are not needed to reproduce this toolchain path.
No Miri checking flag was weakened and no test was excluded to hide the failure.
Root separately reran the final frozen seven S4 tests successfully with Linux
Miri, Rust1.91 i686 and executed WASI. Final Rust1.91 all-targets also confirms
179 passes/one ignored test. These are distinct from the unsuccessful Windows
Miri run.

The fixed S4 selected-route/cumulative-allocation bundle is accepted. Full H3
intent/domain/Gray/oracle coverage, general cleanup, H4 explicit frame color
conversion and C1 Native show-frame/alpha-prefix hookup remain unfinished.
No further scope is started at this checkpoint. Product source, versions,
dependencies and repository commits were not changed by the reviewer.

## Resumed H3 semantics: intent / physical PCS / Gray limited acceptance

The next implementation order is A: designated intent route and same-direction
fallback; B: one physical-D50-XYZ Absolute bridge; C: distinct Gray XYZ/Lab
interpretations; D: checked unclamped device/inverse domains. This implements
the existing H3 design, not a replacement plan. S4's selected ownership and
cumulative allocation contract remains in force.

[ICC.1:2022, 8.10.2 and Table 25](https://www.color.org/specifications/ICC.1-2022-05.pdf)
require the selected A/B tag, then same-direction suffix0 when absent. Absolute
designates suffix1 and is not exempt from this fallback; A2B3/B2A3 are never
selected. The initial implementation/test assumption forbidding Absolute's
fallback was rejected and repaired. A malformed selected tag is not hidden by
fallback, and unused reverse/matrix tags remain unvalidated.

Independent review found two connection errors after Gray Lab was added:
LUT-to-Gray passed converted Lab coordinates into a helper expecting XYZ;
RGB-to-Gray Lab rejected chromatic XYZ instead of using its achromatic channel.
Both have been repaired, with four tracked Gray/PCS regressions. Selected matrix
destinations now receive physical XYZ; Gray Lab uses L*/100, while Gray XYZ uses
Y. Absolute applies the source/destination media-white ratio once in XYZ, without
applying chad again. Non-neutral Lab vectors, both source PCS forms, unequal-white
four-way matrix/LUT pairings, and Gray forward/reverse analytic values pass.

The final independent semantic set passes seven and retains two expected D-slice
failures: unclamped negative device input still succeeds, and an unreachable
inverse sampled-curve value is extrapolated. These are not ignored or accepted.
The prior 54 allocation/parse boundaries pass after two explicit harness updates:
the new private MatrixPlan PCS field is set to the former XYZ value; the obsolete
Gray-Lab-Unsupported expectation becomes physical-XYZ/Lab analytic assertions in
both structural and eager paths, preserving RGB-matrix-Lab rejection. No budget,
allocator, source-preservation or failure-atomicity assertion is weakened.
Product lib33 (including S4's seven tests), LUT30 and transform11 pass independently;
focused five-file formatting and diff checks pass. Ordinary checking retains
two WIP unused groups; full strict cleanup is still open.

### Frozen-source LCMS comparisons

The black-box CLI transport was first calibrated with generated, self-described
Gray XYZ and Gray Lab gamma2 profiles. Gray input is 0..255, not a percentage.
At input127.5 their physical Lab lightness values are respectively 57.0754 and25;
black/white produce0/100. Sharma's 34 printed vectors and metric self-checks pass.
No LCMS implementation source or runtime dependency is used.

On the frozen repaired source, the five pairs below each run all four intents.
RGB uses17^3 points and Gray4096, endpoints included. Both output sets are measured
through the same destination-to-physical-Lab Relative path, with full adaptation,
no BPC and no quantized/bounded flags. Every declared row is present and finite.
The table gives the largest statistic across the four intents for each pair.

| Pair | Points per intent | Maximum p95 DeltaE00 | Maximum DeltaE00 |
| --- | ---: | ---: | ---: |
| v4 preference to sRGB2014 | 4913 | 0.0063801949 | 0.0459857599 |
| Synthetic Gray XYZ gamma2 to sRGB2014 | 4096 | 0.0062008612 | 0.0322419796 |
| Synthetic Gray Lab gamma2 to sRGB2014 | 4096 | 0.0094597953 | 0.0633450190 |
| sRGB2014 to synthetic Gray XYZ gamma2 | 4913 | 0.0012991512 | 0.0034726289 |
| sRGB2014 to synthetic Gray Lab gamma2 | 4913 | 0.0012979954 | 0.0034726289 |

All20 comparisons, totalling91,724 input points, meet the existing median/p95/max
thresholds. Fresh ignored artifacts retain complete CSV/CLI output, settings,
profile and executable hashes, canonical Cargo dependency metadata and identical
source/build/harness manifests before and after the build/comparison. Earlier
diagnostic and S3 artifacts are preserved. These five declared pairs do not cover
all profile classes/models, every LUT format, genuine suffix2 profiles, every
inverse/domain case or all H3. Official equal-white pairs do not replace the
separate unequal-white analytic checks.

A/B/C's fixed semantic slice is accepted; D and the remaining H3 gates are not.
Root separately confirms final all-targets186/one ignored, Gray-focused five tests
on Linux Miri, i686 and executed WASI, plus four Absolute tests on Linux Miri/i686.
These are separate supplementary counts, not additions to the independent set.

Root also reports parent default/highres/encoder all-targets and the seven consumer
configurations passing. Minimal no-default doctests fail the same nine of twelve
examples with highres both off and on because existing examples assume Exif/PNG;
the default/highres doctest configuration passes twelve. This pre-existing feature
assumption remains a full-gate limitation, not a new highres regression. Native
show-frame/prefix hookup and H4 conversion/resolver work remain separate reviews.

## Native prefix hookup review: two finite repairs remain

The frozen hookup keeps master and selected-alpha FramePrefix values alive until
finish and borrows auxiliary payloads through ItemHeaderSource/Parts instead of
constructing cloned AvifInfo values. Actual show_frame is checked. Independent
existing header-admission3, baseline/current grammar2, tracked public hookup2,
native-prefix2 and private hookup2 pass. These results do not close the following
fixed C1 boundaries:

1. **P1 selected-alpha framing is not inspected completely.** A new external
   regression first decodes a positive master/alpha control and verifies alpha
   ID3. It then appends a second frame-header OBU to the selected alpha, confirms
   container parsing retains those exact bytes, and calls the bounded Native API.
   Decoding incorrectly succeeds; two allocations with the master tile payload's
   size are observed. The full-item single-frame walk is applied only to master.
   Reuse one borrowed item-framing validator for both items before either finish
   or master decoding. Keep Legacy's first-frame behavior unchanged.
2. **P1 Native alpha still clones its decoded samples.** After strict alpha
   validation, the common attachment call clones the first PlaneBuffer even on
   the Native branch. Move that validated owner into the master and prove sample
   pointer/content identity in a tracked attachment test; preserve alpha ID3 and
   Legacy behavior. This is the previously approved C1 copy-removal boundary,
   not an expansion into C2 reference/filter/entropy ownership accounting.

Root separately reports Rust1.88 all-targets698/eight ignored on this pre-repair
snapshot and byte-identical Legacy callback/Abort snapshots with highres off/on.
Those existing passes do not detect or override the new Native failures. The
Native hookup remains unaccepted until these two repairs and their observations
pass; the previously accepted parser and grammar-only slices remain separate.

The subsequent runtime repair passes the independent multiple-alpha regression
with zero master-tile-size allocation hits. Both items now use the same borrowed
framing walk before finish; Native consumes the validated alpha PlaneBuffer while
Legacy retains its clone path. Header43 and grammar62 reruns pass (included tests
are not additional independent cases), as do public hookup2/native-prefix2.
Tracked rejection and actual attachment pointer/content identity proofs remain
the final support requirement; pixel equality alone cannot detect a reintroduced
clone.

## H3 D strict-domain review: fixed follow-up boundaries

The old semantic nine and fixed allocation/direction boundaries remain separate
from the new strict-domain probes. Two new edge tests pass: exact device endpoints
are accepted but adjacent/subnormal outside values are rejected; decoded sampled
curve endpoints are accepted but their one-ULP and small outside values are not.
No quantization epsilon expands the supported input or inverse-image domain.

Two finite follow-ups were independently reproduced:

- CompiledProfile's two LUT branches still used the legacy always-clamped
  evaluator. A legal mAB/mBA matrix stage producing normalized1.5 silently clipped
  in both directions despite the directional API's strict contract. Connect both
  branches to the shared strict evaluator and track both directions, preserving
  the explicit clamp=true pair path.
- Parametric function1/2 with threshold2 selects its constant branch throughout
  device[0,1]. The new domain validator nevertheless checked the unused negative
  power base and rejected forward compilation. Validate only the intersection
  of each selected piecewise branch with the supported domain; constant inverse
  rejection remains unchanged. This preserves the existing forward-only contract.

The synthetic mft2 white correction is justified by ICC.1:2022 Table14's exact
PCSXYZ codes7B6B/8000/6996, rather than a relaxed oracle tolerance. Table68 defines
the parametric branch selection. The existing full-grid runner explicitly uses
clamp=true and is not proof of strict-domain error behavior. D remains pending
these finite repairs; class/version routing, execution/worker limits and WIP
Clippy cleanup are the next separate E slices, not additional D acceptance claims.

### Next H3 E slices: bounded implementation order, not acceptance

1. Retain raw version/class in structural Profile metadata and retain requested
   intent, selected tag/model and fallback in the immutable route description.
   Validate executable class/model at the common route-plan entry, before stage
   allocation. For the approved ICC v2/v4 models, Input/Display may use RGB XYZ
   matrix or Gray XYZ/Lab; Output uses Gray or supported LUTs; ColorSpace uses
   supported LUTs, not an invented RGB matrix interpretation. Unsupported
   DeviceLink/Abstract/NamedColor or unknown classes return the existing explicit
   Unsupported category when compiled. Preserve readable raw metadata and the
   compatibility constructors' separate eager-validation path. Test selected
   fallback information, missing/unsupported classes and untouched unused routes.
2. Audit execution allocations separately from immutable compiled storage.
   Worker F32 can call the existing slice core without copying the image.
   U8/U16 conversion can share a fixed pixel/chunk buffer instead of retaining
   two image-sized Vecs. If dynamic buffers remain, account actual retained
   capacities and old-plus-whole-replacement peaks before reserve, with atomic
   failure/retry tests. The allocating F32 wrapper must check pixel/sample/byte
   multiplication and its output bound before reserve. Add private-field limits
   and checked builders where needed; do not add fields to the existing public
   options structs or variants to the exhaustive error enum. Verify cold/warm
   allocation observations, constant-cost worker creation and preserved lengths.
3. Finish the current ICC WIP's unused-item and strict Clippy cleanup without
   suppressing new diagnostics as unrelated baseline. Preserve fixed54 plus the
   seven product S4 tests, accepted semantic/domain assertions and the frozen
   declared-pair oracle recipe. E is not CMYK/MPE/BPC/HDR execution, H4 completion
   or permission to publish/commit the ICC product.

### H3 D final independent decision

D's fixed domain slice is accepted after both repairs. The final external target
passes35 tests: unchanged22, semantic9 and the four new domain boundaries; these
are not35 additional tests. The original fixed54 and the seven product S4 tests
also pass independently. Product LUT36/transform12, six-file focused rustfmt and
diff checks pass. Tracked tests exercise both public compiled LUT directions and
forward-only parametric function1/2 with inverse-constant rejection. The unused
always-clamped LUT wrapper was removed; normal builds retain the two previously
recorded WIP unused items. This is not a strict Clippy or whole-H3 pass.

The same five declared profile pairs were rebuilt and rerun at this frozen D
snapshot: four intents,91,724 points, no missing rows or failures, all existing
DeltaE thresholds met. Per-pair statistics equal the prior A/B/C result. Fresh
ignored artifacts preserve canonical Cargo metadata, source/build hashes before
and after (identical), executable/profile/oracle hashes, settings and raw output;
the prior artifacts remain untouched. The bounded clamp=true oracle comparison
and strict-domain error probes remain separate evidence.

Root separately confirms the final two tracked repairs on Linux Miri, i686 and
executed WASI. These supplementary checks do not close E's class/model metadata,
execution/worker bounds, cleanup, untested route/format coverage or H4.

## Native hookup final review and limited checkpoint candidate

The two fixed Native repairs and their support tests are accepted. The public
extra-alpha-frame regression first requires a successful synthetic alpha control
and exact selected payload retention. Its separate independent public-API probe
continues to observe zero master-tile-size allocations on rejection. The tracked
owner test decodes real master/alpha planes, then calls the same consuming
attachment helper used by production and checks sample pointer, length, capacity,
content and attached planeID3. Legacy keeps its previous clone/validation path.

Final independent execution passes external44 (the prior header43 plus the new
framing probe), grammar62, product library520/six ignored and native boundary11,
hookup3, limits5, phase9, prefix2, rich3. Included tests are not counted again as
new independent evidence. Strict Clippy all-targets, rustfmt, rustdoc and diff
checks pass. Root separately confirms the six focused tracked tests and rustdoc
on Rust1.88, plus the previously recorded Legacy callback/Abort equivalence.

The exact eight modified plus fifteen new AVIF source/test files form a coherent
local checkpoint candidate for accepted container budgets, shared prefix grammar
and Native still/alpha hookup. The additive API and limit rustdocs now explicitly
say that deep tile/entropy, reference/filter scratch and total-live-allocation
coverage are unfinished; AVIS/derived inputs remain explicitly unsupported here.
No Cargo, version, external fixture, encoder, ICC or parent H4 files belong in
this checkpoint. This decision permits the limited local checkpoint, not
publication or a claim that all C1/C2/C3 bounded decoding is complete.

Exact staged inspection found eight literal NUL characters in two synthetic ftyp
byte strings in the phase test. Both were replaced with equivalent Rust escapes,
preserving the encoded fixture bytes. Final independent staged inspection finds
zero NUL characters, a normal text diff, the exact approved23 files, no unstaged
AVIF changes and a clean diff check. Root reran phase9 on Rust1.88 successfully.
The exact staged local checkpoint is accepted; the unfinished decode scopes above
remain explicitly outside this decision.

Root saved the reviewed23-file AVIF checkpoint as
`b5e4de09a8982e9554378670892aa8cb89517aef`; independent object inspection confirms
the exact file inventory and a clean nested worktree. Synchronizing the parent
AVIF gitlink to this exact object is ready for a separate exact staged review;
parent H4 work, ICC work, encoder and versions remain excluded.
