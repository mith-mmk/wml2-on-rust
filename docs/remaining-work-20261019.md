# WML2 continuation inventory (2026-10-19)

This inventory is checked against source, the existing PRs and their exact
heads. It distinguishes implemented fixes from supported-format extensions.

## Preserved checkpoints

- [PR19](https://github.com/mith-mmk/wml2-on-rust/pull/19),
  `8fcce19e4bbe94511e26497349fd811ce0f9cee2`: bounded JPEG ICC assembly,
  JPEG/PNG sample identity, shared metadata lifecycle and encoder/EX boundaries.
  It uses the published own `icc-profile` 0.0.7 PCS correction.
- [PR20](https://github.com/mith-mmk/wml2-on-rust/pull/20),
  `b792e4a759043013c46f12f01638731be70a1bf0`: TIFF palette RGB ICC. It depends
  on PR19 and its PR CI passed49/49. It does not implement general per-page ICC.
- `codex/gif-icc-retention-20261019`,
  `05256e4a83eca9996c6704a87ad25ad719874dd0`: bounded GIF ICC carrier
  acquisition, with49/49 push CI. Multiple carriers remain source records,
  not a fabricated global profile or per-frame transform.
- `codex/jpeg-xl-decode-20261018`,
  `83c1800d21620d4c4e2a316e51916b1a04c16c96`: existing bounded still JXL
  decoding and opaque `jbrd` retention; its56-job CI passed. Shared ICC fixes
  above are not yet integrated into that checkpoint.

The new `codex/icc-remaining-20261019` branch combines the completed ICC
checkpoints without modifying their branches or main. JXL continuation stays
on a separate branch and must explicitly record which shared ICC revision it
integrates. No main merge, version bump or registry publication is requested.

## ICC continuation

- BMP V5 acquisition (F12): implemented in this continuation, with8new
  integration regressions. BMP-only26tests and combined BMP/PNG/GIF/EX53tests
  pass. Six independent BMPs match Pillow pixels before/after. Source/output
  ICC bytes remain exact. Benchmarks preserve hashes and add4allocations/
  717bytes for the344-byte profile; the small ICC case adds0.35microseconds.
- TIFF per-page identity (F09): implemented as indexed display-page source
  records and applicability flags. Generic source/active profiles now require
  equality/applicability across every display page, including RGB-first files.
  Native per-page provenance is retained. A common-profile CMS transform is
  still an explicit separate operation. Three new regressions cover RGB-first
  combinations, the aggregate budget and callback Abort/errors. Four
  independent two-page files preserve all8page rasters exactly. Common source
  bytes are shared once, with page-specific applicability. TIFF-only65tests,
  final TIFF targets68tests/4existing ignored and core364tests/6existing ignored
  passed; counts overlap. Final local paired medians range from+0.77%to+4.91%
  with unchanged pixel hashes and bounded additional metadata storage.
- AVIF F05/F06/F07: the pinned0.1.0 codec selects nclx ahead of ICC, performs
  display transfer before its internal ICC path, and misses actual iref/prem
  alpha references. Native/EX boundaries and own-library CMS integration need
  correction together; changing ICC priority alone is insufficient. The
  WML2 scoped still correction is now implemented with own-library CMS,
  actual item alpha association, and raw/Gray-range reconstruction. Native
  samples remain exact, EX retains source coordinates, and9independent
  references check display output. Managed AVIS and broader display/geometry
  capabilities are not yet covered by this correction.
- PSD source/output naming is now corrected for Gray/CMYK and incompatible
  profiles, preserving compatible RGB/palette declarations and unchanged pixels.
  Shared ICC quota and repeated-resource cleanup are tested. See [PSD ICC](psd-icc.md).
- JPEG2000 now acquires selected JP2 method2 Gray/RGB ICC using the own structural
  parser without compiling a transform. Ordinary, native EX, storage conversion,
  owned adoption and reduced output retain appropriate source/output identity.
  Other ICC device models remain explicitly unsupported; JP2 method3 is ignored
  as an unregistered Part1 method. See [JP2 ICC](jpeg2000-icc.md).
- Shared explicit Gray↔RGB frame conversion now uses checked separate layouts,
  source-coordinate validation, exact alpha copying, preserved provenance and
  an optional output sample-buffer cap. All U8/U16/F32 and mixed-precision
  cases have independent LittleCMS references. See [native ICC layouts](icc-frame-layouts.md).
- General CMYK CMS still needs checked four-channel/4D-CLUT own-library routes
  and native roles/layouts. Arbitrary3CLR and CMYK do not acquire an RGB model
  merely because a transform happens to have a matching channel count.

## JPEG XL continuation

- Saved separately at `codex/jpeg-xl-remaining-20261019`. The79951b7 feature unit
  preserved JXL83c1800 and initially integrated ICCfbec877; its CI56/56 passed.
  This continuation now integrates ICCcc4e4a1e7e23adae5f86d89c5defaa7f540dcff5,
  including PSD,JP2 acquisition, native Gray↔RGB layouts and display device-model validation.
- All8orientations,9/10-bit promotion, shared own-library explicit sRGB display,
  Node WASM61cases and actual Chromium13-case display are independently validated.
  The explicit display route uses reconstruction-profile coordinates and excludes alpha.
- The prior Windows25-file collection/manifest is absent from this cloud
  environment; independent cloud/browser evidence does not execute that corpus.

- Check the pinned codec's actual APIs before extending premultiplied alpha,
  compressed metadata or JPEG reconstruction. Opaque
  `jbrd` retention does not reconstruct the original JPEG.
- HDR/tone mapping, animation, encoding, SIMD and threading have separate
  contracts and must be implemented and tested before any support claim.

Binary work uses bin-rs. Necessary ICC parsing/transforms use the own
icc-profile library. No casual third-party runtime CMS/parser dependency is
introduced; independent Pillow/libjxl/browser tooling is validation tooling.
