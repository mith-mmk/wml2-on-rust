# JPEG XL released and draft status

Read-only source/document reconciliation on 2026-11-06 for #39. Released main is
WML2 0.0.41 at `f2151b2408e9d04f1bc7b4e9e29679e078a57209`; this guide's draft
baseline is #96 at `b5e216d991b1293dd20640422da6d184ee8d5c49` (production through
#95 plus isolated evaluation). No historical tests, browser runs or private
corpus checks were rerun for this documentation change. Existing validation
reports retain their original source revisions and execution environments. The
#46 row additionally links the subsequent unmerged float-HDR implementation; its
new local evidence is recorded separately from this baseline audit.

[Draft #51](https://github.com/mith-mmk/wml2-on-rust/pull/51), remote head
`0526f60fd1a7716afbf528eab1d778567ab78985`, already reconciles released-main
acceptance. It is unchanged. This page adds the later development-stack map;
it does not close #39 or claim those drafts are released.

| Original issue / feature | Released main0.0.41 | Later draft scope and source/test evidence | Actual remaining boundary |
| --- | --- | --- | --- |
| #39 orientation, depth and provenance | All8 orientations and9/10-bit normalization are implemented, not new missing features. | [additional tests](../wml2/tests/jpeg_xl_additional.rs): `all_eight_orientations_match_independent_djxl_after_one_reorientation`, `nine_and_ten_bit_sources_promote_to_normalized_sixteen_without_losing_codes`; historical [layer/browser evidence](jpeg-xl-layers.md). | No new browser/private Windows corpus execution is claimed. Existing manual/downstream checks remain separate. |
| #40 dependency update | Published jxl0.7.4 decoder. | #75: [0.8 migration](jpeg-xl-080-migration.md), [runtime](../wml2/src/jpegxl.rs), ordinary/EX/resource tests; public0.8 pixel decoder plus0.7.4 preflight only. | Bridge removal needs bounded public admission API or an explicit source-maintenance decision. |
| #37 resources / #63 evaluation | Fixed caps, borrowed-input admission and conservative estimates. | #72/#73 finite configuration/fallible adapter copies; #95 palette attribution/reservation. [Resource contract](jpeg-xl-resource-contract.md), [tests](../wml2/tests/jpeg_xl_resources.rs), [palette evidence](jpeg-xl-palette-cache.md). #96 is [isolated entropy evaluation](jpeg-xl080-entropy-budget-evaluation.md). | Codec-wide fallible allocation/shared ownership tracking and2560×1440 admission remain unresolved. #96 is not product propagation. |
| #42 Alpha/extras | Mixed-depth straight full-resolution integer Alpha. | #84–#92: finite associated/subsampled Alpha and preserved Depth/SelectionMask/Alpha planes. [Acceptance audit](jpeg-xl-extra-acceptance-audit.md), [extra-plane tests](../wml2/tests/jpeg_xl_main5_extras.rs), [combined tests](../wml2/tests/jpeg_xl_extra_alpha.rs). | Names unavailable in pinned public API; fixed Main5 count4. Other kinds/storage/combinations keep explicit refusal policy; no unlimited combination requirement is added. |
| #43 animation | Refused; internal still-image layers are not animation. | #93 [native exact rational timing](jpeg-xl-buffered-animation.md); #94 [common callback/EX/sRGB](jpeg-xl-animation-callbacks.md); [native tests](../wml2/tests/jpeg_xl_buffered_animation.rs), [callback tests](../wml2/tests/jpeg_xl_animation_callbacks.rs). | Finite RGB8/16 subset; unsupported profile combinations refused. Fractional-ms timing uses native tick API; common callbacks refuse it explicitly. Unmerged work is not main support. |
| #44 preview | Bounded preview skipping while decoding main image. | #76–#79: raw RGBA16 preview+orientation/ICC and oriented sRGB preview with cancellation; [guide/API](jpeg-xl.md), [tests](../wml2/tests/jpeg_xl_preview_dimensions.rs). | Main-only decode does not validate preview entropy or expose preview pixels. Dedicated APIs have separate supported-profile/resource contracts; drafts remain unmerged. |
| #46 float/HDR | Ordinary/EX refuse float/PQ/HLG. | #80–#83: F32, F16-input→F32, structured encoded PQ/HLG and explicit PQ-nits/HLG-scene linearization; the subsequent [float-HDR stage](jpeg-xl-float-hdr.md) adds F16/F32 encoded inputs. [Guide](jpeg-xl.md), [F32 tests](../wml2/tests/jpeg_xl_native_f32.rs), [HDR tests](../wml2/tests/jpeg_xl_hdr_linear.rs). | Display white/exposure/tone curve/gamut and HLG display conditions are undecided; no implicit tone mapping. |
| #47 progressive | Whole-input decode can decode progressive codestreams; no resumable public session. | [#69 evaluation](https://github.com/mith-mmk/wml2-on-rust/pull/69), immutable head `cdeddc7e30fa464940610ba343458e6642c4bab7`. | Incremental preflight, sealed EOF/finality, bounded provisional snapshots and streaming Abort/backpressure remain product work.0.7 evaluation is not a qualified0.8 session. |
| #48 original JPEG | No bit-exact reconstruction API. | [#60 feasibility](https://github.com/mith-mmk/wml2-on-rust/pull/60), immutable head `3acc06e7a9785f0c04affa5495a4880eb87d99b6`. | Raw jbrd bytes/pixel equivalence are not original-JPEG reconstruction; candidate frame dependencies and expanded-data admission remain unresolved. |
| #49 save/encoder | No JPEG XL saving. | [#61 candidate evaluation](https://github.com/mith-mmk/wml2-on-rust/pull/61), immutable head `49ecd5777ad7b63eda61b6006302a1a1ae2a6bb0`. | Candidate16-bit Alpha errors, thin-image refusals/panics and pre-allocation/cancellation gaps prevent product adoption. |

Main already implements bounded brob/common EXIF/XMP (#35/#36), mixed-depth
Alpha (#50), intrinsic dimensions (#45), cancellation (#31), and orientation /
precision checks. Current limitations must not list these as wholly absent.
Gray ICC expansion is route-specific: native Gray can retain its profile,
ordinary RGBA cannot assign an arbitrary Gray ICC without a CMS, and explicit
sRGB conversion uses the supported Gray/RGB CMS path.

The independent next action is this finite documentation reconciliation. The
[dependency recommendation](jpeg-xl-dependency-integration.md) separates actual
upstream/distribution decisions from implementation work. HDR display policy,
streaming admission and backend replacement are separate scopes, not reasons to
silently weaken limits. No issue closure, merge, approval or release is included.
