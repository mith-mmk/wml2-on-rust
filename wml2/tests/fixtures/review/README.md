# Synthetic review fixtures

## AVIF range

Generated from FFmpeg's synthetic `color=c=gray:s=8x8` source; no third-party image content.

```
ffmpeg -f lavfi -i color=c=gray:s=8x8 -frames:v 1 -c:v libaom-av1 -cpu-used 8 -crf 0 -color_range tv -pix_fmt yuv420p -still-picture 1 limited.avif
ffmpeg -f lavfi -i color=c=gray:s=8x8 -frames:v 1 -c:v libaom-av1 -cpu-used 8 -crf 0 -color_range pc -pix_fmt yuv420p -still-picture 1 full.avif
```

To remove container colour signalling without shifting item offsets, replace the
`colr` box type inside `meta/iprp/ipco` with `free`, retaining its length and payload.
The AV1 sequence header is unmodified. The integration test verifies that the fixed
decoder returns `color_description=None` and that the container has no nclx.

## LittleCMS RGB gamma reference

`gamma2-rgb.icc` and `linear-rgb.icc` are generated matrix/TRC profiles with
identity RGB/XYZ axes and a D50 white point. `lcms-gamma-rgb.csv` contains
256 RGB8 inputs and LittleCMS 2.16 outputs, using relative colorimetric intent
and NOOPTIMIZE. All profiles and input colors are synthetic.

Regenerate with Pillow's ImageCms integration:

```
python test/generate_review_icc_oracle.py OUTPUT_DIRECTORY
```

The Rust test consumes the fixed profiles/CSV without a runtime LittleCMS
dependency. It scales each input to 8/10/12/16 meaningful bits, transforms it,
and compares at the reference's 8-bit precision with a one-code tolerance
for the two implementations' quantization and 10/12-bit input rescaling.
The separate analytic test checks nonidentity output directly at each native
precision. This small reference does not cover every profile class or intent.
