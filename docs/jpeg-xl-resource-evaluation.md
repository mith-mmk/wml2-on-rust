# JPEG XL resource-policy evaluation

For [#37](https://github.com/mith-mmk/wml2-on-rust/issues/37), retain the shipping
caps and API in this batch. The measurements justify improving admission
visibility and defining a caller-selected resource policy; they do **not**
justify increasing caps or replacing the current estimate with a fitted ratio.
No runtime source, dependency pin, Cargo manifest/lock or existing public API
changes. The issue remains open for the eventual implementation and validation.

This branch is based directly on main
`6a48db924b338c5ae7095bea6105fb4dbd436e95`. It does not incorporate or modify
PNG/TIFF, image-info/reduced-JPEG, JXL metadata/preview or cancellation branches.
The [native record](jpeg-xl-resource-evaluation.json) and
[WASM record](jpeg-xl-resource-wasm.json) contain source/artifact/input hashes,
commands' configuration, individual results and measurement scope.

## What the current limits actually mean

The current pinned `jxl =0.7.4` adapter has separate caller limits and fixed
profile/complexity restrictions. `DecodeLimits::unlimited()` disables only
caller limits. It retains every fixed adapter cap and the codec's finite
sample/complexity checks. A canvas quota is not a layer-storage quota.

| Restriction | Current behavior | Evidence in this batch |
| --- | --- | --- |
| Input | 64 MiB, independently of caller input budget | Exact 64 MiB container reaches metadata admission; 64 MiB+1 rejects even with unlimited caller limits |
| Dimensions | 16384 on output/internal padded layer dimensions | Valid 16384×1 accepted; 16385×1 rejected even with unlimited caller limits |
| Canvas | caller pixels and fixed 4 Mi pixels | Kept independently from internal-frame admission |
| Effective canvas basis | `ceil(max(width,16)*max(height,16)*4/3)` ≤4 Mi | 2048×1536 reaches the cap exactly; 2049×1536 and 2560×1440 reject |
| Internal storage | each padded/upsampled layer ≤4 Mi pixels; cumulative f32 planes retained conservatively | Real negative-origin/reference/blend layers; 64-layer preflight accepted with unlimited, 65 rejected |
| Header/TOC | 64 KiB prefixes, 4096 TOC entries, finite permutation allowance | Source audit and existing regression coverage; no new valid 64-KiB/4096-entry boundary fixture is claimed |
| ICC | coded size ≤16 MiB, advertised expansion validated (≤256 MiB), two retained profiles within metadata allowance; entropy admitted before expansion | Existing small Gray/RGB ICC plus valid 1-MiB private ICC tag |
| Auxiliary metadata | aggregate bytes and insertion/box-index cap; 16 KiB generated-profile/key allowance | Valid 1/8-MiB opaque boxes and exact metadata-policy allowance boundary |

The 4/3 factor is an admission/sample-limit allowance, **not** a measurement that
all JPEG XL images physically need 33% padding. The codec's
[check_size_limit](https://github.com/libjxl/jxl-rs/blob/624ce908afcf8eb0dc2585a671535eaabb5d88cc/jxl/src/api/inner/codestream_parser/mod.rs)
counts `max(width,16)*height*(3+extras)` and rejects `>=sample_limit`; its canvas
and frame checks have different geometry, and its palette transforms also use
the limit. WML2 additionally scans padded/upsampled frame sizes, signed crop
extents and TOCs. These are different quantities; replacing all of them with
output pixel count would lose admission coverage.

The current header/layer estimate uses an input/header base, 128 bytes per
maximum admitted effective internal pixel, cumulative f32 colour/extra planes,
ICC coded/expanded copies and entropy allowances. Final metadata copies and
CMS compilation are checked separately. The input-assembly screen is also
separate. The development probe reports the **header/layer stage estimate**;
it is not the final whole-route estimate or a reservation of that many bytes.
See the existing [layer contract](jpeg-xl-layers.md) for the two-layer fixture.

The estimate is explicitly not a global allocation ledger, process RSS bound,
linear-memory ceiling, or promise that the next allocation succeeds. Individual
codec allocations have fallible paths, but this batch does not establish that
all dependency allocations are budget-guarded or recoverable.

## Measured native cost

Linux x86_64 / AMD EPYC 9V74, Rust 1.91.0 release, exact published jxl0.7.4,
`jpeg-xl,image-buffer-ex,color-management`, default codec features off. Base
fixtures use three separate processes per route, each with three warmups and
three timed decodes, pinned to CPU0, without concurrent local builds. Generated
and limit cases use one process/one timed decode after three warmups; their
individual timings are diagnostic snapshots, not stable performance estimates.
This is the unchanged WML2 runtime, not a before/after speedup claim.

Peak requested heap is additional live requested bytes relative to immediately
before decode, with the returned image held through sampling. Source loading,
output hashing and JSON printing are excluded; all public-entrypoint copies,
codec allocations, conversion and output storage during decode are included.
Allocated-byte traffic is recorded separately. System allocator bookkeeping,
stack, mapped memory and transient overlap inside realloc are not counted.
Linux VmHWM is the **whole process** high-water including caller input, runtime,
warmups and allocator retention; it must not be subtracted from these values to
infer decoder RSS. Reserved-but-untouched capacity can make requested heap
larger than resident pages.

| Trusted fixture | Header/layer estimate bytes | Ordinary requested peak | Native EX requested peak | CMS requested peak |
| --- | ---: | ---: | ---: | ---: |
| RGB8 7×5 | 1095740 | 63214 | 63214 (RGBA8) | 74200 |
| Embedded RGB ICC 7×5 | 9490214 | 4205854 | 4205854 (RGBA8) | 4205598 |
| Embedded Gray ICC16 7×5 | 9486994 | explicitly refused | 4204961 (Gray16) | 4204705 |
| JPEG-derived 606×500 | 65215322 | 6178973 | 6178973 (RGBA8) | 7391595 |
| Layered/oriented 924×1386 | 336593332 | 39338179 | 39338179 (RGBA16) | 39338993 |
| Flat RGB 2048×1536, caller unlimited | 575671190 | 25186149 | 25186149 (RGBA8) | 62934278 |

The large ICC entropy allowance dominates a tiny canvas. CMS can materially
increase storage on a flat large image. The layered decode's peak happens in
reconstruction, so CMS does not necessarily raise the peak much. Gray ICC's
ordinary refusal remains a colour-contract decision, not a failed memory test.
These examples cannot establish a safe smaller bytes-per-pixel constant.

For context, median base-route times (µs, ordinary/EX/CMS) were:
RGB7×5 `55.834/62.915/154.914`, RGB-ICC7×5
`73.310/74.603/140.401`, 606×500
`15477.811/15902.721/1038977.453`, layered
`167629.933/159867.330/2520373.742`. Individual process times, output hashes,
allocation traffic and whole-process RSS remain in the JSON. The long CMS paths
are measured work, not inferred from output size.

### Rejections versus actual storage

- 2048×1536 has 3145728 canvas pixels and exactly 4194304 effective pixels.
  Default 512-MiB expanded budget rejects before decoding. Unlimited caller
  budget admits the unchanged runtime, with a 575671190-byte header/layer
  estimate but measured ordinary peak 25186149 bytes (CMS62934278). This gap is
  evidence of conservatism on this input, not a bound for other streams.
- 2560×1440 has 3686400 canvas pixels but 4915200 effective pixels. All WML2
  routes reject at the fixed padded-pixel cap, including unlimited caller
  limits. A **separate trusted-core-only** decode succeeds with 30530846
  requested peak bytes, including RGBA16 output. It skips WML2 admission,
  metadata/callback handling and CMS; it is not an estimate of a newly admitted
  WML2 route and does not authorize raising its caps.
- The 16384×1 Gray case is accepted with a 57324818-byte stage estimate;
  ordinary requested peak is only 278815 bytes. Minimum-height padding and TOC
  policy dominate this unusually thin example. The next width is rejected by
  WML2 even though the separately guarded trusted core can decode it.
- The 64-layer preflight estimate is 2416982500 bytes; preflight itself uses
  only 8710 additional requested bytes. This is **preflight only**; no successful
  64-layer pixel decode/peak is claimed. The 65-layer input is refused. A sum of
  conservatively retained planes is not necessarily simultaneously live memory.

Five generated constant-image core outputs match hashes computed from their
independently prescribed original integer codes normalized to RGBA16. These
checks establish that the particular generated large/thin inputs decode as
intended; hash checks are not a second decoder or a general hostile-input test.
Existing libjxl/djxl/LittleCMS oracle tests remain the pixel-correctness baseline.

### Metadata and input staging

A valid 1-MiB opaque box on the same 7×5 image raises ordinary peak to3146826
bytes; an8-MiB box raises it to25166922. Their pixels hash identically to the
bare fixture. A valid ICC profile with a1-MiB private tag raises tiny-image
ordinary peak to6301966 (CMS6309300), despite its compact coded representation.
Metadata byte/complexity admission cannot be derived from canvas dimensions.

The exact1-MiB-box source-metadata policy threshold is1064995 bytes, including
source header and the16-KiB generated-profile/key reservation. Threshold−1
rejects; threshold admits the inspected and ordinary routes. This is the
current conservative policy allowance, not a count of allocated metadata bytes.

The public slice entrypoints have an additional limitation relevant to any
future strict allocation contract: `draw::image_from_with_limits`, the EX path
and the managed helper construct `bin_rs::reader::BytesReader::new(input)`.
The pinned bin-rs0.0.10 constructor performs `buffer.to_vec()` **before** the
shared guard/JXL length admission. That adapter-owned copy is included in this
measurement, not dismissed as caller allocation.

For an exact64-MiB valid container, default metadata rejection peaks at134218148
requested bytes; unlimited caller metadata/workspace succeeds at201327072.
At64MiB+1, WML2 correctly reports the fixed input-limit error, but the ordinary
slice path has already requested67109203 bytes. The preflight-only inspector
rejects that size using74 bytes because it borrows the supplied input directly.
The latter number must not be advertised as public-API rejection cost.

A future allocation-aware slice API should admit the borrowed length before
constructing an owned reader, or use a borrowed reader. Reader/caller buffering
must remain explicitly outside or inside the contract as appropriate. This
report records the existing shared staging behavior; it does not change all
formats' loaders as an incidental JXL patch.

## WASM and 32-bit evidence

The unchanged runtime compiles for i686 Linux with all three JXL features.
This is **compile only**, not32-bit native execution, RSS or allocation-failure
coverage. Existing signed crop/extent checks and checked usize arithmetic remain
necessary. The64-layer estimate fits u32 but exceeds signed2-GiB; an aggregate
estimate is not itself one contiguous allocation. Every actual allocation also
needs its own representable Layout/capacity and fallible failure path.

Node24.19.0 executes the existing independent-oracle harnesses with
`--wasm-max-mem-pages=1024` (64MiB) and512 (32MiB). Each managed case uses a fresh
instance. There are no host imports. Source/native EX61-case validation passes
at both ceilings, reaching14286848 bytes of linear capacity. The14-case managed
and orientation harness passes at64MiB, maximum54394880 bytes. At32MiB,13cases
pass and the layered display harness traps; its `expect` converts decode errors
into a trap, so that observation alone does **not** show a library panic.

A separate typed-result layer harness avoids `expect` and reference/staging
buffers. With the512-MiB caller expanded policy it:

- succeeds at64-MiB engine ceiling,924×1386, linear capacity43057152;
- returns a normal allocation error at32MiB,
  `memory allocation failed because the memory allocator returned an error`,
  with no successful dimensions/output and capacity30605312;
- returns the explicit preflight workspace-limit error under a256-MiB caller
  expanded budget at either engine ceiling, before pixel reconstruction.

Thus both policy rejection and allocator failure can be observed without
publishing a successful owned image on this fixture. This is finite evidence;
it is not proof that every codec allocation failure is recoverable. Linear
capacity is monotonic allocation capacity, not live requested heap or total
WASM/JS process memory. Node process RSS is recorded separately and includes
engine compilation/runtime. No new Chromium/browser display, Windows/macOS
execution or universal WASM-safe image size is claimed.

## Proposed contract, not an implemented API

Recommend an additive JXL-specific options/profile API, leaving existing
entrypoints/defaults and `unlimited()` behavior compatible. Its first revision
should expose **caller-tightened finite sublimits and cost diagnostics**. Higher
ceilings need a separately reviewed implementation with the gates below; do not
silently reinterpret unlimited as permission to disable internal checks.

Separate the following quantities instead of offering a single “memory limit”:

1. Existing `DecodeLimits` canvas, output/retained bytes, input, metadata and
   visible-frame quotas. Define slice/reader ownership and admit known copies
   before materialization; account ordinary/EX precision conversion and CMS
   source/working/output buffers explicitly.
2. A versioned JXL resource profile: coded/oriented dimension limits, per-frame
   padded/upsampled samples, internal-frame count independent of visible outputs,
   cumulative reference-plane allowance, header/TOC counts and permutation
   workspace, ICC coded/expanded/profile copies, metadata expansion/parse costs.
   All sample/complexity limits stay finite. Profile level5/10 is not a byte budget.
3. A cost report with named stage components, estimate version, largest single
   anticipated allocation, and which costs are verified versus conservative or
   unknown. Distinguish current-policy refusal from invalid structure and
   allocator failure. Preserve existing error compatibility in old entrypoints.
4. If a strict **requested-allocation** guarantee is promised, require an
   audited shared fallible allocation ledger across the codec, adapter and CMS,
   including transient overlap, palettes, entropy, references and profile
   parsing. A pixel estimate alone cannot provide it. Scope stack, allocator,
   caller memory, thread pools and host/linear memory separately. Until that
   exists, name the option an admission/workspace policy, not a process bound.

The4/3 canvas allowance and cumulative retained-plane sum are candidates for a
better proven plan, not constants to scale down from these measurements. A
liveness-aware reference plan would have to cover replacement, invisible/LF
frames, non-final/reference output, blending, padding/upsampling and every
codec-side allocation using those dimensions. Keep the current conservative
fallback for unproven paths rather than admitting them under a smaller guess.

### Dependency and cancellation coordination

[PR54/#40](https://github.com/mith-mmk/wml2-on-rust/pull/54) holds jxl0.8.0 because
its public API hides the0.7.4 ICC/header/TOC parsers. BasicInfo is already too late
for ICC pre-admission, and canvas data cannot bound invisible layers. The
source-prefix extraction used **only by this development probe** is not a
runtime workaround or a second maintained parser to carry into0.8.

Ask for an upstream pre-allocation admission/plan interface and, for a hard heap
contract, a fallible allocation-budget hook. It must expose/check advertised
ICC sizes before expansion, header/TOC/permutation complexity before allocation,
actual internal/reference storage and its lifetime before reconstruction, with
checked platform arithmetic. Preserve preflight on0.7.4 until an equivalent
interface or an explicitly reviewed narrow fork exists. Smaller0.8 core
allocation traffic measured in PR54 does not establish a smaller worst-case cap.

[PR32/#31](https://github.com/mith-mmk/wml2-on-rust/pull/32) cancellation is an
independent typed outcome, not a memory-budget substitute. Preserve pre-cancel
precedence before admission/copying, both sequential runner overrides, all
stage/ICC-batch checks and private-output disposal. Serial parser/allocation/
individual codec tasks remain non-interruptible; do not promise a latency bound
or use cancellation to justify a larger admitted allocation. Re-run that
branch's actual contracts after composing a real policy implementation.

Metadata brob/structured parsing, previews/intrinsic sizes, extras, animation,
HDR and incremental output remain separate branches. Their expansion, new
frame lifetimes and extra output buffers require new cost categories and reruns;
this main-based snapshot does not certify their composed memory behavior.

## Acceptance gates for a runtime follow-up

- Preserve old API outcomes, defaults, independent pixel/ICC/orientation/alpha
  references and Gray ICC refusal, plus PR32 cancellation/error precedence and
  no publication on failures. Feature-off remains valid.
- Test exact limit−1/limit/limit+1 for dimensions, canvas and padded samples,
  header bytes, coded/expanded ICC, auxiliary bytes/box insertion, TOC entries,
  internal frames and each output/CMS stage. Include thin/odd dimensions,
  negative origins, reference replacement/LF frames and many retained layers.
- Verify source and internal geometry independently, including2560×1440 and
  larger varied-content stills; retain finite codec complexity/sample checks.
  Never count a core-only success as successful WML2 admission.
- Exercise allocation failure at each allocation class, many simultaneous known
  buffers and parser limits **before** the relevant allocation. Include the
  public entrypoint reader/copy and CMS parsing/compilation. For a strict ledger,
  assert its invariant over requested live bytes, not just largest allocation.
- Execute actual32-bit and bounded-memory WASM paths, including clean errors and
  no publication near exhausted linear memory; compile success alone is not
  execution. Keep native requested heap, traffic, process RSS and WASM capacities
  separately named. Measure varied real images, not only constant fixtures.
- Resolve the0.8 parser/budget interface before migrating or removing private
  admission assumptions. Evaluate performance and compatibility again on the
  final composed metadata/preview/cancellation base before selecting higher caps.

## Reproduction and scope

```sh
# Matching libjxl0.11.2 development headers and shared runtime are required.
cc -O2 wml2-test/scripts/jpegxl_fixture_encoder.c -Wl,-l:libjxl.so.0.11 -o .test-encoder
# Native timing should run without concurrent builds/tests.
taskset -c 0 python3 wml2-test/scripts/evaluate_jxl_resources.py --output .test-jxl-resource --encoder ./.test-encoder
python3 wml2-test/scripts/evaluate_jxl_resources_wasm.py --output .test-jxl-resources-wasm
cargo check -p wml2 --locked --no-default-features --features jpeg-xl,image-buffer-ex,color-management --target i686-unknown-linux-gnu
cargo test -p wml2 --locked --no-default-features --features jpeg-xl,image-buffer-ex,color-management --lib --test jpeg_xl --test jpeg_xl_additional --test jpeg_xl_layers
```

The scripts require an empty output directory or their own ownership marker;
they reject checkout roots and other nonempty task directories.

The native script generates temporary inputs with independent libjxl0.11.2,
not a modified decoder. It creates a standalone manifest and an admission module
from the unchanged `jpegxl.rs` prefix through preflight, appending only a report
wrapper. A local check shim reproduces the existing limit/error comparison;
no caps or parser operations are replaced. Actual route measurements call the
shipping WML2 public APIs. The inspector is an instrumentation aid, not a new
supported query API or total-admission promise. Core-only mode is restricted by
convention to these trusted fixtures and has an explicit finite canvas/sample
guard; it is unsuitable as an arbitrary-input service.

Local existing-runtime matrix:53tests pass (25unit,22original,3additional,
3layers). Existing harness pixel/orientation/profile assertions run in WASM;
new resource harnesses and generated/limit probes are development evidence.
Runtime diff against main is empty. No merge, release, dependency upgrade or
issue closure is implied by this evaluation.
