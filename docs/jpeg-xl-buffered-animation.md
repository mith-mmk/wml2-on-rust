# Buffered native JPEG XL animation

This unmerged production stage for #43 stacks on #92
(`87b554fd01c258a36cbe9353894fb2681ac8c791`), including #75's published0.8.0
pixel decoder and parser-only0.7.4. Main0.0.41 and dependencies are unchanged.
It reuses #67's C evaluation fixtures, not its trusted-input Rust decoder.

`jpegxl::animation_rgba16_from_with_resource_limits_and_cancel` returns an owned
`JxlAnimationImage`: displayed frames in order, each with full-canvas straight
original-profile RGBA16 and exact `duration_ticks`; source color/Alpha depth,
source orientation, oriented geometry, source/output ICC, exact global source
header, rational ticks-per-second numerator/denominator and raw loop count.
Orientation and crop/reference/blend are applied once by codec coalescing.
Alpha is opaque on RGB input. Samples are normalized16, distinct from original
source depth. No tone mapping, CMS, playback or sleep occurs.

A frame duration in seconds is `duration_ticks * denominator / numerator`;
zero loops means indefinite repetition. No millisecond rounding or display
minimum is invented: the fractional30000/1001 fixture and final visible zero
remain exact. Invisible reference layers are not returned as display frames,
but do consume the coded-layer budget. Every frame stays private until final
stream, scan count, profile and cancellation validation succeeds. Truncation,
late entropy failure, cancellation and allocation failure return an error with
no partially published animation. Source layer crop/blend instructions must
not be applied to the already coalesced canvases.

The initial qualified subset is original-profile RGB integer8/16, optionally
one matching-depth full-resolution straight Alpha; orientation1/6/8, fractional
and variable duration, long u32 durations/loop counts, ICC16, crop/negative origin,
reference blending, hidden layers and final zero are independently exercised.
Gray, mixed/associated/subsampled Alpha, multiple/non-Alpha extras, preview,
SMPTE timecodes, XYB, HDR/float and unsupported ICC remain explicit refusals.
This does not extend the still-image or extra-plane acceptance contracts.

Fixed Main5 **120fps** is checked with exact u128 arithmetic during bounded
all-layer preflight, before pixel decoding. Three old #67 fixtures exceed it
(200fps); they remain negative tests. New C-generated100-tick variants preserve
the old samples and layer operations while using an admitted rate. Fixed64 coded
layers, input/pixel/TOC/name/ICC bounds and caller visible-frame limits remain.
A64-layer C fixture succeeds;65 is rejected. No cap is raised.

All N retained RGBA16 canvases are cumulatively charged as `N * width * height *8`
against animation bytes and live workspace, with no duplicated poster. For three
7x5 canvases,840 bytes succeeds and839 refuses. Frame/timing/format descriptors,
source header, owned ICC copies and profile-generation temporary are admitted
before their fallible adapter reserves. Each coded layer's padded/signed geometry,
retained conservative reference planes, frame/name/TOC/scan records and existing
entropy allowances contribute to the same finite workspace. Successful decode
rechecks the public0.8 header/timing against independently bounded0.7 preflight.
This is conservative codec preflight and adapter accounting, **not** an all-codec
fallible allocator, peak RSS guarantee or resolution-cap removal (#37/#63).

[Oracle manifest](jpeg-xl-buffered-animation-oracle.json) records immutable #67
source hashes and the added C generator/reference. [Validation](jpeg-xl-buffered-animation-validation.json)
records10 supported inputs/91 canvases and6 independently valid refused inputs,
all prefix truncations, late entropy failure, exact joint budgets, all27 cancellation
checkpoints,9 adapter allocation failures, feature coverage and actual WASM.
Native comparison permits at most1 normalized16 unit per sample; the displayed
ICC16 example is byte-identical to C. The display is a diagnostic Pillow CMS
conversion of both outputs and does not add a managed product animation API.

Reproduce the added C cases with installed libjxl0.11.2 headers/library:

```sh
cc -O2 -Wall -Wextra -Werror -I <libjxl-0.11.2>/lib/include wml2-test/scripts/jpegxl_animation_encoder.c /lib/x86_64-linux-gnu/libjxl.so.0.11 -o .test-oracle/encoder
cc -O2 -Wall -Wextra -Werror -I <libjxl-0.11.2>/lib/include wml2-test/scripts/jpegxl_animation_reference.c /lib/x86_64-linux-gnu/libjxl.so.0.11 -o .test-oracle/reference
python3 wml2-test/scripts/generate_jpegxl_buffered_animation.py --work .test-oracle
cargo test -p wml2 --locked --features jpeg-xl,color-management,image-buffer-ex --lib --test jpeg_xl_buffered_animation
python3 wml2-test/scripts/verify_jpegxl_wasm.py --output .test-animation-wasm
```

The nine frozen #67 source files and their committed C goldens are revalidated;
they are not regenerated from a Rust implementation. Preview construction remains
the original fingerprinted evaluation artifact. The manifest retains historical
#67 CMS measurements as provenance; current product output is original-profile.

#93 checkpoint: this stage did not connect the common Draw/ImageBuffer
animation callback, callback `Abort`, EX delivery or managed sRGB display. Existing
ordinary/EX/sRGB/F32/HDR APIs still reject animation rather than returning a silent
first frame. A future callback stage must separately account for its collector's
poster duplication and exact tick-to-callback time policy; it cannot claim this
owned carrier's N-canvas budget or cancellation probe as callback Abort coverage.

The subsequent [callback/EX/sRGB stage](jpeg-xl-animation-callbacks.md) now supplies
that integer-ms callback/owned-frame integration. The native rational API above
retains its original pixel/timing contract and narrower independent validation.
