# Associated and reconstructed Main5 extra planes

This **unmerged production draft** extends #91
(`9efd75638e13132c09cac6ab834db730193e2ecd`) and its #90…#75/#73/#72 stack.
Main0.0.41 remains unchanged. The existing owned extra-plane API now qualifies
associated Alpha and uniform extra reconstruction alongside multiple planes,
reusing the legacy straight-output normalization and bounded resampling screens.

All fixed Main5 slots0..4 remain available: Depth/SelectionMask, duplicate kinds,
and at most one Alpha at any source index. Source depths remain independently
integer8/9/10/12/16, with original-profile Gray/RGB SDR, one full-canvas Modular
frame, no preview/XYB or color resampling. All admitted planes are returned in
source order. No primary Alpha is chosen from multiple Alpha channels.

Associated sources select the **same `AlphaMode::Unpremultiplied`** used by the
existing legacy path. Output `rgba16` stays straight, normalized16 and original
profile, including existing integer clipping and zero-alpha behavior. Source
association is retained, not silently erased. Straight-source hidden RGB is
unchanged. Alpha and other separate plane values never pass through ICC or a
background blend. Depth units, selection thresholds and physical meaning remain
unspecified.

Uniform **effective** extra upsampling1/2/4/8 is qualified, with Alpha metadata
`dim_shift`0..3 and non-Alpha declared shift0. Frame factors and metadata shifts
are distinct: the0.7.4 preflight parser leaves EC factors unshifted when color
upsampling=1, while0.8 applies the metadata shift unconditionally. The explicit
route computes `effective = parsed_raw_factor << dim_shift`, checks the fixed8x
bound, and returns it. It does not trust0.7.4 postprocess to stand in for0.8.

Three additive fields in the already non-exhaustive `JxlExtraPlane` clarify the
output; successful previously admitted inputs retain their existing pixel values:

| Field | Meaning |
| --- | --- |
| `bits_per_sample` | Original coded source depth, unchanged. |
| `sample_bits_per_sample` | Returned integer code precision: source depth for effective1; normalized16 for reconstructed planes. |
| `source_alpha_associated` | Source color association, true only on associated Alpha; returned RGBA16 is straight. |
| `dim_shift` | Original declared metadata shift (Alpha only in this subset). |
| `upsampling` | Effective factor including frame and metadata scaling. |
| `source_dimensions` | Coded plane grid `ceil(coded_canvas/effective)` before orientation. |
| `dimensions` | Oriented decoded full-canvas raster geometry. |

`samples` contains the full raster at `sample_bits_per_sample`. For effective1 it
retains source-depth codes exactly. For effective>1 it contains **normalized
reconstructed16-bit samples**, including the codec's interpolation/quantization;
it does **not** contain the smaller native coded-grid values. Source depth and
grid geometry are preserved as metadata, without claiming a public native-grid
pixel getter. The public rendering interface used here returns reconstructed
rasters; no private pixel parser or implementation is copied. The returned Alpha
plane and RGBA16 alpha are cross-checked in their declared code precision.

The verified C fixture API supplies one global EC resampling setting. Accordingly
this production subset explicitly refuses **mixed effective factors** rather than
claiming an independently unqualified combination. This is a qualification scope,
not an asserted decoder API impossibility. Non-Alpha declared subsampling, float
extras, multiple Alpha, other kinds, HDR/preview/XYB/cropped/multiple layers remain
outside this contract.More than4 extras remain the real fixed Main5 limit; names remain
the pinned public accessor gap. Neither is reclassified as merely unimplemented.

Resource accounting reuses the legacy kernel/rounded-plane allowance **for each
reconstructed plane**, summing every live kernel and conservative rounded f32
plane pair. The largest rounded plane contributes to sample/internal-pixel bounds;
source count contributes to padded/reference workspace. Format/output/result
storage, every color/plane payload, source header/ICC and generated-profile
scratch remain admitted before allocation.4 plane payloads with35 pixels still
need560 output bytes and reject559. Fixed Main5/8x/pixel/input/metadata/byte caps
and finite policy are maintained. Every adapter reserve is fallible and errors
publish no partial color or plane. This remains conservative codec preflight plus
adapter accounting, not a global heap/RSS guarantee or large-image enablement.
Legacy-only sampling qualification is retained separately.

[Independent oracle manifest](jpeg-xl-extra-alpha-oracle.json) and
[validation report](jpeg-xl-extra-alpha-validation.json) record:

- 120 C libjxl0.11.2 vectors:50 associated full-resolution Gray/RGB ×25 color/Alpha
  depth pairs,60 straight/associated declared2/4/8x mixed-depth cases,4 constant
  transparent/opaque controls,2 colored-zero stress inputs,2 reconstructed
  non-Alpha-only cases and2 frame-only4x cases with Alpha declared shift0.
  Counts2..4, all source Alpha positions, duplicate kinds, all8 orientations and
  gamma2 ICC are covered. Full-resolution raw codes recover authored data exactly;
  reconstructed16 planes and straight color match independent C within1 U16 unit.
  Upsampled source pixels are not falsely compared with pre-encoding full-size
  author data. Historical associated refusal input is now explicitly accepted.
- 221 expanded native tests (library99, new extras5), minimal27 and disabled
  targets0. Exact output/workspace, metadata/input/pixel/frame/layer bounds,
  all truncated prefixes,20 cancellation checkpoints,11 actual allocator refusal
  sites including all four plane reserves, recovery and scoped release pass.
- Actual wasm32/Node909 grouped checks,123 new, no host imports. Recorded final
  linear memory is not a peak/global heap ceiling. Windows32 execution is in CI.
- C/product original-profile straight color is independently converted with
  Pillow CMS to sRGB and composited diagnostically; all4 reconstructed planes
  are also displayed. The chosen comparison has identical source arrays and
  diagnostic pixels. This does not introduce a product display API.

CPU0 Rust1.91 release compares rebuilt exact #91 and this source with identical
features/inputs/harness:3 warmups then1000 samples/process,6 alternating pairs.
After/before ratios: ordinary 0.9869, managed 0.9880, explicit_Depth 1.0044. Ranges overlap; no stable regression or speedup
is established. New four-plane associated4x medians are 109.35/109.78/112.02 microseconds
for7x5 input, with no accepted before counterpart. Full blocks/hashes are retained.

Reproduction: build current `jpegxl_multi_fixture_encoder.c`,
`jpegxl_extra_reference.c`, `jpegxl_alpha_reference.c` as
`.test-oracle/{encoder,reference,rgba-reference}` with pinned libjxl0.11.2 headers
and library using `cc -O2 -Wall -Wextra -Werror`.
Run `python3 wml2-test/scripts/generate_jpegxl_extra_alpha.py --work .test-oracle`.
The reference's optional final `straight` argument requests independent C
unpremultiplication and keeps prior default invocation behavior. All source and
fixture/oracle hashes are saved. Existing #42 acceptance conditions are retained;
#42 stays open for mixed sampling, other kinds/storage, names and remaining
combinations, rather than adding a new completion checklist.
