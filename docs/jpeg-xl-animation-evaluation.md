# JPEG XL animation feasibility and proposed contract

This is the contract/evidence phase of #43, based on main
`6a48db924b338c5ae7095bea6105fb4dbd436e95`. **Production integration is on hold.**
The pinned jxl0.7.4 can reconstruct the tested animations correctly, but that is
not a bounded WML2 animation API. All existing WML2 JPEG XL entry points continue
to reject animation; this PR changes no `wml2/src`, production manifest, lock,
cap, support advertisement or published version. No streaming, HDR or encoder
feature is added. The C encoder is an independent test-fixture tool only.

## What was established

Nine independently C-generated/decoded small images cover27 displayed frames:
variable positive durations, fractional NTSC-style ticks, finite/infinite/native
maximum loop counts, cropped and negatively positioned alpha/reference blends,
an invisible zero-duration reference layer, a visible final zero-duration frame,
SMPTE timecodes, long durations, orientations1/6/8,16-bit embedded ICC, variable
alpha beneath ICC, and a real embedded preview followed by three main frames.
The whole native RGBA16 outputs match C libjxl0.11.2 exactly on this Linux run.
Own-ICC sRGB output also matches the independent C-output-profile → LittleCMS2.16
references exactly at8 bits, including alpha. These are finite observations;
verification permits1 u16 native /2 u8 RGB across implementations, with managed
alpha exact. No WML2-generated expected pixels or decoder-self oracle is used.

Actual Node wasm32 execution checks9 headers plus27 native frames: geometry,
raw ticks, computed milliseconds, loop values, all output samples, hidden-layer
suppression, last-zero preservation and preview exclusion. It does not test CMS
or production animation admission/cancellation in WASM.

The trusted probe feeds only frozen small inputs, requests coalescing and high
precision, and uses a sample limit. Its output/coded-frame caps are experiment
bounds, **not** pre-allocation ICC/TOC/entropy/codec workspace admission for
untrusted data. It must not be exposed as a production decoder.

## Frame roles and compositing

| Role | Observed behavior / required transport |
| --- | --- |
| Non-last regular duration0 reference | Decoded internally, retained as needed, no displayed frame or callback boundary |
| Regular displayed frame | Pinned public process returns one full coalesced, oriented canvas, including source crop/reference blending |
| Last regular duration0 | Still displayed: both C and the pinned core include it. Filtering all duration0 frames is wrong |
| Preview | Separate role. It may itself be terminal/visible, but is not an animation index; `scanned_frames` explicitly excludes it |

The internal/reference fixture has4 coded main layers but3 displayed frames;
the preview fixture has1 preview plus3 main layers and3 displayed frames.
Independent C non-coalesced layer headers agree with the pinned header walker
on duration/last status, signed origin, dimensions, blend mode/source and saved
reference. C coalesced frames supply all expected pixels. Pinned `WithFrameInfo`
reports the canvas size, not the cropped source rectangle.

Transport complete coalesced canvases at(0,0) with `NextBlend::Override` and
`NextDispose::None`. Do not replay source crop/blend/disposal on already
composited pixels. Preserve the native orientation-applied contract once. For
sRGB, convert each complete reconstruction using its actual output ICC **after**
codec compositing, exclude alpha, then round toRGBA8. Do not independently
convert/blend source layers in a different colour space.

The valid preview compound uses a C-generated preview frame with animation
syntax, including duration bits. Pasting an ordinary static preview payload into
an animation header omits fields: the independent C decoder rejects that
negative splice. C decoding preview+main and skipping preview agree on every
main sample. This does not qualify all preview+animation sequences or relax
PR #62's terminal-visible preview restriction.

## Timing and loop contract

Use raw integer ticks and the exact timebase:

`milliseconds = duration_ticks * 1000 * tps_denominator / tps_numerator`.

The public core exposes raw ticks in `scanned_frames`, while `frame_header`
exposes floating milliseconds. Do not silently cast those floats to integers.
At30000/1001 ticks/second, durations5/17/3 give5005/30,17017/30,1001/10ms.
Per-frame truncation loses7/6ms per cycle. The proposed initial subset rejects
non-integral milliseconds; a later rational scheduling/rounding policy must be
explicit and preserve the raw timebase/ticks.

A valid source duration4294967295 at1/1024 ticks/second gives
4398046510080000ms. Checked u128 arithmetic and checked u64 conversion avoid
accidental32-bit narrowing. The prototype stores this value without sleeping.
Zero numerator/denominator and overflow are rejected as utility arguments; no
validly encoded zero-timebase JXL is claimed. Pinned header coders encode positive
timebase values. Preserve a nonzero duration on the last frame: the actual core
and C do so, despite the stale `VisibleFrameInfo.duration_ms` comment suggesting
last-frame duration0.

Preserve the native u32 loop value, including0 for indefinite repetition. Decode
one encoded cycle; do not expand iterations into stored frames or byte quotas.
The callback model retains1/3/7/u32::MAX loop values. The fixtures with loop0
exercise fractional/final-zero policies and are core checks rather than strict
transport cases; loop0 itself requires no iteration expansion. WML2's common `loop_count`
documentation is not a universal GIF-repeat/APNG-play normalization contract;
a new JXL API must document native loop semantics without guessing an off-by-one
conversion from another codec.

SMPTE information cannot currently be losslessly transported through the pinned
public frame API: `JxlFrameHeader`/`VisibleFrameInfo` omit it, and low-level
`FrameHeader.timecode` is private (actual compilation fails withE0616). The
animation header does expose the presence flag to the pinned preflight parser.
Initial support must reject that flag or add a qualified accessor; do not parse
Debug strings or silently drop timecodes. The fixture has valid monotonically
increasing values independently reported by C. No invalid-clock acceptance claim
is inferred from pixel decoding.

## Recommended first public subset

Keep existing still/EX/sRGB entry points unchanged. Add an explicit buffered
animation companion, rather than make a still-only JXL route return one frame.
A concrete proposed shape is `animation_from_srgb_with_limits_and_cancel`, gated
by JPEG XL + colour management and sharing PR #32's cancellation implementation.
Return an owned animation result containing the existing ImageBuffer animation
storage plus bounded raw timing data (timebase, per-frame ticks and native loop
count). This is a proposal, **not an exported API in this PR**.

Start with the tested RGB + full-resolution straight same-depth integer8/16
profile, positive exactly representable millisecond displayed durations, no
SMPTE and no preview. Reject other alpha/extra/HDR modes under existing policy.
Final zero-duration and fractional-time streams must remain explicit refusals
until their presentation contracts are defined. Preserve source/reconstruction/
output ICC and source orientation metadata under the established still rules.
EX animation needs a separate storage contract; `ImageBufferEx::init_ex` and the
common EX guard's `next` currently reject it. Do not weaken those checks.

The callback experiment demonstrates five integer-ms transport cases, not a
production acceptance claim. First populate the primary poster, then issue
`Continue` +draw for the first animated layer, followed by `Next` +draw for each
later layer, carrying that frame's delay and full-canvas Override/None controls.
This matches the existing ImageBuffer/legacy GIF convention and retains the
first displayed image in both poster and animation storage. It does not expose
an internal reference or preview as the poster. Delays are metadata, not decoder
sleeps. The model verifies all copied buffers, first_wait_time and native loop
values. Abort at each of the first3 `next` calls stops further model draws and
terminates once; it does not certify decoder work avoidance or production
animation Abort behavior.

## Required admission and cancellation before integration

1. Build a bounded all-coded-frame plan before codec pixels. Retain the existing
   input/header/ICC/TOC/padded-geometry/signed-origin/hard sample guards. Count
   every internal/LF/reference layer in its fixed layer cap and workspace; count
   displayed main frames separately against `frames`. Preview is neither, and
   initially remains rejected for animation. Never remove sample_limit.
2. Admit retained callback pixels cumulatively. ForN fullRGBA8 canvases ofS
   bytes, ImageBuffer keeps a poster plusN animation layers: **(N+1)S**.
   The7×5,3-frame model stores560 bytes, not420. Common guard initialization
   recordsS and0 frame boundaries; every next adds one layer/S and one frame.
   Check N and this total before initialization, not after expensive decoding.
3. Separately admit the encoded input, existing ICC/entropy allowances, maximum
   live decoder working planes, conservatively retained reference/LF planes,
   frame/seek/name/timing metadata and one reusable native/CMS/rounding scratch
   set. Do not count a callback buffer as decoder workspace, or forget that both
   can be live. A staged all-frame vector would add its own cumulative storage;
   preferably append into a private owned collector with one reusable frame.
   These remain conservative known-buffer estimates, not total RSS promises.
4. Keep the owned result private until complete validation, boundary checks and
   the final latched cancellation check. Any error/cancellation drops it. A
   custom callback may already have observed earlier frames; that is a separate
   publication contract and cannot be advertised as atomic cancellation.
5. Reuse the fallible runner and existing checkpoints in every visible-frame
   process/CMS iteration, add bounded frame-boundary checks, and verify every
   cancellation point against independent frame references. Serial preflight,
   individual tasks/allocations and ICC compilation remain non-interruptible;
   no latency bound is implied.

These steps are not established by the trusted probe or by the still-only
resource planner, which rejects animation and multiple displayed frames today.
Shipping before the companion API/zero-time policy and cumulative live-buffer
admission are integrated and tested would confuse pixel feasibility with a safe
animation contract. No upstream version bump is required for the tested core
pixel subset; #40/#54 migration and #37 resource work remain separate.

## Integration evidence and remaining gates

An isolated detached combination of PR #32 atda4f457 and full PR #62 ata69a54f
is `e02f5c7c9df5e43ef2a434267ccc5a90bd9b0538`. Both public branches stay unchanged.
It passes65 existing native tests, the existing86 deterministic static-preview
cancellation points against independent CMS references, and the9 new animation
refusals with18 pre-frame cancellation checkpoints plus limit precedence.
A valid preview+animation never returns a preview/first-frame substitute.
These refusal/pre-frame checks do **not** certify mid-animation cancellation.

Before enabling even the initial subset, qualify all admitted profile/depth/
orientation combinations, real callback Abort/error lifecycle and actual
multi-frame cancellation/late truncation. Test exact frame/canvas/combined-live
workspace boundaries, metadata/TOC/name growth and reference-slot replacement
under32-bit and WASM. Verify the static APIs still reject animation, EX remains
explicit, and unsupported fractional/SMPTE/final-zero/preview cases reject before
publication. Broader rational timing, EX frames and preview/animation combinations
are later extensions, not successes implied by this evaluation.

Reproduce with matching C headers/runtime and LittleCMS:

```sh
python3 wml2-test/scripts/evaluate_jpegxl_animation.py \
  --output .test-jxl-animation \
  --include <libjxl-0.11.2-headers>
python3 wml2-test/scripts/verify_jpegxl_animation_evaluation_wasm.py \
  --output .test-jxl-animation-wasm
```

Only `--write-fixtures` regenerates the frozen inputs/oracles and generated WASM
case table. The source/manifest hashes, exact commands, finite measured results,
composition checks and WASM artifact identity are in
[jpeg-xl-animation-evaluation.json](jpeg-xl-animation-evaluation.json).
The sRGB three-frame grid was exported and visually inspected. Production source
and dependency diffs are empty, so no production performance change or benchmark
speedup is claimed. No merge, release, approval review or issue closure occurs.
