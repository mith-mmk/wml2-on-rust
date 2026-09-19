# TIFF interoperability manifest

`manifest.json` records the pinned external corpus and the samples used for
local interoperability checks. The TIFF binaries are intentionally not
redistributed in this repository; place the corpus `valid/` directory in a
local path and set `WML2_TIFF_CORPUS` to that directory when running the
external sample tests.

The manifest hashes are checked before a sample is decoded. A mismatch is a
fixture failure, not a reason to silently accept a different sample.

## Local commands

Set `WML2_TIFF_CORPUS` to the local `valid/` directory from the pinned corpus:

```powershell
$env:WML2_TIFF_CORPUS = 'D:\data\samples\images\tiff\codec-corpus\valid'
python wml2-test/scripts/verify_tiff_interop_samples.py `
  --corpus $env:WML2_TIFF_CORPUS `
  --manifest wml2/tests/fixtures/tiff_interop/manifest.json
cargo test -p wml2 --test tiff_interop --test tiff_ycbcr --test tiff_jpeg_extend `
  --no-default-features --features tiff-jpeg,idct_llm
```

The oracle harness is optional and is run against the same local corpus:

```powershell
python wml2-test/scripts/tiff_oracle.py `
  --corpus $env:WML2_TIFF_CORPUS `
  --manifest wml2/tests/fixtures/tiff_interop/manifest.json `
  --converter <path-to-converter> `
  --metadata <path-to-metadata> `
  --output-dir <temporary-output>
```

Samples marked `oracle_policy=expected_only` are not accepted or rejected by
ImageMagick/Pillow pixel comparisons. Their WML2 RGBA8 expectations and
explicit error behavior are authoritative; the external tools are diagnostic
only because their old-JPEG, chroma-upsampling, or nonstandard-fax behavior
is not the compatibility contract.
