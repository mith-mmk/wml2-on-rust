# Lab-PCS paired LUT registry regression

`lab-pcs-rgb.icc` is copied byte-for-byte from `icc-profile` 0.0.7's
`tests/fixtures/lab-pcs-rgb.icc`. It is a synthetic RGB monitor calibration
serialized by LittleCMS 2.16, with v4.3 header, Lab PCS, and A2B0 `mAB ` /
B2A0 `mBA ` stages. The 17×17×17 CLUTs sample independent LittleCMS sRGB↔Lab
transforms. The nonsemantic creation date is fixed to 2026-10-18 00:00:00.

SHA256: `1b7dea170895aacd22a8c76344b06c54ccf18882a573698bb8e8a07f6cfec2b4`.

The frozen independent LittleCMS 2.16 RGB16 reference for input
`[32768,32768,32768]` through this saved profile in both directions is
`[32753,32772,32778]`. Settings: relative colorimetric intent 1,
`TYPE_RGB_16`, flags `0x0140` (`cmsFLAGS_NOOPTIMIZE | cmsFLAGS_NOCACHE`),
BPC off, no alpha. The oracle reopens the serialized profile before evaluating.
The WML2 regression allows 32 u16 codes for interpolation/rounding differences.
Published `icc-profile` 0.0.6 produced `[458,182,294]`; 0.0.7 produces
`[32744,32772,32778]` through the public `FrameTransform` adapter.

The test changes only the destination profile's nonsemantic creator signature
to `WML2` to distinguish destination metadata from original source provenance.
It also checks that explicit conversion preserves the original frame/pixels,
source color information and native 16-bit precision. An existing generic
`ICC Profile` tag follows the destination after conversion; `Source ICC Profile`
and typed `source_color` remain original provenance. Conversion does not add
a generic profile tag when the input has none.

Generation script and complete oracle CSV are retained upstream in
[`tests/generate_pcs_lut_oracle.py`](https://github.com/mith-mmk/icc-profile/blob/0.0.7/tests/generate_pcs_lut_oracle.py)
and [`tests/fixtures`](https://github.com/mith-mmk/icc-profile/tree/0.0.7/tests/fixtures).
LittleCMS is fixture-generation tooling only; WML2's test uses the ordinary
published `icc-profile` dependency and adds no runtime or development dependency.
