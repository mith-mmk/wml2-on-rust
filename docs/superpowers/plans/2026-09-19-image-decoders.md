# TGA / PCX / DDS / PIC2 decoder implementation plan

## Scope

Add four independent opt-in Cargo features (`tga`, `pcx`, `dds`, and `pic2`) and feature-gated decoders for those formats to `wml2`, including format detection, public extension reporting, draw dispatch, metadata, and integration tests. Do not change the existing default feature set in this stage. Preserve the existing TIFF work and the pre-existing untracked plan file.

## Stages

1. Add `tga`, `pcx`, `dds`, and `pic2` features and module/dispatch wiring.
2. Implement TGA: indexed, true-color, grayscale, uncompressed and RLE packets; honor origin and palette fields.
3. Implement PCX: header validation, scanline RLE, indexed palettes, planar low-bit-depth data, and RGB planes.
4. Implement DDS: legacy headers, uncompressed masks, DXT1/DXT3/DXT5, ATI1/ATI2, and common DX10 BC/RGBA mappings needed by the collected samples.
5. Implement PIC2: `P2DT` header, optional MacBinary prefix, palette/comment area, P2SS blocks, 15/24-bit output, and safe bounds checks. Keep unsupported block variants explicit.
6. Add focused unit/integration tests using deterministic in-memory fixtures and the collected external samples through an opt-in sample-root environment variable.
7. Run formatting, feature-specific tests, default tests, and a full compile; report verified versus unverified coverage separately.

## Acceptance

- Each format is detected only when its structural signature is valid.
- Collected samples decode to non-zero dimensions without panics or out-of-bounds reads.
- Malformed/truncated inputs return errors.
- Feature-off builds remain valid.
- No external reference source is copied into the crate; it is used only as format evidence.
