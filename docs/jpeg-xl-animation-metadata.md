# Animation callback metadata ownership and admission

This unmerged #37/#43 stage follows #100
`0dc376ef98d8547f58c5849f95f475de8ae9007e` and its #99…#95 stack.
No public signature, dependency, manifest/lock, profile, timing, image cap or
version changes. #96's codec patch remains isolated.

## Known simultaneous owners

Common animation delivery retains the first unpack's raw container metadata
while the native helper unpacks/decodes the same input again. The previous native
budget did not reserve that outer owner before its own allocations. Fixed
callback keys were infallible, and callback record-array replacement lacked its
explicit old/new capacity reservation.

The adapter now sums outer record capacity plus each initialized String/Raw Vec
capacity, using checked byte arithmetic. Only raw container records are accepted
at this private boundary; other types fail closed rather than silently omitting
storage. It reserves these bytes before nested input assembly, Brotli, header,
frame and output admission by passing the reduced workspace quota throughout
native decode. On return, the reservation rejoins the original-limit ledger;
remaining bytes do not increase during that transition.

The direct public native API passes zero resident bytes and retains its previous
pixel/timing/resource behavior. Common ordinary/EX/sRGB animation helpers may need
higher workspace minima. This reservation starts with the already prepared outer
metadata; it does not retroactively change earlier legacy container admission.

Before callback record growth, the new requested array is charged alongside the
already charged old array. Allocator refusal restores the provisional claim and
preserves the old Vec; after success, only the old array charge is released.
Incoming key/raw payload reservations remain held. Sixteen additional slots cover
14 fixed callback entries and at most two structured EXIF/XMP entries. Fixed keys
are separately admitted and allocated with `try_reserve_exact`; refusal rolls back
that key's claim. Existing conservative estimates/copy allowances remain in place,
so this can overcount storage rather than weaken an earlier screen.

All preparation remains private before init/metadata/draw. Error or cancellation
returns no owned image; existing callback Abort, termination and rational/native
versus exact-integer-ms common timing contracts are unchanged. No new probe or
hard cancellation-latency promise is introduced.

This covers requested adapter-owned storage, excluding allocator overhead/RSS.
Main-image generated metadata, structured-parser allocations/common EXIF/XMP key
creation, callback implementations' own allocations and codec-wide entropy/frame/
modular/reference/factory ownership remain separate work. #37/#63 and2560×1440
admission are still unresolved; the published-dependency/API recommendation remains
as recorded in [#97's design](jpeg-xl-dependency-integration.md).

## Evidence and reproduction

[Validation](jpeg-xl-animation-metadata-validation.json) records Linux x86_64 /
Rust1.91:62 minimal and80 expanded library/native-animation/callback tests,
a final targeted resident-denial check and feature-disabled compilation.
Added tests measure capacity slack rather than logical lengths, verify denial
before native work, exact ledger rejoining, old/new overlap and rollback, and
actual allocator-null errors/recovery for every fixed callback key. Pre-cancelled
requests still win over invalid policy/input/resident size.

An opaque17-box wrapper around the fixed C ICC16 animation retains every metadata
record and all three coalesced canvases against independent C U16 vectors. Exact
workspace and one-byte-under refusal are checked. Existing all-boundary Abort,
callback-error, cancellation, precision/EX and independent timing/frame tests pass.

Actual wasm32/Node24.19 passes16 resource groups under each enforced32/64 MiB
linear ceiling, no host imports, including metadata-held callback and direct native
frames plus exact/one-under workspace. Existing composed output/animation WASM
passes932 checks. Native32-bit and local Windows/macOS were not run; inherited
CI jobs include the changed tests/harness. CI completion is not awaited.

Before/after release probes use identical settings and the exact #100 baseline
binary hash from its snapshot. Seven source/sRGB/opaque/brob cases keep identical
poster bytes and match independent C references (observed maximum error0). All
frames are qualified separately by the tests; the benchmark hash covers the poster
while timing includes full animation decode and frame/metadata destruction.
Three alternating paired blocks, three warmups, CPU0, with300/100/30 timed calls
by workload give median ratios0.904–1.027. This noisy local snapshot records both
possible small extra-check cost and variation; it is not a zero-overhead or general
speed claim. A managed checkerboard display was inspected against C's pixels.

Bare ordinary workspace increases1215 bytes (16 record slots plus fixed keys).
For17 opaque boxes, source output grows8494 bytes, including a6646-byte resident
owner plus1848-byte replacement overlap; managed output grows6646 because its
previous CMS peak dominates. Full floors and variability remain in the snapshot.

```sh
cargo test -p wml2 --locked --no-default-features --features jpeg-xl --lib \
  --test jpeg_xl_buffered_animation --test jpeg_xl_animation_callbacks
python3 -B wml2-test/scripts/verify_jpegxl_resources_wasm.py --output .test-owner-wasm
python3 -B wml2-test/scripts/benchmark_jxl_animation_metadata.py \
  --before .test-before-probe --after .test-after-probe --output .test-owner-bench
```

Build both probes as described in [format admission](jpeg-xl-format-admission.md).
The benchmark inserts metadata before the first codestream box, preserving original
control/size-to-EOF semantics. No C fixture is re-encoded with the candidate.
No upstream/fork write, merge, approval, release or issue closure is included.
