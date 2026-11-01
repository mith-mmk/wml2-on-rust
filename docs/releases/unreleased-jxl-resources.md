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

- Attribute jxl0.8's first4008-byte retention to its process-wide implicit-palette cache. Finite resource APIs now conservatively reserve all25 fixed slots (100200 bytes64-bit /99900 bytes32-bit), independent of warm state; legacy admission and fixed caps stay unchanged. This tightens explicit finite workspace minima without claiming codec-wide fallible allocation or a release. See [diagnosis and contract](../jpeg-xl-palette-cache.md).
