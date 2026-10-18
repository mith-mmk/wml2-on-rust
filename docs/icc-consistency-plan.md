# ICC consistency repair stages

This work is separate from JPEG XL implementation. The audit baseline was
`83c1800d21620d4c4e2a316e51916b1a04c16c96` on
`codex/jpeg-xl-decode-20261018`; that branch is not modified here.

The WML2 repair branch is `codex/icc-consistency-20261018`, based on the current
main `f76a59ba65f5de958893429c08fc65cde94ca6b7`. The independent icc-profile branch
is `codex/lab-pcs-lut-20261018`, based on its main
`bdb10985ab2437504a53bf0214df21fa41896e58`. This plan records subsequent work;
it does not claim those stages have been implemented.

## Stage 1: bounded JPEG ICC assembly and paired PCS correction

- WML2 F01/F03: inspect the complete ICC APP2 envelope before binary reads;
  validate sequence, consistent total, uniqueness and completeness; assemble in
  sequence order. Invalid ancillary associations are discarded while raster
  decoding continues. Invalid outer APP lengths return errors. Charge retained
  ICC envelopes, including empty chunks, and the assembled copy to metadata
  limits. Keep opaque bytes and raw APP records; do not require transform support
  merely to retain a profile.
- icc-profile F02: give a destination LUT its own physical PCS before encoding;
  keep physical XYZ for matrix destinations. Cover matrix/LUT and XYZ/Lab
  combinations, Gray matrices, relative/absolute intent, independent physical
  reference vectors and a frozen LittleCMS-generated profile.
- Verify both repairs separately, then test WML2's existing explicit ICC adapter
  against the local corrected library. A scratch Cargo patch is sufficient for
  integration testing; it is not a registry delivery mechanism.

## Stage 2: acquisition and source/output profile identity

- F12/F13: retain bounded BMP V5 embedded ICC and GIF ICCRGBG1012 payloads.
  Palette expansion does not itself require a CMS. Do not open linked BMP
  profiles as an implicit filesystem/network operation.
- F04: distinguish original Gray/CMYK ICC from a compatible current RGB output
  profile in ordinary JPEG/PNG. Apply the same source/output naming principle to
  documented non-managed PSD output without claiming a new automatic-sRGB
  contract.
- F08: keep applicable RGB ICC after TIFF palette expansion. Preserve valid RGB
  ICC after codec YCbCr reconstruction and channel-preserving depth changes.
- Define active-profile applicability separately from opaque source retention;
  short or unsupported profiles must not silently become transformable output
  declarations. Do not replace retention with a transform-compilation gate that
  rejects valid unsupported native profiles.

## Stage 3: shared bridges, encoder boundaries and metadata lifecycle

- F10: preserve source/callback provenance through EX storage changes and
  ordinary bridges. An RGB encoder must not automatically treat a Gray/CMYK
  source profile as its output profile. Add a defensive output-model check or a
  distinct explicit output-profile option. Keep native Gray+Gray ICC valid.
- F11: define one top-level decode boundary that clears stale image metadata.
  Preserve legitimate metadata delivered before init and within animations;
  clearing every callback init is insufficient.
- F09: represent per-frame/page profiles, or explicitly transform to a selected
  common profile. Preserve the existing typed native per-page ICC route. Do not
  silently apply the first page's profile to later pages.
- Keep `FrameMetadata.source_color` as source provenance after explicit CMS.
  Clarify/update any generic active ICC tag independently of that provenance.

## Stage 4: AVIF codec and CMS boundary

- Coordinate avif-rust parser fixes for effective ICC/nclx selection (F05) and
  actual iref/prem alpha references (F07), then verify WML2 boundary declarations.
- Separate codec matrix/range reconstruction from display transfer processing.
  F06 must run the required source ICC transform exactly once; merely changing
  ICC priority does not fix the second transfer operation.
- Consolidate necessary ICC parsing/transforms through mith-mmk/icc-profile.
  Unpremultiply before nonlinear CMS; continue retaining raw signaling in native
  APIs. Do not introduce another product CMS or claim tone mapping is ordinary
  SDR ICC processing.

## Stage 5: explicit managed output capabilities

- Extend WML2's explicit Gray/RGB adapter with separate source/destination
  models, roles, planes, alpha mapping and allocations. icc-profile already
  supports Gray-to-RGB and RGB-to-Gray; deleting the matching-count guard alone
  would write into the wrong layout.
- Add JPEG 2000 component/profile retention and explicit managed-output routes
  separately from its documented ICC rejection.
- Keep JXL-specific acquisition/dispatch on the JPEG XL branch. Later JXL
  integration can call the shared explicit CMS boundary with source pixels/ICC
  and caller-selected destination profile; the generic ICC repair must not
  depend on the external JXL codec or require cherry-picking its decoder.
- CMYK requires a checked icc-profile extension first: four device channels,
  4D CLUTs, intent routes, budgets and independent printer-profile references.
  Then add WML2 native color roles/layouts, including separate alpha, before
  replacing documented device approximation. Do not route through unchecked
  legacy CMYK helpers and label that full CMS support.

## Dependency and delivery constraints

Binary work uses bin-rs; necessary ICC work uses mith-mmk/icc-profile. Outside
external codecs, general third-party runtime dependencies remain essentially
miniz_oxide. The optional SJIS encoding_rs dependency is not removed here.
External codec transitives, build/dev tools, the non-published wml2-test CLI and
independent LittleCMS/Pillow tooling remain distinct categories.

The Stage 1 icc-profile fix does not change its published 0.0.6 artifact. Registry
consumers need a later corrected library release and WML2 dependency/lock update.
Local path-patch tests prove compatibility only. No version bump, publication,
PR, merge or release is part of this stage.

Each later stage should have a scoped patch, independent review, focused
regressions, native-retention controls, image checks and relevant performance
measurements before its normal commit/push checkpoint.
