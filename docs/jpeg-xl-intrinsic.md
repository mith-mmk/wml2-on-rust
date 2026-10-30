# JPEG XL intrinsic dimensions

This change addresses #45 and is stacked on the information API in PR #57,
base `81178d7c7359e211eaea4ebaeca8c6b5b0ad1129`. It keeps jxl 0.7.4 and its
existing pixel/workspace limits. It adds no resampling or animation/preview
runtime support.

## Geometry and metadata contract

| Value | Meaning |
| --- | --- |
| `ImageDimensions.coded` | Encoded pixel canvas before native orientation |
| `ImageDimensions.output` | Pixel canvas on oriented axes, conditional on successful full decoding |
| `ImageDimensions.intrinsic_display` | Explicit intrinsic display dimensions on oriented axes; `None` when not declared |
| `ImageInfo.orientation` | Original native orientation 1–8; already applied by ordinary/EX/CMS decoding |

Orientations 5–8 swap output and intrinsic display axes. Example: coded 2×3,
orientation 6, source intrinsic 19×11 gives output 3×2 and display 11×19.
The decoder returns exactly six pixels, without allocating a 209-pixel display
canvas. Absence remains `None`, not a copy of coded/output dimensions.

Decoded metadata uses the common `coded_width`, `coded_height`, `output_width`,
`output_height`, `intrinsic_display_width`, `intrinsic_display_height` names with
`DataMap::UInt` values. The last two exist only for an explicit declaration.
`JPEG XL Source Intrinsic Width` / `Height` retain the original un-oriented values.
The existing source header and source orientation remain intact, and
`orientation_applied=true` still describes the returned pixels.

The independent C libjxl 0.11.2 BasicInfo oracle swaps pixel dimensions when
orientation is applied, but leaves explicit intrinsic fields on their source
axes. WML2's oriented display contract is therefore an explicit normalization,
not an assertion that C already normalizes those fields. The fixture manifest
records both C orientation modes. Consumers must not apply orientation twice.

## Bounded information query

The query projects only signature, validated public `Size`, default/extra flags,
native orientation, optional intrinsic `Size`, optional preview dimensions and
animation presence. It stops before bit depth, extra-channel/name arrays, ICC,
frame headers, TOCs or pixels. It never initializes a decoder. The private prefix
projection reuses the pinned codec's validated `Size`/`Preview` readers instead
of parsing the entire allocation-capable `FileHeader` repeatedly.

The result deliberately returns unknown color, channel count, bits, alpha and
sample kind. Still organization is known from the animation-presence flag;
animation count remains unknown. A header query can succeed on unsupported or
corrupt later sample/frame data, or with only the queried prefix present. It does
not promise that full decoding is supported or within the caller's pixel limits.

Raw, `jxlc` and sequential `jxlp` sources are supported. Each visited box range
and partial index is checked before reading/skipping it; `ftyp` and visited
`jxll` placement/value are checked. Auxiliary payloads, including `brob`, are
opaque seeks, never decompressed. No later container validation is claimed once
the prefix is complete. The fixed limits are 64 logical prefix bytes and 1024
visited boxes. Existing whole-input, actual metadata-read, minimum 4 KiB workspace
and at-least-one-frame checks apply. Skipped bytes are not metadata reads.
`pixels`/`animation_bytes` do not constrain this allocation-free geometry query.
The reader's original position is restored on success and error under the common
API contract. Caller-owned input/buffering and total process RSS are outside the
known header-workspace estimate.

The pinned public `Size` reader permits axes ≤2^30 and area ≤2^40, including
intrinsic dimensions. These restrictions remain even with unlimited caller
limits. C libjxl accepts some larger display declarations; their rejection is a
conservative supported-subset constraint, not evidence of universally invalid
JPEG XL syntax. A 2^20×2^20 intrinsic declaration on a six-pixel image decodes
under a six-pixel budget. The equivalent huge **coded** dimension fixture is a
header-only oracle; its frame body is not claimed to encode that huge canvas.

## Independent fixtures and validation

`generate_jpegxl_intrinsic_fixtures.py` uses C libjxl 0.11.2, records BasicInfo
with/without keep-orientation and full normalized RGBA16 pixels, and checks the
eight images against existing saved djxl orientation references. Managed ICC16
pixels use the existing independent LittleCMS gamma2-to-sRGB reference rotated
by explicit coordinates, never WML2 output. RGB tolerance is 2/255; alpha and
native ICC16 samples are exact.

The information tests forbid reads from sample/profile/frame data and opaque
auxiliary payloads, and check largest individual allocations ≤8 KiB using the
existing test allocator. Huge coded/display metadata works with `pixels=0` and
`animation_bytes=0`; malformed/truncated/oversized queried sizes, box ranges,
partial ordering, fixed box cap and caller budgets fail without partial results.
Runtime limit failures occur before publication callbacks. New scalar keys/values
are charged to the existing retained metadata and workspace estimates; budgets
can reject an image that previously fit with fewer metadata keys. See the accompanying
[validation report](jpeg-xl-intrinsic-validation.json) for exact commands, counts, WASM hash, display and benchmark
measurements. No cancellation/codec interrupt guarantees are introduced here.
