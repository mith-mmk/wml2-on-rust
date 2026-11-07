# Unreleased

- Fix old-style JPEG TIFF files whose strip data is already inside a complete
  JPEGInterchangeFormat stream, including the public `smallliz.tif` sample.
- Accept missing StripByteCounts for a single uncompressed contiguous RGB8 strip
  when its exact byte length follows from image dimensions. Other missing-count
  layouts retain rejection. Input bounds and decode limits remain enforced.

No version bump or release is included. The private `XING_T24.tiff` sample has
not been tested; generated fixtures cover its reported missing-tag pattern.
See [validation and compatibility scope](../tiff-legacy-layouts.md).

Pull request: [#71](https://github.com/mith-mmk/wml2-on-rust/pull/71).
