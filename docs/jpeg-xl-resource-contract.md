# Finite JPEG XL resource configuration: first runtime phase

This follows #37 and the read-only [PR #63 evaluation](https://github.com/mith-mmk/wml2-on-rust/pull/63).
It is based on released main `f2151b2408e9d04f1bc7b4e9e29679e078a57209`.
PR #63 belongs to the same author and remains unchanged at
`79cb8f579f0601652731d42341d30ba0423b838d`. Its eight changed evaluation files
contain no runtime implementation; this branch does not copy/rewrite that report
or its development dependency graph.
The input-sized reader-copy observation in that older evaluation is historical:
released main already contains PR #66's borrowed JXL admission. Admitted codec
input/codestream staging still exists and is accounted separately here.

**This phase implements finite caller settings and cumulative adapter workspace
checks. It does not enable larger canvases.** The 2560x1440 refusal remains, and
#37 is not complete. No fixed cap, dependency, version or existing API changes.

## Baseline reproduction

Fresh independent C libjxl 0.11.2 lossless original-profile files contain constant
RGB `[64,128,192]`. Their dimensions, prescribed RGBA, source and payload hashes
are frozen in [the fixture manifest](../wml2/tests/fixtures/review/jxl-resource-fixtures.json).
These are generated test inputs, not private files or WML2-encoded references.
The same baseline-compatible [reproducer](../wml2-test/scripts/jpegxl_resource_reproduce.rs)
was linked against released-main source and its unchanged lockfile:

| Input | Default caller limits | Caller unlimited |
| --- | --- | --- |
| 2048x1536, 1483 bytes | `JPEG XL decode workspace` error | Successful 2048x1536 |
| 2560x1440, 8092 bytes | `JPEG XL padded frame pixels` error | Same padded-frame error |

The historical effective basis is
`ceil(max(width,16)*max(height,16)*4/3)`: 4194304 and 4915200 respectively.
It is a sample-limit/admission allowance, not evidence that all images physically
need 33% padding. Actual padded/upsampled layers are parsed and checked separately.

## Explicit APIs

Enable `jpeg-xl` for source-coordinate RGBA8:

```rust
use wml2::{jpegxl::{JxlResourceLimits, image_from_with_resource_limits},
           limits::DecodeLimits};
let resources = JxlResourceLimits {
    workspace_bytes: 128 << 20,
    internal_layers: 8,
    ..Default::default()
};
let image = image_from_with_resource_limits(input, DecodeLimits::default(), resources)?;
```

With `color-management`, the companions are
`image_from_srgb_with_resource_limits` and
`image_from_srgb_with_resource_limits_and_cancel(input, limits, resources, &probe)`.
They return owned, private-until-success `ImageBuffer` results. No callback or
EX resource-options API is added in this first phase; existing EX APIs retain
their `DecodeLimits` and fixed envelope. Explicit arguments avoid a new global or
thread-local policy setting. Existing API signatures and acceptance/error order
are preserved; callers choose the stricter new contract explicitly.

| Resource field | Default and permitted ceiling | Meaning |
| --- | --- | --- |
| `workspace_bytes` | Default 512 MiB; any finite usize value | Effective quota is min(this, caller expanded_bytes); usize::MAX is rejected, including with caller unlimited |
| `internal_pixels` | 4 Mi | Scanned padded/upsampled internal layer and preview geometry, including the existing minimum-16 allowance; independent of output canvas |
| `effective_pixels` | 4 Mi | Historical 4/3/minimum-16 canvas basis, kept separate from physical layer geometry |
| `internal_layers` | 64 | Coded main layers, including invisible/reference work; one skipped preview does not consume this count |
| `toc_entries` | 4096 | Checked before the pinned TOC/permutation reader allocates |
| `icc_coded_bytes` | 16 MiB | Encoded ICC prefix length, checked before ICC entropy work |
| `icc_expanded_bytes` | 256 MiB | Advertised expanded ICC size; common metadata and retained-profile checks still apply |
| `brob_bytes` | 64 MiB | Cumulative decompressed auxiliary bytes, not a per-box reset |

All non-workspace fields can only tighten their shipping ceilings. Zero is valid
and refuses a positive requirement for that resource; zero brob quota can still
admit empty decoded payloads. Fixed input64 MiB, dimension16384, output4 Mi,
header64 KiB, container/profile restrictions and the codec's finite sample guard
remain. Caller pixels/frames/animation_bytes/metadata limits remain independent.
A pre-cancelled request wins over invalid policy/input. Other encountered errors
retain precedence over later probes. Existing non-interruptible work remains so.

## What the cumulative quota charges

The existing header/frame planner retains its conservative input/header base,
128-byte effective-pixel allowance, sum of preceding f32 layer planes, bounded
TOC permutation workspace, ICC coded/expanded allowances and metadata costs.
That existing estimate is not reduced or refitted to the flat-image observations.
This phase adds checks for known overlap instead of claiming the whole estimate
is an exact allocation ledger:

1. Input staging checks `3*input_len`. Raw auxiliary metadata additionally checks
   staging plus `2*retained_metadata` before copying that payload. Brob keeps its
   bounded allocator and cumulative output accounting inside the remaining quota.
2. Preflight counts actual coded main layers, padded layer/preview dimensions,
   the separate effective basis, TOC entries and coded/expanded ICC against the
   selected sublimits. Signed crop, TOC extent and checked arithmetic guards stay.
3. After generated/structured metadata admission, a live adapter ledger starts
   from the retained codec/metadata estimate. It claims aligned output bytes
   before fallible reservation, then any high-depth-to-RGBA8 rounding buffer.
4. CMS claims the separate RGBA16 sample copy. Before invoking the existing own
   CMS, it reserves the two parsed profiles' declared payload copies, tag-range
   arrays and a conservative 4096-byte fixed object allowance per profile.
   Pinned icc-profile0.0.7 stores ranges rather than copies of aliased tags;
   transform compilation's own retained/temporary budget receives only the
   unclaimed remainder. Parsed profiles/compiled transforms die before this
   allowance is released. CMS alpha handling is unchanged.
5. The 8-bit CMS result overlaps the RGBA16 samples until explicit sample drop.
   Aligned output and any final rounding result stay charged while the owned
   RGBA8 collector is admitted and allocated. Only dead storage is released.

Claims use checked arithmetic and fail before the corresponding owned allocation.
The runtime output reservation is fallible; a test injects a real allocator null
at that exact helper and verifies error propagation. That test does not inject
arbitrary dependency allocation failures or claim a global OOM guarantee.

## Why larger images remain gated

The selected implementation is a compatible first step, not a cap increase.
Pinned jxl0.7.4 has no caller byte allocator/ledger hook across reconstruction.
Its entropy context-map implementation uses `Vec::with_capacity`, `vec!` and
collection/HashSet allocations; other frame/profile paths also have infallible
allocations. Its sample guard bounds samples/palette complexity, not all entropy
histograms, reconstruction scratch or process memory. Individual fallible
requests exist, but do not establish complete recoverable denial coverage.

Source anchors: [pinned entropy context maps](https://github.com/libjxl/jxl-rs/blob/624ce908afcf8eb0dc2585a671535eaabb5d88cc/jxl/src/entropy_coding/context_map.rs),
[decoder options](https://github.com/libjxl/jxl-rs/blob/624ce908afcf8eb0dc2585a671535eaabb5d88cc/jxl/src/api/options.rs),
[ICC profile representation](https://docs.rs/crate/icc-profile/0.0.7/source/src/transform/profile.rs),
and [ICC compile admission](https://docs.rs/crate/icc-profile/0.0.7/source/src/transform/compile_budget.rs).

| Choice | Decision and evidence |
| --- | --- |
| Finite caller-tightened envelope plus adapter/CMS overlap checks | Implemented here; makes useful runtime progress without widening untrusted-input exposure or changing existing API contracts |
| Remove the padded screen or raise a cap based on flat-image peaks | Not justified. The #63 trusted-core and flat-input measurements do not bound hostile entropy, reference or ICC paths |
| Qualify a bounded upstream/fork allocation contract before wider admission | Recommended next step. Account/deny pre-allocation codec requests cumulatively, audit fallible denial paths and retained references, and preserve equivalent ICC/TOC preflight. Updating to 0.8 alone does not supply this contract |

There is no strict global heap/RSS promise, allocator bookkeeping estimate,
caller-owned-storage quota, arbitrary callback-memory bound or cancellation
latency guarantee. The new quota deliberately adds buffer storage on top of a
conservative codec estimate and may reject a file with a lower measured live
peak. A larger accepted profile needs its own qualified admission and denial
evidence; the 2560x1440 case remains a frozen negative until then.

## Validation and reproduction

Native tests cover exact/one-under effective geometry, actual layer geometry,
expanded ICC, cumulative brob216/215-byte and final collector workspace
boundaries; invisible/reference layer limits; malformed/truncated envelopes;
independent ordinary/CMS pixels and preview/intrinsic composition; cancellation
at every managed checkpoint; arithmetic overflow; and output allocation refusal.
The existing ordinary/EX/CMS/metadata/borrowed-admission suite is retained.

The new actual-WASM harness runs twelve groups under separately linked32/64 MiB
linear ceilings, with no host imports. It checks small independent ordinary/CMS
outputs, policy/common-limit refusals, the qhd cap rejection, cumulative brob and
cancellation. It does not qualify accepted large images on WASM. i686 Linux
tests compile, but the cloud kernel returns `Exec format error` before execution;
that is compile-only evidence. CI separately executes the target on32-bit Windows
and native Windows/macOS, as well as existing shipping-WASM checks.

The 606x500 new sRGB path matched the frozen independent LittleCMS reference
(max RGB error1, exact alpha); its PNG was visually inspected. Legacy ordinary
release benchmarks use three alternating process pairs, three warmups and eleven
timed iterations each. Pixel hashes and requested allocation count/traffic/live
peak agree with the baseline on all three inputs; timing is a local snapshot,
not a universal speed claim. See [recorded evidence](jpeg-xl-resource-contract-validation.json).

```sh
CARGO_TARGET_DIR=.test-target cargo test -p wml2 --locked --no-default-features \
  --features jpeg-xl,exif,image-buffer-ex,color-management --lib --test jpeg_xl_resources
python3 -B wml2-test/scripts/verify_jpegxl_resources_wasm.py --output .test-resource-wasm
python3 -B wml2-test/scripts/generate_jpegxl_resource_fixtures.py \
  --include /path/to/libjxl-0.11.2/include --output .test-resource-fixtures
```

The small baseline-compatible Rust reproducer can be linked with `rustc --extern
wml2=.test-target/release/libwml2.rlib -L dependency=.test-target/release/deps`
after building the library at either selected revision. Pass the boundary and
qhd fixture paths. Generator and runtime use separate implementations; no
private Windows corpus or TIFF XING_T24 file is used here. No issue closure,
merge, approval, dependency migration or release accompanies this phase.
