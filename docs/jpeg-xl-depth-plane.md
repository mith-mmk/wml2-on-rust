# Bounded separate JPEG XL Depth output

This is a production implementation on an **unmerged draft stack**, based on
PR #87 (`133be97de10c4cd0e9defb29dd26dd77db793ee4`), including #72/#73/#75.
Main0.0.41 has not acquired this functionality. A subsequent additive
[SelectionMask subset](jpeg-xl-selection-plane.md) reuses this API and envelope. It partially implements #42's
non-alpha extra-plane contract; #42 stays open. No dependency versions changed.

`jpegxl::image_with_extra_channels_from_with_resource_limits_and_cancel` returns
`JxlExtraImage` only after both color and extras are complete and input/profile
boundaries pass. The initial envelope is original-profile Gray/RGB integer SDR,
source color and Depth independently8/9/10/12/16 bits, zero or one Depth channel,
full resolution, one full-canvas Modular frame, no preview/XYB/resampling.
Alpha, multiple extras, Spot/Selection/other types, float/subsampled Depth,
animation and unqualified frame combinations explicitly reject. Existing ordinary,
EX, sRGB and float/HDR routes retain their extra-channel refusal contracts.

`rgba16` contains normalized original-profile color coordinates and synthesized
opaque alpha. Gray coordinates are replicated and `color_channels=1` identifies
their original Gray profile. This API does not convert color to sRGB. The owned
source/output ICC values describe the color coordinates; they never apply to
Depth. `extra_planes` contains **all admitted extras**, independently owned from
color; zero extras returns an empty vector. Each plane retains source index, kind,
source depth, dim_shift, source dimensions and oriented output dimensions.
Its `samples: Vec<u16>` contains decoded **source-depth integer codes** in
`0..=(1<<bits_per_sample)-1`, not65535-normalized data unless source depth16.
No distance units, camera model, scale or physical interpretation are inferred.
Both color and plane apply the same declared orientation; source orientation and
coded geometry remain available. Select the returned plane by its source index.

The pinned0.8.0 public `JxlExtraChannel` exposes kind and association only; the
0.7.4 parser's name field is private with no getter. No Debug-string parsing or
private-parser duplication is used. `source_header` retains the exact bounded
codestream header bytes, including any encoded channel names, **opaquely**; this
is not a channel-name accessor, nor a copy of container boxes. Lack of a name API
never means a source name is empty. Name-based selection and multiple-channel
selection/decoding remain #42 residuals, together with other kinds/combinations.

The existing finite Main5 policy and fixed caps are retained. Both output vectors
are cumulatively admitted under `animation_bytes` (350 bytes for35 pixels with
RGBA16 + one Depth plane); workspace includes simultaneous outputs, descriptors,
retained source header/ICC and a bounded generated-profile temporary, in addition
to existing codec preflight. Copies/reservations use fallible allocation, with
checks before them. Cancellation never publishes just one result. This is the
existing conservative finite preflight plus adapter accounting, **not** global
codec heap tracking, an RSS bound or large-image enablement (#37/#63 remain).

Validation is recorded in [the report](jpeg-xl-depth-plane-validation.json) and
[independent oracle manifest](jpeg-xl-depth-plane-oracle.json):50 lossless C
libjxl0.11.2 vectors (Gray/RGB × all25 independent color/Depth depth pairs), all8
orientations, named channels and embedded gamma2 ICC; source Depth codes checked
exactly against authored values through an independent orientation map. Separate
C decoding supplies color values (at most1 U16 unit tolerance). Two C-readable
float/subsampled Depth fixtures verify refusal. No-alpha controls, unsupported
kinds/counts, every truncated prefix, joint output/input/pixel/frame/metadata and
exact workspace boundaries, scoped allocation release, all16 cancellation probes
and actual allocator refusal at8 new adapter allocation sites are covered.
Actual wasm32/Node passes615 grouped checks,55 new for this API, with no host
imports. The recorded memory is the final linear-memory size, not a peak bound.
Native expanded tests, minimal jpeg-xl and disabled-feature tests are recorded;
Windows32 execution is supplied by CI rather than claimed from this Linux host.

Fixture reproduction uses the repository C helpers, the previously hash-verified
libjxl0.11.2 headers/library, and a `.test*` work directory. Build the following
with `cc -O2 -Wall -Wextra -Werror -I "$JXL_INCLUDE"`, link the same0.11.2 library:
`jpegxl_alpha_fixture_encoder.c` as `.test-oracle/encoder`,
`jpegxl_extra_reference.c` as `.test-oracle/reference`, and
`jpegxl_alpha_reference.c` as `.test-oracle/rgba-reference`.
Run `python3 wml2-test/scripts/generate_jpegxl_extra_planes.py --work .test-oracle`.
The manifest records source and fixture SHA256 values. C refuses a float16 Depth
with inexact normalized U16 intermediates, so the negative float fixture uses
exact0/1 endpoints; it tests refusal, not float precision support.

A paired display check of original sRGB color and diagnostic grayscale Depth
matches the C oracle byte-for-byte. Depth grayscale is a test visualization only,
not a product display or metric-depth policy. CPU0 release comparisons use the
same original-profile associated-alpha control and harness,3 warmups then1000
samples,6 alternating process pairs. Median after/before ratios are1.0082 ordinary
and1.0034 managed; the block ranges overlap and do not establish a stable
regression or a speedup. The new explicit API has no accepted before counterpart;
three standalone medians are61.37/61.05/61.71 microseconds on the7×5 RGB16/Depth16
fixture. Full blocks/hashes and display hashes are retained in the report.
