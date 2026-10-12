# High sample depth: ImageBufferEx dependency

High-depth JPEG2000 is deferred. The current decoder accepts unsigned8bit SIZ
components and emits ordinary opaque RGBA8. Its irreversible internal quantized
coefficient bound of24bits is arithmetic precision, not a24bit sample/output API.
This stage does not implement or redesign ImageBufferEx.

At the inspected source snapshot, `wml2/src/draw.rs` defines ImageBuffer storage
as `Option<Vec<u8>>`; DrawCallback::draw passes `&[u8]` RGBA rectangles, and
DrawOptions has no pixel format/depth/stride description. No ImageBufferEx
implementation exists in the current workspace. Sending16bit samples through
that contract would either lose precision or change the meaning of existing
callers' four bytes per pixel. Thus a precision-preserving receiver must precede
high-depth decoder acceptance.

Before connecting high-depth JPEG2000, decide and implement:

- A typed pixel store/view with declared channel layout, unsigned/signed/floating
  sample representation, effective sample precision and alpha range. A first
  unsigned16bit Gray/RGB/RGBA scope is possible; float requirements for9/7 should
  be decided explicitly rather than inferred from internal arithmetic.
- Initialization and rectangle delivery carrying format, row stride, sample/plane
  ordering and ownership/lifetime. Use checked byte/sample counts and alignment;
  byte order must be explicit if views use bytes instead of typed slices.
- Additive receiver/API entry points and an explicit RGBA8 conversion adapter.
  Existing DrawCallback, ImageBuffer fields, encoders and animation semantics
  must remain compatible. Specify rounding, saturation, alpha and colour/profile
  treatment when the caller deliberately requests conversion for display.
- Resource budgets based on actual sample storage and transform scratch, including
  frames/animation. Native and WASM must agree on validation and callback Abort.
- Tests with values distinguishable below one8bit step, signed extrema where
  supported, odd stride/tiles, typed callback delivery, round trips and explicit
  downconversion. No implicit acceptance followed by silent8bit clipping.

Only then extend SIZ/JP2 bit-depth validation, DC shift, quantization dynamic
range, coefficient/transform numeric bounds and RCT/ICT conversion for the chosen
sample/output types. Revisit signed samples, per-component depth, subsampling
and alpha as separate scoped decisions. Raising today's SIZ acceptance alone
would not complete high-depth support.
