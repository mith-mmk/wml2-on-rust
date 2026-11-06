# Fallible output-format descriptor admission

This unmerged product follow-up for #37 builds on #98
`78c74be37582df7e63a1aad8618970f7663358a7`, following #97/#96/#95.
The isolated #96 codec patch remains unapplied to product dependencies. No
manifest/lock, feature, API signature, format envelope, fixed cap or version changes.

## Why this stage

The existing ordinary and float/HDR paths used `vec![None; extras]`; preview
called `JxlPixelFormat::rgba16`, which does the same internally in pinned0.8.
Those small adapter-created allocations could abort on allocation failure before
returning an image error. They were also absent from the explicit finite ledger,
although the decoder takes ownership of the format Vec and retains it through
reconstruction and any CMS/output preparation.

This residual adapter work can proceed without the dependency distribution or
HDR display decisions recorded in [#97's recommendation](jpeg-xl-dependency-integration.md).
It does not implement the broader codec-wide allocator that #37 still needs.

## Behavior and lifetime

A private helper checks `count * size_of::<Option<JxlDataFormat>>()`, admits that
requested payload before allocation, and uses `try_reserve_exact` before filling
the list. Quota denial leaves the ledger unchanged and precedes the allocator
request. Allocator refusal releases the new claim and propagates `TryReserveError`.
Count zero allocates no storage; overflow is rejected. Decoder format contents,
precision and endianness remain identical to the previous constructors.

Float/HDR and preview use their existing finite ledgers. Ordinary finite decode
carries a provisional format claim through source/output metadata copies into
the final output/CMS ledger, so later allocations cannot reuse those bytes while
the format still exists. The pinned decoder's `set_pixel_format` moves the Vec
into its retained parser state. No early release is added after format setup.
Existing animation and separate-extra-plane paths already use fallible, admitted
format lists and are unchanged. Legacy ordinary APIs gain fallible allocation
without new quota checks or changed admission minima.

On this x86_64/Rust1.91 build, one descriptor requests2 bytes. Before/after probes
find exact finite minima grow by2 on ordinary/sRGB, native F32, encoded float HDR
and raw preview Alpha cases; the no-Alpha control is unchanged. This is stricter
finite workspace admission, including possible earlier rejection before profile
copying. The profile-copy denial regression includes the admitted descriptor
before testing its original boundary. No canvas cap is raised.

These are requested payload counts, not allocator rounding/overhead or RSS.
Codec objects/tables/buffers, profile factories, metadata-key strings/generic
metadata-vector growth and callback-owned storage remain outside this new
fallibility claim. A fallible2-byte request does not make the whole decoder OOM
recoverable. #37/#63 and the2560×1440 refusal remain unresolved.

## Evidence and reproduction

[Validation](jpeg-xl-format-admission-validation.json) records native minimal99
and expanded133 tests on Linux x86_64/Rust1.91, plus feature-disabled compilation.
Added tests verify quota rejection before allocation, exact/one-under payload,
zero/overflow, real allocator-null failure with ledger rollback, five actual
legacy/finite/native-F32/HDR/preview routes, typed errors and successful recovery.
Existing independent ordinary/EX/ICC/preview/HDR and resource/cancellation tests
remain in the selected targets. No new fixture or decoder-self oracle is used.

Actual wasm32/Node24.19 passes932 composed ordinary/EX/preview/animation checks
and101 encoded-HDR/linearization checks. Both have no host imports; the HDR
module retains its enforced32 MiB linear-memory ceiling. This does not claim a
new all-codec heap bound or accepted-large-image WASM run. Native32-bit and local
Windows/macOS execution were not performed; existing CI jobs cover these targets.
CI completion is deliberately not awaited.

The independent C ordinary RGBA8 and preview U16 bytes match exactly. A magnified
ordinary Alpha checkerboard display was inspected. All paired before/after
outputs match in six measured routes.

The retained [probe](../wml2-test/scripts/jpegxl_format_budget_probe.rs) links
against either side without changing product dependencies. Build the selected
revision with no defaults and `jpeg-xl,image-buffer-ex,color-management`, then
link with `rustc --edition 2024 -C opt-level=3`, the actual `--extern wml2=<rlib>`
and its matching `-L dependency=<deps>` path. Run:

```sh
.test-probe wml2/tests/fixtures/review/jxl-rgba8.jxl ordinary floor
.test-probe wml2/tests/fixtures/review/jxl-rgba8.jxl ordinary 300 .test-output.rgba
```

Modes are `ordinary`, `managed`, `f32`, `hdr`, `preview` and benchmark-only
`legacy`. Floors search the smallest successful finite workspace and verify one
byte below refuses. Measurements use the exact same probe/feature graph, three
alternating paired blocks of300 timed calls, three warmups/process, CPU0. Initial
ratios range0.966–1.029. A bounded six-pair1000-call follow-up on ordinary/F32/
preview gives1.012/1.009/1.010 with overlapping block ranges. A small additional
cost around1% is visible in these medians; zero overhead is not established.
This narrow result is neither a general throughput promise nor a broad performance
qualification. The new admission and rollback checks remain part of the
measured implementation.

No merge, approval, release or issue closure is included.
