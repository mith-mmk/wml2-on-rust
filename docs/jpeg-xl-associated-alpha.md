# JPEG XL associated integer alpha: first production subset

This report records the original #84 subset, an unmerged follow-up to #83.
The current stack extends integer depths and mixed-depth association in the
[depth-combination follow-up](jpeg-xl-associated-depths.md); the scope and evidence
below describe #84 itself. It implements part of #42. It selects associated
alpha because jxl0.8.0 exposes an explicit straight-output mode that can feed
existing ordinary, EX and own-CMS APIs. Animation/progressive work still needs
separate multi-frame or incremental admission; reconstruction/encoding remain
blocked by the backend contracts recorded in #60/#61. No new output API or
runtime dependency is needed here.

## Accepted and deferred scope

Accept one full-resolution alpha with matching8/16-bit integer Gray/RGB,
original-profile SDR, no preview, and exactly one coded main layer. Existing
supported ICC/profile rules still apply. Source association is cross-checked
between the bounded parser and0.8 public BasicInfo. The full ICC/TOC/frame scan
still runs before pixel allocation. Its existing layer quota is tightened to one
for this new subset; fixed geometry, padding and allocation admission caps remain.

The codec's `AlphaMode::Unpremultiplied` performs an in-place render stage before
WML2 CMS. All associated8/16 inputs use native RGBA16 storage after division;
ordinary RGBA8 uses WML2's existing deterministic normalized rounding, and EX
requires `Rgba16` with `Exact` or an explicit `Round` conversion to RGBA8. This
avoids codec U8 dithering on newly fractional straight coordinates. Source bit
depth stays source metadata; `JPEG XL Source Alpha Associated=true` identifies
the stored association. Public output buffers are straight. ICC source and
reconstruction identities are preserved, and CMS does not transform alpha.

The public codec uses a2^-26 denominator floor at zero alpha, matching libjxl.
Physical premultiplied black at alpha0 stays black. Nonzero stored RGB at alpha0
is tested explicitly: straight integer output saturates nonzero channels while
alpha stays0, matching the independent decoder. Original hidden RGB cannot be
recovered by division. No promise of recovering the pre-association source is
made; quantization during association can already have lost color information.

Deferred parts of the original #42: mixed-depth or9/10/12-bit associated alpha,
associated XYB/float/HDR, preview and multiple coded layers; subsampled alpha;
non-alpha/multiple extra-channel retention/selection/plane API. Those still reject
explicitly. Existing mixed-depth straight alpha remains supported. #42 stays open;
this stack is not published main0.0.41 support.

## Independent evidence and reproduction

`generate_jpegxl_associated_alpha.py` uses the existing C fixture encoder and
reference decoder, pinned installed libjxl0.11.2 and LittleCMS2.16. Build them
with `cc -Wall -Wextra -Werror -I <libjxl-0.11.2/include>` and link the installed
`libjxl.so.0.11`, producing `.test-oracle/encoder` and `.test-oracle/reference`;
then run `python3 -B wml2-test/scripts/generate_jpegxl_associated_alpha.py --work
.test-oracle`. See the [hash manifest](jpeg-xl-associated-alpha-oracle.json).
No candidate pixels construct an oracle.

Fifteen accepted vectors cover Gray/RGB,8/16-bit, alpha0/minimum positive/half/
near-opaque/opaque, orientations1/6/8, two embedded RGB ICC cases, two colored
zero-alpha stress cases and the previously refused `jxl-premult.jxl`. A valid
independently decoded two-layer associated file is retained as a refusal case.
C default output applies orientation; its reported BasicInfo has orientation1.
Native straight RGBA16 allows1u16 RGB difference; alpha is exact. Ordinary output
allows1u8 RGB, and own-CMS output allows2u8 RGB against C output ICC → LittleCMS
relative RGB16/NOOPTIMIZE → explicit8-bit rounding; alpha remains exact. These
are implementation comparison bounds, not a lossless unassociation claim.

Five new tests check pixels, CMS/profile identities, all18 decode/CMS cancellation
checkpoints with zero scoped net retained heap, truncated prefixes and exact
workspace/pixel/input/frame/output/metadata admission. The existing callback
probe adds Abort at init/draw/metadata for legacy/EX and early failures with no
callbacks. Only the old premultiplied fixture moves from refusal to independently
qualified acceptance; mixed associated depth and other unsupported samples remain.
Existing reference-layer behavior for straight-alpha inputs is unchanged.

[Execution and timings](jpeg-xl-associated-alpha-validation.json) distinguish
local minimal/expanded/disabled features and actual Node WASM from the CI platform
matrix. The full WASM harness now executes260 checks, including native/legacy
associated vectors and the multi-layer refusal; no CMS or global heap bound is
inferred from that harness. C+LittleCMS versus WML2 checkerboard displays were
visually compared for the oriented ICC case. Existing straight control benchmarks
use identical source/build/configuration with alternating pinned-CPU processes.
Newly accepted associated timings are recorded separately, since the parent
rejects those inputs and cannot provide a comparable successful decode baseline.

The six alternating process pairs give straight-control medians ordinary
72.302→73.361µs (+1.47%) and managed124.855→125.814µs (+0.77%). Per-block ranges
overlap: ordinary71.155–73.188µs before /71.982–74.220µs after; managed
123.899–127.094µs before /123.754–141.635µs after. These small measured differences
are recorded without claiming either a speedup or an established absence of
regression. Three independent runs of the newly accepted oriented ICC case take
ordinary74.380–77.615µs and managed152.121–153.793µs median per decode. All use
Rust1.91 release, identical feature flags, CPU0,3 warmups then1000 timed samples.
