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
- AVIF C1 parser/prefix/Native hookup checkpoint b5e4de0 is saved. C2 shared
  allocation work remains uncommitted. Items1/3 aggregate history and actual
  excess-capacity/drop-before-restore have finite acceptance; item2 transfer of
  original reservation tickets has limited acceptance for iinf names and its
  outer/sidecar owners only. Other retained-owner transfer remains pending.
- ICC directional Gray/RGB CMS checkpoint 83f857a is saved. Its finite A-E,
  S1-S4 and execution proofs do not establish all-profile/full-H3 acceptance.
  The later reverse reference-black repair and tracked proofs have limited acceptance;
  its six-file checkpoint f5397b6 is saved, and reverse oracle gates have
  separate interpolation, final-clip and explicit-Unsupported qualifications.
- H4 borrowed route planning checkpoint 33fc584 is saved. The private native
  pixel reader's six proof groups have limited acceptance at the A225DB8 snapshot;
  step1 codec-free ledger/ownership extraction has independent mechanical acceptance.
  Step2's public read-only conversion record has finite acceptance; output pending
  admission/materialization is still WIP, so public execution as a whole is not
  accepted. CMS/full-H4 and unconnected-helper diagnostics remain open gates.
- Full H3/H4, decoder total-live ownership and broader fuzz/profile gates remain
  unfinished. No publishing, version bump or wider checkbox completion is authorized.

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

## H3 E1 final class/model and retained-route review

E1 is accepted as a finite functional slice. Raw version/class survive structural
parsing; the common route planner rejects unsupported major versions and
class/model combinations before selected curve allocation. Supported v2/v4
Input/Display matrix and Gray models, monochrome Output models, and ColorSpace
LUT routes are covered without inventing an Output RGB or ColorSpace matrix
interpretation. Selected malformed routes and unused-direction behavior retain
the previously accepted checks.

The immutable compiled stage and Transform retain actual input/output route
metadata, not only temporary planning fields. Independent tests drop Profiles
and original clones and verify distinct input/output version, class, requested
intent, selected tag and model. A new boundary found that missing-tag matrix
selection incorrectly reported no fallback. The final repair and tracked
input/output assertions now distinguish a designated tag from same-direction0
or legal-model fallback; all four intents and both directions pass independently.
The accessor's public documentation should also state this agreed definition.

Final independent execution passes five new E1 tests (the complete included
target has40), the fixed54 and product S4 seven. The old external Gray pair
builder received an explicit supported version header; its allocator observations,
exact/one-under budgets and all assertions were preserved. The larger retained
CompiledDirection header is covered by actual size-based accounting; Transform
still does not charge nonexistent direction heap owners. Restored final product
all-targets pass197/one ignored, and focused non-recursive rustfmt plus diff checks
pass. Two known WIP unused items and strict cleanup remain separate E3 work.

An accidental formatter traversal changed unrelated legacy CMS files during
implementation. Root reports verified backups and an approved exact restoration;
independent final status confirms those legacy differences are gone, and the
full product run above is after restoration. This E1 decision permits the next
execution/worker-bound slice, not all-H3 acceptance, ICC publication or an ICC
product checkpoint. The earlier frozen diagnostic oracle remains evidence for
its recorded snapshot, not an unperformed E1 oracle rerun.

Separately, root saved the Native checkpoint documentation and exact AVIF gitlink
as parent `297e7eaf1a22964f76a3951a98480a7826923448`; parent H4 and ICC work were
excluded from that checkpoint.

## Next decoder allocation slice: candidate design, not C2 acceptance

The next bounded assignment remains normal `av01` still plus its selected alpha.
Current Native preflight checks both prefixes, but header completion then creates
tile descriptors/data/entropy probes and a Vec-based decode plan before the
existing decode-plan check. `alloc_coded_frame_buffers` clones plane layouts and
allocates the outer plane Vec and sample Vecs; crop later owns coded and visible
samples simultaneously. These are concrete connection points, not evidence that
entropy, filters, references or AVIS have been budgeted.

Use two reviewable steps rather than a decoder-wide rewrite:

1. Introduce a private, non-Clone `DecodeBudget` and move-only allocation tickets,
   sharing the existing container allocation engine's checked capacity arithmetic,
   fallible fresh/replacement reservation and scalar checkpoint rules. Keep parser
   count/work policy separate. Do not copy the container helper into another
   allocator, create an unbounded pointer registry or use ambient global state.
   Preserve the existing `NativeDecodeLimits::new` signature and public frame
   types. An additive checked setter can select the new allocation ceiling;
   existing constructor behavior remains its documented partial-bound contract.
   Neither an absent new setting nor this slice implies a total-live guarantee.
   The internal bounded parse/decode entry must carry one budget forward, retaining
   actual metadata/payload owners after parser-only owners have really dropped;
   public parse-only return may end that local accounting. Do not reset each phase
   to the caller's full ceiling or relabel `max_metadata_bytes` as a decoder cap.

   First wire fixed-slot visible/coded plane geometry through the existing
   `plane_layout_for_geometry`, shared decode-plan validation, coded allocation
   and crop. Plan master and selected alpha before their first sample allocation.
   Charge plan/plane outer storage and each actual sample capacity once, including
   completed master storage during alpha construction. Crop checks old plus the
   whole visible candidate before reserve; an already-visible plane keeps pointer
   identity. Move tickets with moved samples; release only after real drop.
   Keep Legacy public allocation/decode wrappers and their error ordering intact.

2. Connect tile/header materialization to that same budget, without a second
   Native grammar. Shared `finish_frame_header`/tile-info parsing needs an internal
   allocation-context adapter, while its old wrapper preserves Legacy behavior.
   Extract checked tile-range inspection from `parse_tile_group` and the existing
   merge path, and have materialization consume those validated ranges. Check all
   selected master/alpha tile counts, ranges and aggregate copy sizes before the
   first tile copy or entropy preparation. Budget TilePayload/TileDecodePlan
   outers, merged data, frame-header tile arrays and retained header owners;
   eliminate per-tile deep copies when one checked final merge suffices. Use the
   same planned records in the decoder, not validation followed by a raw reparse.

Each step needs exact/one-under known-owner peaks, odd coded-padding/crop cases,
master-plus-alpha simultaneous ownership, real reserve denial, injected excess
capacity, candidate drop before counter restoration, source/ticket invariance and
retry on the same surviving state. Preserve the accepted Native framing/alpha
pointer, prefix/wrapper equivalence and Legacy callback regressions. Actual
entropy/CDF/motion/filter/film-grain/super-resolution owners, shared-reference
deduplication, bridge peak handoff and transactional AVIS remain later C2/C3 work.
The private ticket/transaction interface should permit those future owners; this
assignment neither implements them nor adds managed public frames or fallbacks.

## H3 E2 execution review: runtime observations pass, final proof pending

The frozen follow-up shares checked output-length/byte/addressability arithmetic
and uses one fixed64-pixel integer loop for validation and publication. Direct
and worker F32 avoid image-sized copies. The integer validation pass preserves
the previous whole-output-on-error behavior; this does not add an equivalent
global atomicity promise to borrowed F32 execution.

Independent execution passes six new public tests plus the included previous40.
They observe cold/warm heap requests, Gray/RGB channel mappings, U8/U16 rounding,
length errors, late inverse/LUT failures with unchanged entire integer output,
requested exact/one-under/empty limits, the agreed64MiB default and real output
reservation denial followed by successful retry. Three additional private tests
pass scalar length/byte/isize overflow, error precedence and same-plan retry;
impossible slice lengths are tested as scalars, not forged references. All
known-before-allocation failures retain zero allocator requests. The previous
fixed54 also pass after a module-only external harness migration, with their
assertions unchanged.

E2 acceptance remains pending the fixed final evidence bundle: inject excess
capacity through the very same fresh-output core used by the public wrapper,
observe actual candidate deallocation and rejection before fill, and retain
tracked scalar/allocator/cold-warm/late-error regressions. The current product's
three execution tests alone do not prove those properties. Public documentation
must state the default64MiB output ceiling, its use by the old allocating wrapper,
the explicit override and zero's empty-output meaning. This is not a request for
another execution policy or E3 cleanup, and no new runtime failure is claimed
from the passing cases above. Two existing normal unused warnings and broader
strict-Clippy cleanup remain separate work.

## H4 first borrowed-plan review: fixed twelve-boundary NO-GO

The first frozen options/resolver snapshot is plan-only: no output conversion or
CMS compilation is implemented. The parent Cargo change registers its highres
test only; no new dependency or version is involved. Separate flat modules keep
responsibilities apart under the reported directory-creation constraint.
Independent tests use the pinned public ICC dependency, not the local ICC WIP.

The fixed independent set has twelve tests: three pass and nine fail. Passing
observations cover H.273 codes0..5 and AV1 phase translation, explicitly described
F32/linear input with no heap work or original-metadata rediscovery, and per-ICC
requested bounds without copying plus successful retry. The remaining failures
are grouped into these four existing-contract repairs, not converter expansion:

1. Validate the source/destination domain and policy pair, not only each enum.
   Undefined SDR-to-nits/HLG and PQ-to-SDR transitions currently succeed. So do
   different declared whites under RequireSameWhite and nonlinear output from
   premultiplied linear input under PreserveAssociation. Keep defined positive
   routes, including homogeneous linear preservation; do not implement new HDR
   normalization, adaptation math or alpha-association execution in this slice.
2. Check selected borrowed ICC headers/declared ranges/version/channel space
   before accepting the plan, under source/destination ICC bounds. Three bytes
   and a Gray profile assigned to RGB input currently succeed. Selected malformed
   ICC must not fall back to CICP; tracked preference tests need valid synthetic
   profiles instead of declaring truncated bytes a successful interpretation.
3. Resolve native fields independently of colour authority. Known active nclx or
   AV1 range/matrix must fill missing explicit options, while known conflicting
   descriptions must be checked. The current resolver instead requires explicit
   integer options and ignores AV1 range conflicts. Retain resolved facts in the
   borrowed plan; CICP override must not change storage/range/matrix semantics.
4. Include retained borrowed option profiles in the live budget without charging
   the same known owner twice. An8192-byte explicit source/destination profile
   currently passes a4096-byte total-live ceiling. Existing per-ICC bounds,
   borrowed-source immutability and zero profile-copy observations must remain.

The authors have received this fixed bundle. Root separately reports old16 plus
new8 product tests and consumer/dependency isolation checks passing; those do not
validate the incorrect domain/header success expectations or close these nine
failures. No H4 execution, full H3 acceptance or product checkpoint follows from
this first-slice review.

## H3 E2 fresh-output seam follow-up: two fixed candidate failures

The next frozen seam is the production allocating wrapper's actual shared core.
Independent private tests now pass five of seven cases. The excess-capacity
candidate allocates16 bytes against an8-byte ceiling, is really deallocated, and
leaves all four initialized sentinel values unchanged at deallocation: rejection
precedes fill, live bytes return to zero, peak is16, and retry on the same plan
succeeds. Requested rejection precedes the maker; empty output at limit zero also
passes. The original public46 remain passing, with only six being new E2 tests.

Two private failures remain: a capacity-zero candidate for two F32 outputs causes
an implicit16-byte resize allocation and succeeds despite the8-byte ceiling; a
nonempty candidate is accepted as a fresh output. Reject both invalid candidate
states before resize, without another allocation. Complete the already-requested
tracked cold/warm and late-integer-error observations and the output-limit public
documentation; do not broaden this repair into another execution policy.
The fallback accessor documentation is now present. Root's supplemental i686,
actual WASI and Linux-target Miri passes cover the current three private and
three integration tests, not these missing boundaries. E2 remains pending this
finite repair; E3 has not been accepted or implemented by the reviewer.

### E3 bounded cleanup candidate, after E2 acceptance

First remove genuinely unused transform-only state/helpers and resolve local
derive/style diagnostics; keep unrelated legacy cleanup and recursive formatting
out of this assignment. Do not suppress the linter to claim success.
Removing an unused retained matrix field must not remove any existing eager
constructor's semantic validation; separate that validation from storage. Update
post-parse owner tests only where their actual structural ownership changes.

For the large LUT shape enum, preserve no-allocation preflight by moving the
common channel fields and fixed three CurveSetShape slots into an enclosing
LutShape struct. A smaller layout enum then distinguishes MFT counts/matrix/ranges
from mAB/mBA direction/matrix-range/CLUT fields. MFT has empty curve slots; private
constructors preserve the valid-state invariant. The existing nine borrowed
CurvePlan descriptors remain authoritative for both inventory and materialization.
No Box/Vec, raw reparse, reduced CLUT grid support or counts-only substitute is
needed. Verify actual host/i686 structure sizes and avoid enlarging the bounded
stack footprint. Preserve selected-shape allocation-zero tests, all direction and
curve semantics, owned-header exact/one-under budgets and S4 actual-owner/drop
proofs. This is a proposed representation change, not a completed cleanup gate.

## Native C2-1 implementation interface: fixed owner slice, not total-live closure

This refines the approved candidate into six bounded implementation items. The
only proposed public addition is a checked positive ceiling setter on
NativeDecodeLimits, provisionally `with_max_live_allocation_bytes(usize)` returning
Result. Its private optional field is unset by the existing twelve-argument
constructor, preserving that constructor and its documented partial bounds.
Document precisely which owners are covered; neither a missing ceiling nor this
slice establishes a process-wide or complete C2 total-live guarantee.

1. Extract the existing fresh/replacement reserve mechanics from
   `container_budget.rs` into a small private allocation module. A private ledger
   adapter supplies checkpoint, requested/actual admission, commit, release and
   restore; ParseContext keeps its class sublimits and count policy, while a
   non-Clone DecodeBudget holds the shared aggregate ceiling/live/peak state.
   Preserve move-only AllocationToken semantics through a common AllocationTicket
   implementation: exact capacity ownership, explicit adoption, fresh-only
   tokenless calls, and validation before spare-capacity return. The one engine
   must handle old-plus-whole-candidate storage with fallible allocation and
   actual-capacity reconciliation before moving/filling. Candidate failure must
   explicitly drop allocated storage before restoring scalar state. Legacy uses
   its existing reserve policy and error ordering, not the Native ceiling.

2. Add an internal parse handoff used by strict Native decode rather than ending
   accounting through the public `parse_native_info` return. ParseContext may own
   the DecodeBudget and move it out; this avoids a second lifetime parameter or
   a shared/global mutable registry. After projection, explicitly drop parser-only
   MetaState owners, then transfer the surviving rich metadata, ordered-property
   owners and primary/selected-alpha payload capacities into retained tickets.
   Reuse the retained metadata walker, with a separate payload total: the existing
   metadata total intentionally excludes item payload samples. Preserve peak
   history and subtract only real dropped owners; do not seed a fresh full budget
   or substitute the earlier parser live total. Public parse-only return ends its
   local accounting as before. The existing DecodedFrame colour payload clone
   needs the same small fallible copy adapter when it creates another retained
   owner; no new colour metadata behavior is introduced.

3. Put fixed geometry in a private NativePlanePlan with up to three visible/coded
   layouts per item, plus a master/optional-alpha pair. Build it from the accepted
   prefixes using `plane_layout_for_geometry` and the existing coded alignment
   arithmetic before the first sample allocation. Both dimensions and each plane
   byte limit must pass before master allocation. Materialize the plan's layout
   Vec through the shared engine; do not clone coded layouts or recompute a
   separate Native geometry algorithm. The public decode-plan builder remains a
   compatibility wrapper over the same geometry logic. Tile descriptors and
   entropy materialization remain the next slice, not hidden acceptance here.

4. A private FrameAllocationTickets sidecar holds the layout/plane-outer ticket
   and fixed sample tickets; public FrameBuffers/DecodedFrame stay unchanged.
   Pass the sidecar and the same budget through coded allocation, both crop call
   sites in `decoder.rs`, finishing and selected-alpha attachment. A selected
   alpha may reserve the master's four-entry plane outer up front, charging its
   actual capacity once. Keep completed master owners live during alpha decode;
   move the alpha sample ticket with its Vec and release the alpha outer only
   after its consuming iterator really drops. No-op crop keeps the sample pointer.
   For changed crop storage, prepare candidates before publishing replacement;
   failure drops candidates and preserves the surviving old owners and tickets.
   Internal transaction scopes must drop failed owned values before rollback;
   tickets are not an excuse to release storage that remains alive.

5. Root includes the two necessary persistent-plane replacement adapters in this
   same slice: super-resolution sample replacement and film-grain plane source/
   output clones. Otherwise the sidecar would describe obsolete capacities at
   alpha decode or return. Route these allocations through the same old-plus-
   candidate engine, with source clone and output simultaneously charged where
   they really coexist. Share the existing math/RNG/kernel bodies and retain the
   Legacy signatures, output and supported inputs. Do not reject previously
   supported Native inputs to avoid this wiring. Grain LUT/scaling, other filter
   scratch, tile/entropy, CDF/motion, references and AVIS remain explicitly outside
   this slice. End decoder accounting on successful public ownership transfer;
   no managed public frame or caller-lifetime tracking is added.

6. Keep the initial proof set finite: parser-only-drop/retained payload handoff;
   fixed plane exact/one-under including outer capacity; odd coded-padding crop
   with no-op pointer control; master-plus-alpha simultaneous peak and moved
   pointer; real reserve denial/excess-capacity rejection followed by same-state
   retry; and persistent super-resolution/grain replacement peak with unchanged
   math output. Use exact portable capacity/size formulas, not a binary-searched
   self-derived limit. Test known rejection before allocation, candidate drop
   before scalar restoration and source/old-owner/ticket invariance. Keep tests
   separate from product modules and preserve all accepted parser, prefix,
   Native-alpha and Legacy callback regressions. This design is not a test result.

## H3 E2 final independent decision: limited GO

The final shared output core rejects both short-capacity and nonempty candidates
before resize; the two reproduced failures are closed. Requested bounds precede
the candidate maker, actual capacity is checked before fill, and an allocated
overcapacity candidate is really freed with initialized sentinels untouched.
Source values/pointers and the checked plan survive failure, and the same plan
retries successfully. Scalar overflow/addressability and empty/exact/one-under
cases remain covered without constructing invalid references.

Final independent external execution passes46 public tests (six new E2 plus
the previous40) and seven new private tests. Tracked execution passes six private
plus five integration tests after the last test-only strengthening: cold/warm
calls must succeed with identity output, and the late inverse case first proves
all4097 valid samples succeed before changing only the last sample. The independent
public tests additionally exercise all Gray/RGB mappings, U8/U16 rounding,
allocation-zero cold/warm execution and late inverse/LUT failures without partial
integer publication. No assertion or existing limit was weakened.

The runtime snapshot's product all-targets pass208/one ignored; final focused
formatting and diff checks pass. Root additionally reports all six private and
five integration execution tests passing on i686, actual WASI and Linux-target
Miri. Those supplemental runs precede the final two test-only non-vacuity edits;
the final strengthened assertions were independently rerun on the host. The
previous fixed54 and product S4 seven remain intact. Strict Clippy still fails
outside the new execution/worker/limit files; this is E3 work, not a whole-crate
lint pass. Default64MiB, zero/empty behavior, the old allocating wrapper's default
and explicit override are now documented. The E1 fallback accessor definition
is also documented.

Root reran the frozen diagnostic recipe on its recorded E2 runtime snapshot:
five declared pairs, four intents each, total91,724 points and all20 threshold
checks pass (largest DeltaE00 about0.063345019). Independent artifact inspection
confirms all20 summaries and matching before/after source records; binary,
profile, oracle and command provenance are retained with that run. This is the
explicit clamp=true pair set, separate from strict-domain negative tests, not
every ICC profile/class or full H3 coverage. E2 permits the bounded E3 cleanup
assignment only; ICC publication, product checkpoint and full H4 remain unaccepted.

## H4 plan-only follow-up: fixed domain, white and alias table

The initial twelve independent cases now pass. Six additional tests exercise
the same approved resolver conditions, not conversion math or a new destination
feature: a ten-source by ten-destination physical-domain table, known CICP/linear
white agreement in both directions, Gray matrix applicability, explicit filling
of unknown native matrix information, preserved-profile aliases and overlapping
borrowed-profile ranges. On the reviewed intermediate repair these are three
passes and three failures, hence total15/18, not H4 acceptance.

The physical-domain table passes: relative SDR transfer changes are allowed,
PQ/nits do not become HLG scene values without a policy, ICC is not treated as a
nit/HLG route, and encoded destinations receive the same domain checks as linear
ones. Preserved ICC whole/prefix aliases are not charged twice; overlapping
external profiles charge their byte-range union while disjoint profiles still
sum. Successful planning has zero observed heap requests. The three remaining
tests demonstrate known-white mismatches accepted under RequireSameWhite, Gray
luma rejected solely for nonidentity colour signaling, and explicit native
interpretation incorrectly conflicting with unspecified matrix code2.

Gray scalar range/transfer processing must be separate from preserved matrix
signaling. [AV1 sections5.5.2 and6.4.2](https://aomediacodec.github.io/av1-spec/av1-spec.pdf)
do not require identity color signaling for monochrome: color-description fields
precede its no-chroma branch. This is the existing model-applicability condition,
not a request to add chroma conversion or change AVIF decoding. Known conflicting
descriptions must still fail; an unspecified field is not a known conflict.
These finite repairs have been returned to the author. ICC semantic compilation
and any deferred PCS/media-white bridge must remain explicit in planning docs.

Root separately reports the pre-alias-repair highres24 tests, new8 on i686/WASI/
Linux-target Miri, seven consumer configurations plus five dependency-shape
checks, and five alpha-AVIS Legacy snapshots byte-identical with highres off/on.
Those are compatibility/portability evidence, not acceptance of the remaining
three resolver conditions or actual frame conversion.

## ICC E3 independent structural cleanup review

The new LutShape keeps the same authoritative borrowed curve plans in fixed
common slots and a smaller layout enum. It introduces no Box/Vec in preflight,
no count-only replacement and no hidden raw-parser fallback. Matrix storage is
removed from ProfileInner while eager constructor semantic validation remains.
The independently rerun old54 boundaries, E2 public46 (old40 plus new6), private7,
and product all-targets208 with one ignored pass; the product run includes S4's
seven ownership/rollback tests. Normal all-targets check emits no warnings and
all changed/new Rust files pass focused nonrecursive formatting and diff checks.

An ignored independent mirror reconstructs the complete pre-E3 representation
from the recorded original schema, using unchanged component types. E2's frozen
source manifest confirms route_plan and compile_plan remain byte-identical, so
the surrounding route representation is not guessed. Same-compiler measurements
are Shape624->624, LutPlan672->672 and Route936->936 on x64, and348->348,
372->372 and564->564 on i686. The same probe plans mft1/mft2 in both directions
and mAB/mBA in their designated directions with zero allocation and unchanged
source pointer/content. These are
measured mirror/current sizes, not a claimed old binary build or a lower bound.

Root additionally reports i686 all-targets208/1ignored and warning-denied rustdoc
success. Its separately recorded E3 LCMS recipe again passes all20 comparisons
over91,724 points with unchanged before/after source hashes and E2-equivalent
statistics; the explicit clamp=true pair set remains narrower than full H3.

E3 behavior and stack-size boundaries are closed. Strict whole-crate Clippy is
still not green: this independent run reports209 diagnostics in untouched legacy
library modules plus three in the unchanged transform test file. None originates
in the changed transform production files. Before the cohesive local ICC
checkpoint, remove the five existing uncommitted LUT lint suppressions with the
approved tiny private-wrapper/definite-assignment cleanup, preserving all math
and observers. Do not label these WIP suppressions a published baseline, or
silence the remaining whole-repository lint gate. No ICC product commit,
publication, full-H3 or H4 acceptance is claimed by this intermediate review.

### E3 final tiny cleanup and cohesive ICC checkpoint candidate: limited GO

The five LUT suppressions are now removed. Private compatibility entrypoints
are test-only and have one explicit tracked regression; production read_tables
needs no suppression. CLUT weights/corners use complete branch tuple assignment
with unchanged formulas and corner selection. Independent final normal
all-targets check has zero warnings, unit40 passes, and all27 candidate Rust
files pass focused formatting/diff checks. Whole-crate strict Clippy still fails
only in the previously identified untouched modules/test file; it is not waived.

The official-profile test no longer silently succeeds when its file is absent.
It is explicitly ignored in ordinary runs, requires the environment-supplied
fixture path, and fails clearly when invoked without it. Independent explicit
invocation with the official fixture passes one; invocation without the setting
fails one as expected. Tiny-fix all-targets therefore reports208 passes and two
ignored, including the new private compatibility test, with the official-profile
pass counted separately. Old54, E2 public46/private7 and the size/no-allocation
probe also pass after the substantive tiny cleanup; the final annotation-only
removal is followed by the clean check/40-unit rerun.

The cohesive27-file candidate comprises the checked facade, structural profile
parser, direction/route and matrix/LUT plans, shared owner budget, bounded
execution/worker, test-only allocator and their tracked regressions. No Cargo
dependency/version, legacy CMS source, AVIF/encoder or parent H4 change is part
of this ICC candidate. Existing A-D/E1-E2/S1-S4 acceptance and E3's finite cleanup
support a local unpublished checkpoint; no known P1 remains in that fixed scope.
Root may stage those27 files for a separate exact-cached review. Full-H3 profile
coverage, broader fuzz/conformance/release gates and whole-repository strict
Clippy remain open. A checkpoint is not publication or completion of H4.

### H4 same-table final source-interpretation check

The repaired known-white, Gray nonidentity/unspecified signaling, real AV1
monochrome unknown-position and explicit unknown-matrix cases bring the existing
eighteen tests to18/18. Final source inspection identified one additional path
through the same domain condition: explicit CICP transfer8 on an already-declared
nit/HLG frame was allowed to relabel it as relative light. The independent
nineteenth test reproduces acceptance of nit-valued input into encoded sRGB.
Its fixed controls require identity transfer to preserve each declared domain,
including same-domain success and the already-permitted HLG scene/display route.
Return this one path to the author, preserving the shared domain table rather
than rejecting all HDR identity transfers. The source/destination CICP validation
helper duplication is included in that same small repair; no new math is added.
H4 plan-only checkpoint remains pending this final source-route fix.

### ICC checkpoint saved; H4 plan runtime boundaries closed

Root saved the independently reviewed27-file ICC checkpoint as
`83f857a499983e3d5399b803fc7e5ca13f9cba16`; its worktree is clean. The exact cached
inventory was8121 insertions/208 deletions, with no Cargo/version or unrelated
legacy source changes. Version0.0.4 is unchanged; nothing was pushed/published.
The limited local-checkpoint qualifications above remain in force.

H4's TC8 repair now keeps the descriptor's declared linear domain and uses the
same shared source/destination domain table. Independent19/19 passes, including
same-domain identity and HLG scene/display positive controls, along with the
tracked12 and previous16 product tests. Known whites, ICC header/space bounds,
borrowed alias accounting, Gray/AV1 no-chroma applicability, active-source
precedence and source immutability remain covered. The duplicate CICP
primary/transfer checks are shared. Public validate_for documentation explicitly
defers selected ICC execution and PCS/media-white/adaptation gates.

This closes the fixed plan-only runtime conditions, not convert_frame,
quantize_frame, CMS compilation, native reconstruction or output allocation.
Root independently reports final-runtime Rust1.91 i68628, executed WASI12 and
Linux-target Miri12 passes. Default+highres warning-denied rustdoc passes;
no-default+highres still fails on the existing unrelated TiffHeaders link in
metadata documentation. A final independent strict-Clippy run found four new
collapsible-if sites in convert_plan (duplicated by lib/test diagnostics).
These require the approved syntax-only cleanup before the six-file H4
checkpoint; do not merge them into the old module baseline or suppress them.

### H4 planning slice final checkpoint decision: limited GO

The syntax-only repair closes those four new Clippy findings without changing
conditions or evaluation order. On the final frozen source, independent19/19
and tracked12/12 pass again; focused formatting/diff checks pass and strict
Clippy reports zero diagnostics in the six-file candidate. The whole invocation
still has unrelated baseline diagnostics, so this is not a whole-repository lint
pass. Root also reruns final-style i68612 and default+highres warning-free check;
the preceding WASI/Miri12 results refer to the semantically equivalent pre-style
snapshot, not an unperformed final-style rerun.

The six files are the test registration, highres module export, options facade,
borrowed plan, options types and dedicated options tests. Cargo changes only add
the highres-gated test target; no dependency, version, AVIF/encoder, ICC source or
legacy callback change is included. These files plus this acceptance record are
approved for an exact-cached parent checkpoint review. The next private native
pixel-access design is separate and is not part of this checkpoint. H4 execution,
CMS wiring, output ownership, quantization and full-H3/C2/C3 completion remain open.

### H4 planning checkpoint saved

Root saved the exact reviewed seven-file parent checkpoint as
`33fc5848adfdfeb1191533b99ae114e8bcd5af7f` (1795 insertions). AVIF C2 work and
untracked JXL were excluded; dependencies and versions were unchanged. Root then
reran actual WASI12 and Linux-target Miri12 on the final style-only source, both
passing. These are post-commit supplements to the earlier pre-style evidence,
not retroactive claims about which snapshot those earlier runs used.

## H4 next finite slice: borrowed native pixel access (design, not acceptance)

This concretizes only the native-reconstruction part of the approved H4 order.
No product implementation or new test pass is claimed. Retain the accepted
plan-only19 boundaries and tracked12; do not reopen their domain/white/ICC-alias
policy or broaden this slice to transfer functions, primary conversion, CMS,
quantization, output allocation, geometry application or AVIF decoding.

### One checked plan and fixed borrowed addressing

Add a small private `convert_native.rs`, wired from the existing convert facade;
the accepted flat-module layout remains usable. Keep dedicated tests separate.
Use `NativePixelReader<'a>::inspect(source, options, limits)` to call the existing
`ConversionPlan::inspect` exactly once, retain that plan and the immutable source
borrow, and construct a fixed role/address table. Small private plan accessors
may expose already-resolved facts; do not duplicate resolution or header parsing.
Future execution must consume this retained plan rather than inspect again.
Do not call the reader from public `validate_for` merely to make it reachable:
that facade retains its accepted plan-only contract, including representable
F32 YCbCr buffers. Reader-specific Unsupported conditions apply only when the
new reader is actually requested. Until execution wiring exists, any resulting
unreachable-private-code diagnostics are explicit WIP, not suppressed with new
allow/expect attributes or described as a warning-free checkpoint. A later
checkpoint review must distinguish tested private math from a wired public path.

`pixel(x, y)` returns a private value containing `Gray(f32)` or `Rgb([f32; 3])`,
plus separate `Option<f32>` alpha. Keep Gray as one channel for a later Gray ICC
route; neutral RGB expansion is not this stage. The reader owns no Vec, Box,
plane/layout clone, profile copy or full-image intermediate. Its lifetime also
covers borrowed option profiles; no borrowed storage escapes its checked plan.

Map descriptor roles to plane index and the matching channel offset once.
Address in sample units using checked `y*row_stride + x*pixel_stride + offset`,
not byte strides or assumed planar order. RGB/Gray/Y and alpha are full-size;
Cb/Cr have equal checked 444, 422 or 420 subsampling/dimensions. Other valid typed
layouts remain representable but this reader returns Unsupported, not a guessed
resampling. Alpha stays full resolution. Bounds errors use existing typed errors;
do not index out of range or use unsafe reads. Validation covers addressed samples
only, preserving the existing rule that unused padding is not image content.

### Exact integer range and matrix policy

The following are inverse reconstruction formulas, not an encoder round-trip
claim. They were checked against [H.273 (07/2024), section8.3 equations27-38,
45-47 and Table4](https://www.itu.int/rec/T-REC-H.273-202407-I/en).
For each channel's own meaningful precision `b`, define `M = 2^b - 1` and,
for limited range only, `s = 2^(b-8)`:

| Channel | Full range | Limited range |
| --- | --- | --- |
| Gray, Y, R, G, B | `code / M` | `(code - 16*s) / (219*s)` |
| Cb, Cr | `(code - 2^(b-1)) / M` | `(code - 128*s) / (224*s)` |
| Alpha | `code / M` | Not a color-range operation; use full-range alpha |

Convert to a signed/floating intermediate before subtracting. Full-range chroma
has asymmetric endpoints; its denominator is not `2^b`. Limited range requires
`b >= 8`; full range uses the already-validated precision within U8/U16 storage.
Required regression depths are8/10/12, not an invented restriction on other
valid full-range precisions. Color and alpha use their respective plane precision.
Do not clamp valid integer-code excursions to nominal legal-range endpoints.

For NCL reconstruction use `R = Y + 2*(1-Kr)*Cr`,
`B = Y + 2*(1-Kb)*Cb`, `G = (Y-Kr*R-Kb*B)/(1-Kr-Kb)`.
The supported `(Kr,Kb)` pairs are code1 `(0.2126,0.0722)`,
codes5/6 `(0.299,0.114)` and code9 `(0.2627,0.0593)`.
Compute with f64 intermediates and return f32. Identity reads explicit RGB roles:
stored GBR therefore yields `[Red, Green, Blue]`, with no color-difference offset.
Gray performs only scalar range expansion, irrespective of preserved nonidentity
matrix signaling. No constant-luminance/YCgCo/ICtCp approximation is introduced.
Finite reconstructed negative/over-one colors survive for the later stage's
domain check; alpha never chooses, clears, multiplies or divides color samples.

### Located chroma, F32 and alpha boundaries

H.273 section8.7/Table8 defines frame offsets in luma-centre units. Existing
validated location codes map as follows; AV1 translation remains in the resolver.

| H.273 code | x phase | y phase |
| --- | --- | --- |
| 0 | 0 | 0.5 |
| 1 | 0.5 | 0.5 |
| 2 | 0 | 0 |
| 3 | 0.5 | 0 |
| 4 | 0 | 1 |
| 5 | 0.5 | 1 |

For a subsampled axis with factor `f`, the chroma coordinate is
`q = (luma_index - phase)/f`; use f64, `i = floor(q)` and weight `q-i`.
Clamp neighbor indices `i` and `i+1` separately to the valid plane edge, then
interpolate horizontally and vertically. A full-resolution axis uses its direct
integer coordinate with no phase/filter. Thus 420 uses both phases, 422 uses
only x, and 444 uses neither. Chroma dimensions are ceiling divisions, including
odd and single-pixel edges. Applying the horizontal phase to 422 and using
separable bilinear/constant-edge extension are this implementation's documented
policy, not a claim that H.273 mandates that interpolation kernel. Progressive
typed frames only are addressed here; no field/interlaced resampling is added.

F32 supports the already-declared normalized encoded Gray/RGB and declared linear
Gray/RGB meanings only. Read those values unchanged, preserving their plan/domain;
do not reapply integer range expansion or infer float code units from precision32.
Addressed NaN/Inf and alpha outside `[0,1]` retain existing frame-validation errors.
Finite color excursions are not silently clipped. F32 YCbCr is explicitly
Unsupported in this reader because its float chroma centre/unit is not defined
by the current public contract; the public PixelBuffer type remains unchanged.
F32 alpha is unchanged; integer alpha is normalized independently as above.
Association remains the plan's accepted association; no unpremultiplication or
special hidden-color branch at zero alpha is permitted.

### Fixed six proof groups and review boundary

1. Role/address fixtures: planar RGB/GBR, interleaved reordered roles/offsets,
   row/pixel padding and nontrivial strides. Check exact coordinate selection,
   invalid coordinates, unchanged sample/metadata pointers and contents. Padding
   sentinels must never be mistaken for samples; no allocation follows construction.
2. Range fixtures:8/10/12-bit full/limited black, white, neutral, endpoints and
   excursions, including asymmetric full-range chroma. Alpha0/1/mid/max has
   distinct hidden colors and an independent meaningful-depth control.
3. Matrix fixtures: independent f64 fixed colored/neutral vectors for1,5/6,9
   and role-ordered identity; Gray ignores nonapplicable matrix math. Preserve
   negative/over-one results. Use the existing SDR absolute-error ceiling1e-5,
   without borrowing expected values from the production helper.
4. Chroma fixtures:5x3 odd420/422/444 ramps and impulses across all six phases,
   plus1x1/1xN edges. Assert source-plane coordinates/edge selection first, then
   numerical reconstruction. A different external filter is diagnostic only.
5. F32/alpha fixtures: normalized Gray/RGB and declared-linear passthrough,
   finite excursions, missing interpretation/nonfinite failure and explicit
   F32-YCbCr Unsupported. Prove no transfer/primary/CMS or alpha color processing.
6. Allocation/compatibility fixtures: observe zero heap requests during successful
   cold inspect and repeated pixel access, with no layout/profile/sample clones.
   Typed failures preserve source and produce no output owner; existing allocated
   error strings are not misreported as an allocation-free failure contract.
   Keep prior19+12 plan tests, highres-only/no-codec feature checks and applicable
   host/MSRV/i686/WASI/Miri checks. No new public API or decoder dependency is added.

The coding assignment stops at this private reader and its tracked proofs.
ICC/CICP transfer, physical-PCS/white bridges, new destination metadata, bounded
output ownership, quantization and full H4 acceptance require subsequent review.

### Post-checkpoint reverse ICC diagnostics and finite repair candidate

The clean ICC checkpoint `83f857a499983e3d5399b803fc7e5ca13f9cba16` remains
a finite historical checkpoint, not full H3 acceptance. Its original five pairs,
four intents and91,724 points still meet all20 recorded thresholds. Root's new
RGB17^3 reverse-pair runs expose additional unmet comparisons; process exit0 is
not a threshold pass. No product, profile original, version or checkpoint was
changed during this investigation.

The original new results remain preserved:

| Pair | Intent | Original median / p95 / maximum DeltaE00 | Result |
| --- | --- | --- | --- |
| Appearance -> sRGB2014 | all four | maximum .048446831 | PASS |
| sRGB2014 -> Appearance | P and S | .373303635 /1.309673164 /6.537849982 | FAIL |
| sRGB2014 -> Appearance | R and A | .006802322 /.615860652 /3.885155738 | FAIL |
| sRGB2014 -> Preference | P and S | .234020995 /.816858064 /3.114566495 | FAIL |
| sRGB2014 -> Preference | R and A | .008705059 /3.567973108 /15.886471466 | FAIL |

The frozen subject uses `clamp=true`; the original floating LCMS reference did
not bound its final device output. A separate control clips only that reference
RGB to[0,1] before the unchanged destination-to-physical-Lab measurement. It
does not change either CMS or the original evidence. With the same thresholds
(median<=.1,p95<=.25,maximum<=1), R and A now pass for both destinations:
Appearance .005541248 /.025492666 /.317001038 and Preference
.005085490 /.021330451 /.217386260. P and S remain outside the thresholds.

Selected-route observations are explicit: sRGB2014 is a v2 matrix/TRC source;
the two v4 destinations select B2A0 for P, fall back to that same B2A0 for S,
and select B2A1 for R/A. A synthetic identity-XYZ diagnostic bridge reproduces
the saved product device output exactly, isolating the destination stage.

[ICC.1:2022 sections6.2.4,6.2.5 and6.3.4.3/Table16](https://www.color.org/specification/ICC.1-2022-05.pdf)
distinguish perceptual reference-medium encoding from colorimetric PCS and
vendor-specific saturation rendering. The documented zero-black adjustment is
`adjusted[c] = xyz[c] * (1 - black[c]/white[c]) + black[c]`, with
`black=[.003357,.003479,.002869]` and `white=[.9642,1,.8249]`.
This preserves white and is not authorization for arbitrary BPC or tone mapping.
Neither all v2 LUTs nor designated saturation tags can simply be assumed to use
the same black convention.

The independent diagnostic applied those fixed published constants, without
fitting. In a diagnostic profile copy only, B2A1 points at the exact original
B2A0 byte range, so LCMS Relative evaluates that selected stage without the
original perceptual pair connection. Feeding the adjusted physical XYZ through
this stage reproduces the original LCMS P result within maximum DeltaE00
.08794035 (Appearance) and .10437183 (Preference). The copy is an isolation
instrument, not a new conforming colorimetric profile or a replacement fixture.

The residual product-versus-LCMS B2A0 difference has a separate explanation.
An independent f64 evaluator uses the recorded tag bytes, ICC curve/matrix
layouts and elementary four-vertex tetrahedral or eight-vertex trilinear
weights. It does not use third-party CMS implementation source. Across both
unadjusted and adjusted4913-point inputs:

| Destination | Maximum RGB difference: product vs independent tetrahedral | Maximum DeltaE00: LCMS vs independent trilinear |
| --- | --- | --- |
| Appearance | .000007908 | .099472 |
| Preference | .000001423 | .070007 |

Thus these observations do not establish a defective tetrahedral evaluator,
wrong PCS scale or wrong mBA stage order. The approved production3D tetrahedral
contract stays unchanged. The LCMS CLI help exposes no interpolation-selection
switch; no same-method public API configuration has been established in this
review. The matched trilinear calculation is diagnostic only, not a substitute
production algorithm or a newly declared full external-oracle pass.

Finite next repair candidate, requiring explicit assignment before coding:

1. Add one private, allocation-free selected-pair PCS connection plan beside
   existing Absolute connection handling. For the diagnosed v2 matrix/TRC ->
   v4 selected B2A0 path, apply the fixed reference-black affine in physical XYZ
   after source evaluation and before destination PCS encoding. Bind selection
   and version/model facts once; do not rediscover tags during execution.
   The presently demonstrated activation is RGB Display (`mntr`), source v2
   with actual matrix fallback and zero black, destination v4 with actual B2A0
   LUT selection. These are explicit conditions, not a claim about all accepted
   Input/Output/ColorSpace classes. Before widening this table, distinguish
   nonzero matrix black, v2 LUT conventions and each class's selected-tag meaning;
   profile version alone does not prove a zero-black convention.
   Within the recognized requested-P/S, v2-RGB-matrix -> v4-selected-B2A0
   connection family, a nonzero source black or an as-yet-unproved class pairing
   must produce explicit Unsupported during borrowed pair planning, not silently
   skip the bridge and return the previously mismatched color result. This
   bounded rejection does not extend to Gray, matrix-to-matrix, R/A or the old
   forward routes. Determine zero black from the selected matrix/TRC endpoints,
   not merely the class/version label; do not add a fitted epsilon to relabel it.
2. Cover requested P and the observed S-to-B2A0 fallback explicitly. Do not
   apply the rule to every S request, every v2 profile or both directions merely
   from version numbers. Preserve R/A, matrix-to-matrix, same-encoding LUT pairs
   and the previously accepted forward-pair behavior. Any wider activation table
   needs separately demonstrated source/destination black conventions.
3. Preserve standalone `CompiledProfile` physical-D50-XYZ semantics, selected
   route metadata, strict-domain behavior, no-BPC policy, worker allocation
   guarantees and exact owner accounting. Keep one pair-level affine seam;
   do not insert profile clones, extra stages or a second interpolation engine.
4. Track analytic black, white, neutral and colored XYZ expectations from the
   fixed affine, plus P/S-fallback activation and R/A/designated-S nonactivation.
   Add a curved synthetic LUT with fixed independent tetrahedral expectations,
   so the test cannot pass by silently replacing tetrahedral with trilinear.
   Include unchanged old forward five-pair results, matrix-to-matrix, Gray and
   Absolute controls, plus the reverse selected direction and explicit
   nonzero-black/unproved-class Unsupported within the recognized family. A
   wider policy is not accepted from the two real profiles alone.
5. Re-run prior54 boundaries, S4 real-owner7, execution limits, selected-route
   and strict-domain regressions, then both old and new frozen-profile recipes.
   Keep original false results and the new final-clip controls separate. Until
   a public-API/CLI same-method LCMS configuration is verified, use independent
   tetrahedral mathematics for this interpolation boundary and report the
   LCMS method-dependent residual as an open external comparison, not a relaxed
   threshold. No new CMS plugin/oracle implementation is authorized here.

Complete evidence is retained in ignored `.test-reverse-diagnosis-math-v2-83f857a`
with executable/profile/oracle hashes, source-before/after records, commands,
per-point traces and `source_unchanged=true`. The earlier math-only attempt
stopped on an initially unsupported diagnostic parametric curve and is not
counted as a completed run; the versioned successor handles the actual curves.
Source inspection confirms the ICC product remains clean at83f857a. This closes
the diagnosis, not the missing connection implementation or full H3 gate.

### Native C2-1 items1/2 frozen review: three finite remaining conditions

This is a review of the shared allocation engine and parser-to-decoder handoff
only. Coded/crop planes, persistent replacement adapters and all C2/C3 totals
remain later items. The reviewed allocation module SHA256 starts3810E3FD1080
and container budget SHA256 starts16177F18943E; root and reviewer hashes match.

The shared `replace_vec` engine now serves both ParseContext and DecodeBudget.
Legacy still uses its existing reserve branch. The parser explicitly drops
MetaState before retaining the projected owners, and strict Native decode keeps
the moved budget alive. Requested admission precedes allocation, actual admission
precedes element movement, and candidate error branches explicitly drop the
replacement before restoring scalar state. These source-level improvements do
not by themselves complete every proof below.

Independent results on this freeze:

- Existing boundary40PASS; count/stack target173PASS contains the previous170
  plus three included new product tests, not173 independent new cases.
- Header-plan43, prefix-grammar62 and the existing OBU/Native-hookup targets
  pass. Included legacy/product tests are not summed as unique new evidence.
- New `c2_handoff_boundary`3PASS: both real adapters enforce
  other8+old8+candidate16 at exact32/one-under31, reject a real16-byte allocator
  denial with one matching request, preserve old pointer/content/capacity/ticket
  and checkpoint, then retry successfully on the same ledger. The positive
  setter and rejected zero preserve the existing twelve-argument constructor.
- A synthetic real parser fixture with257-byte ICC and513-byte item payloads
  compares actual retained heap allocations with the returned accounting.
  Primary-only live1494/observed peak2150 and primary-plus-alpha live2106/peak2998
  match their retained owner sums. Three ICC projections total771 and are a
  metadata subset, not an additional aggregate charge. Source bytes are unchanged.
  These measured host figures are diagnostics, not portable hardcoded ceilings.
- Root separately reports Rust1.88 lib523PASS/6ignored and Linux-target Miri
  for the three product handoff tests. Those do not replace the missing tests.

Limited decision: shared-engine extraction and observed retained quantities are
sound on these fixtures, but items1/2 are not yet accepted as complete. Keep the
following single, finite repair bundle; do not reopen accepted C1 behavior:

1. Preserve aggregate history. DecodeBudget currently stores only separate
   metadata/payload peaks. Add the actual simultaneous aggregate peak to the
   shared checkpoint/restore state; update it after successful admission and
   preserve it through handoff. ICC is a metadata subset. Do not reconstruct
   aggregate peak by adding class peaks from different times. Track disjoint
   class peaks, simultaneous old-plus-candidate peak, failed-admission rollback
   and successful retained-owner handoff with one fixed event sequence.
2. Transfer retained owner authority, not only reconstructed totals. Currently
   `into_budget(metadata,payload,icc)` assigns scalar live counts and returns no
   per-owner tickets. Carry move-only tickets for the actual retained rich,
   ordered-property and primary/alpha payload owners through a private handoff
   value, preserving the original accounting authority. A nonallocating owner
   walker may validate capacity/class/identity totals, but is not a new fresh
   charge. Dropped parser owners must be released after destruction. Any private
   variable-length ticket storage must itself be fallibly allocated and charged;
   do not introduce an uncharged registry or a managed public frame. Track a
   real parser result, ICC subset and primary/alpha pointers, then consume/move
   retained tickets into the same ledger without double adoption. Keep the
   DecodedFrame color clone and plane work explicitly for their subsequent items.
3. Complete the real candidate proof through this same production engine. Add
   one narrow candidate-maker seam with a normal `try_reserve_exact` wrapper,
   not another replacement algorithm. Reject nonempty or too-short candidates
   before append can allocate implicitly. With other8+old8, requested16 and
   ceiling40, inject actual32: it fits alone but48 does not. Observe actual
   deallocation before ledger restoration, unchanged old owner/ticket/source and
   full checkpoint, then actual16 retry on the same state. Keep fresh/adopted,
   stale/wrong-class, spare-capacity, checked arithmetic and real allocator
   denial controls; transplant the real parser/allocator assertions into tracked
   separate tests. Current tracked3 do not cover this excess-capacity condition.

The final `release(old_bytes)` after publication is not currently shown to be a
runtime defect for valid ledger/token state: old bytes were already charged or
explicitly adopted, actual new capacity was charged in the same class, and
release subtracts only the still-accounted old amount. ICC must remain a subset
of metadata. Thus the two present adapters cannot underflow on this valid path.
Document that invariant on the ledger commit boundary and retain an actual
replacement test for it; arbitrary fallible third-party ledger implementations
are not covered by that argument. Any future adapter that can reject commit
release must validate it before publication or provide an infallible admitted
commit, rather than returning an error after changing the old owner.

The new ceiling's documentation should distinguish covered parser temporary and
retained owners from still-unwired decoder owners. Neither these successful
checks nor the current parked budget extends the public total-live guarantee.

### H4 reader WIP: root consumer-isolation supplement

Root freshly ran the independent consumer matrix on the current dirty snapshot:
all seven configurations passed check and execution, and all five dependency-tree
assertions passed, including no ICC with highres disabled and no AVIF with
highres alone. This confirms compilation and dependency isolation, not native
pixel correctness or H4 completion. The intentionally unconnected private reader
currently adds14 dead-code WIP warnings; the two no-default draw warnings remain
the separate baseline. No warning-free gate is claimed. The author's six fixed
reader proof groups are still being completed; final independent acceptance
awaits a frozen implementation and tests. C2's three recorded repair conditions
remain open, without additional exploration here.

On this reader-WIP/C2-frozen snapshot, root also compared the normal legacy
alpha-AVIS callback output with highres off/on using the existing animated alpha
Exif/XMP fixture:995,183 bytes were identical (SHA256
`F5BB0557B86BA3A1BDE3E2B061535D29442679680BFD6F89F2811E7C26601166`).
The initial ignored `.test-native-reader-legacy-20260828` record covered normal
execution only. Root subsequently reran metadata/init/next/draw Abort snapshots:
all four were byte-identical with highres off/on. Their lengths were respectively
995,183/543,811/544,039/634,078 bytes, preserved in the corresponding ignored
`-metadata`, `-init`, `-next` and `-draw` records. Metadata output equals the normal
snapshot, so this proves unchanged behavior, not effective metadata abortion.

### Reverse reference-black frozen runtime: independent finite acceptance

The six-file ICC repair was reviewed against the preceding finite family, without
changing the tetrahedral interpolation contract or the standalone compiled-profile
physical-PCS contract. The source route must actually be RGB matrix/TRC, v2; the
destination must actually select v4 B2A0; the requested intent must be Perceptual
or Saturation. Both classes must be monitor, and the selected borrowed curves and
matrix must evaluate device zero to exactly physical XYZ zero. A recognized family
with nonzero black or an unproved class returns Unsupported before materialization.
Other versions/models, designated B2A2, Relative/Absolute, Gray and the old forward
routes are not indiscriminately remapped. Shared parametric evaluation is reused
for functions0-4; the affine is applied once at the physical-XYZ pair seam.

Independent `reverse_bridge_boundary` passes5 tests: analytic black/white/neutral/
colour and unchanged standalone PCS; selected version/model/intent activation;
curved2-cube CLUT with tetrahedral min(x,y)/min(y,z)/min(z,x), distinguished from
trilinear products; actual allocation-zero Unsupported checks; and parametric0-4
zero/nonzero endpoint controls, including the selected lower branch. The existing
tracked reverse3 also pass. The old54 independent boundaries all pass, separately
filtered as7+7+22+3+7+4+4. Product S4's real-allocation/drop/retry7 pass using its
own allocator. E2 public46 (old40+6) and private7 pass. These overlapping suites
are not summed into a new unique-test count.

The private include harnesses needed only root Transform/error aliases for the
new tracked module. An initial unfiltered include run failed two product S4
allocator observations because that harness uses a different global observer;
the unchanged independent filters and the actual product allocator runs above
are the relevant separate evidence, not a claim that those include failures passed.
No runtime defect was reproduced in this finite repair. Checkpoint acceptance
still awaits transplanting the five proof groups into tracked tests and focused
edition2021 formatting: the initial six-file formatting check failed style/import
layout. Library Clippy emitted no new transform diagnostic, but whole-library
Clippy still failed with the separate legacy208 warnings/one error; no blanket
lint pass is claimed.

Root's frozen repair recipe has source-before/after equality and preserves all20
old five-pair/four-intent thresholds over91,724 points (worst maximum0.0633450190).
The new Appearance reverse P/S comparison improves to median0.1550472309,
p950.7583151242 and maximum2.5539668013, but still FAILS the unchanged gate.
Its original unbounded-reference R/A comparison remains a failure; the earlier
same-final-clip control remains distinct. Preference has class spac and now
returns the explicitly designed Unsupported, not a numerical pass. Preserve the
original failed artifacts and the separate interpolation/final-clip diagnosis.
Root additionally reports the initial tracked reverse3 passing on i686 Rust1.91.

### H4 private native reader: six-group finite acceptance

The final runtime SHA256 is
`A225DB8C4FAE669CE84EBBB397AFEDB9D49413E873051AA7071320C3488EE87C`;
the numeric and allocator test hashes are respectively
`DA2E306AA219E7D6A88E0D562480BDF9DB12C53DA8703CB37F2882BB9C1E5C0E` and
`56D1FA02818AC8EEF49B311C8E8AFED847DAE0DCC7523DCED5DA99DA00EDFC64`.
Independent review confirms all three hashes and reruns the ordinary-cfg reader
against an independent global allocator, without substituting tracked-test hooks.

All six fixed groups pass: role/offset/stride/GBR and distinct hidden colours;
8/10/12 full/limited expansion with independent alpha endpoints; NCL codes1/5/6/9
against independent f64 arithmetic including excursions; 420/422/444 at5x3,1x1
and1x5 with all six chroma phases against an independent tent-kernel calculation;
normalized encoded and declared-linear F32 Gray/RGB with a nonvacuous private
F32-YCbCr rejection; and real heap observations. The allocation positive control
allocates4096 bytes, while cold inspection and20 repeated full-pixel traversals
for Gray/RGB/YCbCr allocate zero bytes. Source contents and padding remain unchanged.
Gray stays one colour channel and alpha is separate/full-range, not colour-scaled.

Product numeric9 plus allocator3 pass independently; focused edition2024 formatting
passes. Sample extraction is shared through sample_value; interpolation and matrix
math stay f64 until the final f32 value. The final limited-range scale uses checked
integer shifting. Root's earlier Miri black-endpoint tiny residual was fixed by
that exact scale construction, without relaxing epsilon/clamp/test expectations.
Root reran all12 on i686 Rust1.91, Linux-target Miri and actual Node/WASI successfully
at this final snapshot. This is internal reader acceptance only: no public
conversion/output/CMS hookup, no F32-YCbCr convention, no whole-H4 completion and
no warning-free checkpoint are implied. The14 unconnected-reader diagnostics are
explicit WIP and are not suppressed with new allow/expect attributes.

### Reverse reference-black tracked supplement: six-file stage candidate

The author's final tracked5 now cover the same finite independent observations:
standalone physical XYZ and neutral0.25 as well as pair black/white/colour,
version/intent/model activation, zero-allocation class/nonzero rejection, curved
tetrahedral interpolation and parametric0-4 endpoints. Independent readback and
execution confirm5/5; the independent five-test runner also passes. Focused
edition2021 formatting of all six changed files and diff checking pass. The final
supplement changes tests/style only. New transform Clippy diagnostics remain zero;
the three unchanged transform/tests.rs warnings and whole-crate legacy failures
remain separate. The six files are compile.rs, compile_plan.rs, curve.rs,
curve_plan.rs, route_plan.rs and reverse_diagnostics_tests.rs. They are a finite
stage candidate, not full reverse-oracle or H3 acceptance.

Final exact staged review confirms only those six files,614 insertions/5 deletions,
no unstaged product diff, and a clean cached diff check; no Cargo/version/legacy
changes are included. Root separately reran the completed tracked5 on Rust1.91
i686 and Linux-target Miri, all passing. This staged checkpoint has finite GO;
the earlier interpolation/final-clip/Unsupported qualifications remain unchanged.

Root saved that exact six-file checkpoint as
`f5397b69f05b21faecd554508f13b3f59d4838e9` and confirmed a clean ICC worktree.
Version0.0.4 and the parent's public ICC pin are unchanged; no push/publication
was performed. The saved commit does not close the remaining reverse oracle gates.

### H4 next candidate: two-step public relative-linear conversion hookup

This is a finite implementation design following the accepted private reader,
not an implementation or acceptance claim. The first step mechanically extracts
shared construction ownership; the second connects actual relative-light colour
conversion. Keep the public ICC dependency at c96f6e3 and version0.0.4 unchanged.
No ignored local CMS patch may silently become a public dependency replacement.

1. **Codec-free construction seam, then independent regression.** Move the existing
   ConstructionLedger/fresh/replacement/candidate/failpoint engine out of
   highres/avif into a private highres allocation module. Keep an AVIF re-export
   or import adapter as needed, one implementation and unchanged B/C observations.
   Move the generic ledger-backed ColorInformationSet copier to a codec-free
   ownership helper; retain codec-specific RichAvifInfo mapping in avif. Do not
   make this extraction depend on the unfinished nested decoder C2 ledger. Expose
   the existing private ImageFrame::from_parts to highres without the AVIF gate
   when it gains this real caller. Before public hookup, rerun existing AVIF
   mapping/candidate/ICC ownership tests and highres-only/default-off consumers.
   Any transaction-order improvement required by the new owner tests must be
   separately identified, not hidden as a mechanical move.

2. **One borrowed plan and a small executable subset.** Add the already approved
   public convert_frame(source, options, limits) returning ImageFrame or
   ProcessingError. NativePixelReader owns the single inspected ConversionPlan;
   expose private borrowed/copy access to that plan instead of calling inspect
   again. Compile a fixed-size RelativeColorPlan from its actual selected source
   and destination. Accept CICP TC1/6/8/13/14/15 or already-declared relative-linear
   input, to LinearRelative RGB with D65 primaries from709/2020/P3-D65. Source Gray
   remains scalar through inverse transfer and is then replicated to neutral RGB;
   it is not passed to an RGB ICC profile. Read through the accepted native reader,
   preserving its role/range/chroma/NCL semantics. Work in fixed scalar/pixel or
   bounded row storage, not a full intermediate RGB frame. Validate the actual
   chosen primary matrix as finite/nonsingular before output allocation; use a
   shared checked f64 matrix helper and same-D65 RGB-to-XYZ-to-RGB composition.
   Equal primaries may use identity. Arbitrary primaries, Bradford/different-white,
   HDR/PQ/HLG, encoded destinations, ICC destinations, quantization and association
   changes remain explicit Unsupported in this execution subset, not no-ops.

3. **Transfer and alpha are explicit.** Follow
   [H.273 (07/2024), TransferCharacteristics and ColourPrimaries](https://www.itu.int/rec/T-REC-H.273-202407-I/en).
   TC1/6/14/15 share alpha=1.099296826809442..., beta=0.018053968510807...;
   invert using V/4.5 below4.5*beta and ((V+alpha-1)/alpha)^(1/0.45)
   above it. TC13 uses the specified continuous value/slope junction with
   exponent1/2.4 and slope12.92, not an unrelated rounded gamma shortcut.
   Derived constants are alpha=1.0550107189475866, beta=0.0030412825601275183
   and encoded junction0.03929337067684754. TC13 matrix0 has normalized bounds;
   its nonzero-matrix branch is the signed extension. Select that distinction
   from the selected CICP interpretation, without changing native reconstruction.
   TC1/6/14/15 and encoded TC8 reject V outside[0,1] without epsilon or clipping;
   TC13 matrix0 likewise uses[0,1]. TC13 nonzero-matrix input may retain finite
   excursions through the signed inverse. Already-linear relative input, including
   an explicit TC8 interpretation, preserves its declared units and finite
   excursions. Reject nonfinite arithmetic or an unrepresentable f32 result.
   No display EOTF/BT.1886 or assumed SDR-white-nits operation is substituted.
   Preserve straight alpha independently and transform hidden colour at alpha0
   normally. Already-linear homogeneous primary conversion may preserve premultiplied
   association without division; encoded nonlinear premultiplication remains rejected.

4. **Output ownership and honest metadata.** Return full-resolution planar F32
   Red/Green/Blue plus Alpha when present, meaningful_bits32, LinearRelative and
   the chosen destination primaries. Its active colour set contains no stale source
   ICC/nclx/AV1 description. Preserve original FrameMetadata ICC type/bytes, nclx,
   AV1, unknown colour payloads, ordered geometry, pixi, sequence/timing and source
   dimensions; do not apply geometry or pretend retained pixi describes output.
   Use ledger-backed fallible copies, not ImageFrame::clone or infallible layout/
   metadata builders. Add the previously approved checked last-conversion record
   with private fields/accessors: selected source/override, destination/domain,
   intent, native interpretation, white policy and preserved alpha policy; no
   quantization/tone-map action is recorded as performed. Preserve Eq/API meanings.
   A second conversion must consult the new active domain/primaries and record,
   never reapply preserved source metadata. Active ICC is still authoritative:
   this first executable slice returns Unsupported for selected ICC, even beside
   usable CICP. Only an explicit CICP override can choose that other interpretation;
   retain and record the original ICC rather than deleting it.

5. **All covered owners, both final and construction peak.** Preflight checked
   width*height*4, channel/plane counts and all requested output ownership before
   any candidate allocation. Include source descriptor/pixels/metadata capacities,
   nonaliased option-profile ranges, destination sample vectors, Plane/PlaneDescriptor
   outer vectors, each role/offset allocation, copied metadata/provenance/ICC and
   conversion record storage. Distinguish output-final frame/metadata/ICC budgets
   from source+output live ownership. Keep future planned owners reserved while
   reconciling every actual capacity, including per-plane and per-ICC ceilings,
   before filling/copying that owner. Use one shared candidate engine: no unchecked
   resize, Vec clone, implicit append growth or full-image scratch. Drop all partial
   outputs before rollback/error; return no partial ImageFrame and leave source
   pointers/contents/timing unchanged. Exact/one-under tests use portable size_of
   inventory, and real candidate overcapacity/allocator failure tests observe drop
   and same-state retry rather than just a final error variant.

   First executable metadata scope requires FrameMetadata.tags().capacity()==0.
   A nonempty map, or an empty but allocated map after clear, is Unsupported at
   convert_frame before output candidates: HashMap capacity does not expose all
   bucket/control allocation bytes. Do not misstate capacity*entry-size as a proven
   total, delete tags, or use an uncharged clone. This restriction is not added to
   the general frame API or plan-only validate_for. A checked bounded HashMap/Exif
   preservation extension remains a later task. All named original colour/geometry/
   pixi/timing owners above remain mandatory in this first subset.

6. **Order and fixed acceptance.** Borrowed frame/metadata validation, route and
   subset checks, primary-matrix validation, complete requested ownership admission,
   actual-capacity reconciliation, then pixel execution and final validation.
   Known Unsupported conditions make zero output/layout/metadata/CMS candidates;
   existing ProcessingError String diagnostics may allocate and are not falsely
   called total-heap-zero failures. Late per-pixel domain errors destroy candidates.
   Track branch neighbours/endpoints and1-ULP outside domains, Gray neutral, sYCC
   negative/over-one controls, all three same-D65 primary pairs and inverse roundtrips,
   singular rejection, the previous native six groups, straight alpha0/1/mid/max
   with distinct hidden colours, double conversion/active-source separation, and
   full preserved metadata. Require relative-light numeric error<=1e-5 against
   independent f64 formulas; retain the existing native reconstruction tolerance.
   Observe exact/under requested admission, extra actual capacity, real allocation
   failure at each owner, no leak/no partial result/source immutability and retry.
   Keep old19 independent/12 tracked planning checks, highres-only without codecs,
   default-off/callback compatibility, applicable MSRV/i686/WASI/Miri and new-file
   lint checks. Successful public use should remove the reader's dead-code warnings
   through real calls, not allow/expect suppression. This does not close full H4,
   ICC integration, HDR, quantization, generic tag preservation or decoder C2.

### C2 fixed-three repair: independent proof preparation, not acceptance

While the author's C2 repair is still changing, the independent harness now has
a real-parser primary/alpha/ICC aggregate-live/peak comparison and three shared
candidate checks. The latter delegate to both real adapters, observe actual
candidate deallocation at the restore call, and require unchanged old owner/token
plus same-ledger retry; they cover actual32 with old8+other8 under ceiling40,
short capacity and nonempty candidate rejection. Existing parser and C1 assertions
are unchanged. Compilation reached an in-progress non-Copy ticket migration error
in the product test accessor, so these new checks have no execution verdict yet.
Actual retained-owner authority transfer, rather than renamed scalar categories,
remains the second fixed condition. Await the declared freeze; no plane/entropy,
new C1 or other decoder scope is added by this preparation.

### C2 fixed-three follow-up: items1/3 runtime evidence and item2 move seams

The subsequently frozen allocation module SHA256 startsB27FFCB47CB0 and budget
module starts64E9C21F78C6. Independent candidate3/3 and real-parser aggregate1/1
pass. The history target filters137 included tests; this is one new independent
history test, not138 new tests. Both real adapters reject actual32 with old8 and
other8 under ceiling40, deallocate the candidate before restore, preserve the old
pointer/content/capacity/ticket and complete checkpoint, and retry on that same
ledger. Short and nonempty candidates are rejected before append can allocate.
The parser fixture compares live and simultaneous peak with real allocations for
primary-only and primary-plus-alpha, including ICC as a metadata subset.

This supports the finite runtime boundaries of items1/3, not complete C2 or item2.
Root separately reports Rust1.88 lib527PASS/6ignored and Linux-target Miri for
the seven tracked handoff tests. The tracked overcapacity test currently observes
the error and unchanged state, not actual deallocation at restore; it covers only
DecodeBudget. Its candidate-shape test does not include a short empty candidate.
Transplant the existing independent observer assertions into relative test-only
helpers rather than treating those weaker tests as equivalent. The tracked
aggregate event test also releases metadata while its Vec is still alive; drop
that owner before release so the recorded event sequence describes actual owners.

Item2 remains a genuine missing implementation: the current six-category handoff
constructs new AllocationTokens and assigns completed capacity totals to them.
Removing Copy does not make those values the reservation-time authority. The
following private move seams concretize the already approved condition; they are
not an instruction to redesign the public parser or to start plane/entropy work.

1. **Allocation leaves retain their actual ticket.** The local payload token in
   container_native_plan::native_direct_item_payload currently disappears when
   only Vec is returned. Return a private value-plus-owner bundle from this core.
   Likewise, copy_bytes_with_class, copy_string_with_context and clone_pixi must
   use the existing with-token reserve engine and return/move its original token:
   one for a Vec/String, up to two fixed slots for pixi. String::from_utf8 moves
   the same Vec allocation and ticket. Use a common allocation core with Legacy
   compatibility wrappers; no copied reserve algorithm or new Legacy allocation.

2. **The Native projected-info return carries its sidecar.** Add private owners
   alongside ParsedAvif.info, leaving public AvifInfo unchanged. Carry the ftyp
   compatible-brands ticket, PrimaryItemMetadata's fixed pixi/color/av1C tickets,
   primary payload ticket, and alpha owners through their existing private return
   chain into this field. Alpha assembly retains the outer auxiliary Vec ticket
   and each auxiliary String/payload ticket. Its temporary item_ids Vec is dropped
   by IntoIter before its existing ticket is released; moved String tickets remain
   with AuxiliaryImage, not with that temporary outer allocation. Keep Native's
   existing selected-item checks and Legacy's existing ordering and duplicate rule.

3. **Rich and ordered projections return their own original tickets.** Make the
   private collect_color_information_with_context result carry ICC, unknown-colr
   outer Vec and each unknown payload ticket. Make primary_property_records_with_context
   carry the records outer Vec plus actual nested record owners; the individual
   property-record helper returns fixed zero/one/two-owner parts. The current local
   unknown_token survives growth but is lost on return; move it instead. Preserve
   last-ICC selection and the existing nclx-preferred legacy projection. Replacing
   an already-owned ICC/pixi/config creates and admits the new candidate first,
   then destroys the replaced value, releases its ticket and retains the new one.

4. **Parser-only destruction releases real authority.** MetaState already owns
   eight outer collection tokens and association records own association tokens.
   Keep those. Its nested iloc extents/indexes, reference targets, alternate IDs,
   item-name strings and property pixi/color/av1C/auxiliary strings need their
   reservation tokens carried by the corresponding private parser result into a
   Native-only temporary-owner sidecar. Once rich and ordered projection is done,
   detach the token bundle, actually drop MetaState and every nested value, then
   release those tokens. Do not lower live counts before destruction or rebuild
   authority from a post-drop capacity walk. Error exits drop candidates/values
   before the enclosing checkpoint is restored; avoid duplicate release/restore.
   Reuse the existing parse_iinf/iloc/iref/grpl_owned_with_context return seams:
   these already return original outer tokens and need their nested sidecars,
   not a second parser. Their release_item_infos/locations/references and
   release_alternate_entity_groups helpers currently debit computed capacities
   before clear; clear also leaves the outer allocation alive. On the Native
   path take the replaced value and original tickets, destroy the entire old
   value, then release. Keep merge_ipma's existing move of association tickets
   and release after the consumed incoming Vec is destroyed.

5. **Variable owner storage is necessary and budgeted.** Unknown colour payloads,
   ordered property records and alpha owners have variable counts under existing
   limits. Six aggregate categories cannot represent their independent owners.
   Fixed local slots suffice for single/pixi/primary-metadata parts; a Native-only
   ticket Vec may hold variable parts. Its backing capacity must itself use the
   same fallible engine and Metadata charge, with one separate backing token rather
   than recursively registering itself. Reserve slots before moving tickets;
   growth admits old plus whole candidate and preserves existing tickets on failure.
   This does not authorize an uncharged registry or new public managed-frame type.

6. **Handoff consumes, audits and moves; it does not re-charge.** At
   parse_rich_info_with_limits_and_budget, move the ParsedAvif, rich-colour and
   ordered-property owner bundles into RetainedOwnerHandoff and then DecodeBudget.
   The same ledger already contains their charges after temporary owners were
   released. Remove scalar live reseeding from this path. Capacity/class/pointer
   walkers may audit the one-to-one owner correspondence but must never mint
   tickets. A real ICC allocation has one Icc ticket, which contributes to both
   Metadata and its ICC subset; do not invent an IccSubset allocation. Keep public
   retained-metadata reporting distinct from private ticket-storage overhead, but
   include both in actual live/peak budget inventory. The existing public wrapper
   may discard private accounting only when ownership leaves the bounded internal
   operation. Strict Native decode keeps the private budget/bundle alive.

Acceptance remains the same real-parser primary/alpha/ICC pointer and capacity
fixture, plus move-only ticket identities, destruction/release order, no duplicate
adoption, ticket-storage requested/actual capacity and same-state failure/retry.
The previous aggregate/history assertions must include any newly real bookkeeping
allocations instead of relying on the old host-specific totals. Main reads this
design before author implementation resumes; items1/3 are frozen separately.

The next test-only follow-up adds both real adapters, short/nonempty candidates
and the missing drop(metadata) before release; the author reports eight tracked
tests passing. Independent source review still finds no actual deallocation
observer or restore-time assertion in that tracked helper. Result/error, pointer,
checkpoint and retry checks alone do not prove this ordering. Keep the single
remaining tracked-proof condition open until the already-passing external
candidate observer is transplanted without weakening its observation. This is
not a newly demonstrated runtime defect; item2 also remains implementation-pending.

The final observer follow-up now preserves the missing observation. A cfg(test)
System allocator records only the tracked candidate's pointer and allocation size;
the ledger wrapper asserts one real deallocation before delegating restore to each
actual adapter. The 32/4/16-capacity controls assert one request, one destruction,
no realloc, unchanged old/other owners and tickets, complete checkpoint equality
and successful same-ledger retry. The observer never dereferences freed memory or
reads a mutably borrowed ledger through another alias. The aggregate event test
now destroys its metadata Vec before release.

Independent final execution: product handoff9PASS, candidate3PASS, real-parser
history1PASS and the previous handoff3PASS. The history command filters139 included
tests, not139 new successes. Its harness adds only a compile-time helper alias for
the included product tests; the product observer proof is executed separately
with its actual global allocator. Actual parser live/peak remain1494/2150 for
primary-only and2106/2998 with alpha; ICC771 is still a metadata subset. Allocation
and budget source hashes remainB27FFCB47CB0 and64E9C21F78C6. The new tracked-test
hash startsFBAE7D54CF78 and observer hash starts16D69307EA96.

The runtime/observer boundary for items1/3 is accepted. Strict Clippy lib+tests
with warnings denied passes. The initially detected lib.rs test-module ordering
difference was corrected without changing either observer/test hash; final
focused formatting and diff checks pass. Root additionally reports nightly
Linux-target Miri handoff9/9PASS. Main has read the six move seams and authorized
item2 implementation. Actual reservation-ticket handoff is still pending; no
public total-live, plane/entropy or full C2 acceptance is implied by these results.

### ICC saved-checkpoint package supplement

Root reports that clean f5397b69 passes offline locked cargo package listing,
archive creation and extracted-library verification:64 files,549.3KiB unpacked
and102.1KiB compressed. The archive includes reverse_diagnostics_tests and excludes
ignored work artifacts/external samples. Existing exclude settings skip three
examples and emit packaging warnings; a host canonicalization warning is separate.
This is package-build evidence for the finite saved checkpoint, not publication,
a version/pin update or completion of all release/oracle gates.

Root also reports clean f5397b69 Rust1.91 wasm32-wasip1 build and actual Node WASI
execution: lib45, compile-limits4, E1-route4, parse2, execution5, LUT-shape5, LUT35
and transform11 pass,111 total; the explicit external-profile LUT fixture remains
ignored. The transform thread-sharing test aborts because this WASI runtime lacks
std::thread::spawn support. The eleven remaining transform cases were rerun with
only that test excluded. This is not an all-WASI-suite pass; the same thread-sharing
case passes a fresh native Rust1.91 run1/1. Native parallel execution and the WASI
runtime limitation remain distinct.

### H4 public execution: independent preparation only

Ignored execution_math selfchecks2PASS cover the approved exact H.273 junctions,
value/slope continuity, signed controls, and independently solved D65 primary
white/roundtrip/singular controls. They do not execute the product converter.
The separate public execution harness now prepares six fixed groups for transfer
and domains, primary pairs, Gray/alpha/hidden colour, original/active metadata and
double conversion, pre-output Unsupported/tag controls, and real allocator
failure/source-preserving retry. Its included old planning tests and math
selfchecks are not new execution successes. Product code is still being wired;
await stable conversion-record symbols and the declared freeze before acceptance.
Full per-owner requested/actual/peak proof will reuse the existing ownership
boundary seams rather than substituting the sample-allocation check alone.

### ICC reverse oracle: CLI and public-API uint16 diagnostics

Clean f5397b69 was used for two separate finite investigations of the same
sRGB2014-to-Appearance pair. Existing float-oracle artifacts and FAIL results
are retained. Neither diagnostic changes product interpolation, thresholds,
profile bytes or the previously accepted five-pair/20-case scope.

The CLI investigation calibrates `-e -w` as uint16 **output** while RGB input
remains displayed0..255. It compares float/encoded output with `-s` separately
off/on, `-c0`, all four intents and17-cubed4913 points. P/S encoded output is
exactly the rounded/clipped printed float result for all14739 channels, with
or without `-s`; maximum DeltaE00 remains2.5539668013FAIL. Independent trilinear
maximum RGB difference is0.0004716746, versus tetrahedral0.1657758351; product
versus independent tetrahedral is0.0000075167. R/A encoded final-clip diagnostics
pass with maximum0.3961801700, but do not replace the original unbounded float
FAIL. Thus encoded CLI output is not evidence of a matched-tetrahedral path.
Artifacts are retained as `.test-u16-small-f5397b69` and
`.test-u16-grid-f5397b69` with profile, executable, source and setting provenance.

The follow-up links only the existing LCMS import library/DLL and reads only
public `lcms2.h` declarations. It calls `cmsCreateTransform` with TYPE_RGB_16
on both sides and NOOPTIMIZE alone, with no BPC. A separate NULLTRANSFORM control
preserves nine calibration words exactly; that flag is absent from comparisons.
Input words are round(grid*65535), with the same words dequantized to product
f32. Maximum difference from the ideal grid is0.0000076294. Integer output is
full-range0..65535 and inherently bounded; product clamp=true is explicit.
The DLL public version function returns2190, separately recorded from its
package-directory label. No implementation source is consulted.

This API path also does not match tetrahedral on the observed B2A0 pair:
P/S independent trilinear maximum RGB difference is0.0004158916, versus
tetrahedral0.1657822011; product-versus-tetrahedral is0.0000069946. Over4913
points per intent, P/S DeltaE00 median/p95/max are0.1550955943/0.7583151242/
2.5539668013FAIL. R/A are0.0055730347/0.0253324348/0.3961801700PASS only for
this bounded uint16 diagnostic. This numerical separation supports the method
distinction for this pair, not a general claim about all LCMS pipeline choices.

The public-API artifacts are `.test-api-u16-small-f5397b69` and
`.test-api-u16-grid-f5397b69-final`; the intervening grid run is retained too.
All43 ICC source/Cargo fingerprints remain unchanged and the checkout is clean;
profile/header/import-library/DLL/metric/helper hashes and commands are retained.
The final grid repeat also has identical pre/post binary SHA256 beginningBC1351FC.
Both diagnostics use the existing same-destination physical-Lab measurement and
the34-vector Sharma selfcheck, maximum error0.0000494990. The old float reverse
gate is still open. No matched-method oracle or full-H3 completion is claimed.

### H4 step2 candidate: fixed execution review and bounded repair bundle

The public execution candidate is **not accepted**. Independent public groups
initially run5PASS/1FAIL: TC13 nonzero-matrix input-0.5 produces-0.03869969 instead
of the approved signed inverse-0.2140458425. The author's abs/sign branch repair
closes that failure; the same six groups now run6PASS. Their21 filtered cases
are the included old19 route checks and2 independent-math selfchecks, not new
execution successes. The old route harness separately runs19PASS; the product's
six convert_frame-named tests also pass. This does not establish complete owner
admission, actual-capacity or failure-order guarantees.

The six additional subcases of the same approved numeric/ownership groups run
0PASS/6FAIL in execution_owner_boundary. They establish these finite defects:

- TC1/6/8/14/15 with matrix1 accept encoded-0.1 and1.1. Only TC13's nonzero-matrix
  branch has the approved signed extension. Bounds cannot be selected solely by
  whether matrix coefficients equal zero.
- A1024-byte U8 input plane fits max_plane_bytes2048, yet conversion returns
  three4096-byte F32 output planes and makes all three sample candidates.
  Gray-to-RGB similarly succeeds with max_channels1 or max_planes1. Output counts
  and per-plane ceilings are missing from preflight.
- Both source and independently constructed output pass plan validation with
  max_frame_bytes12852, but conversion rejects that same limit. The measured
  source/output heap owners are3636/12844; the existing resource convention adds
  retained nclx amounts16/8 respectively. The output formula additionally charges
  whole stack headers and embedded layouts, and the ledger seeds borrowed source
  ownership into the output-frame budget rather than live-only ownership.
- A live ceiling one byte below the known total source/output requirement makes
  two4096-byte sample allocations before rejecting the third. All remaining
  requested owners have not been reserved before the first candidate.
- Denying the actual4093-byte retained ICC copy produces exactly one allocation
  request and leaves the source pointer/content unchanged, but reports
  Invalid(InvalidMetadata), not the typed Allocation error. The new metadata copier
  bypasses the already extracted ledger-backed colour copier.

The first final-frame test draft omitted the existing retained-nclx budget
amounts and failed its output precondition; only the corrected case above is a
conversion-defect observation. An old native-reader include harness also needed
the mechanical allocation-module import after step1; its initial compile failure
was harness wiring, not a product failure. The latest numeric-repaired execution
source hash starts74AF582A; metadata08AE1EA4, allocationBF693EFE and
ownershipF59E8ACD identify the reviewed ownership snapshot.

Close this existing bundle without extending conversion scope:

1. **Finish the existing transfer predicate.** TC1/6/14/15 and encoded TC8 always
   require[0,1]; only TC13/nonzero-matrix may use signed finite excursions. Keep
   the corrected TC13 abs/sign branch and common TC1/6/14/15 formula. Preserve
   declared LinearRelative plus explicit TC8 excursions. Track negative/over-one
   controls for both matrix0/nonzero, not just the original positive junctions.

2. **Plan and admit every covered owner through the existing engine.** A private
   fixed OutputOwnershipPlan contains at most four sample owners, descriptor and
   pixel-plane outer owners, each layout offset/role owner, and borrowed metadata
   copy plans. Use checked lengths for requested new copies and capacities for
   existing source owners. Preserve the established metadata accounting convention
   without adding whole stack headers or charging embedded layouts twice. Seed
   borrowed source and nonaliased option storage as live-only, with output-frame
   and output-metadata counters initially separate. Preflight output plane/channel
   counts, per-plane/per-ICC ceilings, final frame/metadata and source-plus-output
   live totals before any sample/layout/metadata candidate.

   Hold the full remaining requested inventory pending while each owner is
   materialized. Consume that owner's pending reservation through the same shared
   fresh-candidate/reconciliation engine; do not build a second allocator or count
   its request twice. Actual excess capacity must be checked with future owners
   still reserved before filling/copying. On failure, destroy candidate and all
   partial output owners before restoring the admitted checkpoint. Production
   execution and injected-capacity tests must consume this same admitted plan.

   Build layouts from ledger-created offset/role vectors, then move them into
   existing checked constructors. Ordinary planar constructors contain implicit
   vec allocations and are unsuitable for this checked path. Keep outer vectors
   fallible and reconciled; preserve the fixed array of sample owners, without
   introducing a temporary full-frame or pair-vector allocation. Extend/reuse the
   codec-free colour copier and one FrameMetadata copier accepting the same ledger
   for provenance, ICC, unknown payloads, geometry and pixi. Keep named source
   metadata/timing intact and tags capacity0 restriction local to convert_frame.
   Return ProcessingError::Allocation for real allocation failures; do not relabel
   them invalid input. Per-ICC and per-plane actual capacities need their own
   checks even when aggregate live storage would fit.

3. **Finish the already specified observation surface and proofs.** The conversion
   record and last_conversion getter are currently crate-private; complete the
   agreed private-field/read-only-accessor record surface so public callers can
   inspect selected authority, domains/primaries, intent, native interpretation,
   white and alpha policy without mutation. Preserve original/active separation
   and the already passing second-conversion behavior. Add tracked portable
   exact/one-under inventory tests, requested-before-first-maker, actual excess
   capacity with future pending owners, per-plane/per-ICC limits, and real
   allocation failure at each covered owner. Observe drop-before-restore, source
   pointer/content/timing invariance and same admitted-state retry, using the
   existing allocator/candidate helpers. The passing single sample-allocation
   test is not a substitute for this complete fixed ownership bundle.

Reuse the six public groups, six ownership/domain subcases and old19 route cases;
the included copies are never counted again. New-file formatting/lint checks and
applicable portability remain required after repair. Root's pre-repair lib29
i686/WASI/Miri successes cover that earlier product subset only. Source inspection
finds the requested checked copy helpers still unused in highres-only builds;
complete their real hookup rather than suppressing diagnostics. No ICC dependency
change, HDR/output-format expansion, generic tags or decoder C2 work belongs to
this repair. Full H4 and this public execution checkpoint remain open.

### C2 item2 next finite slice: iinf names and outer owner only

This is a read-only implementation handoff for the paused WIP, not acceptance.
Keep items1/3's accepted shared-engine behavior. Only iinf/infe item-name Strings,
the containing ItemInfo Vec and their Native bookkeeping are in this slice;
iloc/iref/grpl/property owners and the complete ParsedAvif handoff remain later.
The current parse_iinf_owned_with_context returns only the outer token;
read_c_string_with_context loses each name token. release_item_infos debits
name capacities before clear, while final MetaState destruction drops names first
but still debits a capacity sum. These two paths must consume the same original
name authorities, not two independent reconstructed byte totals.

1. **Small private return bundle, unchanged public values.** Introduce a private
   OwnedItemInfos containing the existing Vec<ItemInfo>, its original outer token,
   and Option<NativeItemNameOwners>. The Native-only sidecar holds one move-only
   name token per actual infe result, plus a separate token for its own Vec backing.
   Empty names have zero-byte tokens and allocate no String storage. Keep ItemInfo
   and the Legacy Vec element layout unchanged; Legacy always has None and performs
   no sidecar allocation. MetaState keeps its current item_infos/outer-token fields
   and one optional name-owner field. No generic retained-owner registry is needed
   for these parser-only allocations.

2. **Preserve one grammar and one allocation engine.** parse_infe's shared core
   returns the ItemInfo with the original name token; its compatibility wrapper
   returns only ItemInfo. Likewise parse_iinf's compatibility wrapper unwraps the
   owned result. Share the current C-string slice/UTF-8 check and String-copy core,
   changing only the private ticket return. Preserve missing-NUL handling, version
   checks, skipped non-infe children, diagnostics and Legacy allocation order.
   Native parse_iinf reserves its outer Vec as today, then reserves entry_count
   name-ticket slots using the existing Metadata fresh/replacement engine before
   inserting names; only actual infe results populate slots. The sidecar's actual
   capacity is charged and audited before population. Do not append unreserved
   tickets or add a token field to every Legacy ItemInfo.

3. **One retirement helper at the two actual callsites.** In
   parse_meta_children_with_context's iinf arm, parse the complete incoming bundle
   while the old MetaState bundle remains alive and charged. Only after success,
   take the old Vec/token/optional sidecar together, retire it, and move the incoming
   bundle into the same fields. The helper takes ownership, drops the ItemInfo Vec
   (therefore all its Strings) before releasing original name and outer tokens,
   then destroys the name-ticket Vec before releasing its separate backing token.
   Validate the release bookkeeping before this irreversible retirement so valid
   admitted state cannot introduce a fallible post-publication release. No clear
   retaining an allocated outer Vec, capacity-sum debit or duplicate token release.
   At release_native_parser_metadata, detach and retire this identical iinf bundle;
   remove only its item_info_name_bytes sum and separate outer release there.
   Other MetaState owners keep their current deferred implementation.

4. **Local candidate failure preserves the old state.** Keep the new bundle local
   during parsing. On malformed input or allocation/admission failure, first drop
   all partially made incoming names, outer storage and ticket storage, then restore
   the allocation checkpoint. Use the existing allocation-ledger restore seam,
   not ParseContext's whole-parse rollback that clears unrelated retained tickets.
   Work counters remain consumed according to the existing C1 policy; do not reset
   them for retry. This slice must not register names in the global retained-owner
   handoff. A retry uses the same ParseContext and old MetaState with sufficient
   remaining work allowance, not a newly created ledger.

5. **Finite tracked fixtures and exact observations.** Use synthetic iinf versions0/1
   and infe2/3 with two nonempty names plus an empty-name control. Track each actual
   String pointer/bytes/class from reservation through MetaState and both retirement
   paths, alongside the actual outer and sidecar backing allocations. Reuse the
   existing test allocator/observer, adding only a bounded pointer-registration
   seam if needed; observe real deallocation before the corresponding debit,
   exactly once. Successful replacement has old plus complete incoming ownership
   alive at peak; final retirement leaves no iinf-owned charge. Portable exact and
   one-under formulas include entry_count*sizeof(ItemInfo), admitted name bytes,
   and entry_count*sizeof(AllocationToken), with actual capacities reconciled.
   Deny outer/name/sidecar allocation, and inject excess sidecar capacity through
   the same production candidate engine: old pointers/content/tickets and full
   accounting checkpoint survive, partial candidates are gone at restore, then
   same-state retry succeeds. Compare Legacy result/error and allocator request
   shape with its frozen path; no sidecar request may appear in Legacy. Include
   repeated iinf replacement and final MetaState retirement in the real private
   parser callchain, not just a standalone ticket sum.

Changed callsites are limited to parse_meta_children's iinf arm,
parse_iinf_owned/compatibility wrappers, the shared parse_infe/C-string copy return,
MetaState's iinf fields, and its iinf retirement helper/final destructuring.
Move the private owner container/helper to a small container_iinf_owners module
if needed; keep grammar in the existing parser and tests in a separate test file.
The current check/Clippy/fmt and reported529/709 test successes are WIP regression
evidence, not this new original-ticket proof or full item2/C2 completion.

### Additional root verification notes

Root reports the TC13 abs/sign-fixed execution tests pass Linux-target Miri7/7;
this is not a fresh full-library run or closure of the six owner/domain defects.
Five fresh normal dependency-tree checks also pass, preserving the public ICC
c96f6e3 pin and highres-only without AVIF; these are not a build/run matrix pass.
For clean ICC f5397b6, Rust1.91 offline cargo doc --no-deps generates documentation
without warnings, and cargo test --doc runs1PASS/8 existing ignored. This verifies
generation and that single doctest, not execution of every new example/doc path.

### H4 fixed-bundle re-review: runtime counterexamples green, admission incomplete

At execution125E537E/allocationD2BA14DA/metadata56558F27, the existing independent
execution6, old route19 and ownership/domain6 all pass:31 unique tests. Included
copies remain filtered21/27 and are not additional results. Product execute tests
run7PASS separately (the original six plus TC13). Focused five-file formatting and
diff checks pass. The six previously demonstrated counterexamples are closed:
bounded-transfer selection, output plane/channel ceilings, final-frame separation,
known requested-total precheck and typed ICC allocation failure now behave correctly.

**Step2 remains NO-GO on the existing fixed items2/3.** Source review shows that
OutputOwnershipPlan performs check_charge/check_charge_metadata, but does not admit
or hold a pending balance. Sequential allocations therefore reconcile actual
capacity against only completed owners, without the still-required future owners.
The plan is not consumed by a production materialization seam, and the generic
sample allocator has no per-plane actual-capacity ceiling or injected maker. The
current sample helper restores ledger fields before its rejected Vec is implicitly
dropped. These are the outstanding same-plan/actual-capacity/drop-order conditions,
not new requirements inferred from the now-passing counterexamples.

The new frame seed correctly makes source storage live-only, but metadata remains
seeded with source_metadata_bytes. The output metadata plan still reuses source
capacity totals rather than requested clone lengths. Complete the already approved
source-live versus output-final metadata distinction, with fresh-copy lengths plus
the established inline metadata amounts and actual-capacity reconciliation. The
ledger-backed colour copier is now shared by the two callers and real ICC-copy
failure is typed Allocation; preserve that improvement. Per-ICC actual checks exist
in its candidate core, but must also run while the remaining output inventory is
reserved. Do not weaken the previous per-owner engine tests or AVIF adapter behavior.

The remaining bounded work is to turn the current plan into an actual admitted
inventory, hold all requested owners pending, and have the production materializer
and injected-owner tests use that same plan and ledger. For each owner exchange its
pending request for actual capacity before fill/copy, preserving future reservations;
on error destroy the candidate and partial outputs before restoring the admitted
checkpoint. Retry that same plan on that same ledger. Track portable exact/under,
extra capacity, per-plane/per-ICC ceilings and real failure/drop at every covered
owner through this seam. The seven product tests contain no such pipeline proof;
do not replace it with fresh convert_frame calls or hand-constructed error results.

LastConversion, its enums and FrameMetadata::last_conversion remain crate-private;
the agreed read-only public record accessors are still absent. Complete that surface
and its external authority/domain/native/white/alpha checks without making fields
mutable. Root separately reports a new useless_conversion diagnostic at the metadata
copy call and remaining unconnected-helper/record dead-code diagnostics under strict
Clippy. Remove the redundant conversion and resolve usage/appropriate feature scope
without allow/expect suppression; do not label this a warning-free checkpoint or
confuse unchanged no-default draw warnings with new highres diagnostics.

This re-review adds no probes or conversion features. Continue only the previously
approved pending-inventory, original admitted-plan proof and record/lint bundle;
full H4, CMS integration, generic tags and decoder C2 remain out of this repair.

### H4 next small slices: public record, then bound output admission

These concretize the remaining fixed items2/3 above; they are design, not a new
runtime verdict. The read-only record/lint slice may precede the admission slice.
No conversion route, dependency, public resource limit or existing test is removed.

**Read-only record slice.** Make LastConversion, NativeInterpretation and the existing
record enums public and re-export them from highres. Keep both structs' fields
private, expose read-only Copy getters, and make FrameMetadata::last_conversion
public; construction/set_last_conversion stays crate-private. Retain the existing
source authority marker, source/destination domains, destination primaries, intent,
native interpretation, white adaptation and alpha fields. Add exactly:

```text
source_cicp() -> Option<NclxColorInformation>
source_primaries() -> RgbPrimaries
```

Populate these from the selected, already inspected SourceRoute. Cicp stores Some
of its selected CICP and the corresponding known source primaries; Linear stores
None and its selected declared primaries. Do not reread preserved original colour
metadata to reconstruct the choice. CICP signaling and applied native interpretation
remain separate: an explicit native range/matrix choice belongs to the native field,
not a rewritten source_cicp. Declared LinearRelative plus explicit TC8 still records
that selected explicit CICP, while retaining the declared source domain.
External public-only assertions cover active CICP, explicit CP9/TC1 over differing
active CP1/TC13, a second already-linear conversion (None and the preceding output's
primaries), and declared-linear explicit TC8. Retain the existing intent/native/
white/alpha assertions. Remove the redundant error conversion and resolve genuine
use/feature scope of new helpers, without lint suppressions or semantic changes.

**Admission slice: one borrowed inventory and one existing allocation engine.**
Split planning/materialization into a small private module if needed. The following
names are illustrative; the ownership/state relationships are required:

```text
OutputOwnershipPlan<'source>        // fixed plane owners + borrowed metadata walk
AdmittedOutputPlan<'source> { plan, ledger, pending, owner_cursor }
Pending { frame, live, metadata }   // reservation balances, not actual allocations

plan = OutputOwnershipPlan::inspect(source, conversion_plan, limits)
ledger = ConstructionLedger::new_with_ownership(0, 0, source_live, limits)
admitted = plan.admit(ledger)       // all requested future owners checked and held
frame = materialize_with(&mut admitted, production_maker)
```

The admitted object owns the sole construction ledger and the pending balances;
do not maintain an independent second budget or recreate the ledger per owner.
Here pending.live equals pending.frame because this slice's planned heap owners
all survive in the output; the fixed temporary arrays are stack storage. Source
capacity is live-only, with output frame and metadata initially zero. Plan fresh
metadata copies from source lengths, not source capacities. Keep the established
inline metadata charges once in the output inventory, separately from heap makers;
do not charge nested inline headers again when their outer element already owns
them. Borrow metadata and use a fixed cursor/index over its owners, not an allocated
owner list. Keep the existing shared colour/metadata copy implementation.

At admission use checked sums for every requested output owner, aggregate frame/
live/metadata, per-plane and per-ICC ceilings. Store the complete pending balances
only after successful checks. Known rejection precedes all output candidates.
For each sample, outer vector, layout-offset/role vector and metadata owner, the
materializer asks the same admitted object for the next typed candidate:

```text
admitted.fresh_with<T>(owner_key, count, maker) -> Result<Vec<T>, ProcessingError>
  request = plan.next_expected<T>(owner_cursor, owner_key, count)
  candidate = shared_fresh_engine.make_checked(request, maker)
  actual = checked(candidate.capacity * sizeof(T))
  next_actual = ledger.actual + actual_in_owner_classes
  next_pending = pending - request_in_owner_classes
  check(next_actual + next_pending, actual_per_owner_limit)
  commit(next_actual, next_pending, next_cursor)  // all together, before fill/copy
  return candidate
```

This extends/factors the existing fresh candidate core behind try_new_vec and
try_new_metadata_vec_with_limit, not a second allocator or conversion-only copy
engine. The maker receives the real requested count; the shared core checks the
request before invoking it, then requires an empty candidate with capacity at least
count, checked actual bytes and the same per-owner ceiling. Existing unplanned
AVIF wrappers use the same core with no pending reservation; their ordering/errors
and replacement behavior remain covered by the prior owner/candidate regressions.
All metadata copiers consume their corresponding admitted requests through this
core too; keeping them on a separate plain-ledger path would bypass future owners.

Keep this owner's request pending during maker execution. Reconcile it atomically
with actual capacity only after all checks; never release all future reservations
to make room for an excessive candidate. A rejected candidate is explicitly dropped
before any checkpoint restore or caller-visible error; no fill/copy/implicit reserve
occurs first. A successful candidate is charged at its actual capacity before fill.
Failure to validate any prospective state must not partially update counters/cursor.

**One production retry seam, not fresh-call retry.** materialize_with borrows the
admitted object mutably and saves its complete admitted checkpoint (actual counters,
pending counters and owner cursor). Its inner builder owns every partial output;
on Err it unwinds/drops those owners before returning to the restoring outer scope:

```text
checkpoint = admitted.checkpoint()
result = build_output_with(admitted, maker)  // all partial owners local to builder
match result:
  Err(error): restore(checkpoint); return Err(error)
  Ok(frame):  verify pending exhausted and actual final inventory
              on verification error: drop(frame); restore(checkpoint); return Err
              otherwise: return frame
```

The returned error path therefore retains the same borrowed plan/source, ledger
and full initial reservation, ready for a second materialize_with call. Do not
restore inside the builder while its locals still own output allocations. Successful
publication leaves pending zero and the final actual frame/metadata plus source-live
inventory consistent with the frame's capacity walk. No new public managed frame.

Reuse the current test allocator/restore observer and inject the maker through this
production seam. The fixed proofs remain: portable exact/one-under requested totals;
extra capacity while later sample/layout/metadata owners remain pending; actual
per-plane/per-ICC refusal before fill; typed real allocation failure at each covered
owner; candidate and completed partial-owner deallocation visible at restore; source
pointer/content unchanged; then success using that same admitted object/ledger.
Include spare source metadata capacity to distinguish source live bytes from fresh
clone requests. Keep public6 + route19 + owner6 green; do not count included copies,
toy errors, independent maker-only tests or fresh convert_frame retry as this proof.

### C2 iinf-only frozen review: movement present, retirement proof still open

At container11C7E3B8/iinf-owners4B036209/budget2A582020, the new independent
c2_iinf_boundary target executes5 unique tests:4PASS/1FAIL, with136 included tests
filtered out. The shared allocation engine remains B27FFCB4. These fixtures exercise
only the approved iinf slice, not complete item2 or the other parser owners.

The four passing tests cover iinf0/1 with infe2/3, two nonempty names plus one empty
name, the original name token returned by the shared infe parser, Native-only
sidecar allocation, and actual final deallocation. Requested exact and one-under
formulas include each String, ItemInfo outer and AllocationToken sidecar backing.
Real allocation denial at outer, sidecar, first name and second name preserves old
Vec/name pointers, content, outer token and sidecar debug state plus the full
allocation checkpoint; retry uses that same ParseContext and MetaState. Successful
replacement observes old plus complete incoming ownership at peak. Malformed UTF-8
preserves old state and the Legacy error, while its iinf work remains consumed.
Legacy creates no name sidecar or sidecar-sized allocation. These observations
verify live ownership after return, not the instant of each token debit.

The failing test is a concrete Legacy diagnostic regression: denying a17-byte name
allocation now returns InvalidParam("AVIF owned string allocation failed") instead
of the prior "AVIF string allocation failed". The shared C-string path changed to
copy_string_owned's label. Preserve the old diagnostic through the shared copy core;
do not create a second name parser or change grammar to repair the message.

**The iinf-only slice remains NO-GO on its existing retirement/proof conditions.**
Reservation tokens now travel with the name sidecar into MetaState, but retirement
still sums token bytes, drops that token Vec, and performs one class-byte release.
The old MetaState bundle has already been taken and its values destroyed before
the fallible summation/releases. For a correctly admitted exclusively owned bundle,
the current totals should cover those releases; no malformed-input underflow is
claimed. Nevertheless, this is not the specified prevalidated individual-token
retirement, and no new tracked iinf test or debit-time observer proves that contract.

The bounded completion is still the same five conditions, concretely:

1. Validate this iinf bundle's original name/outer/backing authorities against the
   current ledger before taking or dropping the previous state. A stack-only checked
   aggregate may be used to prove release safety, but not replace the authorities.
   Account for incoming ownership that is already live. After validation and actual
   infos/String destruction, consume each original name token and the outer token;
   then destroy the sidecar Vec and consume its backing token. Both replacement and
   final parser retirement call this one helper. Do not introduce a fallible failure
   after old-state destruction without proving it impossible for the validated state.
   Legacy has no sidecar and zero accounting tickets; preserve its heap behavior.

2. Add the already requested tracked real-parser tests in a separate file. Reuse
   the existing test allocator with bounded pointer registrations and a test-only
   debit observer, so each name, outer and sidecar deallocation is visible before
   its corresponding token release, exactly once. Preserve all passing external
   exact/under, malformed-input, replacement-peak and four real-denial/retry controls.
   Whole checkpoint equality and final live zero alone are not debit-order proof.

3. Wire the sidecar's excessive-capacity maker through its actual production reserve
   path and shared replacement engine, not a separately constructed toy sidecar.
   A narrowly scoped test-only maker hook may be used without a new allocator or
   duplicate grammar. Reject before populating tickets, observe candidate/partial
   owner destruction at the real parser restore, keep old pointers/tickets/checkpoint,
   then retry through the same context/state. This injection seam is absent in the
   frozen iinf path; the old shared-engine excess-capacity test does not substitute
   for its parser hookup. No other iloc/property/ParsedAvif scope is added.

Items1/3's accepted baseline was independently rerun: real-parser history1PASS
(139 included tests filtered), shared candidate3PASS, product handoff9PASS. Focused
container/owner-file formatting and AVIF diff checks pass. Temporary included-harness
observer dead-code warnings are not product strict-lint results. The author's
529/709 regression counts do not contain the missing new iinf tracked proof.

Root separately reports clean ICC f5397b6's explicitly configured official v4 mAB
fixture test runs native1PASS with --exact --ignored. That is profile compilation/
finite-output evidence, not a new LCMS accuracy gate or an all-profile/WASI result.

### H4 public-record slice accepted; C2 Legacy label regression closed

At metadata5E522569/execute5427F621/testsB9ACBEF7/mod5327B532, all four recorded
source hashes match before and after independent execution. The public-only
execution_record_boundary tests run4PASS (27 included cases filtered): active CICP
with applied native interpretation; explicit CP9/TC1 instead of active CP1/TC13;
second conversion from the preceding P3 output to2020, recording P3 and no CICP;
and declared LinearRelative with explicit CP9/TC8, retaining domain and excursions.
Selected CICP range and applied native range are independently checked in the
explicit-authority case. The first active fixture initially requested a conflicting
native range and correctly hit the existing refusal; correcting that invalid
positive fixture did not require a product change or alter the rejection contract.

Source review confirms public highres re-exports, private LastConversion and
NativeInterpretation fields, read-only Copy getters, and the public metadata getter.
Construction/setters remain crate-private. Codes and primaries come from the selected
route/transform, not preserved original metadata. All existing public6, owner6 and
route19 rerunPASS:31 unique regression tests, in addition to the new four. Included
copies are not counted twice. Focused four-file formatting and diff checks pass.
**Fixed item3's read-only record is limited GO; item2 pending admission remains
unimplemented in this snapshot and is not accepted by these results.** New helper
dead-code diagnostics remain distinct from the two no-default draw warnings; no
whole strict-lint or public-execution checkpoint GO is asserted.

Root independently reports this record snapshot's execution8PASS on Rust1.91
i686, actual WASI and Linux-target Miri, plus highres-only all-targets success.
No-default highres documentation generation succeeds with the existing unrelated
TiffHeaders link warning. The public ICC dependency pin remains unchanged.

C2's existing independent iinf5 reruns5PASS at container6E0DC78A/owners4B036209
(137 included cases filtered), with both hashes unchanged around this run. The
17-byte real allocation-denial test now receives the historical Legacy diagnostic.
This closes that one regression; the separate prevalidated individual-token
retirement, real debit-time observer and parser sidecar-excess-capacity proof remain
open, with author implementation proceeding under the existing bounded approval.

Root also reports the default-feature workspace regression finishes successfully:
library30, all integrations and doctests12. Its initial offline attempt lacked
existing wml2-test dependencies; authorized retrieval followed by rerun succeeded.
That legacy test package's ICC0.0.3/40ff38dd dependency is separate from the
highres runtime's public pin. This is default/feature-off regression evidence,
not execution of the pending highres ownership implementation.

### C2 iinf three-condition re-review: two bounded repairs remain

Frozen containerE49E3965/owners2738DDCA/budget1A9369E0/allocation9B273E30,
tracked-tests5925CFFE/observer28A0DFD4 remain unchanged before and after this run.
The previous independent iinf5 all pass. One additional positive control within
the already required repeated-iinf/Legacy condition fails: after iinf0/infe2 with
one old name, iinf1/infe3 with a new name plus an empty name returns
InvalidParam("iinf entries owner token is stale"). Native actual-capacity/token
equality is now checked in the Legacy path, whose allocation tickets intentionally
remain zero. This changes previously accepted Legacy replacement behavior.

**The iinf bundle remains NO-GO for two bounded repairs, not a widened audit.**

1. Apply original-ticket retirement validation only to Native accounting. Preserve
   Legacy's no-sidecar/uncharged heap behavior and successful repeated replacement,
   plus the already repaired name-allocation diagnostic. Add the failing successful
   replacement control to the tracked tests; do not reject duplicate iinf as a new
   workaround. Native validation checks the original classes, actual owner capacities,
   checked combined quantity and ledger lower bound before old values are taken.
   Its retirement now drops infos/Strings, consumes individual name tokens, consumes
   the outer token, drops the sidecar Vec and consumes its backing token. Keep this
   shared implementation and its measured debit masks; redundant repeated name/outer
   summation after the shared validation need not become another implementation.

2. Complete the existing sidecar failure observation at the real parser restore.
   The new test reaches the actual parser and shared reserve engine, rejects excess
   capacity, preserves the old state and retries the same context successfully.
   However, it checks candidate drops only after run_iinf_test returns; neither
   AllocationLedger::restore nor the parser's outer restore records the required
   deallocation snapshot. Add a test-only observation at these actual restoration
   boundaries. The inner engine restore must see the rejected sidecar gone while
   the earlier incoming outer Vec may still be alive; the later parser restore must
   see all partial incoming owners gone. Preserve old pointers/tokens/accounting and
   same-state retry. Do not demand every incoming owner be dead at the earlier
   inner checkpoint, or substitute post-return live-zero for either observation.

The bounded multi-pointer debit observer and individual-token source wiring are
real improvements, and the Native direct-retirement tracked test passes. Its
sidecar injection currently allocates the requested candidate, then enlarges it
before registration. The asserted zero reallocations therefore covers only the
registered candidate's later lifetime, not that setup growth; report it accurately
or make the injected candidate in one fallible allocation. Reuse the same helper,
not a second allocator or replacement parser.

Independent results: iinf5PASS plus repeated-Legacy1FAIL (139 included tests filtered),
old real-parser history1PASS (142 filtered), shared candidate3PASS and product
handoff12PASS (old9 plus iinf3). The external observer adapter was updated to the
same frozen test helper body, changing only global-allocator registration to keep
the consumers' existing observers. Its included cases are not counted as executions.
Six-file focused formatting and AVIF diff checks pass. Root independently reports
handoff12PASS on MSRV1.88, i6861.91 and Linux-target Miri, and MSRV all-targets
unit532/integration180PASS with8 ignored overall. Existing decode bench completion
and FFmpeg generation are supplementary; conditional skips were not exhaustively
excluded. These broad passes do not contain the new failing Legacy control or
prove the still-missing parser-restore observation. Other item2 owners/full C2 and
checkpoint staging remain outside this verdict.

### C2 iinf final two-repair review: limited acceptance

The two repairs above now have finite acceptance at frozen containerE49E3965,
ownersC0924D68/budget364250C4/allocationBABE958A,
tracked-tests5BB91CA7/observerAA5DB71A. Native original-ticket validation no longer
rejects Legacy's deliberately uncharged collection. The previously failing
iinf0/infe2 to iinf1/infe3 replacement now succeeds, alongside the tracked
repeated-Legacy positive control and historical name-allocation diagnostic.

The real ParseContext restore operation records deallocation masks immediately
before restoring accounting. At the inner allocation failure, the rejected
sidecar is gone while the earlier incoming outer Vec remains live. At the outer
parser restore, both incoming owners are gone. The tracked parser case verifies
these two distinct boundaries, old outer/name pointers, contents, original
ticket state and accounting checkpoint; retry uses the same context/state and
final retirement reaches zero live bytes. Individual name/outer/sidecar debit
ordering and prevalidation remain intact. The injected candidate still uses a
requested reserve followed by test-only excess reserve before registration:
zero observed reallocations describes its registered lifetime, not that setup.
This is the previously permitted accurate-reporting alternative, not evidence
of a single allocation or a new production requirement.

Independent executions: iinf6PASS (old5 plus the repaired Legacy control),
real-parser history1PASS and shared candidate3PASS, ten unique cases total.
Product handoff13PASS includes the four iinf tests and prior nine tests; its
scalar handoff cases do not prove other original-owner transfers. Included
external cases were filtered, not double counted. The external observer adapter
was refreshed to the frozen helper body with only global-allocator registration
supplied by its existing consumers. Six-file focused formatting and AVIF diff
checks pass; these six source hashes are unchanged across review. Root separately
confirms the final handoff13 on MSRV1.88, Linux-target Miri and actual Node/WASI1.91.
The WASI build retains the unrelated grid-composition test import warning. The earlier
712-pass all-targets result belongs to the preceding snapshot, not this final
one. No product edits or staging were performed by this review. Other item2
owners, complete C2/decoder ownership and any cohesive checkpoint decision
remain separate and unfinished.

### H4 pending-admission freeze review: fixed condition2 remains NO-GO

Reviewed output-planEEEF21FF/testsD45FBB5A, allocation21D59799,
metadata1C9B25E9 and execute576700A7; their hashes remain unchanged across the
focused review. The existing public execution6, route19 and public record4 pass.
Owner6 now has5PASS/1FAIL: 34 of these35 unique fixed cases pass, not a complete
acceptance result. The independent mirror needed only the new output_plan module
registration; the earlier unresolved import was a harness build issue, not a
product runtime failure. Test expectations were not weakened.

The failing existing future-owner control uses RGB U8 width1024, whose observed
source heap is3636 plus16 logical inline bytes. The F32 result owns12844 heap bytes
plus8 inline nclx bytes. Both inputs fit their independent limits, but total-live
16503 is one below the required16504. Conversion nevertheless makes three4096-byte
sample allocations and returns Ok(frame). OutputOwnershipPlan inspects the logical
metadata amount, then planned_heap_bytes subtracts inline metadata for admission;
the zero-seeded ledger never charges that omitted amount later. This is the same
approved output inventory condition, not a request for additional owner classes.

Prior shared-ledger/AVIF regressions also expose two failures. The step1-adapted
extraction harness, without changed expectations, gives boundary40PASS/2FAIL,
candidate8PASS and ICC-copy4PASS; the JSON helper test occurs in all three targets
and is not three independent proofs. The original parent harness still names the
removed avif/allocation module, so its build failure was discarded in favour of
the already adapted harness. Failing B2 controls are fresh active clone length
versus retained source capacity, and native-outer release before the late clone.
The first rejects exact metadata4139 for an ICC len1/cap4096 source and len1 active
clone. The latter rejects a measured heap peak8780 plus fixed headers696 and
logical inline16. ColorInformationSet::fresh_owned_bytes now uses capacities and
omits inline colour fields, unlike its existing length-plus-inline contract used
by AVIF mapping. Preserve that established contract when sharing fresh/retained
inventory; neither successful Legacy callback snapshots nor broad tests close
these precise old-wrapper regressions.

The remaining source/proof gaps are still the previously specified condition2:
owner claims compare byte quantities rather than key/type/count, and advance the
cursor before the maker can fail; all metadata is one cursor claim followed by a
plain mutable ledger, not individually matched borrowed owner requests. Pending
heap balances do exist, remain held while a candidate is made, and the shared
pending function checks actual capacity plus future balances before committing.
Those are useful improvements, but production materialize_frame still selects
its own makers rather than accepting the same injected maker used by tests.
The generic materialize_with closure does not itself provide that production
seam. Completion checks only a heap lower bound/cursor/pending, not returned
frame capacity equality. Its two tracked tests pass but use a hand-returned error,
immediately dropped vectors and a u8 substitute for an f32 owner; they do not
establish same-admitted real-frame retry or deallocation at restore.

Five-file focused formatting and parent diff checks pass. Existing highres-only
dead-code diagnostics remain; this is not a strict-lint GO. Keep fixed public
condition3 accepted and do not reinterpret condition2 as complete execution/H4.
Root additionally confirms current AVIS normal/init/next/draw Legacy snapshots
remain feature-off/on byte-identical to the recorded hashes; these are old-API
compatibility observations, not tests of this new conversion's owner ledger.

### H4 condition2 repair sequence: the same requirements in three small slices

**A. Restore exact inventory and admission first.** Keep source capacity live-only
and initial output actual frame/metadata zero. Define the fresh heap and existing
logical inline components separately, but reserve their complete sum; do not
silently discard the inline part because it has no allocator call. Transfer each
inline component from pending to actual when the corresponding output metadata
is built, once. Fresh clone requests use lengths; retained source/output walks use
capacities. Preserve AVIF's existing fresh_owned_bytes semantics, preferably as a
delegation to the equivalent common fresh-retained calculation rather than another
formula. The first small checkpoint must restore owner4 and both old B2 failures
without changing expected limits, clone order or native-outer lifetime. It is not
condition2 GO until B/C also hold.

**B. Wire the actual assembler and one typed owner sink.** Keep the existing
MaterializeContext/reader/ConversionPlan borrowed for the attempt instead of
re-inspecting on retry. A private PreparedConversion can contain this context and
the AdmittedOutputPlan; no public frame/API type is needed. Provide the following
private relationship, with names illustrative:

```text
prepared.materialize_with(&mut maker) -> Result<ImageFrame>
  checkpoint = admitted.checkpoint()
  result = materialize_frame(&context, &mut admitted, maker)
  Err: all inner locals have dropped; observe_restore; restore(checkpoint)
  Ok(frame): compare actual frame/metadata/source-live capacity walk and pending0
             mismatch: drop(frame); observe_restore; restore(checkpoint)
```

Public convert_frame prepares once and calls this exact method with the ordinary
fallible maker. Tests supply a different maker to the same builder, not an arbitrary
closure replacing the builder. The builder still owns all partial sample vectors,
outer collections, layouts and metadata; its error exits drop them before the
outer restore. The same prepared object remains usable for retry.

Replace the metadata_ledger_mut escape with a private owner sink used by both
FrameMetadata and ColorInformationSet copying. Its next expected request is derived
from the borrowed source and fixed plane plan: sample(index), descriptor/pixel outer,
descriptor offset/role/pixel offset(index), coded/render geometry, pixi bits/extended,
provenance, ICC, unknown-colour outer and payload(index). Inline metadata uses a
non-allocating event in this same order. No heap list of requests is necessary.
Each request validates key, element kind and count, not just count*size. Typed
entry helpers or a private sealed element-kind trait can keep this check behind
the generic core, so u8[4] cannot satisfy an f32[1] sample request.

The sink's checked fresh operation delegates to the shared allocation core with
pending policy; the ordinary ledger adapter uses that same core with no pending
reservation. Factor the existing clone/copy helper over this private sink, preserving
its copy ordering and error types; do not add a second metadata clone implementation.
All per-ICC/per-plane limits reach the same core before requested/actual admission.
Compute prospective actual/pending/cursor without mutating them; invoke the maker
while the original reservation is held, reject invalid candidates before copying,
and commit all three only on success. On failure the cursor also stays unchanged.
Preserve the old AVIF no-pending path and its candidate/replacement regressions.

**C. Prove that exact production path.** Connect the already prepared rich/lean
fixtures and bounded allocator/restore observers to B. Retain the existing exact/
one-under, sample20-to32 with later owners pending, actual plane/ICC cap, real
allocation-denial-at-each-covered-owner, no-fill-before-admission, source identity,
drop-at-restore and same-prepared-object retry observations. Final success must
actually construct the equivalent ImageFrame and match its capacity walk. Keep
public6/route19/owner6/record4 and the old shared-ledger suites unchanged. This is
the established one proof bundle, not extra colour math, metadata families, CMS,
dependency changes or a replacement public API. Scope additions remain stopped.

### C2 next candidate: iloc-only original-owner movement

This is a proposed implementation slice, not acceptance or permission to expand
item2. Current source already returns three original outer tickets from
parse_iloc_owned_with_context, but nested ItemLocation.extents and each index Vec
use tokenless reservation. MetaState retains all three outer collections;
replacement releases scalar nested capacities and clears collections, while final
parser retirement separately walks capacities. Only these iloc paths change here.

1. **Private bundle, unchanged value shapes.** Add a small container_iloc_owners
   module with OwnedItemLocations: the existing locations/methods/index collections,
   their three outer tickets, and Option<NativeIlocOwners>. The Native-only sidecar
   holds one Vec of ExtentOwnerTickets { extents, indexes } per parsed item plus
   its original backing ticket. These are move-only AllocationTokens, not recomputed
   byte claims. Keep MetaState's existing value/outer-token fields and add one
   optional sidecar; do not add mandatory fields to ItemLocation/ItemExtent or
   change compatibility wrapper return tuples. Legacy always uses None and incurs
   no sidecar storage/allocation. A zero-item collection has no heap backing.

2. **Return the actual reservation authority.** Keep parse_iloc's single grammar,
   field readers, version0/1/2 rules and existing allocation labels. Its three
   current outer reservations stay on the common engine. Native then reserves
   item_count sidecar entries through that engine, checking requested and actual
   capacity before inserting tickets. Inside each existing extent loop, use two
   local zero tokens with try_reserve_with_token instead of the tokenless calls;
   after successful item parsing move those tokens into the aligned sidecar entry
   together with the values into their existing collections. Do not mint tokens
   from final capacity or register these parser-only owners in the retained-output
   registry. Empty extent/index vectors retain zero-byte original tokens. Legacy
   follows the same grammar with uncharged local tokens and no sidecar push.

3. **Shared replacement/final retirement.** Before taking old MetaState fields,
   validate Native outer/nested/backing token classes and actual capacities,
   collection/sidecar lengths and item-id alignment, checked combined bytes and
   the ledger's release lower bound. Match by original ordinal, without sorting
   or deduplicating existing values. For valid admitted state, later release must
   not introduce a new error after old owners are destroyed. The common retire
   helper owns and drops all three value collections (including every nested Vec),
   consumes each original nested token and the three outer tokens exactly once,
   then drops the sidecar Vec before consuming its backing token. Do not use clear,
   scalar capacity debits or new replacement allocations for retirement. Legacy
   skips Native token validation and simply drops the replaced old values.
   release_native_parser_metadata validates/detaches the same iloc bundle and
   invokes the same retire helper; remove its iloc-only capacity sums and separate
   outer-token releases. Leave iinf and every other owner's implementation alone.

4. **One local replacement transaction.** In parse_meta_children's iloc arm save
   the allocation checkpoint, parse the entire incoming bundle while old values
   remain live, then prevalidate/retire old and publish incoming. Any incoming parse
   failure drops partial values and sidecar before the outer ledger restore;
   failure of prevalidation also drops the complete incoming bundle before restore,
   without taking old fields. Use the existing AllocationLedger restore observer,
   not a whole-parser reset that discards unrelated tickets. Work counters remain
   consumed. The existing inner engine restore may observe earlier incoming owners
   still live; the outer parser restore must observe all partial incoming owners
   gone. Same-context/state retry keeps all old pointers/content/tickets intact.

5. **Fixed evidence, reusing iinf's observer.** Synthetic versions0/1/2 cover empty,
   one and multiple extents, existing zero/4/8-byte fields and construction methods
   already accepted for that entrypoint. Keep Legacy method2 and Native method2
   rejection unchanged; external references and unsupported widths retain their
   current errors. Use the real meta-child parser for repeated replacement and
   final retirement, not merely a helper sum. Exact/one-under formulas include
   N*sizeof(ItemLocation), N*sizeof((u32,u16)), N*sizeof((u32,Vec<u64>)), the nested
   extent/index owners, and N*sizeof(ExtentOwnerTickets), all reconciled to actual
   capacities. Deny each outer, nested and sidecar allocation; inject sidecar excess
   through the existing maker seam. Observe real owner deallocation before its
   individual debit and at inner/outer restoration, peak old+incoming ownership,
   same-state retry and final zero iloc charges. Preserve source bytes, Legacy
   successful repeated replacement/result/error/request shape, accepted iinf6,
   history1/candidate3 and tracked handoff13. The existing bounded observer needs
   no new allocator or unbounded pointer list; keep fixtures within its slots.

Changed callsites are limited to MetaState's optional iloc sidecar, the owned
parse_iloc result and its compatibility wrappers, its two nested reservations,
the iloc meta-child arm and iloc final retirement. Tests belong in a separate file.
No iref/grpl/property/payload/decoded-plane ownership, new public API or full
ParsedAvif handoff completion is included. Implementation awaits main approval.

### H4 condition2 repair A: limited acceptance

At allocation3C435CA6/metadataA4AEDAB7/output-plan17511817,
plan-testsCB1E8905/executeCBD95B45, the three fixed failures above are repaired.
The highres-only owner4 control passes: the independently measured one-under
live limit rejects before any4096-byte output sample candidate. The standalone
extraction harness passes all eight existing B2 controls, including the two
previous failures for fresh clone length and release-before-late-clone peak.
These are nine executed fixed regression cases, not the complete public35 suite.
The harness retains its frozen AVIF dependency and does not build the concurrently
edited iloc source; no current avif+highres full-build success is inferred.

Source review confirms admission now retains the complete logical output/metadata
sum, including inline fields. The inline event checks prospective actual/pending
counters before committing once after metadata construction; its cursor advances
only after that commit. AVIF fresh_owned_bytes delegates to the common fresh
length-plus-inline calculation, restoring its prior semantics without a duplicate
formula. Five-file focused formatting/diff checks pass and source hashes remain
unchanged across this review. This accepts A only: typed per-owner metadata
consumption, injected production materializer, exact final capacity walk and real
same-admitted allocator/drop/retry proof remain B/C work. Existing aggregate/toy
tests are not promoted to those missing proofs; full execution/H4 remains open.

### H4 B1/B2 private wiring clarification

This splits the already approved B implementation, without adding acceptance
conditions. A remains accepted; B1 alone leaves B2 and the full C proof unfinished.

**B1: retain the real conversion and inject only its candidate maker.** The actual
callchain must be public convert_frame -> PreparedConversion::new ->
prepared.materialize_with -> the existing materialize_frame. The caller supplies
an allocator strategy, never a closure that substitutes for materialize_frame.
Illustrative private signatures (not public API) are:

```text
PreparedConversion<'src, 'opt> {
    context: MaterializeContext<'src, 'opt>, admitted: AdmittedOutputPlan
}
new(source: &'src ImageFrame, options: &'opt ColorConvertOptions<'src>,
    limits: &ResourceLimits) -> Result<Self>
materialize_with<M: CandidateMaker>(&mut self, maker: &mut M) -> Result<ImageFrame>
materialize_frame<M: CandidateMaker>(context: &MaterializeContext,
    admitted: &mut AdmittedOutputPlan, maker: &mut M) -> Result<ImageFrame>

trait CandidateMaker {
    fn make<T: OwnerElement>(&mut self, key: OwnerKey, count: usize) -> Result<Vec<T>>;
}
```

Use static generic dispatch; no Box/dyn allocator or heap owner list is needed.
The context borrows source/options supplied by the caller (not another field of
self), retains the one ConversionPlan and NativePixelReader::from_plan result,
dimensions/counts and required ceilings, plus the validated RelativeTransform and
read-only record facts. Constructor inspection and route rejection happen once.
Retry borrows this same context; it neither moves out the reader nor calls inspect
again. ResourceLimits may be copied or its required ceilings retained privately.

OrdinaryMaker makes an empty fallible Vec for the requested count. For B1 route
sample(index), descriptor outer, pixel outer, descriptor offset/role/pixel offset
through admitted.fresh_with(key,count,maker), including output_plane's allocations.
That function validates the fixed request and computes next_cursor before allocation,
then calls the common reserved-candidate engine with a maker closure. Commit cursor
only after the engine successfully commits actual/pending; the closure creates a
candidate only, never constructs a frame. The maker receives the same key and
count that production uses. All sample filling/plane assembly remains in the one
existing materialize_frame body. Metadata's present aggregate path may remain
temporarily in B1, but must be labelled unconnected to per-owner injection until B2.

PreparedConversion owns the outer checkpoint/restore wrapper. materialize_frame
owns all partial vectors and metadata in locals. On its error, those locals drop
before the wrapper's test observer and restore. On success, compute the actual
frame walk with existing descriptor.owned_bytes + pixels.owned_bytes +
metadata.metadata_bytes, not the requested plan total. Compare exact frame and
metadata counters, source-live plus actual frame, pending zero and completed
cursor. Retain the existing inline convention without inventing extra headers.
On a verification failure drop the returned frame before observer/restore.
The observer reads only safe snapshots at this boundary, not aliased mutable
ledger state from GlobalAlloc. Prepared state survives errors for real retry;
an already published success is not silently reset into a second ownership run.

**B2: make existing metadata copying generic over the same owner sink.** Extend
the borrowed plan's fixed cursor into source metadata, replacing metadata_ledger_mut
as the conversion escape. Keep one clone implementation for each existing metadata
type, parameterized over a private sink rather than a concrete ConstructionLedger:

```text
trait OwnerSink {
    fn fresh<T: OwnerElement>(&mut self, key: OwnerKey, count: usize) -> Result<Vec<T>>;
    fn commit_inline(&mut self, key: InlineKey, bytes: usize) -> Result<()>;
}
copy<T: Copy + OwnerElement, S: OwnerSink>(sink: &mut S, key, source: &[T])
    -> Result<Vec<T>>              // fresh first, then extend; never reserve after fill
FrameMetadata::clone_with<S: OwnerSink>(&self, sink: &mut S) -> Result<Self>
ColorInformationSet::clone_with<S: OwnerSink>(&self, sink: &mut S) -> Result<Self>
```

A short-lived conversion sink borrows admitted plus maker, delegating every fresh
request to B1's checked core. The ordinary ledger adapter preserves existing AVIF
wrappers, allocation order and error handling with no pending policy. Factor the
existing try_clone_for_conversion_with_ledger / try_clone_owned_with_ledger /
ownership copy helpers into these shared implementations, retaining compatibility
wrappers rather than maintaining duplicate clone bodies. Do not route conversion
metadata through a naked mutable ledger or charge one opaque metadata byte blob.

Use distinct keys for CodedGeometry/RenderGeometry, PixiBits/PixiExtended,
ColorProvenance, IccProfile, UnknownColorOuter and UnknownColorPayload(index),
plus the established inline events. For example, coded geometry requests
fresh::<GeometryOperation>(CodedGeometry, source.coded_geometry.len()); ICC uses
fresh::<u8>(IccProfile, profile.len()) with the plan's ICC ceiling; an unknown
payload uses its own index/count before copying. A private sealed OwnerElement
kind (or equivalent typed entry helpers) distinguishes f32 samples from u8 data
and same-sized structural elements. Validate key, kind and count against the
borrowed source before the maker; carry its per-owner ceiling to actual capacity
checking. No input-capacity-to-request substitution or byte-size-only match.

B1 can be reviewed as actual assembler/restore/final-walk wiring while B2 is still
pending. C then runs the already specified complete production allocator bundle
after B2, including metadata candidate excess, all covered owner failures and
same-prepared-object retry. Toy closure tests and immediately dropped vectors
cannot substitute for either stage. Existing public6/route19/owner6/record4 and
old ledger/AVIF regression expectations remain unchanged.

### C2 iloc-only frozen independent review: finite runtime GO

The frozen iloc runtime satisfies the five conditions above; no production fix
or other-owner expansion is requested. Original nested reservation tokens move
through the Native-only sidecar into MetaState; replacement and final retirement
consume those tickets without capacity reseeding. Legacy keeps its three value
collections, existing grammar/errors and no sidecar allocation.

Independent `c2_iloc_boundary` executes five unique tests, all PASS:

- Versions0/1/2, zero/4/8-byte fields, empty/one/multiple extent shapes retain every
  id/method/base/index/offset/length. Independent sizeof inventory formulas,
  actual-capacity checks, exact/one-under budgets, source preservation and final
  zero iloc charges pass; expected totals are not learned from the ledger.
- Real allocation denial covers each of the three outer vectors, both nested
  vectors and Native sidecar storage. Every failure preserves old pointers,
  complete value/ticket state and the allocation checkpoint; retry uses that
  same context/state successfully. Old-plus-incoming peak remains charged.
- Real meta-child replacement and release_native_parser_metadata retirement
  observe each registered old owner deallocated exactly once before its debit.
  Final retirement also emits five zero-byte non-iloc token-release events;
  those are distinguished from the six iloc owner releases, not counted as
  additional iloc owners or failures.
- A real sidecar excess candidate is dead at the inner engine restore while the
  three earlier incoming outer vectors remain live. All four incoming owners
  are dead at the outer parser restore; old state survives and same-state retry
  succeeds. The existing excess injector may perform a second reservation before
  candidate registration; this is not a single-allocation claim.
- Legacy repeated successful replacement, malformed replacement, method2 and
  unsupported version/field-width/external-reference diagnostics remain intact;
  Native method2 remains explicitly unsupported. Legacy sidecar controls observe
  no sidecar request.

Two implementation-order differences are accepted as safe finite equivalents.
The iloc arm temporarily moves old fields into a private bundle before validation,
but no allocation/publication/callback intervenes and validation failure restores
every field before dropping incoming owners and restoring the checkpoint. The
retire helper drops all three value collections first, consumes nested tickets,
drops/debits the sidecar, then consumes the already-dead outer owners' tickets.
This differs from the proposed outer-before-sidecar sequence but never debits a
live owner; delaying outer debits is conservative. Checked combined metadata
bytes/classes/capacities and the release lower bound establish that subsequent
individual releases cannot fail for a valid admitted parser state. No malformed
input underflow is inferred from hypothetical corrupted private tokens.

Regression reruns: independent iinf6, candidate3 and history1 PASS. The history
target also ran three already-included compatibility cases; these are not three
new history proofs. Product handoff13 and iloc4 PASS. The iloc target's 104 filtered
included tests are not executed evidence. Root separately reports MSRV1.88 and
Linux-target Miri iloc4 PASS, plus MSRV native boundary11/hookup3/limits5/phase9
(28) PASS; these do not replace the independent allocation/shape observations.
Focused edition2024 formatting of the three iloc files and nested diff-check PASS.
Six frozen source SHA256 values remained unchanged across this review, including
container B4DBF5F3, iloc owners 3C9D50DB and iloc tests DFEA9AC5.

Runtime is finite GO; tracked-proof supplementation remains before declaring this
slice's permanent regression bundle complete. Existing tracked4 uses a measured
ledger total for exact limits, mostly width4 fixtures, helper retirement rather
than the full final-parser path, and no complete real per-owner denial matrix.
Move the independent five proofs into the existing separate iloc test module,
reusing its fixture/observer helpers, without duplicating runtime code or changing
acceptance expectations. Main will authorize that test-only follow-up separately.
This is not full item2/ParsedAvif handoff, other owners, decoded-plane ownership,
full C2 or approval to commit the entire currently mixed AVIF diff.

### H4 B1 first frozen review: assembler connected, finite NO-GO

The real chain is now public convert_frame -> PreparedConversion::new ->
prepared.materialize_with -> materialize_frame. Nonmetadata samples, descriptor
and pixel outer vectors, offsets and roles use the same CandidateMaker path.
One ConversionPlan and NativePixelReader are retained; a successful publication
cannot be repeated. Metadata's aggregate path remains explicitly B2 work, not a
new B1 finding. A's accepted pending/inline accounting is unchanged.

Three independent B1 probes execute against frozen production function bodies:
one PASS, two FAIL. The passing probe injects Sample1 failure after the first real
sample vector, retries the same PreparedConversion, obtains the same ImageFrame
as public convert_frame, checks pending zero/source unchanged, and confirms that
a second publication is rejected without calling the maker. The two failures are
the existing B1 contracts below, not additional scope:

1. **Actual-capacity completion.** A two-pixel RGB source with generous limits and
   a maker returning Sample0 capacity3 for requested count2 passes the common
   reserved-candidate engine, but complete_for rejects the completed frame because
   actual_total differs from plan.total_bytes. A valid four-byte surplus is legal
   when actual frame/live/plane limits hold. In output_plan::complete_for (around
   line383), compare the actual descriptor+pixel+metadata capacity walk to the
   exact committed frame/metadata counters and source-live+actual frame; retain
   pending-zero/completed-cursor checks. Requested plan totals initialize pending,
   not an equality requirement on actual capacities. Keep checked actual ceilings;
   do not clamp capacities or alter the source to force equality.
2. **Atomic owner cursor.** fresh_with/fresh_with_limit call claim_keyed_owner
   before allocation. A Sample0 Allocation error leaves pending unchanged but
   moves cursor0 to1. In output_plan around245/259/330, validate key/count/type and
   calculate next_cursor without mutation, run the common candidate engine, then
   commit cursor only on success. Outer PreparedConversion retry currently masks
   this mismatch by restoring the whole checkpoint; it does not satisfy the
   promised per-owner transaction. Keep both limited and unlimited entry helpers
   on that shared sequence, with the same key/count sent to the production maker.
3. **Retained preparation facts.** MaterializeContext around142 does not retain
   RelativeTransform or LastConversion facts: materialize_frame rebuilds them
   around170/243 on every retry. Compute/validate these once in new, before output
   admission/materialization, and keep the existing Copy facts in the context.
   Reuse them in the same assembler; do not repeat plan inspection or add output
   allocations to preparation. This is the previously specified prepared context,
   not a new color-math requirement.
4. **Real restore observation.** The two error branches of PreparedConversion's
   wrapper around112 drop inner locals/returned frame before restore by source
   control flow, but no test observer is connected at that actual boundary. Add
   the existing bounded test-only owner/drop snapshot hook immediately before
   restore in both branches, after those drops. Observe actual production-created
   nonmetadata owners on one later-owner failure, checkpoint restoration and same
   prepared-object retry. Do not substitute a closure that builds/drops toy Vecs,
   read aliased ledger state from GlobalAlloc or add a second global allocator.
   B2's per-metadata-owner injection and C's full failure matrix remain later work.

The ignored B1 harness snapshots only execute/output-plan module bodies, adjusts
their existing test-module paths, and appends independent probes; all production
function bodies compare equal after newline normalization. Initial harness-only
duplicate-global-allocator and absent-default-builder compile errors were fixed
without product changes before the three tests executed. Public regression uses
the actual dependency: execution6/route19/owner6/record4 = 35 unique PASS. Old
AVIF owner targets boundary42/candidate8/ICC4 PASS; their shared JSON test is
included three times and is not three independent ownership proofs. Root reports
MSRV1.91 i686 and Linux-target Miri execute9 PASS, including the tracked prepared
retry; these do not cover the two failing independent boundaries or C's proof.

Frozen hashes stayed unchanged: execute B37A0E32, output plan E6EF3FBB,
execute tests E3D6D0BA, shared allocation 3C435CA6. Focused edition2024 formatting
and diff-check PASS; no whole-feature strict-lint clearance is claimed. Repair
only these four already-approved B1 points, retain the fixed tests/old35/AVIF
regressions, then re-freeze before B2. No production edits or commit were made
by this review; all H4 and pending condition2 remain incomplete.

### H4 B1 final independent review: finite GO

This supersedes the preceding B1 NO-GO for the four specified repairs only.
Actual sample capacity above the requested count is accepted when its checked
frame/live/plane ceilings hold, and completion compares actual owned capacity to
the committed counters. A failed candidate preserves the owner cursor and pending
balances. RelativeTransform and LastConversion facts are retained by the original
PreparedConversion and are not rebuilt on retry.

Both real wrapper error branches call the test observer after partial owners or
the returned frame have dropped and immediately before restoring the checkpoint.
The tracked proof registers the ordinary Sample0 Vec's pointer and capacity bytes;
the existing single allocator recognizes only that pointer/layout and the hook
captures its deallocated flag and count1. Error String deallocation cannot stand
in for that owner. The independent additional probe checks Sample0 still live at
Sample1 failure, the captured target drop, source preservation, TLS isolation,
previous observer/target restoration, and same-PreparedConversion success after
observer cleanup. No allocator reads an aliased construction ledger.

Independent B1 probes4 PASS; the combined B1 target29 PASS also includes product
and previously covered tests. Existing public execution6/route19/owner6/record4
(35 unique) and AVIF boundary42/candidate8/ICC4 PASS; shared included cases are not
new independent proofs. Snapshot function bodies match production after newline
and test-module-path normalization. Source hashes remain unchanged through review
(execute 6EE9C795, output plan 44D8D4E0, execution tests 0F151C80, allocator
91C8B86E, test allocator 172642AA); focused formatting and diff-check PASS.
No full-feature lint clearance or B2/C completion follows from this finite GO.

### H4 B2 bounded implementation contract and independent review matrix

This is the next private wiring slice, based on the B1/B2 clarification above and
the current source. B2 and C are not implemented or verified by this document.
Do not expand conversion routes, public APIs, features, dependencies or versions.
The existing tags-capacity rejection, ICC authority, colour math, geometry/timing
preservation and explicit conversion record remain unchanged.

#### Current source boundaries

| Boundary | Current function or responsibility | Required B2 change |
| --- | --- | --- |
| `highres/output_plan.rs` | `OutputOwnershipPlan::inspect`, `expected_owner`, `AdmittedOutputPlan::fresh_with` | Extend the fixed cursor into borrowed source metadata; check key, element kind, count and owner ceiling before the maker. |
| `highres/output_plan.rs` | `metadata_ledger_mut`, `commit_metadata_inline`, `complete_for` | Remove the conversion's naked-ledger/aggregate-metadata escape. Validate individual metadata/inline events and compare final actual metadata with the ledger, not requested metadata bytes. |
| `highres/ownership.rs` | `copy_with_ledger`, `copy_icc_with_ledger`, `clone_color_information_with_ledger` | Keep compatibility wrappers; route their copies through one private typed OwnerSink implementation. |
| `highres/metadata.rs` | `ColorInformationSet::try_clone_owned`, `try_clone_owned_with_ledger`, `FrameMetadata::try_clone_for_conversion_with_ledger` | Use one clone body per metadata type, generic over the sink. Preserve the legacy error boundary and allocation order. |
| `highres/allocation.rs` | `try_new_metadata_vec_with_limit`, `try_new_pending_vec`, `commit_pending_inline` | Reuse the same checked fresh-candidate core for reserved and ordinary metadata owners; provide only the narrow internal entry needed by the sink. |
| `highres/convert_execute.rs` | `materialize_frame` metadata call | Create a short-lived sink borrowing admitted plan plus maker; clone through it and end that borrow before `complete_for`/outer restore. |
| `highres/avif/metadata.rs` and `highres/avif/mapping.rs` | Rich metadata projection, provenance growth, active-colour clone | Adapt existing copy call sites without changing projection order, native geometry, ownership transfer, growth transactions or legacy error contracts. |

Keep trait/key definitions in a small codec-free ownership module, with private
re-exports if needed to preserve internal call sites. The large metadata module
must not acquire a second copy implementation: private clone bodies and their
borrowed metadata inventory may be extracted into a child module able to access
the existing private fields. Public struct fields and signatures stay unchanged.

#### One typed request stream

Retain static generic dispatch. A sealed `OwnerElement` supplies an `OwnerKind`;
do not infer kind from `size_of`, use type-name strings, cast structural storage
to bytes, or allocate a type-erased owner list. Kinds distinguish at least u8,
f32, usize, ChannelRole, PlaneDescriptor, Plane<f32>, GeometryOperation,
PixelChannelInformation, ColorProvenance and UnknownColorInformation. In
particular, u8 and ColorProvenance are distinct even when both occupy one byte.
UnknownColorInformation is a non-Copy outer element; reserve its Vec first and
move individually copied payloads into it. CandidateMaker and both B1 fresh
helpers carry the same sealed element bound.

The plan borrows FrameMetadata. Its metadata cursor is a fixed phase plus an
unknown-payload index, with checked advancement and no heap request list. A peek
returns the expected key/kind/count/class/ceiling and prospective next cursor
without mutation. Preserve the existing conversion clone order below. Each Vec
owner is a distinct event even for count0; an absent Option has no heap event,
while Some(empty) retains its presence. Zero requests need not allocate but must
not hide an excessive returned capacity or advance twice.

| Conversion metadata order/key | Exact element/count source | Owner limits |
| --- | --- | --- |
| CodedGeometry | GeometryOperation / coded geometry length | frame, live, metadata |
| RenderGeometry | GeometryOperation / render geometry length | frame, live, metadata |
| PixiBits, when pixel information exists | u8 / bits-per-channel length | frame, live, metadata |
| PixiExtended, when its Option exists | PixelChannelInformation / extended-channel length | frame, live, metadata |
| ColorProvenance | ColorProvenance / source-colour provenance length | frame, live, metadata |
| IccProfile, when present | u8 / ICC payload length | frame, live, metadata, per-ICC |
| UnknownColorOuter | UnknownColorInformation / unknown-colour element count | frame, live, metadata |
| UnknownColorPayload(index), in source order | u8 / that indexed payload length | frame, live, metadata |
| Inline events | SourceNclx=8 if present; SourceAv1=AV1_COLOR_INFORMATION_BYTES if present; CodedDimensions=8 and RenderDimensions=8 when present | frame, live, metadata; no candidate allocation |

Inline keys describe existing logical charges, not extra heap allocations.
Commit source-colour inline events once as that clone is assembled; commit the
dimension events once as FrameMetadata is assembled. Do not invent charges for
LastConversion, Vec headers inside outer elements, or other inline fields absent
from the established metadata walk. Optional absent events are skipped by the
same borrowed cursor. Fresh requests use lengths; borrowed source capacity stays
in source-live accounting and is never copied into a destination request.

The minimal private sink operations remain typed `fresh(key,count)` and
`commit_inline(key,bytes)`. A shared copy helper calls fresh first, then
extend_from_slice without reserve/growth during population. The conversion sink
borrows `&mut AdmittedOutputPlan` and `&mut CandidateMaker` only for that operation.
It does not return a ledger reference, own another budget, or accept a closure
that substitutes a metadata/frame builder. All makers receive the real key and
count; per-ICC requested and actual limits apply only to IccProfile, not arbitrary
u8 payloads. Unknown payload indices with identical lengths are still distinct.

Ordinary ledger and no-ledger adapters use the same clone bodies. Their explicit
non-pending policy preserves the current AVIF wrapper accounting, caller-owned
inline accounting, allocation order and typed errors; it must not create a new
pending reservation or double-charge inline fields. Preserve the existing AVIF
provenance/unknown-vector replacement helpers and their old-plus-new peak policy;
B2's conversion clone owns fresh vectors and must not call a growth helper.
Translate legacy HighresError results only at their existing compatibility
boundary, with the same operation-specific failure meaning. A shared clone body
must not use setters that silently append provenance after the reserved copy.

#### Transactions and completion

Admission checks and reserves every requested output owner once, including all
metadata and inline events. During any maker call, that owner's request and all
future requests are still pending. Before filling the candidate, check empty
shape, minimum count, checked actual capacity, owner ceiling and actual-plus-
remaining-pending frame/live/metadata limits. Commit actual counters, pending
deduction and cursor together only on success. Rejected candidates drop before
returning; per-owner failure changes none of those values or source storage.

Earlier successful owners remain charged while their partial clones are alive.
Do not restore the outer checkpoint from inside a clone helper. On a later error,
the real materialize_frame drops all partial metadata and pixel owners, then B1's
observer runs and the one PreparedConversion checkpoint restores actual, pending
and cursor. Retry uses that same prepared object and borrowed source. Completion
requires pending0, a completed cursor and exact actual capacity-walk/ledger
agreement for frame, live and metadata. The current comparison of actual
metadata_bytes to plan.metadata_bytes must be removed: a legal metadata surplus
is treated exactly like B1's legal sample surplus, not rejected or clamped to the
request. Keep all actual ceilings and the source-live contribution.

#### Independent checks prepared for B2 and subsequent C

Reuse the existing rich pending fixture and independent sizeof/length inventory,
plus B1's real assembler and bounded pointer observer. These rows are planned
proofs, not executed results; no new acceptance threshold is introduced.

| Probe | Required observation | Review stage |
| --- | --- | --- |
| Rich metadata request trace, optional absence/empty/spare controls | Every key above reaches the same maker exactly once with the correct kind/count; source capacity does not inflate requests and every field/payload is preserved. | B2 wiring |
| Wrong key, wrong kind of equal byte size, wrong count, duplicate/out-of-order unknown payload | Reject before maker/copy; actual/pending/cursor unchanged. Include u8 versus ColorProvenance and byte-size-equivalent structural requests. | B2 wiring |
| ICC or later unknown-payload candidate failure after earlier metadata owners | Typed failure; registered rejected/earlier owners drop before the real restore; source addresses/content survive; same prepared object succeeds after failpoint cleanup. | B2 focused transaction |
| Inline replay/wrong key/wrong byte amount and absent-option controls | No allocation; no duplicate logical charge; atomic rejection or exact single pending-to-actual transfer. | B2 wiring |
| Independent output Q/metadata M/source S, exact and one-under | Initial actual output0 with pending Q/M, source live-only; known refusal before first candidate; source independently fits the selected limits. | C aggregate proof |
| Actual sample20-to32, legal metadata surplus, ICC4093-to4160 | Future reservations remain held; exact surplus headroom succeeds, one-under fails before fill; separate max-plane/max-ICC actual ceilings cannot be bypassed. | C actual capacity proof |
| Real allocation denial at each metadata key, including distinct unknown payload indices, plus B1 nonmetadata owners | Keyed failure does not rely solely on allocation-size matching; every real owner is accounted and observed at restore; no population/reallocation after rejection. | C complete failure matrix |
| All-source ownership and all pending/actual/cursor state across failure/retry | No partial success, source mutation, stale active colour or duplicate publication; final actual walk equals counters after same-Prepared retry. | C complete transaction matrix |

Run the unchanged public35 and AVIF owner boundary42/candidate8/ICC4 regressions
when adapting shared helpers. Retain highres-only and AVIF-enabled compile/tests
and their feature isolation; new warnings are not excused as baseline. B2 review
must confirm all production metadata owners are connected before C's complete
allocation matrix is claimed. Full H4 colour routes, full ICC oracle/intent gates,
native decoder work and the overall highres objective remain separately open.

### H4 B2 final independent review: bounded runtime/ownership GO

The final reviewed source resolves the earlier B2 runtime NO-GO findings. This
is a finite judgement for typed owner wiring, focused failure transactions and
legacy/AVIF compatibility, not completion of C or approval of a checkpoint
commit/release. The lint cleanup gate below remains open.

Reviewed fingerprints: output plan `017C8637`, executor `734C6627`, metadata
`D50744A4`, shared ownership `5C92C9E5`, allocation `AEDB5C72`, AVIF metadata
`EFF79D8E`. The 23 relevant Rust files were unchanged across the final review
runs. Independent output/executor snapshots match the production bodies after
newline normalization and test-module path adaptation; no expectations were
weakened. Only temporary independent harnesses and this review record were
edited by the reviewer, with no product/API/dependency/version/commit changes.

Verified boundaries:

- All nine rich-fixture metadata heap requests reach the real maker with exact
  key, sealed element kind and source length: coded/render geometry, pixi bits
  and extended channels, provenance, ICC, unknown outer and payload indices0/1.
  Source spare capacities do not inflate requests. Optional absent and empty
  spare-vector controls retain the required count0 owner events.
- u8 versus same-size ColorProvenance, wrong counts and repeated unknown indices
  reject before the maker without changing actual/pending/cursor. An unrelated
  same-size newtype cannot implement OwnerElement: the independent negative
  compilation fails specifically with E0277 for the sealed bound.
- Heap keys cannot be committed as inline, including empty coded geometry.
  Inline wrong-size/replay controls are atomic. Unknown payload lookup uses a
  direct checked index, with no repeated scan over preceding payloads.
- Legal extra capacity for each metadata owner completes the real frame and
  matches the actual ownership walk/ledger. The per-ICC actual ceiling rejects
  before commit while an equally large unknown u8 payload remains permitted.
  The naked aggregate ledger and aggregate inline conversion escape are gone.
- Keyed failure at every rich metadata owner restores actual/pending/cursor and
  source addresses/content; the same PreparedConversion retries successfully.
  The real wrapper's drop-before-restore observer sees the registered Sample0
  allocation deallocated once. A later unknown-payload failure independently
  proves the earlier real ICC allocation is also released before restore.
- Ordinary and ledger colour/frame cloning share the sink clone body. AVIF
  pixi mapping allocates its typed destination fallibly before pushing values,
  with no uncharged mapped temporary. A zero metadata budget refuses before
  any pixi candidate allocation. Existing projection order and replacement
  old-plus-new peak regression tests remain passing.
- Real allocation denial preserves the four legacy operation-specific errors
  for provenance, ICC, unknown outer and unknown payload. The optional sink now
  restores the caller's ledger Option before propagating either ResourceLimit
  or Allocation; the same slot retries with accounting still enabled.

Executed results:

- B1/B2 combined target: 36/36, including seven new focused B2 tests.
- Existing public35: pass; target counts19/27/33/31 include shared tests and are
  not four disjoint proof sets.
- AVIF/shared-owner targets: boundary51/51, candidate10/10, ICC6/6. These include
  reused product tests and seven new independent compatibility/optional-sink
  checks; do not add the target counts as unique external proofs.
- Direct highres-only and AVIF+highres checks: pass. Focused format/diff checks
  pass. The expected sealed negative compilation is not a product build failure.

Remaining gates:

- New dead-code fallout is not waived as baseline: remove the obsolete
  metadata_heap_bytes field and unused compatibility-copy remnants where no
  caller remains, and cfg-gate genuinely test-only expected_owner_bytes /
  claim_owner_bytes and feature-specific helpers. Current direct checks report
  15 warnings for highres-only and 9 for AVIF+highres, including older diagnostics;
  a clean strict-lint/checkpoint gate has not been established.
- C still owns the complete real-allocator denial matrix for every output owner,
  independently calculated Q/M/S exact and one-under budgets, and the full
  future-reservation/actual-capacity/source-live matrix. The keyed B2 failures
  and selected real denial probes above do not substitute for those proofs.
- Full H4 conversion routes, ICC oracle/intent gates, native decode work and
  overall highres completion remain open. No commit, version or publication is
  authorized by this finite review result.

### H4 B2 warning-only review and C execution order

The first warning-only snapshot changes only allocation, metadata, output-plan
and ownership source files; the other 19 reviewed Rust files, including the
executor, native reader and AVIF projection/mapping bodies, retain the preceding
review hashes. The output-plan snapshot diff removes the unused stored
metadata_heap_bytes member and adds test cfg to two test-only methods; its
runtime validation and keyed transaction bodies are unchanged. Metadata clone
and optional-sink restoration paths retain their B2 behaviour while unused
wrappers and feature-specific imports/helpers are removed or cfg-gated.

Independent no-default checks of that first cleanup snapshot pass. AVIF+highres
reports only the previously documented NativePixelReader::inspect warning.
Highres-only reports four diagnostic groups: the two existing draw diagnostics,
the same reader warning and a remaining AVIF-off ConstructionLedger dead-code
group. The last group is not a clean-warning result: charge/reconcile/rollback,
release_live/charge_metadata, replacement helpers and try_new_vec need the
appropriate AVIF/test cfg if they have no other callers. This record does not
waive remaining new warnings or claim a whole-workspace strict-Clippy pass.

The subsequent AVIF-only cfg attempt reduces ordinary highres checks to the
known draw/reader diagnostics, but is temporarily NO-GO for test compatibility:
both the synchronized independent target and the product no-default highres
lib-test build fail E0599 at output_plan's test-only try_new_vec caller. The
ledger try_new_vec and its charge/reconcile/rollback dependency chain must
remain available under cfg(test) as well as AVIF. No test assertion or runtime
accounting change is needed to correct this cfg mismatch.

The following any(AVIF,test) correction resolves E0599: the product no-default
highres all-targets run passes, including36 unit tests, and AVIF+highres check
passes with the known reader warning only. Normal highres library diagnostics
are now just draw2/reader1. Its lib-test build still emits allocation dead-code
groups for AVIF-only helpers/guard construction; do not describe all-targets as
free of B2 allocation warnings until those remaining cfg boundaries are checked.

#### C: concrete bounded proof order (not executed here)

No conversion route, sample format, colour policy, public API, dependency or
version change belongs to C. Implement only the previously approved complete real
allocation and exact-boundary proof matrix against the existing materializer.
If an observable boundary is missing, use the smallest bounded test-only hook
in the existing test allocator/observer; do not introduce another allocator,
another builder or another accounting engine.

1. Freeze source and independent inventories. Reuse the five-pixel RGBA rich
   fixture with ICC length4093, optional source spare capacity, geometry, pixi,
   nclx/AV1 and timing. Include two equal-length unknown payloads to distinguish
   their keys. Retain a lean Gray source whose output is larger than its input
   for tight frame/live limits. Compute requested output Q, metadata M and
   borrowed source S independently from public type sizes, lengths and actual
   source capacities; never copy the product plan's total into the expected
   value. Before every constrained test, prove that the source itself fits its
   selected limits. Tight metadata tests need a source-fitting non-spare control;
   rejection during source validation is not an output admission proof.
2. Prove admission before allocation. At entry, actual frame/metadata are0,
   actual live is S, and pending frame/metadata are Q/M. Test frame Q versus Q-1,
   live S+Q versus S+Q-1 and metadata M versus M-1 with independently fitting
   source controls. A known admission refusal must occur before the first
   output maker. Keep borrowed spare-capacity checks separate from these exact
   requested-output controls.
3. Prove requested-versus-actual reconciliation. A sample request20 returning32
   needs exactly12 extra frame/live bytes while all future reservations remain
   pending; exact headroom succeeds and one-under refuses before fill. An ICC
   request4093 returning4160 needs exactly67 extra metadata/frame/live bytes.
   Isolate max-plane and max-ICC actual ceilings with ample other budgets and a
   source ICC capacity4093. Include legal metadata surplus, an unknown u8 payload
   not subject to the ICC cap, and zero-count owners returning spare capacity.
   Compare actual/pending/cursor before and after every rejected candidate.
4. Deny each real output allocation. For four planes and two unknown payloads,
   the rich inventory has27 heap requests: four samples, two outer collections,
   twelve layout/role owners and nine metadata owners. Exercise every key/kind/
   count/ordinal separately, plus the existing inline controls (no allocator).
   Arm denial only around the selected real OrdinaryMaker reserve; a maker that
   merely returns a fabricated Allocation error is not this proof. Do not select
   targets solely by allocation size. Keep one allocator with thread-local,
   RAII-restored observation/denial state and no observation allocations inside
   allocator callbacks. Diagnostic strings are not output owners.
5. Observe unwind and retry. Register actual output pointer/layout/key identities
   in a fixed bounded observer, including nested metadata payloads. At the real
   wrapper boundary after inner owners drop and before checkpoint restoration,
   every registered partial-output allocation must be deallocated exactly once;
   borrowed source pointers/content/capacities must remain unchanged. Observe
   ledger snapshots at safe call sites, not through aliased mutable references
   in GlobalAlloc. Verify rejected candidates were not populated or reallocated;
   never inspect uninitialized structural bytes. Restore actual, pending, cursor
   and optional ledger borrows together. Clear the failure guards and retry the
   same PreparedConversion, preserving the prepared transform/record and the
   single-publication contract. Final output and actual ownership counters must
   match the successful reference and complete with pending0.
6. Close only the proven matrix. Repeat observer cleanup/thread-isolation and
   failure-then-success controls, then run unchanged B1/B2, public35, AVIF owner/
   candidate/ICC and feature-isolation regressions. Preserve the original
   assertions when adapting test module paths or cfg wiring. Count shared
   included tests only once; list unrun rows explicitly. C is complete only when
   every real denial and exact/one-under row has evidence. It does not complete
   other H4 routes, ICC oracle/intent work, native decoding or overall highres.

#### First C slice and metadata-boundary qualification

Start with C1 inventory/admission only: freeze and connect the independent rich
Q/M/S inventory, observe entry actual frame/metadata0 and live S with pending
Q/M, then use the lean source-fitting Gray control for frame Q/Q-1 and live
S+Q/S+Q-1. Do not implement actual-capacity surplus or the full real-denial matrix
in this first slice. Successful controls use the real PreparedConversion and
completion walk; no second accounting implementation is added to production.

The metadata M-1 row needs a distinct private admission-only control: retained
source metadata is at least its fresh-clone M, so the same public limit M-1
necessarily rejects that source before output admission. Non-spare input does
not remove this inequality. Keep the valid public early-refusal test, but do not
count it as output admission evidence. For the private ledger/admission proof,
retain measured source-live S and explicitly identify the independently selected
output-only metadata ceiling; it is not a public end-to-end source-fitting case.
This qualification supersedes any implication above that the positive M-1 public
row can reach output admission while its unchanged source fits the same limit.

#### Final cfg confirmation and C1 start boundary

Independent confirmation of allocation fingerprint `1DB15DA8` passes the
no-default highres product lib-test build with only the two existing draw
warnings, and AVIF+highres check with only the known reader warning. No B2
allocation warning appears in these targets. This closes the temporary cfg/lint
findings above for the reviewed targets; it does not claim a whole-workspace
strict-Clippy result. Diff checking passes and the reviewer makes no product
changes.

C1 may proceed with exactly these test-only deliverables:

- `highres/output_plan_tests.rs`: independent rich Q/M/S inventory and admitted
  entry assertions (actual frame/metadata0, actual live S, pending frame Q and
  metadata M). Keep any shared fixture in a small cfg(test)-only support module.
- `highres/convert_execute_tests.rs`: lean Gray frame Q/Q-1 and live S+Q/S+Q-1
  pairs using real PreparedConversion. Verify source validation separately,
  rejection before the first maker, and successful completion/counter agreement.
- Independent review harnesses: retain the existing pending fixture/inventory
  and B1/B2 probes; add C1-specific tests without weakening previous assertions
  or making product plan totals their oracle.

Only those inventory, entry and frame/live admission pairs are this first
implementation slice. Metadata M/M-1 is a separate private admission-only proof,
not the public source-rejection path and not part of this first slice. Actual
capacity surplus, full real-denial/unwind matrices, new conversion routes and
public API/dependency/version changes remain outside C1.

#### C1 resumed test-only review: acceptance still pending

The resumed Gray test now uses the real PreparedConversion, checks initial
actual frame/metadata0 and pending Q/M, and observes completed actual frame,
metadata, live and cursor state. Its focused test passes. Independent current-
runtime probes also pass for rich inventory/completion, Gray linear admission,
and Gray+nclx admission with the constructor's separate active/source-colour
owners. The combined review target passes41 tests; its included product tests
and diagnostic test are not new independent proofs. The independent subsets are
C1 inventory/admission3, prior B1 tests4 and prior B2 tests7. The prior public
route/execution/owner/record tests pass35 unique cases.

This snapshot is not accepted as the tracked C1 deliverable yet:

- The tracked rich inventory/admitted-entry test is absent; the output-plan
  test file currently adds only the accounting snapshot helper to its old tests.
- Each constrained Gray source must independently pass its selected limits
  before testing output admission; that separate validation is still missing.
- The refusal maker is created inside the successful preparation closure. When
  preparation refuses admission, the zero-call assertion is skipped. Create the
  maker outside that composed operation and assert its count after either result.
- Active provenance capacity in the expected source inventory is assumed to be8.
  Record the fixture's actual capacity before transferring ownership instead.

Runtime fingerprints remain unchanged: execute `734C6627`, output plan
`41BB2027`, metadata `834B4A5C`, and allocation `1DB15DA8`. Repair only these
test-coverage gaps before re-review; metadata M-1 and the later actual-capacity/
real-denial matrix remain outside this first C1 slice.

The next test-only revision resolves the Gray source-fit and maker-observation
gaps. Its source formula now reads the fixture's actual provenance capacity;
the assertion of capacity8 is only a check of the deliberately requested fixture
shape. A cfg(test)-only metadata capacity accessor supports this observation.
The two focused C1 tests and the combined42-test target pass. Acceptance remains
pending for the rich inventory: the newly named rich fixture is still only
Gray+nclx, with the same owners as the lean control. It does not cover the already
specified RGBA, ICC, unknown-colour payloads, geometry and pixi owner inventory.
Transfer that existing rich proof into tracked tests without weakening its
independent Q/M/S calculation; no new runtime behavior is requested.

#### C1 final independent review: finite GO

The final tracked revision closes the rich-fixture gap: the five-pixel RGBA
fixture now covers ICC, two unknown-colour payloads, geometry and pixi, with an
independent inventory of requested Q/M and actual borrowed-source S. The Gray
controls separately prove source-fit and refusal before the first maker. The
rich successful control uses the real PreparedConversion and completion walk.
The four focused C1 tests pass; the independent combined target passes44 tests.
That combined count includes shared product/regression tests and must not be
reported as44 new independent C1 proofs. The root no-default highres all-targets
run also passes. This is a test-only finite GO: runtime behavior is unchanged,
and capacity observation additions are cfg(test)-only. It does not complete C2,
the private metadata M/M-1 admission proof, real-allocation denial/unwind,
remaining H4 routes, ICC oracle gates or overall highres.

#### H4 C2a: next bounded actual-capacity proof

Only requested-versus-actual output accounting is authorized for this next
slice. Preserve C1 and add a packed control with the same logical rich metadata:
five-pixel RGBA, ICC length4093, two distinct unknown payloads of length13,
geometry, pixi, nclx/AV1 and timing. Use the existing explicit CICP(1,8,0,true)
to linear-relative sRGB route; ICC remains retained metadata, not an executed
profile. Keep the active descriptor colour set empty. The existing C1 fixture
with ICC spare disabled is not a packed control: its other owners still have
spare capacity. Measure all actual source capacities and assert source validation
under every selected public limit before claiming an output boundary.

Derive Q/M/S independently from type sizes, lengths and measured capacities.
Admission starts at actual frame/metadata0, live S, pending Q/M and cursor0.
For an accepted owner, actual counters gain its actual capacity, pending loses
only its requested bytes and cursor advances once. Metadata owners affect all
three actual counters; ordinary owners affect frame/live only. Future owners
remain pending during each check. A rejected candidate must leave the complete
pre-call actual/pending/cursor state unchanged and be dropped before return.

The C2a boundary rows are:

- Sample(0), five F32 values: request20, actual32 bytes. Prove frame Q+12/Q+11
  and live S+Q+12/S+Q+11 separately; isolate max-plane32/31 with other limits
  ample. After acceptance the first-owner state is actual(32,0,S+32),
  pending(Q-20,M), cursor1.
- ICC request4093, actual4160: prove the +67 exact/one-under frame, live and
  metadata rows separately. Isolate max-ICC4160/4159 with source ICC capacity4093.
- Legal provenance surplus, length3/capacity5: charge the additional two element
  sizes to metadata/frame/live. UnknownPayload(1), length13/capacity4160, must
  remain legal under max-ICC4093 when all other budgets allow it.
- An empty coded-geometry owner returning capacity1 must charge that actual
  capacity despite its zero requested bytes and still advance its owner cursor.

Use the existing CandidateMaker seam, returning fresh empty vectors and measuring
their actual capacity; all untargeted owners use OrdinaryMaker. Reuse typed keys,
not byte-size-only selection. Success must pass the real materializer and
completion walk, end with pending0/cursor31 for the rich fixture, and agree with
an independent actual-capacity ownership walk. Refusal is ResourceLimit; do not
inspect uninitialized structural bytes. Test-only changes belong in the existing
output-plan/execution tests and minimal cfg(test) observation support. Do not
change runtime accounting, conversion routes, public APIs, dependencies or
versions in this slice; an exposed runtime defect requires separate review.

C2a remains unverified until its focused independent boundary checks and the
unchanged B1/B2/C1, feature-isolation and relevant regression gates pass. Selected
real OrdinaryMaker reserve denial, allocator registry expansion, drop-before-
restore/retry proofs, the full27-owner denial matrix and private M/M-1 admission
remain later slices. No C2 completion or publication is implied by C1's finite GO.

##### C2a independent review: finite GO

The focused `c2_` set passes 8/8 and the no-default `highres` library set passes
48/48. The packed source, independent Q/M/S inventory, sample and ICC exact/
one-under rows, provenance and unknown-payload surplus, zero-count geometry and
the real prepared completion walk satisfy the bounded C2a contract. Changes for
this slice are test-only, apart from minimal `cfg(test)` observation accessors;
runtime accounting, conversion routes, public APIs, dependencies and versions
are unchanged. C2b real allocator denial/unwind and C2c's complete owner matrix
have not started, so this is not overall C2 or H4 acceptance.

##### C2b independent review: finite GO

The focused `c2b_` set passes 3/3 and the neighbouring `c2_` set passes 8/8.
Real one-shot allocation denial is proven for a partial sample prefix and for
the late second unknown-colour payload. At the restore boundary, every
registered successful owner has already been deallocated exactly once, the
denied candidate was never registered, the complete frame/metadata/live,
pending and cursor snapshot equals its pre-call state, and the borrowed source
owner pointers, lengths, capacities and values remain unchanged. Retrying the
same PreparedConversion produces a full `ImageFrame`-equal reference result and
preserves single-publication behavior.

A separately spawned and barrier-synchronized worker confirms that the
one-shot allocator denial is thread-local: arming the main thread does not
affect the worker conversion, only the next main-thread allocation is denied,
and guard cleanup permits the following allocation. The allocator and
deallocation observers, source-capacity accessors and new assertions are all
`cfg(test)`-only. Runtime conversion behavior is unchanged. The Miri component
is not installed in this environment, so C2b Miri was not run and is not
claimed. C2c's complete owner matrix has not started; this remains a finite C2b
GO, not overall C2 or H4 acceptance.

##### C2c-1 proposed first finite owner-matrix slice

Do not start with all27 owners. The first C2c slice should close only the five
non-metadata structural allocation keys for plane0 of the packed RGB control:
`DescriptorOuter`, `PixelOuter`, `DescriptorLayout { plane: 0 }`,
`Role { plane: 0 }` and `PixelLayout { plane: 0 }`. Samples remain covered by
C2b's real-denial proof, while ICC/unknown payload metadata stays outside this
slice. Each row must assert its typed key and logical element count, exactly one
real allocator denial, no registration for the denied candidate, exact
deallocation of every successful prefix owner before restore, equality of the
full accounting checkpoint, comprehensive source-owner immutability, and an
`ImageFrame`-equal retry/reference result from the same PreparedConversion.

Add two non-denial controls rather than pretending that every owner event owns
heap memory:

- Zero-count gate: use an empty coded-geometry event and an allocation counter
  to prove that cursor advancement and zero pending-byte consumption perform no
  allocator call. Do not arm a denial inside a zero-count maker and attribute a
  later denial to that key.
- Inline gate: cover `SourceNclx` first. Assert that it consumes its exact inline
  metadata/frame/live bytes and advances the cursor without invoking
  CandidateMaker or registering an allocation. The remaining inline AV1 and
  dimension events stay for later matrix slices.

Run the existing spawned-thread C2b isolation test unchanged as the thread gate
for C2c-1; do not multiply thread tests per key. Run the existing cleanup probe
after all rows so no one-shot denial or observer registry leaks between tests.
Success criteria for this slice are the five structural denial rows, one
zero-count row, one inline row, unchanged C2b 3/3 and C2a `c2_` 8/8. Do not add
public APIs, runtime allocator hooks, conversion routes, dependencies or
versions. Plane1/2 repetitions, sample-key completion, all metadata allocation
keys, the other inline events, the private M/M-1 proof and the complete27-owner
matrix remain explicitly unstarted after C2c-1.

##### C2c-1 independent review: finite GO

The current test-only snapshot satisfies the bounded C2c-1 contract. The five
structural denial rows record each successful owner's exact ordered
`(key, logical count, ordinal)` prefix, prove that the denied request has its
expected key/count/ordinal but is absent from the fixed no-allocation registry,
and observe every registered owner deallocated exactly once before the complete
accounting checkpoint is restored. The borrowed source remains unchanged and
the same `PreparedConversion` retries to an `ImageFrame`-equal reference.

The zero-count coded-geometry and inline `SourceNclx` controls advance their
owner cursors and update only the specified accounting without allocator or
registry activity. Fresh independent runs pass `c2c1_` 7/7, unchanged `c2b_`
3/3 and unchanged C2a `c2_` 8/8. The C2c-1 test file passes focused rustfmt and
the repository diff whitespace check passes. This is finite C2c-1 GO only;
runtime behavior, public APIs, dependencies and versions are unchanged.

##### C2c-2 proposed plane1-3 structural-owner slice

Keep the same packed RGBA control and repeat all seven C2c-1 tests. Add only the
nine structural allocation denials for the remaining output planes. With the
four sample owners at ordinals0-3, `DescriptorOuter` at4, `PixelOuter` at5 and
the three plane0 owners at6-8, the new denied requests are:

- `DescriptorLayout { plane: 1 }`: key4, count1, ordinal9;
  `Role { plane: 1 }`: key5, count1, ordinal10;
  `PixelLayout { plane: 1 }`: key6, count1, ordinal11.
- `DescriptorLayout { plane: 2 }`: key4, count1, ordinal12;
  `Role { plane: 2 }`: key5, count1, ordinal13;
  `PixelLayout { plane: 2 }`: key6, count1, ordinal14.
- `DescriptorLayout { plane: 3 }`: key4, count1, ordinal15;
  `Role { plane: 3 }`: key5, count1, ordinal16;
  `PixelLayout { plane: 3 }`: key6, count1, ordinal17.

For every row, assert the typed `OwnerKey` as well as its compact registry key,
logical count and ordinal. The registered owners must be exactly the complete
ordered successful prefix before that request; the denied request must not be
registered. Preserve the C2c-1 requirements for one real denial, exact-once
prefix deallocation before restore, full checkpoint equality, comprehensive
source-owner immutability, cleanup with no TLS/registry leak, and an equal retry
from the same `PreparedConversion`.

Do not add another zero-count or inline case in C2c-2. Rerun the existing empty
coded-geometry and `SourceNclx` controls unchanged; `SourceAv1`, coded/render
dimensions and all metadata-allocation owners remain later slices. Samples
remain covered by C2b and plane0 remains covered by C2c-1.

The bounded gate is the nine new denial rows plus C2c-1 7/7, C2b 3/3 and C2a
`c2_` 8/8, including the existing spawned-thread isolation and cleanup probes.
Also require the no-default `highres` library regression, focused rustfmt for
every changed Rust test file and the diff whitespace check. Keep changes in
tests and minimal `cfg(test)` observation support only. Do not change runtime
accounting, conversion routes, public APIs, dependencies or versions. Metadata
owners, remaining inline events, private M/M-1 admission and the complete
27-owner matrix remain explicitly unstarted after C2c-2.

##### C2c-2 independent review: finite GO

The bounded plane1-3 slice is complete. All nine denial rows pass and compare
the typed `OwnerKey` in addition to the compact registry key, logical count and
ordinal, so the repeated compact keys cannot hide a plane-identity collision.
Each row uses a real allocator denial, excludes the denied candidate from the
registry, observes the exact ordered prefix deallocated once before restore,
restores every ledger counter and the owner cursor, preserves the complete
borrowed source, and retries the same `PreparedConversion` to the reference
result. The shared spawned-thread denial and cleanup probes remain unchanged.

Fresh focused runs pass C2c-2 9/9, C2c-1 7/7 (including the zero-count and
inline controls) and C2b 3/3. The C2c-owned Rust test file passes focused
rustfmt and the repository diff whitespace check passes. Remaining whole-tree
rustfmt output is confined to pre-existing WIP files outside this bounded
slice. This is finite C2c-2 GO only; runtime behavior, public APIs,
dependencies and versions are unchanged.

##### C2c-3 proposed geometry and pixel-information owner slice

Keep the same packed RGBA fixture and add only the four heap-owning metadata
denials immediately after the 18 output sample/structural owners:

- `CodedGeometry`: key7, count2, ordinal18.
- `RenderGeometry`: key8, count1, ordinal19.
- `PixiBits`: key9, count4, ordinal20.
- `PixiExtended`: key10, count4, ordinal21.

For each row, compare the denied request's typed `OwnerKey`, compact key,
logical count and ordinal. The fixed no-allocation registry must contain
exactly the complete ordered successful prefix and must not contain the denied
candidate. Require one real allocator denial, exact-once prefix deallocation
before restore, exact restoration of frame/metadata/live counters and owner
cursor, complete source-owner immutability, an equal retry from the same
`PreparedConversion`, and cleanup with no TLS denial or registry leak.

Do not add new zero-count or inline fixtures. Rerun the existing empty
`CodedGeometry` and `SourceNclx` controls unchanged, including their proof that
no candidate allocation or registry entry occurs. Rerun C2c-1 7/7, C2c-2 9/9,
C2b 3/3 and the unchanged C2a `c2_` 8/8 matrix, including the single existing
spawned-thread isolation test and cleanup probe. Also require the no-default
`highres` library regression, focused rustfmt for every changed Rust test file
and the diff whitespace check.

This slice explicitly excludes `Provenance`, `IccProfile`, `UnknownOuter` and
all `UnknownPayload` owners, as well as `SourceAv1`, coded/render dimension
inline events and the private metadata M/M-1 admission boundary. Those owners
must not be folded into C2c-3 merely to extend the ordinal prefix. Do not
change runtime accounting, conversion routes, public APIs, dependencies or
versions; implementation remains tests plus minimal `cfg(test)` observation
support only.

##### C2c-3 independent review: finite GO

The bounded geometry and pixel-information slice is complete. The four real
allocation-denial rows observe exactly `CodedGeometry` key7/count2/ordinal18,
`RenderGeometry` key8/count1/ordinal19, `PixiBits` key9/count4/ordinal20 and
`PixiExtended` key10/count4/ordinal21. Each denied candidate is absent from the
fixed registry while the complete ordered successful prefix is registered and
deallocated exactly once before the full frame/metadata/live/pending/cursor
checkpoint is restored. The borrowed source owners remain pointer-, content-
and capacity-identical, and the same `PreparedConversion` retries to the
reference `ImageFrame`.

Fresh independent runs pass C2c-3 4/4, C2c-2 9/9, C2c-1 7/7, C2b 3/3 and the
unchanged C2a matrix 8/8. The no-default `highres` library passes 71/71,
including the existing spawned-thread denial-isolation and cleanup controls.
The C2c-owned Rust test file passes focused rustfmt and the repository diff
whitespace check passes. Other dirty files are pre-existing WIP outside this
bounded review. This is finite C2c-3 GO only; the excluded metadata owners,
inline events and M/M-1 boundary remain unfinished, and runtime behavior,
public APIs, dependencies and versions are unchanged.

##### C2c-4 proposed remaining heap-metadata owner slice

Keep the same packed RGBA fixture and the accepted owner prefix through
`PixiExtended` ordinal21. Cover the five remaining heap-metadata requests in
their exact admitted order:

- `Provenance`: key11, count3, ordinal22.
- `IccProfile`: key12, count4093, ordinal23.
- `UnknownOuter`: key13, count2, ordinal24.
- `UnknownPayload(0)`: key0x40, count13, ordinal25.
- `UnknownPayload(1)`: key0x41, count13, ordinal26.

Add four new C2c-4 denial rows for ordinals22-25. The existing C2b
`UnknownPayload(1)` real-denial row already proves ordinal26 with the complete
ordered prefix through payload0, so retain and count that row as the fifth
C2c-4 owner proof instead of duplicating it. It remains part of the unchanged
C2b 3/3 regression as well. For every row, compare the typed `OwnerKey`, compact
key, logical count and ordinal; exclude the denied candidate from the fixed
registry; observe every successful prefix owner deallocated exactly once before
restore; restore all accounting and cursor state; preserve every borrowed
source owner; and retry the same `PreparedConversion` to the reference result.

Keep the ICC limit semantically distinct from general metadata ownership. The
real `IccProfile` denial must run with `max_icc_bytes` large enough to admit its
logical count so the observed error is the allocator denial. Rerun the existing
ICC-capacity boundary proving that ICC requested/actual capacity is governed by
`max_icc_bytes`, and rerun `c2_unknown_payload_capacity_is_not_limited_by_max_icc`
to prove that either unknown payload is governed by frame/live/metadata limits,
not by the ICC limit. Do not relabel an ICC limit rejection as an allocation
denial or reuse the ICC key for unknown payloads.

The bounded gate is the four new C2c-4 rows plus the shared existing payload1
row, C2c-3 4/4, C2c-2 9/9, C2c-1 7/7, C2b 3/3 and C2a 8/8. Preserve the single
spawned-thread isolation test, cleanup probe, no-default `highres` library
regression, focused rustfmt and diff whitespace checks. Do not add new
zero-count or inline fixtures. `SourceAv1`, coded/render dimension inline
events, the private metadata M/M-1 admission boundary and the complete
transaction matrix remain explicitly excluded after C2c-4. Do not change
runtime accounting, conversion routes, public APIs, dependencies or versions;
implementation remains tests plus minimal `cfg(test)` observation support only.

##### C2c-4 independent review: finite GO

The bounded remaining heap-metadata slice is complete. The four new rows match
the admitted sequence exactly: `Provenance` key11/count3/ordinal22,
`IccProfile` key12/count4093/ordinal23, `UnknownOuter` key13/count2/ordinal24
and `UnknownPayload(0)` key0x40/count13/ordinal25. The unchanged C2b
`UnknownPayload(1)` row supplies key0x41/count13/ordinal26 with the complete
prefix through payload0, so the five remaining heap requests form one
continuous proof rather than overlapping fixtures.

Each real denial returns `Allocation`, leaves the denied candidate outside the
fixed registry, observes every successful prefix owner deallocated exactly once
before restore, restores the complete accounting/cursor snapshot, preserves the
source owners and retries the same `PreparedConversion` to the reference result.
The ICC row runs at logical `max_icc_bytes=4093`, so its 4093-byte request is
admitted before the real allocator denial. The separate actual-capacity boundary
still rejects a 4160-byte ICC candidate at limit4159 as `ResourceLimit`, while
the unknown-payload boundary admits its 4160-byte capacity with
`max_icc_bytes=4093`; ICC and general metadata limits therefore remain distinct.

Fresh independent runs pass C2c-4 4/4, shared C2b 3/3, C2c-3 4/4, C2c-2 9/9,
C2c-1 7/7 and the unchanged C2a eight-test inventory. The no-default `highres`
library passes75/75, including the single spawned-thread TLS isolation and
cleanup probe. Focused rustfmt for the C2c-owned test file and the repository
diff whitespace check pass. This is finite C2c-4 GO only; inline events, the
private metadata M/M-1 boundary and the complete transaction matrix remain
unfinished. Runtime behavior, public APIs, dependencies and versions are
unchanged.

##### C2c-5 proposed remaining inline-event slice

Keep the packed RGBA fixture and all accepted heap-owner proofs unchanged. Add
only three table-driven inline rows after the existing heap prefix and
`SourceNclx` event:

- `SourceAv1`: inline key16, byte count `AV1_COLOR_INFORMATION_BYTES` (currently
  8), ordinal28.
- `CodedDimensions`: inline key17, byte count8, ordinal29.
- `RenderDimensions`: inline key18, byte count8, ordinal30.

For each row, consume the exact admitted prefix through the immediately
preceding event, then snapshot pending frame/metadata bytes, current
frame/metadata/live accounting and owner cursor. `commit_metadata_event` must
accept only the expected typed key and exact byte count, advance the cursor by
one, subtract exactly that byte count from both pending counters and add it
exactly once to frame, metadata and live accounting. Checkpoint restoration must
return every scalar and the cursor to the pre-event snapshot; recommitting the
same event must reproduce the same result.

Inline commitment must not call the allocator or register a heap owner. Arm the
existing one-shot real-allocation denial around each commit, verify the commit
succeeds with zero observed allocations and an unchanged empty registry, then
prove the denial remains armed by consuming it with a separate one-byte reserve.
Drop the guard and run the ordinary cleanup reserve successfully. Do not create
new heap candidates or another inline fixture. Rerun the accepted C2c-1
`SourceNclx` zero-allocation/accounting test as the preceding inline control.

The bounded gate is C2c-5 3/3, C2c-4 4/4 plus the shared C2b payload1 row,
C2c-3 4/4, C2c-2 9/9, C2c-1 7/7, C2b 3/3 and C2a 8/8. Preserve the single
spawned-thread TLS isolation test, cleanup probe, no-default `highres` library
regression, focused rustfmt and diff whitespace checks. The private metadata
M/M-1 admission boundary and the complete transaction matrix remain explicitly
excluded after C2c-5. Do not change runtime accounting, conversion routes,
public APIs, dependencies or versions; implementation remains tests plus
minimal `cfg(test)` observation support only.

##### C2c-6 proposed remaining sample-owner slice

Keep the same five-pixel packed RGBA fixture and every accepted C2 test
unchanged. Add only the three missing real-allocation denial rows for sample
owners0,1 and3. Their admitted identities and exact successful prefixes are:

- `Sample(0)`: compact key `0x10`, count5, ordinal0, with an empty prefix.
- `Sample(1)`: compact key `0x11`, count5, ordinal1, with prefix
  `[(0x10, 5, 0)]`.
- `Sample(3)`: compact key `0x13`, count5, ordinal3, with prefix
  `[(0x10, 5, 0), (0x11, 5, 1), (0x12, 5, 2)]`.

Do not duplicate or rename the accepted C2b `Sample(2)` row. It already proves
typed key `Sample(2)`, compact key `0x12`, count5 and ordinal2 after the exact
prefix `[(0x10, 5, 0), (0x11, 5, 1)]`; retain it unchanged as the fourth sample
owner proof. Together, these three new rows and that shared C2b row close only
the four sample-owner entries of the27-request real-denial inventory.

Use the existing real-denial helper and the same PreparedConversion fixture.
For each new row, assert the typed `OwnerKey`, compact key, logical count and
ordinal; exactly one real allocator denial; absence of the denied candidate
from the fixed registry; and registry equality with the complete ordered prefix.
Every registered prefix owner must be deallocated exactly once before the full
frame/metadata/live/pending/cursor checkpoint is restored. Preserve all borrowed
source pointers, lengths, capacities and values, then retry the same
PreparedConversion to an `ImageFrame`-equal reference and retain the existing
single-publication and cleanup behavior. The empty-prefix Sample0 row must
explicitly observe zero registered/deallocated prefix owners rather than skip
the restore-boundary assertions.

No new per-plane actual-capacity case is required in this slice. All four sample
owners use the same F32 count5 allocation/accounting path; C2a already proves
the requested20-to-actual32 reconciliation, exact/one-under frame/live headroom
and max-plane32/31 boundary on `Sample(0)`, while the existing prepared completion
walk exercises all sample owners. Nevertheless, rerun the unchanged C2a `c2_`
8/8 gate, including `c2_sample_capacity_uses_frame_live_tight_limits_and_plane_cap`,
`c2_actual_capacity_deltas_are_charged_from_independent_prefixes` and
`c2_real_prepared_conversion_walks_actual_capacity_and_completes`; this is a
regression requirement, not authorization to add duplicated Sample1/3 surplus
rows or alter actual-capacity accounting.

The bounded implementation gate is new C2c-6 3/3 plus unchanged C2b 3/3,
C2c-1 7/7, C2c-2 9/9, C2c-3 4/4, C2c-4 4/4, the three logical C2c-5 inline rows
in their single table-driven Rust test, and C2a 8/8. The combined `c2`-named Rust
test inventory should therefore be39/39, counting the C2c-5 table as one test.
Also preserve the single spawned-thread TLS-isolation proof, registry/denial
cleanup, no-default `highres` library regression, focused rustfmt for the changed
test file and the repository diff whitespace check. Do not change runtime code,
accounting, conversion routes, public APIs, dependencies or versions; no new
observation accessor should be necessary.

This finite slice excludes private metadata M/M-1 admission, any new actual-
capacity/surplus matrix, the full source/actual/pending/cursor transaction matrix
and Miri. Passing C2c-6 completes the27 heap-request denial inventory only; it
does not complete overall C2, H4, ICC integration or `highres` acceptance.

##### C2c-6 independent review: finite GO

The three missing sample-owner rows complete the bounded real-denial inventory.
`Sample(0)` is observed as typed key `Sample(0)`, compact key `0x10`, count5 and
ordinal0 with an explicitly empty registry: zero registered prefix owners and
zero prefix deallocations at the restore boundary. `Sample(1)` is key `0x11`,
count5 and ordinal1 after exactly `Sample(0)`. `Sample(3)` is key `0x13`, count5
and ordinal3 after exactly samples0,1 and2. The unchanged C2b `Sample(2)` row
remains the fourth sample proof and is not duplicated.

Each new row consumes exactly one real allocator denial, never registers the
denied candidate, and keeps the fixed registry equal to the complete ordered
successful prefix. Every registered owner is deallocated exactly once before
the full frame/metadata/live/pending/cursor checkpoint is restored. All borrowed
source pointers, lengths, capacities and values remain unchanged. Retrying the
same `PreparedConversion` produces an `ImageFrame` equal to the reference,
subsequent publication is rejected, and the TLS denial/registry cleanup probe
succeeds.

Fresh independent runs pass C2c-6 3/3, unchanged C2b 3/3, the exact C2a eight-
test inventory 8/8, combined `c2` 39/39 and the no-default `highres` library
79/79. Focused rustfmt for `convert_execute_tests.rs` and the repository diff
whitespace check pass. The only library diagnostics are the two pre-existing
no-default draw warnings. This is finite C2c-6 GO only: it closes the27 heap-
request denial inventory but not private metadata admission, the complete C2
transaction matrix, H4, ICC integration or overall `highres` acceptance.

##### C2d proposed private metadata M/M-1 admission boundary

Implement C2d only as the qualified private admission proof described above.
Reuse the packed rich source and its independent C2 inventory. Let `M` be the
fresh output metadata ownership computed by that inventory from public/test-
visible type sizes, logical lengths and source values; do not read the product
plan total back into the expected value. Let `Q` and `S` remain the independently
computed requested output and borrowed-source live totals. Before entering the
private seam, run a separate public control proving that the source fits a
source-appropriate metadata/frame/live limit. Also explicitly show that a public
end-to-end limit of `M-1` rejects the retained source first. That rejection is a
source-validation control and must never be counted as output-admission evidence.

Construct the same `OutputOwnershipPlan` under a generous, source-fitting limit,
then exercise only its existing private admission seam with separately bounded
`ConstructionLedger` instances. Seed each ledger with actual frame0, metadata0
and live-only `S`; keep frame and live ceilings sufficient for `Q` and `S+Q` so
metadata is the sole changing boundary. With the private metadata ceiling set
to exactly `M`, `admit` must succeed without allocating and the existing
`c1_accounting_snapshot` must report actual frame0, actual metadata0, actual
live `S`, pending `(Q, M)` and cursor0. With an otherwise identical ceiling of
`M-1`, the same already-inspected plan must fail in `admit` as
`ProcessingError::ResourceLimit` before any `CandidateMaker` or output owner is
created. This exact/one-under pair is the lower-bound proof for pending output
metadata; it is deliberately not a public conversion path because the unchanged
source cannot fit the same `M-1` metadata ceiling.

Use only current test-private components: `OutputOwnershipPlan::inspect`,
`ConstructionLedger::new_with_ownership`, `OutputOwnershipPlan::admit`, the
independent `c2_rich_inventory`, `c2_source_snapshot`/source validation controls,
and `AdmittedOutputPlan::c1_accounting_snapshot`. The plan and admitted types are
already private to `highres`; tests in `output_plan_tests.rs` can exercise them
without a public accessor. No new allocator, builder, accounting engine or
runtime hook is authorized. If an allocation observation is desired, reuse the
existing cfg(test) allocation guard; do not add another global allocator or
observer. The successful exact-M admission need not materialize output because
the accepted C2 completion walk already covers construction and C2d is only an
admission boundary.

The bounded implementation gate is one tracked table/pair proving public source
fit and public M-1 early rejection separately from private exact-M success and
private M-1 refusal. Rerun C2d focused tests, combined `c2` 40/40 if represented
as one additional Rust test, unchanged C2c-6 3/3, C2b 3/3, C2a 8/8, the no-
default `highres` library, focused rustfmt for the changed test file and the
repository diff whitespace check. Preserve the spawned-thread TLS proof and all
27 real-denial rows unchanged.

C2d must not change runtime accounting, conversion routes, product visibility,
public APIs, dependencies or versions. Actual-capacity/surplus additions, new
failure owners, materialization retry changes, the full source/actual/pending/
cursor transaction matrix, Miri, H4 route completion and ICC integration remain
explicitly excluded after this slice.

##### C2d independent review: finite GO

The qualified metadata admission boundary is complete. The packed rich source
and `c2_rich_inventory` derive S, M and Q from test-visible source capacities,
logical lengths and public owner type sizes; the expected values are not read
back from `OutputOwnershipPlan`. A public source control fits at metadata M,
while public M-1 returns `ResourceLimit` during retained-source validation. That
early refusal remains explicitly separate from output admission evidence.

The same already-inspected, copyable private plan is then admitted with two
independent ledgers seeded at actual frame0, metadata0 and live S. Exact M
succeeds and reports `(frame=0, metadata=0, live=S, pending=(Q,M), cursor=0)`.
Private M-1 returns `ResourceLimit` from `admit`; that API has no candidate maker
or output-owner construction path, and `reserve_output` commits pending values
only after every checked boundary succeeds. Source pointer, length, capacity,
value, metadata and timing snapshots remain unchanged after the public refusal,
exact admission and private refusal.

Fresh independent runs pass focused C2d 1/1, combined `c2` 40/40 and the no-
default `highres` library 80/80. Focused Rust formatting for
`output_plan_tests.rs` and the repository whitespace diff check pass. The only
library warnings are the two pre-existing no-default draw diagnostics; the
filtered non-test build also reports the already documented unwired native-reader
diagnostic. No runtime code, public API, dependency or version changes belong to
this finite GO. The actual-capacity transaction matrix and Miri remain open.

##### C2e transaction-matrix roadmap and bounded first slice (design only)

C2e must not repeat the completed denial matrix. All 27 heap requests already
have a real allocator-denial row with typed key/count/ordinal, complete successful
prefix ownership, exact-once prefix deallocation before restore, complete
frame/metadata/live/pending/cursor restoration, borrowed-source preservation and
same-`PreparedConversion` retry. This includes samples0-3, descriptor/pixel outer
owners, all four descriptor-layout/role/pixel-layout triples, coded/render
geometry, pixi bits/extended, provenance, ICC, unknown outer and both indexed
unknown payloads. The four inline metadata events have allocator-free atomic
commit/restore/recommit coverage, and empty coded geometry has its zero-count
cursor/accounting control. These accepted rows are regressions, not work to
reimplement or multiply into another all-owner table.

The missing transaction evidence is narrower: a candidate allocation succeeds,
but its actual capacity exceeds the requested ownership and a real frame/live/
metadata or per-owner ceiling rejects that candidate. Existing tests prove the
scalar boundaries for Sample0 request20 -> actual32, ICC4093 ->4160,
provenance3 ->5, unknown payload1 length13 -> capacity4160 and empty coded
geometry count0 -> capacity1. A separate real Prepared completion walk accepts
the four nonzero surpluses together. Those tests do not yet prove for every such
rejection that the allocated surplus candidate drops before the outer checkpoint
is restored, that all earlier output owners drop exactly once, or that the same
Prepared object can retry without stale actual/pending/cursor state.

Close that gap in bounded slices rather than an unsafe all-at-once matrix:

| Slice | New transaction evidence | Existing evidence retained, not duplicated |
| --- | --- | --- |
| C2e-1 | Sample0 request5 f32 /20 bytes returning capacity8 /32 bytes. Exact +12 success, then independent frame Q+11, live S+Q+11 and max-plane31 refusals. | C2a scalar Sample0 boundaries and C2c-6 Sample0 real allocator-denial/retry. |
| C2e-2 | Late ICC request4093 returning capacity4160. Independent frame/live/metadata +66 and max-ICC4159 refusals after the exact preceding prefix. | C2a ICC scalar boundaries and C2c-4 ICC allocator-denial/retry. |
| C2e-3 | Provenance3 -> capacity5, unknown payload1 length13 -> capacity4160, and zero-count coded geometry -> capacity1, each against its applicable frame/live/metadata limits. | C2a positive/scalar controls and the corresponding C2c denial/retry rows. Unknown payload must remain outside max-ICC policy. |
| C2e-4 | Aggregate four-nonzero-surplus completion and one-under combined frame/live/metadata rows, using the fixed independent delta inventory and final actual ownership walk. | The existing positive Prepared completion walk; no second accounting engine or duplicated per-owner denials. |

Only C2e-1 is the next authorized implementation slice. Reuse the packed rich
source, independent Q/M/S inventory, existing Prepared wrapper, candidate-maker
seam, restore observer and source snapshot. Do not add a runtime hook, public
accessor, allocator, builder or ledger. The controlled maker performs an actual
fallible Sample0 allocation and returns capacity8 for the logical count5; it
must not fabricate `Allocation` or `ResourceLimit` before the candidate exists.
Register that candidate with the existing bounded deallocation observer.

First run one exact control with frame Q+12, live S+Q+12 and max-plane32. It must
complete with actual Sample0 bytes32, all later ownership still accounted, final
pending0/cursor complete and the independent final output walk equal to the
ledger. Then run three otherwise-ample one-under rows: frame Q+11, live S+Q+11
and max-plane31. Requested admission must succeed in every row. The first and
only maker request before refusal is typed `Sample(0)`, count5, ordinal0. The
capacity8 candidate is allocated, remains unpopulated, is never published, and
is deallocated exactly once before the complete initial snapshot
`(0,0,S,(Q,M),0)` is restored. The fixed owner registry has no successful prefix
for this ordinal0 case. Borrowed source pointers, capacities, contents, metadata
and timing remain unchanged. With the surplus maker cleared, the same Prepared
object must retry through the ordinary exact-count path to the reference frame,
then retain the single-publication rejection and cleanup behaviour.

Keep the three refusal authorities distinct: ample max-plane for frame/live
rows, ample frame/live for max-plane, and ample metadata/ICC/input/parser limits
for all rows. Assert the typed `ResourceLimit`; a real allocation denial is not
the requested event. Do not add Sample1-3 surplus cases: they use the same f32
owner path and already have complete ordinary allocation-denial transactions.
Do not start ICC, metadata surplus, zero-count or aggregate C2e work in C2e-1.

The C2e-1 bounded gate is its exact control plus the three rejection rows,
unchanged C2d1, combined C2 inventory, no-default `highres` library, the single
TLS isolation/cleanup proof, focused Rust formatting for changed test files and
the repository whitespace diff check. Report unique Rust test counts rather
than summing shared filters. No runtime accounting, conversion route, product
visibility, dependency or version change is permitted.

Before C2e-1 review, probe Miri availability explicitly with the selected
nightly toolchain and record the command result. If `cargo miri` is available,
run the focused C2e-1 transaction tests under Miri as well as the host tests;
do not disable pointer/provenance checks or the allocator observer to obtain a
pass. If the component/toolchain is unavailable, record that exact condition as
an open portability gate: host finite acceptance may be reported separately,
but neither Miri success nor complete C2 acceptance may be claimed. Installing
or changing a toolchain is not part of this docs-only design step.

##### C2e-1 independent result: host finite GO, Miri gate still NO-GO

The bounded host slice is accepted. The focused C2e-1 test passes 1/1, the
unique C2-filtered library inventory passes 41/41, and the complete no-default
`highres` library passes 81/81. Focused formatting of
`convert_execute_tests.rs` and the repository whitespace diff check also pass.
Only the two previously recorded no-default draw warnings remain in the library
test build.

The reviewed test uses a real fallible Sample0 allocation: count5 requests20
bytes and the controlled candidate has length0, capacity8 and32 allocated
bytes. The exact Q+12, S+Q+12, max-plane32 row completes all31 ownership events,
finishes with pending0, and its independent actual output walk equals the
ledger. Dropping that published frame deallocates the registered Sample0 owner
exactly once. The independent frame Q+11, live S+Q+11 and max-plane31 rows each
admit the requested plan, issue only typed `Sample(0)`, count5, ordinal0, reject
the still-empty capacity8 candidate as `ResourceLimit`, and observe that
candidate deallocated exactly once before the complete initial
`(frame0, metadata0, source-live S, pending(Q,M), cursor0)` checkpoint is
restored. The successful-prefix registry is correctly empty at ordinal0.
Complete borrowed-source pointers, lengths, capacities, contents, metadata and
timing remain unchanged. Each same `PreparedConversion` then retries through
the ordinary exact-count path to a separately produced reference frame and
rejects a second publication. This is finite C2e-1 host evidence only; C2e-2
through C2e-4 remain unimplemented.

Miri is installed as `miri 0.1.0 (bff8e12ff5 2026-08-26)`, but the Miri gate is
not accepted. The focused C2e-1 test itself reports `ok`; afterward, libtest
teardown fails under Stacked Borrows in the pre-existing test global allocator:
`CountingAllocator::dealloc` at `convert_native_alloc_tests.rs:91` delegates to
the Windows `System::dealloc`, whose high-alignment path reads its hidden header
immediately before the user pointer. An otherwise identical Miri run filtered
to a nonexistent test reproduces the same failure after `running 0 tests` while
dropping libtest's channel owner. That zero-test control proves the current
failure is independent of C2e-1 candidate execution and its observer state. It
does not make the failure ignorable: Miri success and complete C2 acceptance
remain open.

##### Bounded test-only Miri allocator compatibility repair

Do not obtain a green run by disabling Stacked/Tree Borrows, ignoring the
failure, leaking allocations, fabricating deallocation counters, skipping the
C2e-1 test, or compiling out all drop observation. The Windows `System`
allocator may use storage before a high-alignment returned pointer. Wrapping it
as the crate's custom `GlobalAlloc` causes the wrapper return retag to cover the
user layout but not that hidden header, so calling `System::dealloc` through the
wrapper is not a provenance-safe Miri strategy. Reimplementing a general
high-alignment allocator or a recursively allocating pointer map inside this
test module is outside the bounded repair and would add less trustworthy code
than the behavior being tested.

Use a two-part, test-only proof instead:

1. Keep the current `CountingAllocator` unchanged for ordinary host tests. It
   remains the authority for real allocation, exact pointer/layout identity and
   exact-once physical deallocation. Host C2e-1 and the existing complete denial
   matrix must continue to pass without weakened assertions.
2. Under `cfg(miri)`, do not install a test `#[global_allocator]`; Miri uses
   Rust's default allocator source directly. The host-only
   `CountingAllocator` remains unchanged as the physical observer. Preserve
   the same C2e-1 test and all its requested/actual accounting, error precedence,
   empty-candidate, checkpoint, source, retry, reference and single-publication
   assertions. Replace only the raw global-deallocator observation with a
   fixed, nonallocating test observer at the production candidate-drop boundary:
   capture the registered pointer/layout identity before `drop(candidate)`, call
   `drop(candidate)`, then mark that same identity dropped before invoking the
   outer restore observer. Never dereference or reconstruct the pointer after
   drop. For the exact successful row, use the equivalent test helper around
   `drop(output)`: capture the already registered Sample0 identity, drop the
   complete frame, then record the logical completion. This helper must not be
   used by ordinary host tests. The host configuration must additionally require
   the raw allocator's deallocation count at both rejection and final-output
   drop, so these logical Miri observations cannot substitute for or weaken the
   host physical-deallocation proof.

The compatibility code must remain inside `cfg(test)`/`cfg(miri)` seams; it may
not change a public API, runtime owner representation, accounting rule or error.
Use fixed `Cell` state only, with RAII restoration and a dedicated cleanup test;
do not allocate from allocator callbacks. Assert backend selection explicitly:
host tests require the raw `CountingAllocator` observer, while Miri tests require
the post-drop logical observer and must fail if the target identity was never
registered, was marked before drop, was marked more than once or survived into
restore. The existing zero-prefix registry assertion remains active.

The exact repair gate is conjunctive, not alternative: host focused C2e-1 plus
the C2 inventory and TLS cleanup must retain their real deallocation assertions;
the zero-test Miri control must exit successfully; focused C2e-1 under Miri must
execute all four rows with pointer/provenance checking enabled; a Miri-specific
observer test must prove one after-drop/before-restore notification and RAII
cleanup; and the no-default `highres` library, focused formatting and whitespace
checks must remain green. Only those combined results close the Miri portability
item for C2e-1. They do not accept the remaining C2e slices or all of H4.

##### Bounded test-only Miri allocator repair result

The final Windows configuration uses no Miri-specific global allocator; the
Miri test binary therefore uses its default allocator, while host builds keep
the `CountingAllocator` observer and its pointer/size/alignment deallocation
checks. Windows nightly Miri (`miri 0.1.0 (bff8e12ff5 2026-08-26)`) passed the
zero-test control, the focused C2e-1 transaction test (1/1), the dedicated
observer test (1/1), and integration `--test highres_safety` (1/1). Host focused
C2e-1 passed 1/1, the C2 aggregate passed 41/41, and the no-default `highres`
library passed 81/81. Independently, sol verified the same three bounded
controls under Linux `x86_64-unknown-linux-gnu` Miri: zero-test control,
focused C2e-1, and observer, all passed. These are bounded Windows/Linux
results only; they do not claim broad or full-repository Miri coverage, and do
not accept the remaining C2e slices or all of H4.

##### C2e-2 independent result: finite GO

C2e-2 is a test-only ICC actual-capacity transaction checkpoint, independently
reviewed by sol. Its controlled maker requests the late ICC owner at 4093 bytes
and allocates an empty candidate with capacity 4160 bytes (+67). The exact row
accepts the actual ownership. Four separate one-under rows then admit the
requested plan but reject the allocated candidate as `ResourceLimit`: frame,
live, metadata, and `max_icc_bytes` respectively. Each row verifies that the
ICC candidate is dropped before checkpoint restore, the 23-owner successful
prefix is dropped exactly once, borrowed source pointers/capacities/contents
and metadata remain unchanged, the same Prepared conversion retries to the
ordinary reference output, and a second materialization is rejected.

The recorded commands pass: focused C2e-2 1/1;
`cargo test --offline -p wml2 --no-default-features --features highres --lib c2`
42/42; the no-default `highres` library 82/82; and
`cargo test --offline -p wml2 --no-default-features --features "avif,highres"
--lib highres::` 103/103. Focused `rustfmt --edition 2024 --check
wml2/src/highres/convert_execute_tests.rs` and `git diff --check` pass. This
is C2e-2 evidence only: C2e-3 provenance/unknown-payload/zero-count actual
capacity transactions, C2e-4 aggregate transaction evidence, and every
remaining C2/H4 acceptance gate remain open.

##### C2e-3 independent result: finite GO

C2e-3 is a test-only actual-capacity transaction checkpoint for the remaining
metadata-backed candidates.  It independently covers provenance request3 with
capacity5, unknown payload1 request13 with capacity4160, and zero-count coded
geometry with capacity1.  For each owner, the exact actual capacity succeeds;
the independent one-under frame, live and metadata rows admit the requested
plan and then reject the real empty candidate as `ResourceLimit`.  The target
is never published, the complete checkpoint is restored, the rich source's
borrowed storage and metadata remain unchanged, the same Prepared conversion
retries to the ordinary reference result, and a second publication is rejected.

On ordinary host builds, `CountingAllocator` remains the physical authority:
every successful prefix owner is registered with its pointer/size/alignment and
is observed deallocated exactly once before restore.  Under `cfg(miri)`, raw
prefix deallocation counts are deliberately not asserted because that backend
cannot observe those allocator hooks.  Miri still verifies the complete prefix
registration shape (`OwnerKey`, key, count and ordinal), and it requires the
target's captured identity to receive exactly one logical notification only
after its owning candidate has been dropped and before checkpoint restoration.
This is an observation-boundary split, not a substitute for the host physical
proof; it does not fabricate prefix drops or alter runtime accounting.

The recorded C2e-3 commands pass: host focused C2e-3 3/3; nightly Miri focused
C2e-3 3/3 for both Windows and Linux `x86_64-unknown-linux-gnu`; the dedicated
Miri observer cleanup test 1/1 for both targets; and a zero-test Miri control
for both targets.  The host C2-filtered library inventory passes 45/45 and the
complete no-default `highres` library passes 85/85.  Focused Rust formatting of
`convert_execute_tests.rs` and `git diff --check` pass.  Existing no-default
draw warnings remain outside this slice.  This is C2e-3 evidence only: C2e-4
aggregate transaction coverage and all remaining C2/H4 acceptance work remain
open.

##### C2e-4 aggregate actual-capacity result: finite GO

C2e-4 combines the four real surplus-capacity candidates in one test-only
owner walk: Sample0 request5 to capacity8 (+12 frame/live bytes), provenance
request3 to capacity5 (+2 frame/live/metadata bytes), ICC request4093 to
capacity4160 (+67 frame/live/metadata bytes), and UnknownPayload1 request13 to
capacity4160 (+4147 frame/live/metadata bytes).  Their aggregate is +4228 for
frame/live and +4216 for metadata.  The exact row succeeds and its actual
owned-byte walk matches the output ledger.  The frame, live and metadata M-1
rows still admit the requested plan, construct all 27 candidates, and then
reject the final ordinal26 `UnknownPayload(1)` capacity as `ResourceLimit`.
This proves all preceding admissions succeeded before the final actual-capacity
failure.  Each rejection drops that target before checkpoint restoration,
restores the initial accounting snapshot, leaves all borrowed source storage,
metadata and timing unchanged, retries through the ordinary exact-count path,
and rejects a second publication.

On host builds the complete 26-owner prefix is observed with real
pointer/size/alignment deallocation and every prefix allocation is dropped
exactly once before restore.  Under `cfg(miri)`, the fixed nonallocating
observer requires the registered final target's logical notification strictly
after its owning candidate drop and before restore, while the complete prefix
`OwnerKey`/key/count/ordinal shape remains checked.  C2e-2 now uses the same
host-physical/Miri-logical observation split; it retains all target, prefix,
checkpoint, source and retry assertions rather than weakening its host proof.

Recorded commands pass: host C2e-4 1/1, host C2e 6/6, host C2-filtered library
46/46, and the no-default `highres` library 86/86.  Nightly Miri passes the
zero-test control, C2e-4 1/1, C2e 6/6 and the observer cleanup test 1/1 on both
Windows and Linux `x86_64-unknown-linux-gnu`.  Focused Rust formatting of
`convert_execute_tests.rs` and whitespace diff checks pass; the two existing
no-default draw warnings remain outside this slice.  The finite C2e aggregate
is complete.  It does not close C2 as a whole, broader highres validation,
AVIF/ICC conformance or any remaining H4 work.

##### C2-2A strict-native still allocation slice: bounded host/Linux-Miri GO

This finite nested-AVIF slice covers only strict native decoding of a normal
`av01` still and its single selected auxiliary alpha. `DecodeBudget::check_live`
now checks the checked sum of retained metadata, payload, existing
`frame_live`, and the pending allocation; it therefore cannot admit a crop or
alpha candidate by ignoring already-live coded/master storage. The legacy
decoder, its public API, its twelve-argument `NativeDecodeLimits` constructor,
AVIS, derived grid/`sato`, tile/entropy state and WML2 bridge are outside this
slice.

The coded `FrameBuffers` outer plane vector and sample vectors are charged by
actual capacity. Crop retains coded storage while admitting the visible
candidate, then retires the old ticket only after replacement. Selected alpha
keeps both master and alpha live while the master outer plane vector grows; the
runtime `grow_outer_for_alpha` transaction is the same path exercised by the
actual-capacity test seam. Dedicated M/M-1/retry regressions cover both cases:
the exact actual capacity succeeds; one byte under rejects after candidate
creation; the candidate is observed dropped before `DecodeBudget` checkpoint
restore; source/master pointers, ownership tickets and accounting snapshots
remain intact; and a retry succeeds. A no-crop regression also preserves the
existing native sample pointer.

Nested offline `--all-targets` passes with 546 library tests passing and six
ignored, alongside the exercised integration suites (including 4, 2, 19 and
109-test suites). Focused Linux `x86_64-unknown-linux-gnu` nightly Miri passes
the crop and alpha outer-growth cases independently. Focused Rust formatting
and nested whitespace-diff checks pass.

This is a bounded host/Linux-Miri GO only. Windows Miri still has an open
zero-control teardown UB in the test allocator's Windows `System::dealloc`
high-alignment path; it is a harness gate and is not counted as a Windows Miri
pass or worked around here. Broader strict decode allocation coverage,
AVIS/stateful decoding, grid/`sato`, tiles/entropy/filter/bridge allocations,
and the remainder of C2 stay incomplete.

##### C2-2B strict-native normal/split/selected-alpha header materialization: bounded GO

This bounded nested-AVIF result covers one strict-native primary route and its
one selected auxiliary alpha route, each as either a self-contained `OBU_FRAME`
or a strict `FRAME_HEADER` plus `TILE_GROUP` sequence. The shared classifier is
borrowed and fail-closed: it accepts exactly one normal frame or one complete
split sequence, and rejects ambiguous/extra coded frames before header or tile
materialization. The normal route reaches `parse_tile_info_with_budget` through
`finish_frame_header_with_budget`; the split route uses the corresponding
strict bounded parser. Legacy `parse_tile_info`, `finish_frame_header`, and
the legacy prefix wrapper retain their public behavior and allocation paths.

The selected alpha uses the same strict classification and bounded header/tile
materialization as the primary. A classifier `Unsupported` error is annotated
only at the alpha boundary as `AVIF alpha auxiliary item: ...`; a duplicate or
otherwise unsupported alpha frame therefore retains `alpha` error context,
while primary errors and non-`Unsupported` error variants are propagated
unchanged. This is an error-reporting boundary only: it does not relax the
classifier or add an allocation.

`mi_col_starts` and `mi_row_starts` begin empty and every push uses the shared
`DecodeBudget` fresh-replacement transaction. For normal and split selected
routes, the covered TileInfo vectors, copied tile payload, TileGroup descriptor
vector, and FrameDecodePlan plane/tile vectors have exact actual-capacity (`M`)
success and one-byte-under (`M-1`) rejection evidence. On `M-1`, the candidate
is created only after admission, dropped before budget restoration, the input
source remains unchanged, and the same source retries successfully at `M`.
The strict header sidecar also releases TileInfo vectors before its ticket is
released. These remain owner-local transactions, not a total decoder bound.

Host evidence includes `native_hookup` 3/3 (including the duplicate-alpha
error contract), focused strict-split and alpha-prefix tests, `native_` 41/41,
and nested `cargo test --lib` with 563 passed and 6 ignored; formatting and
nested whitespace-diff checks pass. Linux `x86_64-unknown-linux-gnu` Miri
passes the borrowed strict-split selector with normal isolation. The
fixture-reading alpha-prefix case passes with `MIRIFLAGS=-Zmiri-disable-isolation`
solely to permit file access; this does not alter allocation, provenance, or
drop validation.

This is a C2-2B bounded GO, not C2 acceptance. Exclusions include the
`TileInfo` forced-matrix path, split/merged temporary ownership outside the
covered routes, entropy/CDF/motion/filter state, multi-frame and AVIS
transactionality, derived grid/`sato`, full alpha/master combined-header peaks,
table-driven all-owner allocation coverage, the WML2 bridge, and wider C2/C3
gates. Windows Miri remains open: its zero-test teardown hits the documented
Windows `System::dealloc` Stacked-Borrows UB and is neither counted as passed
nor worked around. No public API, legacy behavior, version, commit, or release
action follows from this result.

##### Public high-resolution AVIF still bridge: limited GO

The additive `highres::avif::decode_native` bridge is now verified for the
ordinary primary `av01` still path. With `avif,highres` enabled it preserves
native U16 planes, meaningful bit depth, subsampling/layout, alpha ownership,
ICC bytes and type, nclx, unknown `colr` type/payload, pixel aspect ratio and
container geometry metadata without applying an implicit RGBA8 conversion,
color conversion, clamp, crop, rotation or mirror. The legacy AVIF API and
callback path are unchanged; `highres` alone does not enable AVIF.

The bridge's feature-gated integration suite has six passing tests, including
strict native-reference comparisons for gray and 4:2:0/4:2:2/4:4:4 8/10/12-bit
inputs, alpha, geometry, ICC+nclx coexistence and unknown-colour retention.
The Windows normal-process smoke test and offline feature-on/no-default checks
also pass. Missing required derived/sequence fixtures are hard failures, and
the staging test verifies the concrete codec `Unsupported` error category.
`DecodeError` is deliberately `#[non_exhaustive]`: its two stable categories
are retained while future high-resolution decoder/mapping error classes can be
added without another breaking enum change.

This is a limited bridge GO only. AVIS/stateful sequence decoding, derived
grid/`sato` composition, ICC execution/CMS conversion, CICP transfer and
primaries conversion, PQ/HLG output, U8/U16 output quantization, high-precision
AVIF encoding, and the independent codec/oracle/conformance gates remain open.
No release, version, commit or publish action follows from this checkpoint.

##### Pause checkpoint: strict AVIS resource accounting (not accepted)

Work is paused at the strict nested-AVIF AVIS decoder before any WML2 public
sequence bridge is added. The completed, independently reviewed slices are:
allocation-free child-box traversal with legacy malformed-sibling diagnostic
ordering preserved; selected-track-only compressed-payload retention while
still validating every sibling table; and selected timing-vector preflight,
token handoff, rollback, and retry handling. The decoder prepare path also has
transactional candidate-ticket cleanup and constructor clone preflight.

The AV1 state-inventory integration is deliberately **not accepted**. Its
review found that a sequence sample may duplicate an in-sample Sequence Header
for every coded unit. The current state-refresh plan must derive that scratch
from the sample itself, rather than the cleared primary-item payload, and must
prove through the real `prepare_next_frame` path that a one-byte-under limit
does not enter split/decode allocation, leaves no live ticket, and permits a
retry. Resume with that P0 repair, then repeat the independent strict-AVIS
review before exposing any `highres` AVIS API.

No version, commit, push, tag, publish, or release action has been performed.
