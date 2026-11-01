# First-use implicit-palette retention and finite admission

This unmerged production/diagnostic stage stacks on #94
(`148240653e253043c5b4cba02e773fe7fe6ede1e`). It resolves the attribution question
left by #43 and continues original #37/#63's allocation-aware resource work.
It changes no dependency, fixed cap, rational/integer-ms timing policy, main or
release. #74's0.7.4 evaluation patch is not applied.

## Attribution, not a blanket leak assertion

The reported **4008 requested bytes** are four live allocations:1716 +768 +1500
for three fixed RGB i32 arrays, plus24 for their three-pointer boxed descriptor.
Allocation stacks resolve to published jxl0.8.0
`frame/modular/transforms/palette.rs`, `ImplicitPalette::new` and
`Palette::new`'s static `IMPLICIT_PALETTES[bit_depth].get_or_init`.
The locked-archive source hash is
`358253013cf07ab3c7cfd845d559a4baadcb08be94986336c0a886ead625db1d`.
The allocation owner is a **process-wide `OnceLock<Box<ImplicitPalette>>`**, not
an image/collector or TLS cache. Its reference has static lifetime. Image drop,
Abort or decode error deliberately does not clear it; Rust statics are retained
until process exit. There is no private-cache mutation or forced freeing here.

The source has25 slots indexed by `min(source_bit_depth,24)`. Each entry owns
three fixed-size arrays of3×143,3×64 and3×125 i32s, plus3 thin pointers. Therefore
requested heap per entry is4008 on64-bit /3996 on32-bit; worst case all25 slots
is100200 /99900. This is per linked jxl0.8 instance, independent of image size,
frame count and thread count. It excludes allocator headers/rounding, inline
static OnceLock storage, other codec caches and process RSS.

The [validation report](jpeg-xl-palette-cache-validation.json) records fresh
Linux x86_64 processes and symbolized allocation stacks:

| First operation in a fresh process | First net requested bytes | Following same-operation calls |
| --- | ---: | ---: |
| Owned common success |4008|0,0|
| Native rational API success |4008|0,0|
| begin Abort before input decode |0|0,0|
| init Abort after private decode |4008|0,0|
| draw Abort |4008|0,0|
| Early truncation |0|0,0|
| Late entropy error |4008|0,0|
| Too-small finite header/cache quota |0|0,0|

Switching16→8→16→8 gives4008,4008,0,0. Two sequential fresh worker threads
both decoding16 give4008,0: the second thread reuses the process cache.
These observations agree with source ownership and slot bounds. They establish
this allocation's attribution and reuse under the tested paths; they do **not**
establish absence of all leaks, peak heap/RSS, or other-depth/input behavior.
The32-bit byte count is statically derived; it is exercised by the new unit test
in Windows32 CI, not claimed as a locally traced32-bit allocation result.

## Next original resource step

#94's finite resource estimate did not give this persistent codec cache an
explicit reservation. #37/#63's next small implementation now admits the
**entire25-slot requested-heap upper bound** alongside header/metadata workspace
before the pixel codec can initialize it. Every finite `JxlResourceLimits` call
holds the same conservative allowance, cold or warm. It never looks into private
cache state and never depends on which thread/input initialized a slot. Concurrent
admissions may count shared memory twice, which is conservative for this per-call
contract. Reservations are estimates, not ownership or cache deallocation.

Legacy admission (`workspace_bytes == usize::MAX` internal policy) is unchanged;
the public finite policy continues rejecting that sentinel. Explicit finite
workspace minima grow by100200 /99900 bytes; defaults and output pixels remain
the same. Limits tight enough to fit only the earlier estimate now return the
existing quota error. No global-heap guarantee is added: upstream cache
construction remains infallible and other codec-owned allocations remain outside
an enforceable ledger. #37 large-image/cumulative codec work and #63 stay open.
The bounded preflight retry also uses the same header/cache allowance, preserving
quota refusal before another header parse.

## Reproduction and checks

The diagnostic uses a fixed TLS pointer table, matching actual alloc/free/realloc
addresses without allocating its own tracking table. Allocation stacks use GCC
unwinding, symbolized by `addr2line` after tracking stops. Every mode runs in a new
process, while repeated calls share that process. Thread creation and output are
outside the measured interval. Empty baselines, external frees and table overflow
are checked. This measures Rust requested bytes on the calling thread, not all
C allocations, allocator overhead or RSS. The diagnostic is Linux x86_64 tooling,
not a product dependency or a portable allocator implementation.

```sh
python3 wml2-test/scripts/verify_jpegxl_palette_cache.py --output .test-palette-cache
cargo test -p wml2 --locked --features jpeg-xl,image-buffer-ex,color-management --lib --test jpeg_xl_resources --test jpeg_xl_animation_callbacks --test jpeg_xl_buffered_animation
python3 wml2-test/scripts/verify_jpegxl_wasm.py --output .test-cache-wasm
```

The source/probe hashes, fresh-process matrix and selected stacks are committed
in validation. The stable Linux JXL+EX CI row repeats this attribution diagnostic;
normal macOS/Windows/minimal/EX/CMS and actual Windows32 regressions also run.
Local253 native tests (103 library),19 minimal cases,35 CI-script tests and actual
wasm32+Node932 checks pass. Existing fallible adapter/OOM and cancellation tests
are retained. The independent C+CMS3-frame display is byte-identical and inspected.
Equal-build CPU0 release benchmarking against exact #94 uses6 alternating paired
blocks×1000 samples after3 warmups: ordinary1.0125, managed1.0099, Depth0.9885 median
ratios. The tiny ordinary/managed cases have roughly1% higher medians in this
sample; variation overlaps and this single-host measurement does not establish
a universal regression or speedup. No resolution-cap removal is claimed.

## #43 acceptance checkpoint

#93/#94 supply the original finite animation acceptance scope: independent
multi-frame/variable-time/loop/Alpha/crop/reference/orientation/ICC checks,
frame/output/workspace limits, malformed/truncated input, Abort and cancellation.
The first-use retained bytes now have an identified persistent owner and a source
bound. #43 stays open while unmerged. Fractional-ms common callbacks still refuse
explicitly and the rational native API stays exact; no extra Gray/preview/SMPTE/
Alpha/profile matrix is invented as a new completion condition. The next work
here is #37/#63's known allocation class, rather than extending #43 endlessly.
