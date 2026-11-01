# Bounded two-plane JPEG XL output

This is an **unmerged production draft**, based on #89
(`cb41c10084c22d738735edfac4a5d0263e6f9aa3`) and its #88…#75/#73/#72 stack.
Main0.0.41 is unchanged. It implements a bounded multiple-plane subset of #42;
#42 remains open. No product dependency changes are made.

The existing `image_with_extra_channels_from_with_resource_limits_and_cancel`
API now returns **all zero, one or two** admitted Depth/SelectionMask planes in
source order. Arbitrary order and duplicate kinds are supported. Select returned
planes by their retained `source_index`; duplicate kinds are not merged. Each
plane has its own owned buffer, actual kind, source integer depth, source/output
geometry and dim_shift. Codes stay at source depth, with no normalization to65535
unless the source itself has16 bits. Depth units, mask thresholds, Alpha/coverage
interpretation and compositing are not inferred.

Color and each extra depth are independently8/9/10/12/16 bits. The existing
original-profile Gray/RGB SDR, full-resolution, single full-canvas Modular,
no-preview/XYB/resampling envelope stays. Color is original-profile normalized
RGBA16 with opaque alpha; Gray coordinates are replicated and retain their Gray
profile identity. All planes apply the same source orientation. ICC describes
color only and never transforms extra codes.

Three or more extras, Alpha alone/combined, Spot/other kinds, float/subsampled
extras and unqualified layers reject the entire request, without silently
selecting, blending or dropping a channel. Ordinary/EX/sRGB/F32/F16/HDR APIs keep
their refusal contracts. A legacy fixture named
`jxl-selection-refused-selection-multiple.jxl` is now explicitly accepted as two
SelectionMask planes; its name and older reports reflect the historical #89
one-plane contract. Historical manifests remain snapshots rather than being
rewritten as current product acceptance lists.

Names still lack a pinned upstream public accessor. Exact bounded source header
bytes retain names opaquely; this does not expose names or implement name-based
selection. No private parser copy or Debug-string parsing is introduced. Returning
all admitted planes permits caller-side index selection; selective decode and
larger counts are not claimed.

Existing input/pixel/metadata/workspace limits, Main5 policy and fixed codec caps
are maintained. Only this API's supported plane-count envelope changes from1 to2.
Checked arithmetic admits the sum of color and **all** plane payloads under
`animation_bytes` and workspace before allocations.35 pixels require420 bytes
(RGBA16=280 plus two70-byte planes);419 refuses. Accounting includes simultaneous
plane-vector handles, format/output/result descriptors, retained header/ICC and
profile temporary storage, along with existing conservative codec preflight.
The codec preflight already uses the actual extra count for padded/ref/workspace
samples; this is retained. No global codec allocator/RSS bound or large-image
capability is claimed (#37/#63 remain).

Each plane reserve is fallible, including the second reserve after the first is
live. Two initialized disjoint Vec owners supply exclusive byte views through
`iter_mut`; their borrowed output buffers die before ownership transfers to the
result. Output is published only after both planes/color complete, final stream
and profile boundaries pass, and sample ranges/cancellation are checked.
Errors drop all owned intermediate storage and never return a partial result.

[Independent oracle manifest](jpeg-xl-two-plane-oracle.json) and
[validation report](jpeg-xl-two-plane-validation.json) record:

- 54 independently C-encoded/C-decoded libjxl0.11.2 cases: Gray/RGB ×25 positional
  depth pairs with both kind orders, plus duplicate-kind controls with identical
  and mixed depths. Distinct patterns expose swapped/aliased/lost planes. All8
  orientations and embedded gamma2 ICC are covered. Both decoded source-code
  planes match authored data through an independent orientation map exactly;
  color agrees with C within1 U16 unit.
- Five C-readable negative inputs:3 planes, Spot mixed with Depth, float16,
  subsampled, and multiple layers. Existing Alpha-combination refusal stays.
- 209 expanded native tests (library97, existing Depth6/Selection6, two-plane5),
  minimal jpeg-xl17 and disabled targets0. Tests cover exact combined output and
  workspace boundaries, metadata/input/pixel/frame/layer limits, every truncated
  prefix, all17 cancellation checkpoints, scoped allocation release,9 actual
  allocator refusal sites (including both plane reserves) and recovery.
- Actual wasm32/Node727 grouped checks,57 new for this subset, without host
  imports. The recorded final memory size is not a peak/global heap guarantee.
  Windows32 execution is verified by CI rather than claimed on this Linux host.
- Displayed C/product sRGB color plus both raw-plane diagnostic grayscale images
  match byte-for-byte. Diagnostic grayscale is not a product rendering policy.

CPU0 release comparisons rebuild exact #89 and the new source with identical
features/inputs/harness;3 warmups then1000 samples/process,6 alternating pairs.
After/before median ratios are ordinary1.0097, managed1.0050 and explicit single
Depth1.0004. Block ranges overlap; no stable regression or speedup is established.
New two-plane standalone medians are67.65/68.14/66.46µs for7×5 RGB8/Mask8/Depth8,
with no accepted before counterpart. All blocks and output hashes are retained.

To reproduce, build current `jpegxl_multi_fixture_encoder.c` as
`.test-oracle/encoder`, `jpegxl_extra_reference.c` as `.test-oracle/reference`,
and `jpegxl_alpha_reference.c` as `.test-oracle/rgba-reference`, using the pinned
libjxl0.11.2 headers/library and `cc -O2 -Wall -Wextra -Werror -I "$JXL_INCLUDE"`.
Run `python3 wml2-test/scripts/generate_jpegxl_two_planes.py --work .test-oracle`.
The reference remains compatible with prior one-plane invocations and writes
separate numbered outputs for multiple planes. The manifest records generator,
encoder, reference, fixture and independent oracle hashes. Names are distinct
in the C fixtures but are retained opaquely by the product.
