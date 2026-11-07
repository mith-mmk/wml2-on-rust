# Bounded JPEG XL animation in WML2 0.0.42

Merged through [#103](https://github.com/mith-mmk/wml2-on-rust/pull/103),
based on the 0.8 migration [#102](https://github.com/mith-mmk/wml2-on-rust/pull/102)
(`282056388c3d4c442be9f7275d102128e807972e`).
Transplants #93/#94's animation processing, not their preview/HDR/extra-plane or
public resource-policy dependencies. No encoder implementation or changes.

## Contract

Common image/callback APIs decode admitted original-profile RGB8/16 with optional
matching straight full-resolution Alpha. Codec coalescing applies crop/reference,
blend and orientation once. Each displayed frame is a full canvas; invisible
layers consume the unchanged 64-layer cap, not playback frames. Loop zero means
indefinite repetition. The poster is frame0, not an extra playback delay.

The existing callback clock is integer milliseconds. Unrepresentable fractional
waits are refused before init; long waits use checked u128/u64 arithmetic.
`animation_rgba16_from_with_limits_and_cancel` preserves exact rational ticks
with coalesced native-endian RGBA16 and source/output ICC. No rounding or playback
policy is invented. Visible final-zero frames remain visible with zero delay.

EX callbacks declare Rgba16 LE, retaining original coded depth in metadata. The
collector exposes immutable `AnimationFrameEx` through `animation()` and raw
`loop_count()`. Existing precision policies still apply. Static picking/conversion
refuses animated state rather than losing frames. This adds no encoder.
Explicit `image_from_srgb*` uses the existing CMS per frame, excluding alpha.
Raw source coordinates/profile tagging and sRGB output remain distinct.

All decoding, final stream validation, CMS and output preparation precede
init/draw; owned errors/cancellation expose no partial image. Callback side effects
remain incremental. Existing Abort is stop-success, with exactly one terminate;
callback errors propagate. Identified animation failures terminate before init.
Earlier container/header failures retain the existing begin-only behavior.

Existing input/dimension/pixel/TOC/Main5/120fps caps and DecodeLimits remain.
Frame count, source canvases, prepared output, collector poster/frames, metadata
and scratch are admitted cumulatively with checked arithmetic and fallible
buffer reservations. A private operation-local allowance uses expanded_bytes;
there is no new public resource policy, codec heap tracking or RSS guarantee.

Preview-bearing animation, timecodes, XYB/HDR, associated/subsampled or mixed-depth
Alpha and general extra channels remain explicit refusals. These are limitations
of this bounded animation profile, not features silently converted or discarded.
The old large-image restriction remains; it is not a prerequisite for this series.

## Initial validation (bde4518)

112 enabled tests passed across library, EX collector/contracts, still JXL,
additional JXL and seven animation tests. Minimal jpeg-xl-only: six animation
tests passed. Feature-off target compiles with zero animation tests. Existing
independent C/djxl/LCMS oracles cover all canvases, variable/fractional/long/final-zero
timing, loops, crop/reference/alpha blend, orientation, ICC and sRGB. 64-layer
success/65-layer refusal, truncation at every byte, every callback Abort/error
boundary, cooperative cancellation checkpoints, cumulative output boundary and
actual allocation refusal are checked. Decoder outputs were rendered and inspected.
No fresh C-oracle generation is claimed: frozen goldens are replayed with source
commit and hashes in [oracle manifest](jpeg-xl-focused-animation-oracle.json).

Actual wasm32/Node24.19 passed 236 cases, including native rational animation,
common callbacks, EX, cancellation and frame limits. Other native 32-bit platforms
are CI coverage, not locally executed claims.

Three balanced release benchmark blocks use the same Rust1.91/source path/features
and existing allocator-meter harness. Relative to migration-only, still medians
change by about 0–2%; recorded peak requested allocations are unchanged on those
cases. Variable ICC16 animation takes about 0.182ms and peaks at about 4.20MB on
this host. Against main, photo peak decreases 5,995,746 → 4,382,783 bytes; tiny RGB
peak increases 64,222 → 71,912. Requested bytes are observed allocation deltas, not
RSS or an allocation contract. No speed claim is made for all inputs.
Raw trials: [validation JSON](jpeg-xl-focused-animation-validation.json).

These results describe the original implementation checkpoints, not a fresh
release-tag CI run. See [0.0.42 release notes](releases/0.0.42.md) for release
scope and validation. The separate XYB-animation follow-up #104 is not included.

## PR103 review: resident metadata during nested decode

The common path retains its reader and expanded container metadata while the
native animation decoder unpacks the container again. Previously their existing
allowance was claimed only after native decoding, leaving that peak unaccounted.
It is now deducted from expanded_bytes before nested input assembly, Brotli,
preflight and pixels, then carried into the returned private allowance exactly
once. Native-only calls reserve zero external bytes. No public API/cap changes.

New tests reject before unpack at the assembly boundary, exercise exact quota
transfer, overflow-sized reservations, cancellation precedence and recovery. A
four-brob container retains all four expanded payloads across successful decode;
the exact workspace boundary, failure before image publication, truncation and
retry are checked. Current verification: 36 library + 6 brob + 8 animation tests;
minimal jpeg-xl-only 29 library + 7 animation tests; actual WASM 236 cases passed.
All three rendered sRGB frames retain their inspected hashes. Three alternating
release blocks against bde4518 give median ratios 1.024 (plain) / 0.927 (metadata),
with identical pixels and observed allocation peaks. Shared-host variation does
not establish a speed improvement. Raw trials are in the validation JSON.
