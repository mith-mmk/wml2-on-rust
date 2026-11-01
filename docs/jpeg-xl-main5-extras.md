# Main5 extra planes with straight Alpha

This **unmerged production draft** extends #90
(`1e2a27f0dc9a94cb70f8991c2b44b01cf7c6d868`) and its #89…#75/#73/#72 stack.
Main0.0.41 is unchanged. A subsequent [associated/reconstructed extension](jpeg-xl-extra-alpha.md)
qualifies associated Alpha and uniform1/2/4/8x extra reconstruction; the straight/full-resolution
reports below remain the historical #91 snapshot. It addresses the original #42 multiple-plane and Alpha
combination requirements as one generalization of the shipping envelope.

The earlier2-plane bound was an implementation-stage restriction. The pinned0.8.0
codec's `api/codestream_parser/image_info.rs::parse_file_header` actually limits
Main5 to **4 extra channels** (Main10 permits256). The adapter fixes
`max_profile_level=Main5`, so4 is a technical shipping limit, not another arbitrary
stage ceiling. The0.7.4 preflight parser's256-channel ceiling does not grant
Main10 permission. No profile/pixel/input/byte/workspace caps or dependencies
change. The report records matching locked-archive hashes for these statements.

The existing `image_with_extra_channels_from_with_resource_limits_and_cancel`
API returns **all0..4 admitted extras**, with arbitrary Depth/SelectionMask order,
duplicate kinds and **at most one straight Alpha at any source index**. Each
plane retains source index/kind/depth/dim_shift/source and oriented geometry in
an independent owned buffer. There is no type-based merging or silent selection.
Caller-side index selection is available; selective decoding and names are not.
`Alpha` is appended to the non-exhaustive kind enum, preserving existing variant
positions. No new entry function or resource-policy field is introduced.

Color and every extra depth are independently integer8/9/10/12/16. The envelope
remains original-profile Gray/RGB SDR, full-resolution, one full-canvas Modular
frame, without preview/XYB/resampling. Gray coordinates are replicated into RGB
lanes but keep their Gray profile identity via `color_channels=1` and source ICC.
Color is normalized original-profile **straight RGBA16**. Without Alpha it is
opaque. With Alpha, the RGBA lane is normalized to16 bits, while the separate
Alpha plane keeps exact decoded **source-depth codes** (8-bit code128 stays128).
Both outputs are intentionally retained and counted. Integer normalization may
differ by1 U16 unit at rounding boundaries; the adapter cross-checks RGBA alpha
against the raw plane. No premultiplication/unpremultiplication occurs for these
straight sources; colored zero-alpha RGB is preserved. ICC never transforms
Alpha or any other raw extra. No Depth units, mask threshold/coverage or
compositing operation is inferred.

The remaining boundaries are distinct:

| Item | Current status / reason |
| --- | --- |
|0..4 Depth/SelectionMask + zero/one straight Alpha | Implemented within the fixed Main5 envelope. |
| More than4 extras | Real Main5 limit; requires an explicitly reconsidered profile/resource policy. |
| Associated Alpha with separately returned extras | Not implemented/qualified here. Public AlphaMode exists; this is not claimed as an upstream API blocker. Existing legacy associated normalization remains. |
| Multiple Alpha roles | Explicitly refused; no implicit choice of primary Alpha. |
| Float/subsampled extras, preview/XYB/HDR/other layers and other kinds | Unimplemented/unqualified for this owned extra-plane contract; ordinary/EX behavior stays unchanged. |
| Names/name-based selection | Real pinned public API gap:0.8 descriptors omit names,0.7.4 parser names are private. Header bytes remain opaque; no private parser clone. |

All unsupported combinations reject the whole request. Historical fixtures named
`alpha-refused-multiple` (Alpha+Depth) and `two-refused-three` are now explicitly
accepted by this API; ordinary/EX/sRGB/F32/F16/HDR routes retain their previous
contracts, including single-Alpha acceptance and non-alpha-extra refusal.
Historical manifests describe their original commits rather than current
acceptance. #42 remains open for the residual contract combinations and names.

Preflight uses the actual channel count for sample/padded/reference workspace.
Fixed four-slot descriptor/Vec-handle storage is bounded without dynamic header
allocation. Adapter accounting admits simultaneous color + **every** raw plane,
all live format/output/result descriptors, retained header/ICC and profile scratch
before allocation.35 pixels with4 planes require560 output bytes and reject559.
Each reserve is fallible; an error on the last plane drops the previous three.
The initialized separate Vec owners supply disjoint byte views via `iter_mut`;
result ownership transfers only after complete decode, final profile/stream,
source-code ranges, Alpha consistency and cancellation checks. Errors publish
nothing. This is conservative codec preflight plus cumulative adapter accounting,
not a global codec heap/RSS bound or large-image enablement (#37/#63 remain).

[Oracle manifest](jpeg-xl-main5-extras-oracle.json) and
[validation report](jpeg-xl-main5-extras-validation.json) retain:

- 56 independent C libjxl0.11.2 positive cases: Gray/RGB ×25 color/Alpha depth
  pairs, counts1..4, Alpha in every position including all4 positions in4-plane
  files, ordered/duplicate Depth/SelectionMask and separate source patterns.
  All8 orientations, gamma2 ICC, transparent/partial/opaque pixels and whole-image
  transparent/opaque Alpha are covered. C color/Alpha agrees with authored
  straight coordinates (including hidden RGB), and every raw plane exactly
  matches authored codes via an independent orientation map. Product color
  agrees with C within1 U16 unit; all raw source codes match exactly.
- Seven C-readable refusals:5 extras, multiple/associated Alpha, Spot mixing,
  float, subsampled, and layers.5 extras are a valid C input but exceed Main5.
- 215 expanded native tests (library98, Main5 extras5), minimal22, disabled targets0;
  exact560/559 output/workspace boundaries, metadata/input/pixel/frame/layer limits,
  every truncation, all19 cancellation checkpoints,11 actual reserve failures
  including the third/fourth plane, recovery and scoped net allocation release.
- Actual wasm32/Node786 grouped checks,59 new, without host imports. Recorded
  final linear memory is not peak/RSS. Windows32 execution is verified in CI.
- A displayed diagnostic converts original Gamma2 color with Pillow CMS to sRGB
  and composites straight Alpha over a checkerboard, alongside all4 raw planes.
  C/product diagnostic pixels match. This is verification, not a product CMS or
  mask-rendering API; raw plane hashes and diagnostic difference are recorded.

CPU0 Rust1.91 release compares rebuilt exact #90 and final source with identical
features/inputs/harness:3 warmups,1000 samples/process,6 alternating pairs. Median
after/before ratios: ordinary 0.9914, managed 0.9927, explicit_Depth 1.0092. Block ranges overlap; no stable regression or
speedup is established. New four-plane Alpha medians are 87.74/88.28/89.28 microseconds
for7x5 RGB8/Alpha12/Mask16/Depth8/Mask9. No accepted before counterpart exists.
All block medians, output hashes and build details remain in the report.

Reproduction: build `jpegxl_multi_fixture_encoder.c`, `jpegxl_extra_reference.c`,
`jpegxl_alpha_reference.c` as `.test-oracle/{encoder,reference,rgba-reference}`
with pinned libjxl0.11.2 headers/library and `cc -O2 -Wall -Wextra -Werror`.
Run `python3 wml2-test/scripts/generate_jpegxl_main5_extras.py --work .test-oracle`.
Source and fixture/oracle SHA256 values are in the manifest. The reference keeps
its earlier1-plane output convention and emits separate numbered files otherwise.
