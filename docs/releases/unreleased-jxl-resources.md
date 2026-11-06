# Unreleased JPEG XL resource configuration

- Add explicit source-coordinate RGBA8 and sRGB APIs with finite,
  caller-tightened internal layer/padding, TOC, ICC and auxiliary expansion
  limits. The workspace quota adds simultaneously live adapter output, rounding,
  collector, CMS samples and profile parse storage to existing codec estimates.
- Preserve existing APIs, hard caps, dependencies and pixel behavior. This first
  implementation phase does not enable larger canvases: 2560x1440 remains
  refused. Codec-wide byte admission and recoverable allocation failures remain
  prerequisites for qualifying a broader resource envelope.

No version bump, merge or release is included. Detailed scope and remaining
decisions: [resource contract](../jpeg-xl-resource-contract.md).

Draft pull request: [#72](https://github.com/mith-mmk/wml2-on-rust/pull/72).

- Attribute jxl0.8's first4008-byte retention to its process-wide implicit-palette cache. Finite resource APIs now conservatively reserve all25 fixed slots (100200 bytes64-bit /99900 bytes32-bit), independent of warm state; legacy admission and fixed caps stay unchanged. This tightens explicit finite workspace minima without claiming codec-wide fallible allocation or a release. See [diagnosis and contract](../jpeg-xl-palette-cache.md) and draft [PR #95](https://github.com/mith-mmk/wml2-on-rust/pull/95).

- Add an isolated, hash-verified jxl0.8 context-map / reusable LZ77 scratch budget evaluation. It changes no product dependency or admission cap; decoder-wide propagation remains unimplemented. See [evaluation and remaining integration boundaries](../jpeg-xl080-entropy-budget-evaluation.md).

- Pre-admit and fallibly allocate ordinary, float/HDR and preview output-format descriptor lists. Finite workspace retains their payload charge through decode/CMS; legacy admission stays unchanged. This is a small [adapter follow-up](../jpeg-xl-format-admission.md), not codec-wide heap coverage.

- Make container metadata keys and record-array growth fallible; pre-admit old/new backing storage and carry retained record bytes into finite Brotli/header workspace. Preserve key names, logical metadata quotas and legacy admission. See [contract and evidence](../jpeg-xl-container-metadata.md).
