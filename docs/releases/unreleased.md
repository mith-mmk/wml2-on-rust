# Unreleased

## JPEG XL SDR animation

- Extend the bounded jxl 0.8 animation adapter to XYB RGB8/16 and Gray8/16 with
  optional matching-depth straight alpha. No new encoder or HDR processing.
- Preserve distinct source/output ICCs and tag RGBA Gray expansion correctly.
  Explicit sRGB APIs transform from reconstruction ICC; arbitrary Gray output
  ICC expansion is refused rather than mislabeled.
- Add independent libjxl all-frame Gray and differing-ICC XYB oracles, plus
  native/WASM/Viewer checks. See [integration validation](../jpeg-xl-sdr-validation.md).

These changes are not in the published 0.0.42 package. No merge, release or
deployment is performed by this validation. CI/CD completion is not awaited.
See [0.0.42 release notes](0.0.42.md) for the previously merged JPEG XL/TIFF base.
