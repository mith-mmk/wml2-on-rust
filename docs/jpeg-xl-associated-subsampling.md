# Associated declared subsampled alpha

This unmerged follow-up to #86 implements the associated+subsampled integer
combination in #42. Published0.8.0 already upsamples alpha before the in-place
unpremultiply stage. The original-profile SDR path then supplies straight RGB to
WML2 CMS. Independent C libjxl's explicit unpremultiply output qualifies this
composition; no upstream patch, display policy or new public function/type is
needed. General extra-plane APIs remain a different original residual item.

## Contract and unchanged boundaries

The product change removes only the associated flag from #86's subsampling
refusal. Keep the8/9/10/12/16-bit whitelist, one alpha, declared factors2/4/8,
original-profile Gray/RGB SDR, no preview, one full-canvas Modular frame and raw
color/EC factor1. Keep the existing parser-compatibility validation, factor-rounded
internal pixel/sample checks, resampling-kernel/plane admission and owned
output/CMS/metadata accounting. The new unpremultiply stage is in-place and adds
no full image buffer. All shipping caps remain. Broader frames, XYB/float/HDR,
preview and non-alpha/multiple extras still reject. This is not main0.0.41 support.

Native decoder output is straight normalized RGBA16, with the source association,
depths, dim_shift and logical coded alpha dimensions retained as source metadata.
Interpolation precedes division by the interpolated alpha; CMS follows that
straightening and copies alpha unchanged. Do not interpret source-associated
metadata as an instruction to unpremultiply output again. Downsampling and
premultiplication may already have lost information; original hidden or
pre-association RGB is not recovered by this API. Existing unsigned-output
saturation and zero-alpha normalization remain the codec's qualified behavior.

EX conversion retains the existing value-aware Exact/Round contract. Native
Rgba16 is always available; Rgba8 Exact succeeds only if every normalized16-bit
sample is exactly representable in8 bits (multiples of257). In particular, the
new all-zero associated transparent control narrows exactly. Other values need
Round. Tests verify this rule and that explicit Round equals ordinary output;
no converter behavior or format capability is relaxed for the new inputs.

## Independent evidence and reproduction

Use #86's C build instructions for `jpegxl_alpha_fixture_encoder.c` and
`jpegxl_alpha_reference.c`, installed libjxl0.11.2 and LittleCMS2.16. Run
`python3 -B wml2-test/scripts/generate_jpegxl_associated_subsampling.py --work .test-oracle`.
The [manifest](jpeg-xl-associated-subsampling-oracle.json) records49 accepted C
oracles:42 Gray/RGB cases (3 factors ×7 representative depth pairs, all8
orientations, odd33×17),6 tiny zero/opaque controls, and the formerly refused
associated2× sample. C is explicitly requested to unpremultiply. Expected pixels
are never derived from candidate output. A valid C-decoded associated multi-layer
file remains a refusal and supplements #86's straight multi-layer refusal.

The shared production tests retain all49 straight cases and add49 associated
cases. Native RGB and alpha differ by at most1u16 against C; transparent/opaque
controls are exact. Ordinary output allows1u8; four ICC/depth/association cases
compare independent C output ICC→LittleCMS relative RGB16/NOOPTIMIZE→explicit
rounding within2u8 RGB/1u8 alpha. Candidate CMS alpha equals its ordinary alpha
exactly, and source/reconstruction ICC bytes match the embedded profile.

Four2×/8× ICC inputs (two straight/two associated) each exercise24 cancellation
checkpoints with zero scoped net retained allocation. New associated inputs test
the exact workspace boundary,959/960 rounded-grid admission, input/pixel/frame/
output/metadata limits, all truncated prefixes, callback Abort and no callbacks
on early refusal. Source-association metadata and Exact/Round behavior have a
separate regression. Preview absence is a successful None for admitted inputs.

[Validation and timings](jpeg-xl-associated-subsampling-validation.json) record
145 expanded tests,34 minimal tests and disabled compilation. Actual Node24 WASM
executes560 checks without host imports, including all new native/legacy pixels,
source descriptors, value-aware conversion and resource boundaries. macOS,
Windows and actual Windows32 belong to CI, not local Linux execution. Scoped
allocation checks and the existing finite/conservative ledgers do not establish
a global codec allocator, stack/RSS, peak-memory or latency guarantee.

The associated8× ICC output is rendered over a checkerboard beside independent
C+LittleCMS output and visually inspected. Primary timing uses six alternating
same-input/build/CPU0 process pairs,3 warmups then1000 samples. Ordinary first
shows ratio1.0632 with clustered slow after-blocks; one bounded longer repeat
(5000 samples/process, six alternating pairs) shows ratio0.9842 and slow intervals
in both variants. Managed primary ratio0.9992. No reproducible slowdown or speedup
is inferred; all blocks and new-input costs are retained without further sampling.
No merge, approval or release. #42 remains open for general extra-channel and
broader alpha/frame support; no new original completion conditions are added.
