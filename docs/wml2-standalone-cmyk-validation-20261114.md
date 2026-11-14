# WML2 standalone CMYK validation (2026-11-14)

This follow-up tests WML2 itself, not Viewer behavior. It adds tests only; no
production decoder/ICC implementation changes, merge, or publication. The backend
is registry icc-profile 0.1.0 with the checksum recorded in Cargo.lock.

## Results on macOS arm64 / Rust 1.97.0

- `cargo test --locked -p wml2 --lib --tests --features 'color-management psd'`:
  **380 passed, zero failed, 10 ignored**.
- The same command with `--features psd` (color management disabled):
  **335 passed, zero failed, seven ignored**.
- `cargo fmt -p wml2 --check` and `git diff --check` passed.
- The new opt-in real YCCK route test passed once with color management on and
  once with it off. These two runs are additional to the ordinary-suite counts.

## What was strengthened

### JPEG / YCCK

The existing backend-to-backend comparisons are retained but are no longer the
only color oracle. A new independent piecewise-sRGB/subtractive-product formula
checks all pixels of nonuniform baseline/progressive CMYK, with and without Adobe
inversion, and independently encoded YCCK. The synthetic profile's XYZ matrix uses
fixed-point nodes; display RGB tolerance is at most three 8-bit values. This is an
analytic profile test, not a claim about arbitrary printing profiles.

Broken, truncated, and valid-but-RGB profiles on baseline/progressive/YCCK produce
exactly one CMYK warning and preserve every legacy pixel. Active source ICC is not
left attached to transformed/display RGB output. An 8192-byte expanded budget fits
the plain JPEG but rejects the CMYK converter workspace: baseline and progressive
both return an error with **zero draw callbacks**, rather than silently falling
back. This proves this preparation-failure case, not image-wide rollback.

The local 500x333 photograph has four components, APP14 transform=2 (**YCCK**), and
557168 embedded ICC bytes. Its ICC-stripped derivative retains APP14 and scan bytes.
With color management on, both decode but their pixels differ and the managed path
retains Source ICC Profile without a CMYK fallback warning. With color management
off, the original and stripped pixels are identical and disabled-color-management
is reported. Both paths preserve dimensions and opaque alpha. The earlier 256x256
text JPEG is Adobe CMYK (transform=0), without ICC. The previous five-image PNG
visual/byte comparisons remain applicable because production code is unchanged.
No private real image/profile is added to the repository.

### Native precision and chunk boundaries

A 514-pixel varied u16 test places two inputs that round to the same u8 values on
opposite sides of the internal 256-pixel boundary. Their u16 RGB outputs must differ,
all alpha values must match, and independent analytic RGB error is at most 160 u16
values (less than one display-u8 step, allowing fixed-point matrix/node error).

A 513-pixel u32 test checks the independent analytic display result within one u8
value. More than 32 components must differ by at least two values from an explicitly
premature-u8-rounded path. This rules out early input quantization for these probes.

### PSD and TIFF intersections

- PSD: 259x2 nonuniform pixels, 8/16 bit, raw/RLE/ZIP/ZIP-prediction: **8 combinations**.
- TIFF: 259x3 nonuniform pixels, 8/16 bit, both byte orders, strip/tile,
  chunky/planar, raw/deflate, Classic/BigTIFF: **64 combinations**, including final
  short strips and right/bottom tile padding.
- Every combination checks independent analytic RGB within three u8 values,
  exact alpha, warning count, preserved Source ICC, and no active source ICC on RGB.
- Existing tests also exercise packed 6-bit/native 32-bit TIFF, invalid profiles,
  wrong models, alpha rules, unsupported InkSet, and ordinary format regressions.

## Scope and remaining gaps

The full local suites above include ordinary JPEG/PNG/GIF/WebP/BMP/TIFF/PSD and
other enabled default-format regressions, not every optional codec configuration.
Hosted CI independently covers additional platforms/features; inspect the exact
commit's run rather than reusing a prior green run.

Not added here: exhaustive real-printing-profile/intent/BPC colorimetric comparison;
CMYK PSD raster-layer compression/opacity combinations; ICC-enabled TIFF LZW,
predictor and packed-bit layout cross-products; per-page changed-profile failure
injection; a transform failure injected after previously published chunks; allocator
fault injection; full peak-memory or controlled performance benchmarking. The
shared converter explicitly does not promise image-wide rollback. JPEG's inherited
generic plane-4 warning is separate from an actual CMYK fallback warning.

## Reproduction of real route checks

Set `WML2_CMYK_PROFILE_JPEG` to the local tagged YCCK photograph, then run:

```sh
cargo test --locked -p wml2 --features 'color-management psd' \
  --test cmyk_external_validation \
  real_ycck_profile_and_profile_removed_select_expected_routes -- --ignored
cargo test --locked -p wml2 --features psd \
  --test cmyk_external_validation \
  real_ycck_profile_and_profile_removed_select_expected_routes -- --ignored
```
