# SDR Gray animation

The existing bounded `jxl = 0.8.0` animation adapter now admits integer Gray8/16
and GrayAlpha8/16, with one optional matching-depth, straight, full-resolution
alpha channel. This is an adapter extension, not a new decoder or encoder.

## Output and colour contract

- Native animation and EX delivery remain coalesced full-canvas RGBA16. Gray is
  replicated into R/G/B; missing alpha is opaque. Ordinary callbacks round once
  from normalized u16 to RGBA8. No 8-bit intermediate is used for 16-bit inputs.
- The original Gray ICC is retained separately. Simple Gray colour signalling
  gets a matching RGB neutral-axis output profile using the same white point,
  transfer function and rendering intent, with sRGB primaries. This labels the
  replicated coordinates without changing sample values or applying a CMS.
- `output_icc` and callback `ICC Profile` describe the RGBA output, not the
  one-channel source. The explicit sRGB route transforms from that output
  profile and leaves alpha unchanged. The tested fixtures use D65/sRGB transfer.
- Expansion of an arbitrary Gray reconstruction/output ICC is explicitly
  refused, including on the managed animation route. The tested refusal uses
  original-profile embedded Gray ICC. An embedded source ICC does not itself
  force refusal if the codec supplies a representable fallback output profile;
  XYB Gray combinations are not claimed as verified here. A general Gray output
  ICC/CMS extension is deferred rather than silently assigning an unrelated RGB
  profile. The still-image Gray/ICC contract is unchanged.

The existing animation resource caps, layer coalescing, timing, orientation,
loop semantics and callback lifecycle are unchanged. All decoding, final-stream
checks, profile preparation and CMS precede init/draw. Preview-bearing animation,
SMPTE timecodes, HDR/float, unsupported depths, associated/subsampled/mixed-depth
alpha and other extra channels retain their existing explicit refusals.

## Independent validation

Four synthetic 32x24, three-frame animations cover Gray8, GrayAlpha8, Gray16 and
GrayAlpha16. Official libjxl cjxl 0.12.0 (`4128790`) losslessly encodes APNG sources;
independent djxl produces all displayed canvases. Source formulas, commands and
hashes are reproducible with
`wml2/tests/fixtures/review/generate_jxl_gray_animation.py` and recorded in the
adjacent `jxl-animation-gray.json`. Only jxlp framing is stripped from the encoded
file. No WML2/jxl-rs pixel output is used to construct the oracles.

The 16-bit references require PAM MAXVAL 65535 and preserve true low bits. The
8-bit sources produce PAM MAXVAL 255 even with djxl's 16-bit option; those samples
are honestly expanded by 257. RGB references receive opaque alpha, while alpha
references retain all four channels. All four native and EX results match every
reference sample exactly, including alpha and transparent colour. Gray16 has
6,894 samples not divisible by 257; GrayAlpha16 has 9,189. Source formulas are
also checked independently of the golden bytes.

Ordinary RGBA8 frames match normalized reference rounding exactly. Explicit sRGB
frames agree within one RGB8 unit, with alpha exact. Tests check all three frames,
50/90/130ms waits, loop zero, poster equality and distinct Gray/RGB profile tags.
The source APNG's finite loop count is not preserved by cjxl, so the contract
asserts the encoded JXL loop count rather than claiming APNG loop preservation.

A fifth real fixture embeds arbitrary Gray gamma2 ICC and must fail before
init/draw. Tests additionally cover frame/output/workspace/metadata/pixel limits,
every-byte truncation, trailing data, every native cancellation checkpoint,
callback failure atomicity and recovery. Existing RGB/XYB focused animation
regressions continue passing.

On Windows x64 / Rust 1.99.0, the four Gray tests and nine existing focused
animation tests pass with jpeg-xl, image-buffer-ex and color-management. The
jpeg-xl-only Gray target passes three tests; the feature-disabled target compiles
and runs zero tests. The generator was rerun with identical image/reference
hashes. All twelve decoded frame renders were inspected: grayscale patterns,
ramps and changing alpha are readable without visible corruption. Display
inspection is supplementary, not evidence of sample precision.

Five release blocks of 200 native decodes, after ten warmups, give median times
of 0.471ms (Gray8), 0.564ms (GrayAlpha8), 0.310ms (Gray16) and 0.566ms
(GrayAlpha16) on this shared host. These are newly supported cases, not a
comparison with the formerly unsupported decoder path. They do not establish
a general speed claim. Raw trials, oracle errors and validation details are in
[jpeg-xl-gray-animation-validation.json](jpeg-xl-gray-animation-validation.json).
Existing RGB/XYB before-after benchmarks and WASM/Viewer checks are recorded
separately by the integration validation. CI/CD completion is not awaited.
