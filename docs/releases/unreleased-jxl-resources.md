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
