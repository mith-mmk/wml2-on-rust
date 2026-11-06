# Bounded container metadata keys and record storage

This unmerged #37 product stage follows #99
`75d205240088d9f114c52df29268311f0d90578c`. #95–#98 retain their existing stack
and contracts; #96's isolated codec patch remains unapplied. No dependency,
manifest/lock, public API signature, image/box envelope, fixed cap or version changes.

## Allocation and admission

Container source/decoded keys previously used infallible formatting, including
one temporary String per hexadecimal byte, followed by infallible record `push`
growth. The new builder computes checked key length before metadata admission,
reserves fallibly once, and writes the same decimal index/lowercase FourCC hex
schema. No key spelling, raw payload, decoded payload or logical `metadata_bytes`
meaning changes. Binary box types remain encoded as hex, never interpreted as UTF8.

Container record capacity grows geometrically through `try_reserve_exact`. Finite
admission checks existing plus replacement backing bytes before growth, alongside
the existing3× input and2× logical retained-metadata allowances. Allocation failure
preserves the previous Vec and payload owners. This is a live/overlap estimate,
not accumulated allocation traffic. The existing auxiliary-box-index limit1024
derives a maximum2048 source+decoded records; it is not a larger/new box cap.

The current backing-array allowance also enters finite Brotli expansion and the
Input's header/preflight workspace. Header-allocation denial uses the same basis
in the animation-fallback retry guard. Legacy paths gain fallible allocation
without these new workspace checks. Logical metadata limits and cumulative brob
limits keep their existing values; explicit finite workspace minima can tighten.

On the measured x86_64 build a record occupies56 requested bytes. One opaque-box
case reserves4 slots (+224 finite bytes); the frozen compressed case reserves8
(+448).1024 opaque records add57344 bytes;2048 empty-brob records add114688. The
bare codestream control is unchanged. Actual source/decoded contents and generated
metadata remain identical in regression tests.

This is container-stage requested storage, not allocator rounding, RSS or a shared
whole-codec heap contract. Later generated-key/record growth, callback-held arrays
(including metadata retained while an animation helper unpacks a second copy),
codec entropy tables, frame/modular/reference buffers and factories still need a
unified ownership ledger. One Input array's admission does not cover every other
simultaneously live owner. The dependency/public-API decision in
[the integration recommendation](jpeg-xl-dependency-integration.md) remains pending.
#37/#63 and the2560×1440 refusal are not completed or relaxed.

## Verification and reproduction

[Recorded validation](jpeg-xl-container-metadata-validation.json) pins source,
probe, binary and input hashes and complete before/after measurements. Native
Linux x86_64/Rust1.91 passes66 minimal and133 expanded library, ordinary, brob,
metadata and resource tests; feature-disabled compilation passes.

Added checks cover old/new capacity overlap, exact/one-under quota, overflow,
zero retained-key quota before allocation, index decimal boundaries through
usize::MAX, binary types and exact legacy key schema. Sixteen actual allocator-null
injections cover initial/replacement arrays, source/decoded keys and raw copies
on plain/compressed, legacy/finite routes; failures preserve owned data, return
errors without an image and recover. Maximum1024 empty brob boxes retain all2048
records and decoded index1024; the next auxiliary box is rejected.

Actual wasm32/Node24.19 passes15 resource groups under each32/64 MiB linear-memory
ceiling, with no imports and enforced growth rejection. New groups include the
independently verified compressed/plain files and the2048-record/next-box boundary.
The existing composed ordinary/EX/preview/animation harness passes932 checks.
Native32-bit and local Windows/macOS were not executed; existing CI jobs include
the changed native tests and extended resource harness. CI completion is not awaited.

Seven matched before/after probes include bare, opaque, independently generated
plain/brob,1024 opaque/empty-brob, and an opaque box appended to the original float
HDR container (preserving its original control/Level signaling). All output bytes
match frozen independent C RGBA8/F32 goldens. A magnified Alpha checkerboard of the
compressed metadata result was inspected against C's pixels. No private sample
or new WML2-derived pixel oracle is used.

The before binary is the exact #99 artifact hash already recorded in its snapshot.
The same public probe/feature graph is rebuilt after this change. CPU0 measurements
use three alternating pairs with three warmups and300/100/20/10 timed calls per
case according to the recorded workload. Final ratios span0.575–0.991; the two
1024-box cases measure0.575 and0.729. These are bounded local observations, not a
general throughput claim. Exact and one-byte-below finite floors are tested on
both revisions; the deltas match record capacity rather than historical traffic.

```sh
cargo test -p wml2 --locked --no-default-features --features jpeg-xl --lib \
  --test jpeg_xl_brob --test jpeg_xl_resources
python3 -B wml2-test/scripts/verify_jpegxl_resources_wasm.py \
  --output .test-container-wasm
python3 -B wml2-test/scripts/benchmark_jxl_container_metadata.py \
  --before .test-before-probe --after .test-after-probe --output .test-container-bench
```

Build/link the two probes with the identical settings described in
[the format admission stage](jpeg-xl-format-admission.md). The benchmark creates
fresh `.test*` output, standard container wrappers around frozen C codestreams and
uses existing C Brotli data; it does not re-encode pixels with the candidate.
No merge, approval, release, upstream write or issue closure is included.
