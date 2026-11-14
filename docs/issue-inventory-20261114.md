# Open issue inventory (2026-11-14)

This is a source and GitHub-state review of released main
`a6bdc86a65b5a182f14209ad79d5b3fdb760488a` (WML2 0.0.46), not a new
execution of historical tests. The repository had twelve open issues and three
open draft PRs at review time. Issue descriptions that quote older revisions
are historical problem statements, not the current support table.

## Priorities and boundaries

1. Add observable, typed color-processing outcomes (#113) without changing the
   color algorithms or breaking existing metadata/callback consumers.
2. Validate and review that implementation before preparing a new version.
3. Adopt the validated library in WML2Viewer afterward. Paintcore is outside
   this work.

The former broad preview/HDR/extra-plane/resource-policy stack was deliberately
closed; see [the recorded disposition](jpeg-xl-focused-integration.md).
An open umbrella issue is not an instruction to revive that stack. No issue or
PR was closed, merged or released as part of this inventory.

## Released support versus remaining work

| Issue | Released-main evidence | Remaining work / disposition |
| --- | --- | --- |
| [#37](https://github.com/mith-mmk/wml2-on-rust/issues/37) resource limits | [Current limits](jpeg-xl.md#limits-and-cancellation) retain fixed padded-frame/input limits and an expanded-workspace estimate. | Not a process-RSS or arbitrary codec-allocation guarantee; do not simply lift caps or claim large-image support. |
| [#39](https://github.com/mith-mmk/wml2-on-rust/issues/39) documentation | [Current JXL guide](jpeg-xl.md) separates current scope from historical validation, covers eight orientations, 9/10-bit promotion and historical browser evidence. | This inventory supplies the issue/source/test mapping. Historical results are not reruns; broader external-plugin checks remain separate. |
| [#40](https://github.com/mith-mmk/wml2-on-rust/issues/40) jxl 0.8 | [Focused migration](jpeg-xl-focused-integration.md) and [0.0.42 notes](releases/0.0.42.md): published 0.8 pixel decoder, aliased 0.7.4 preflight. | Original premise of a 0.7.4-only decoder is obsolete. Removing the preflight dependency needs a separate bounded-parser solution. |
| [#42](https://github.com/mith-mmk/wml2-on-rust/issues/42) alpha/extra channels | [Still-image precision](jpeg-xl.md#still-image-profile-and-precision) supports mixed-depth straight full-resolution alpha. | Associated/subsampled alpha and general extra-channel output remain unsupported. |
| [#43](https://github.com/mith-mmk/wml2-on-rust/issues/43) animation | [0.0.43 animation contract](jpeg-xl.md#bounded-animation-in-0043) covers bounded SDR RGB/XYB/Gray 8/16, frame timing and composition. | Preview/timecodes/HDR and unsupported alpha/channel combinations remain rejected; this is not full JPEG XL conformance. |
| [#44](https://github.com/mith-mmk/wml2-on-rust/issues/44) preview | [Preview contract](jpeg-xl-preview.md) supports bounded skip for main-image decoding. | Preview extraction is not in released main. Do not confuse it with progressive output. |
| [#46](https://github.com/mith-mmk/wml2-on-rust/issues/46) HDR/float | [Current rejection scope](jpeg-xl.md#still-image-profile-and-precision) remains explicit. | No implicit tone mapping, HDR-as-sRGB, or revival of closed extension stack. |
| [#47](https://github.com/mith-mmk/wml2-on-rust/issues/47) progressive | Existing whole-input APIs decode supported progressive streams. | Draft [#69](https://github.com/mith-mmk/wml2-on-rust/pull/69) evaluates a future resumable API; production admission/EOF/publication remain unresolved. |
| [#48](https://github.com/mith-mmk/wml2-on-rust/issues/48) original JPEG | Opaque jbrd preservation and image decoding do not reconstruct original JPEG bytes. | Draft [#60](https://github.com/mith-mmk/wml2-on-rust/pull/60) records backend and bounded-memory blockers. |
| [#49](https://github.com/mith-mmk/wml2-on-rust/issues/49) encoder | JPEG XL remains decode-only. | Draft [#61](https://github.com/mith-mmk/wml2-on-rust/pull/61) records qualified sample formats, alpha failures and unresolved working-memory/cancellation admission; not a production encoder. |
| [#110](https://github.com/mith-mmk/wml2-on-rust/issues/110) 0.0.45 tracking | [Canonical 0.0.45 notes](releases/0.0.45.md), merged PR #111, published v0.0.45; main has subsequently reached 0.0.46. | Release-tracking reconciliation, not a request to repeat dependency adoption. Historical acceptance evidence must be checked before closing. |
| [#113](https://github.com/mith-mmk/wml2-on-rust/issues/113) color outcomes | 0.0.46 has bounded CMYK conversion, fallback warnings and source/output profile metadata. | Profile presence and absence of warnings do not prove applied conversion. New typed per-output observations are required; unreported states must be unknown. |

## Existing test/source correspondence

- `wml2/src/jpegxl.rs`, `jpeg_xl.rs` and `jpeg_xl_additional.rs`: ordinary/EX,
  integer depth, orientation, metadata, CMS and cancellation contracts.
- `jpeg_xl_brob.rs`, `jpeg_xl_metadata.rs`, `jpeg_xl_slice_admission.rs`:
  bounded container/metadata and input admission.
- `jpeg_xl_focused_animation.rs`, `jpeg_xl_xyb_animation.rs`,
  `jpeg_xl_gray_animation.rs`, `jpeg_xl_xyb_icc_animation.rs`: released animation
  subsets and color/profile behavior.
- `jpeg_xl/alpha.rs`, `jpeg_xl/alpha_cases.rs`, `jpeg_xl/preview.rs`:
  supported subsets and explicit refusals, not proof of general alpha/preview APIs.
- [Historical native/WASM/oracle/benchmark record](jpeg-xl-focused-integration.md#migration-validation)
  identifies its baseline, environment and measurement limitations.

## Draft checkpoints (unchanged)

- #60: `3acc06e7a9785f0c04affa5495a4880eb87d99b6`.
- #61: `49ecd5777ad7b63eda61b6006302a1a1ae2a6bb0`.
- #69: `cdeddc7e30fa464940610ba343458e6642c4bab7`.

These are existing evaluation results, not newly rerun qualifications. Their
old-base CI does not qualify an integration into today's main.
