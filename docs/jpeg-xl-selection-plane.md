# Bounded JPEG XL SelectionMask planes

This **unmerged production draft** extends #88 on its existing stack (#87 through
#75/#73/#72). Main0.0.41 is unchanged; #42 stays open. A subsequent
[two-plane extension](jpeg-xl-two-planes.md) admits Depth/SelectionMask pairs; the
one-plane envelope and refusal reports below describe the historical #89 snapshot. SelectionMask is the next
small subset because its kind, integer depth and geometry can be retained without
inventing Spot/CFA interpretation, Alpha normalization, multi-plane selection or
unavailable channel names. No dependency or resource-cap changes are made.

The existing `image_with_extra_channels_from_with_resource_limits_and_cancel`
entry point now admits zero or one **Depth OR SelectionMask** channel and returns
its actual `JxlExtraChannelKind`. `SelectionMask` is an additive variant of the
already non-exhaustive enum. All existing Depth behavior remains supported.

The admitted envelope stays original-profile Gray/RGB SDR with independent color
and extra depths8/9/10/12/16, one full-canvas Modular frame, no preview/XYB or
resampling, and full-resolution extra data. Every admitted channel is returned;
multiple channels, Alpha (alone or combined), Spot/other kinds, float/subsampled
SelectionMask, animation and unqualified frame layouts reject. Both C-valid
SelectionMask+SelectionMask and the existing Alpha+Depth inputs test count/type
refusal. No additional channel is silently discarded or rendered into RGBA.

The extra plane contains **decoded source-depth codes**, not a65535-normalized
mask: e.g. an8-bit code128 stays128 in `Vec<u16>`. It retains source index0,
SelectionMask kind, source depth/dim_shift and coded/oriented dimensions. Color
remains normalized original-profile RGBA16 (Gray coordinates replicated and
identified by `color_channels=1`), with synthesized opaque alpha. Both outputs
apply the same declared orientation. Color ICC never transforms mask codes.
No threshold, coverage/weight normalization, Alpha interpretation or compositing
operation is inferred; callers use the raw codes according to their application.

Name-based selection remains unavailable: pinned0.8 public descriptors expose
kind/association, while0.7.4 parser names are private. Exact bounded source-header
bytes preserve encoded names opaquely, without pretending to expose their names
or copying a private parser. Source index selection of the returned single plane
is available; decode-time multi-plane selection is not claimed.

The existing conservative codec preflight and cumulative adapter accounting are
reused. Color+plane output bytes, descriptors, retained header/ICC and temporary
profile storage remain admitted before allocations. The35-pixel example requires
350 output bytes and rejects349. Fallible allocation refuses safely; cancellation
and allocator errors return neither a partial color result nor a partial plane.
No new unsafe conversion, all-codec heap/RSS guarantee or large-image capability
is introduced. The fixed Main5 policy and all caps remain; #37/#63 stay residual.

[Independent oracle manifest](jpeg-xl-selection-plane-oracle.json) and
[validation report](jpeg-xl-selection-plane-validation.json) record:

- 50 lossless C libjxl0.11.2 vectors: Gray/RGB × all25 color/SelectionMask depth
  pairs, all8 orientations, arbitrary named channels and embedded gamma2 ICC.
  Mask source codes exactly match both C-decoded codes and authored data through
  an independent orientation map. Color agrees within1 U16 unit.
- Three additional C-decoded unsupported inputs: float16, subsampled, and two
  SelectionMask planes. Existing unknown/Spot/Alpha-count refusals remain.
- 203 expanded native tests, including96 library tests,6 SelectionMask and6 Depth
  groups. Exact cumulative output/workspace, metadata/input/pixel/frame limits,
  every truncated prefix, all16 cancellation checkpoints, scoped net allocation
  release, and actual allocator refusal at8 new-route sites plus recovery pass.
  Minimal jpeg-xl passes both plane targets; disabled targets compile and run0.
- Actual wasm32/Node passes670 grouped checks without host imports,55 added for
  SelectionMask. Recorded final memory size is not a peak or global heap bound.
  Windows32 is verified by CI rather than claimed as a Linux-host execution.
- A displayed sRGB color/mask diagnostic matches independent C arrays exactly.
  Diagnostic grayscale scales raw mask codes only for inspection; it is not a
  product mask-rendering policy.

Benchmark controls rebuild exact #88 (`64c7a7c65cfab08d35d6be9fa6c2452e7e0f4d3c`)
and the new source with Rust1.91 release and identical features/harness/inputs,
on CPU0. Each uses3 warmups then1000 samples per process,6 alternating pairs.
After/before median ratios: ordinary0.9913, managed0.9796, explicit Depth0.9925.
Block ranges overlap with visible shared-host variation, so these are no measured
stable regression, not speedup claims. Three standalone new SelectionMask medians
are61.01/61.69/60.35µs for7×5 RGB16/mask16; before #88 rejects this input.
The report retains all block medians, output hashes and source/archive hashes.

Reproduce fixtures using the same pinned libjxl0.11.2 headers/library and build
commands as [#88](jpeg-xl-depth-plane.md), using the current encoder/extra-reference
helpers and an existing `.test-oracle` directory. Run
`python3 wml2-test/scripts/generate_jpegxl_selection_planes.py --work .test-oracle`.
It verifies the C runtime/header version, records hashes, and builds no product
oracle from WML2 output. Historical Depth reports remain snapshots of #88; their
source hashes refer to that commit, while this report records the additive helpers.

#42 still tracks multiple planes, Alpha combinations, other kinds/storage and
name-based selection. This completes the declared SelectionMask subset only.
