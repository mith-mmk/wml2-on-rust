# TIFF interoperability manifest

`manifest.json` records the pinned external corpus and the samples used for
local interoperability checks. The TIFF binaries are intentionally not
redistributed in this repository; place the corpus `valid/` directory in a
local path and set `WML2_TIFF_CORPUS` to that directory when running the
external sample tests.

The manifest hashes are checked before a sample is decoded. A mismatch is a
fixture failure, not a reason to silently accept a different sample.
