# Declared straight subsampled alpha

This unmerged follow-up to #85 implements the next part of #42: declared
`dim_shift=1/2/3` straight integer alpha, reconstructed by published jxl0.8.0 at
2/4/8× to the full output canvas. It reuses the codec resampler and existing
ordinary/EX/own-CMS outputs. No new dependency, API or display policy is needed.
General extra-plane retention/selection remains a different, unimplemented part
of the original Issue; this is production integration, not another evaluation PR.

## Accepted envelope and resources

Keep the existing8/9/10/12/16-bit color/alpha whitelist, original-profile Gray/RGB
SDR, one straight alpha, no preview and one full-canvas Modular frame. The color
upsampling and raw frame EC factor must both be1; no additional per-frame alpha
reduction or cropped/VarDCT/layer composition is admitted by this new route.
Associated subsampling, float/HDR/XYB, preview and multiple/non-alpha extras still
reject explicitly. Existing full-resolution associated/mixed alpha is unchanged.
#42 remains open; this is not main0.0.41 support.

The parser-only0.7.4 `FrameHeader::postprocess` applies `dim_shift` only when
color upsampling>1, while pixel codec0.8.0 applies it unconditionally. For this
route, the old parser therefore retains raw EC1. Preflight explicitly validates
that raw value with color factor1, and budgets the actual0.8 factor `1<<dim_shift`.
It does not compare the old postprocessed value to an assumed0.8 value, skip the
frame scan or relax the codec's limits. Registry source hashes/checksum checks
and executed oracle results are recorded in the validation JSON.

Before codec allocation, add the0.8 resampler's `factor*5*5*factor` F32 kernel
(`100*factor²` bytes), plus two conservative F32 alpha planes on the grid rounded up to the factor
(`8*rounded_alpha_pixels`) alongside the existing codec/row/retained-plane estimate.
Modular frames have no color padding: that rounded alpha grid also tightens the
existing internal-pixel check and sample/workspace estimate. A33×17 image at8×
therefore needs40×24 internal alpha pixels: cap959 rejects and960 admits.
This additional allowance remains in the retained workspace through the existing
output/CMS/collector accounting. Source alpha is smaller, but no reduced-plane
saving is used to lower admission. Caller limits, geometry/padding/sample/TOC/
ICC/channel caps remain, with the existing coded-layer quota tightened to one
for this route. These are conservative admission plus owned-buffer ledgers,
not a global allocator, stack/RSS or measured peak guarantee.

Native output is normalized RGBA16 after interpolation. Ordinary RGBA8 uses
existing explicit rounding; EX Exact uses Rgba16 and lower precision requires
Round. Source depth metadata is retained. New `JPEG XL Source Alpha Dim Shift`,
`JPEG XL Source Alpha Width` and `JPEG XL Source Alpha Height` describe the
logical coded alpha sample grid before padding/orientation (`ceil(coded_axis /
2^dim_shift)`). They do not describe a returned raw plane. Output geometry is the
full, already oriented image; no source orientation is applied again by callers.
Interpolation can differ between independent implementations by1u16 in the tested
vectors. This is distinct from CMS: WML2 copies its own interpolated alpha exactly
through CMS; no color transform is applied to alpha.

## Independent qualification

Build `jpegxl_alpha_fixture_encoder.c` and `jpegxl_alpha_reference.c` with
`-Wall -Wextra -Werror` against installed libjxl0.11.2, producing
`.test-oracle/encoder` and `.test-oracle/reference`. Then run
`python3 -B wml2-test/scripts/generate_jpegxl_subsampled_alpha.py --work .test-oracle`.
[Oracle hashes](jpeg-xl-subsampled-alpha-oracle.json) record:

- 42 Gray/RGB cases:3 declared factors ×7 representative color/alpha pairs,
  covering each supported depth, all8 orientations and odd33×17 geometry.
- 6 tiny7×5 zero/opaque controls, including the1×1 alpha grid at8×.
- The historical valid16/8 subsampled refusal, now an independent positive oracle.
- Two C-decodable refusals: associated subsampling and multiple coded layers.

Independent C output—not candidate output—constructs normalized RGBA16 references.
Two RGB ICC cases compare C output-profile → LittleCMS2.16 relative RGB16 with
NOOPTIMIZE, explicit final8-bit rounding and alpha copied. Source/reconstruction
ICC bytes are independently checked against the embedded profile. Native RGB
and alpha differences are bounded by1u16; ordinary output by1u8, managed RGB by
2u8. Fully transparent/opaque native controls are exact. CMS alpha equals WML2
ordinary alpha exactly; its C comparison allows the1u8 interpolation/rounding
bound. No claim of recovering alpha values lost by source downsampling is made.

Four new production tests exercise49 accepted inputs, source-grid descriptors, successful absence of preview,
EX precision refusal, exact workspace boundary, input/pixel/frame/output/metadata
limits, every truncated prefix, the two valid unsupported fixtures and scoped
zero net retained heap. Existing callback tests add Abort and early-limit/no-
callback checks on a new8× ICC input. Both2× and8× ICC decodes expose24 tested
cancellation checkpoints each, with kernel and owned storage released on errors.

[Execution and timing evidence](jpeg-xl-subsampled-alpha-validation.json) record
144 expanded and33 minimal passing tests and feature-disabled compilation.
Actual Node24 WASM executes461 checks, including all new native/legacy pixels,
source-grid dimensions, exact one-byte workspace admission and explicit refusals.
Final observed WASM memory size is not a worst-case peak or global bound. The
existing CI target lists include macOS/Windows and actual Windows32 ordinary/EX/
disabled execution; these are CI rather than local Linux results.

The oriented8× ICC case is rendered over a checkerboard with C+LittleCMS and
WML2 output and visually compared. Six alternating same-build/input/CPU0 control pairs record per-process medians
and ranges in the validation JSON. Ranges overlap; no speedup inference is made.
New33×17 8× costs are recorded separately because the parent rejects that input.
Three warmups then1000 samples/process, Rust1.91 release and identical features;
full per-block variability is retained in JSON. No merge, approval or release.


The subsequent [associated subsampling stage](jpeg-xl-associated-subsampling.md)
implements that integer combination within the same bounded frame envelope. EX
Exact remains value-aware: native Rgba16 can narrow exactly when every value is
representable; otherwise use Round. Earlier descriptions of Rgba16 as required
refer to native output, not an unconditional rejection of exact narrowing.
