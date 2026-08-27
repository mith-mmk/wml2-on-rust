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
