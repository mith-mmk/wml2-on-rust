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
- PSD source/output naming and JPEG2000 ICC acquisition remain distinct format
  work. Explicit JPEG2000 ICC rejection is an advertised capability gap, not
  silently successful decoding.
- Shared explicit Gray-to-RGB/RGB-to-Gray output needs checked separate layouts;
  the own ICC library already supports the transforms. General CMYK CMS needs
  a checked four-channel/4D-CLUT own-library route and native roles/layouts;
  unchecked legacy CMYK helpers are not a replacement.

## JPEG XL continuation

- The separate continuation now verifies all8orientations against djxl and
  displays13independent cases in actual Chromium WASM/canvas, including the
  official RGB/Gray still images. Explicit sRGB output shares the own-library
  checked1/3→3-channel ICC helper with AVIF, uses the reconstruction profile,
  and excludes alpha. Existing native/ordinary contracts are preserved.
- Coded9/10-bit sources now promote to normalized16-bit with recoverable
  source codes. Node WASM checks61cases without host imports.
- The prior Windows25-file collection and manifest are absent from this cloud
  workspace; cloud/browser evidence cannot claim to execute that collection.
- Check the pinned codec's actual APIs before extending 9/10-bit,
  premultiplied alpha, compressed metadata or JPEG reconstruction. Opaque
  `jbrd` retention does not reconstruct the original JPEG.
- HDR/tone mapping, animation, encoding, SIMD and threading have separate
  contracts and must be implemented and tested before any support claim.

Binary work uses bin-rs. Necessary ICC parsing/transforms use the own
icc-profile library. No casual third-party runtime CMS/parser dependency is
introduced; independent Pillow/libjxl/browser tooling is validation tooling.
