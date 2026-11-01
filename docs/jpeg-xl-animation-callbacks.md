# JPEG XL animation callbacks, EX storage and sRGB

This unmerged production stage for #43 stacks on #93
(`79040da059d3d5c3aeef9db93da479295e137fda`) and its #92…#75/#73/#72 draft chain.
It extends the [native buffered animation](jpeg-xl-buffered-animation.md), keeps
#42's [finite acceptance audit](jpeg-xl-extra-acceptance-audit.md), and changes no
product dependency, fixed Main5 cap, release or main0.0.41 source.

## Public behavior

Common `draw::image_from`/`image_decoder_with_limits` and the resource-limited JXL
image API now deliver admitted original-profile RGB8/16 animations as RGBA8.
`image_from_srgb*` applies the existing own ICC engine to every coalesced RGBA16
canvas before final8-bit rounding; Alpha is excluded. Source/reconstruction ICC
and the actual output ICC tag remain distinct. `image_from_with_resource_limits_and_cancel`
adds the matching source-profile owned cancellation route without requiring CMS.
All still-image behavior and the exact rational native API remain available.

`image_from_ex`/`image_decoder_ex_with_limits` declare packed little-endian
`Rgba16` callback source bytes for every animation frame, normalized16 with the
original coded color/Alpha depth retained in metadata. `ImageBufferEx` keeps the
requested storage and Exact/Round precision policy. New `animation()` exposes
all immutable `AnimationFrameEx` records (`control`, `PixelBytesEx`), and
`loop_count()` exposes the raw loop field. `bytes()` is the first-frame poster.
EX source declaration is the byte layout, not a claim of16-bit source coding.
Static picking, `to_rgba8` and highres pixel export explicitly refuse animated
state instead of silently discarding frames. No animation encoder is added.

Existing `NextOptions::await_time` is integer **milliseconds**. Callback delivery
requires `ticks * denominator *1000` to divide exactly by the numerator; a checked
u128 calculation precedes u64 conversion. Fractional-ms input is refused before
initialization, with a message directing callers to #93's exact tick API. No
rounding, minimum delay, sleep or playback policy is invented. The C-generated
30000/1001 fixture still succeeds through that exact native API. A separately
C-validated maximum denominator1024 / u32-duration / u32-loop fixture checks long
representable waits; pixel hashes equal the existing C `long` goldens. Zero loops
means indefinite repetition; visible final-zero duration stays zero. Invalid or
unsupported timing is never returned as a silent first-frame success.

The callback stream is begin, init, metadata, first-frame poster draw, then one
next/draw pair for **each** displayed frame (including frame0), and terminate.
The collector poster is not an additional playback frame. `Continue` starts
frame0; later frames use `Next`. Full oriented canvases use `Override` and no
disposal. Crop, signed origin, reference and Alpha blending were already applied
once by codec coalescing; consumers must not apply the source layer operations
again. Invisible reference layers consume coded-layer resources, not callbacks.

## Lifecycle and ownership

All frame entropy, timing, profiles, CMS, final stream validation and prepared
byte allocations complete privately before init/metadata/draw. Pixel/CMS errors
therefore cannot publish provisional frames. A container rejected before it can
be identified as animation keeps the shared pre-init error behavior (begin only).
An identified animation failure terminates once, including pre-init failure.

Callback Abort preserves the existing stop-success contract: no subsequent
metadata/next/draw callbacks, exactly one terminate, including Abort in begin,
init, metadata, poster, each next/draw and end. A callback error stops the same
way but propagates its original error. External callback side effects are
incremental and cannot be rolled back. Owned image helpers publish only complete
success; cancellation/errors drop their private collector and return no image.
Cancellation is cooperative with no hard latency bound, covers codec groups,
CMS batches, byte preparation and every publication boundary, and pre-cancellation
wins over malformed input or invalid policy. Source-profile and managed helpers
share this contract. EX owned decode retains its existing non-cancellable API;
EX callback Abort and raw native cancellation remain independently available.

## Finite resource contract

The native helper carries its conservative codec/metadata workspace reservation
into delivery, retaining all decoded RGBA16 frames while all prepared callback
bytes, collector poster+frames, records, timing table, metadata/profile copies,
reader/container metadata and EX conversion scratch are admitted. During CMS,
future output/collector claims remain held alongside parsing, compiled transform
and batch scratch. Structured XMP/EXIF reuses the existing bounded parser and
charges new retained output to the same workspace. All adapter Vec reservations
are fallible; existing callback-owned metadata and third-party codec allocations
are not reclassified as globally fallible or RSS-bounded.

For3 canvases of7x5 pixels, ordinary output jointly needs840 native +420 prepared
+560 poster/frames = **1820 bytes**, refusing1819. EX requires840 native +840
prepared +1120 poster/frames +280 conversion scratch = **3080**, refusing3079.
Workspace includes records and metadata as well; these are payload quota examples,
not total heap sizes. EX guard counts actual declared source format/stride;
`ImageBufferEx::next` independently checks count and cumulative storage, reserves
records/payload fallibly, and accepts only full-canvas coalesced replacements.
Cropped, blended or disposal-bearing EX transport remains explicitly unsupported.
Fixed120fps,64 coded layers and every existing input/pixel/TOC/ICC cap stay in place.
#37/#63's codec-wide allocator and large-image problems are not solved here.

## Evidence and acceptance

[Independent display manifest](jpeg-xl-animation-callback-oracle.json) and
[validation](jpeg-xl-animation-callback-validation.json) record9 independent C
inputs/88 canvases compared through ordinary, normalized16 EX and managed output.
Raw differences are at most1 normalized16 unit; independent LittleCMS2.16 display
comparison allows2 RGB8 units and requires exact Alpha. The inspected ICC16
example is byte-identical for all3 managed frames. The new generator invokes the
C libjxl0.11.2 reference and uses its actual output ICC, not WML2-produced expected
pixels. Profile parsing/compilation remains the existing own engine in production.

Net requested-byte checks use a warmup decode. The first4008 bytes are now
[attributed to the pinned codec's process-global implicit-palette cache](jpeg-xl-palette-cache.md),
not the image or a TLS cache. Subsequent decode/drop and measured cancellations
have zero net increase; this is not a general leak-absence or global heap claim.

Native252 tests include102 library tests,17 shared EX tests, the existing JXL
regressions and6 new callback tests. Every callback Abort/error boundary,73 managed
cancellation checkpoints,7 delivery reserve failures plus2 shared EX storage/
conversion failures and9 existing native reserve sites are verified. Failed EX
allocation/precision conversion does not append an incomplete frame or mutate its
previous pixels. Minimal JXL9 tests, disabled targets and actual wasm32+Node932
checks include all9 callback/EX inputs, exact delays/loops, cumulative bytes, all
source-profile cancellation checkpoints and normal/EX Abort cleanup. Local native
32-bit execution is not claimed; Windows32 execution is in CI.

The original #43 acceptance areas are now represented in this finite draft stack:
multiple/variable frames, loop/Alpha/crop/reference/orientation/ICC independent
oracles; distinct hidden/display layers; exact timing; cumulative frame/output/
workspace limits; truncation and late entropy failures; callback Abort and existing
cancellation. #43 remains open until reviewed/integrated. The qualified RGB8/16
subset still refuses Gray, preview/SMPTE, mixed/associated/subsampled Alpha, other
extras, XYB/HDR. Fractional-ms callback support needs an explicit shared timing API
extension; it is an interface restriction, whereas those extra profile/mode
combinations remain unimplemented/unqualified. No unlimited combination matrix
is introduced as a new acceptance condition.

Reproduce references with the existing C0.11.2 reference executable, then:

```sh
python3 wml2-test/scripts/generate_jpegxl_animation_display.py --work .test-oracle
cargo test -p wml2 --locked --features jpeg-xl,color-management,image-buffer-ex --lib --test image_buffer_ex --test jpeg_xl_animation_callbacks --test jpeg_xl_buffered_animation
python3 wml2-test/scripts/verify_jpegxl_wasm.py --output .test-animation-wasm
```

The validation file retains equal-build release benchmark distributions against
exact #93, source hashes and visual comparison hashes. Existing ordinary/managed/
Depth/native-animation medians overlap variation; no stable speedup or regression
is claimed. New3-frame owned callback decode is measured separately; #93 refused
that route, so there is no matching before throughput claim.
