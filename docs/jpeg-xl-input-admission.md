# JPEG XL slice input admission

The ordinary, EX and managed slice entrypoints previously constructed an owned
`BytesReader` before input/workspace admission. Its `new(&[u8])` copies the entire
slice. A64MiB+1 signature-bearing JXL input therefore requested an input-sized
allocation before returning the expected fixed input-limit error. This also
applied to a caller-tightened input/assembly budget.

Recognized JXL slices now use the existing `StreamReader<Cursor<&[u8]>>` through
the same guarded decoder APIs. The managed JXL-specific helper borrows its input
unconditionally, including malformed signatures. bin-rs0.0.10 already supports
this reader on WASM. There is no new unsafe production code or public API.
`image_from`/`image_load` share the existing explicit-limit path; the custom
`image_loader` callback and EX default/explicit-limit variants also borrow JXL.

Format detection uses the existing feature-gated detector and signature rules.
Non-JXL and JXL-disabled routes retain their owned-reader behavior. This is not
a general fix for staging every unknown/other-format input. Reader-based APIs
also retain their existing ownership: a caller-created owned reader may already
have allocated its input before WML2 is called.

The same guard still checks caller input limits before dispatch, and the JXL
adapter checks its fixed input and assembly workspace limits before allocating
its owned codestream buffers. No caps, default budget, workspace formula,
precision/ICC/orientation behavior, error order or dependency versions change.
Later source assembly, metadata, codec/CMS and output allocations still occur
once admitted. This change does **not** provide a total requested-heap, process
RSS or WASM memory guarantee, nor complete #37's configurable-policy work.
The separate [PR63 evaluation](https://github.com/mith-mmk/wml2-on-rust/pull/63)
keeps its original pre-fix snapshot; it is not rewritten as post-fix evidence.

## Allocation regression evidence

The new native test first failed on main `6a48db9` for all three explicit-limit
routes. Input Vec construction is outside metering; every allocation/zeroed
allocation/realloc request during the public call is counted. Maximum request,
total requested bytes and count are independently checked, so many smaller
allocations cannot silently replace the avoided copy.

| Raw-signature64MiB+1 input | Before allocations / total requested / largest | After allocations / total requested / largest |
| --- | --- | --- |
| Ordinary with limits | 10 /67109451 /67108865 | 4 /338 /256 |
| EX with limits | 10 /67109451 /67108865 | 4 /338 /256 |
| Managed with limits | 4 /67108947 /67108865 | 3 /82 /42 |

All calls return `Out of memory: JPEG XL input exceeds decode limit`. After the
fix, both raw and container signatures and all default/legacy aliases pass.
Caller input and assembly-budget refusal tests also check bounded allocations.
The oversized test payloads have recognizable signatures; input refusal is
intentionally checked before pixel/codestream validity. No huge fixtures are
stored in Git. Managed invalid-signature oversized input preserves limit-error
precedence and avoids the copy too.

Other tests compare normal pixels/metadata/orientation and every short RGB
prefix/budget error with the original owned-reader path; container signature
boundaries, Gray ICC refusal, Exact precision refusal, PNG fallback and borrowed
input lifetime are covered. Independent existing pixel references remain in the
ordinary/EX/CMS suites. Local default+JXL matrix:107tests passed; standalone JXL-only41passed; JXL disabled:
1passed and the new target correctly empty. Native Clippy completes with existing
warnings and no new source/test diagnostics; formatting/diff checks pass.

## WASM execution

The existing61-case source/EX harness passes and the14-case managed/orientation
harness passes with its independent pixel/profile assertions (Node, no host
imports). The managed harness reaches54394880 bytes of linear capacity under a
64MiB per-memory ceiling. These capacities are not live heap or process RSS.

The new fixed-input `jxl_input_admission_wasm.rs` harness allocates the caller's
64MiB+1 Vec **before** metering and invokes ordinary, EX, managed and malformed-
signature managed entrypoints. Each case uses a fresh instance under a128MiB
per-memory ceiling. All correctly refuse; ordinary/EX make4requests totaling322
bytes, largest256; managed makes3 totaling66, largest42 (32-bit layouts differ
from native). Linear capacity before/after the decoder is68354048/68354048.
This is finite evidence for admission of these fixed cases, not a heap bound for
other images or host memory.

## PR32 cancellation composition

An isolated local no-commit composition of candidate `ca3df4e` with cancellation
PR32 head `da4f45764995110502a1a0a3cd9e933d86a5872f` applied without conflicts.
The borrowed-reader change automatically lands in the shared managed helper;
its `cancellation.check()` remains before reader construction and validation.
The full PR32 matrix plus slice tests and two composition checks passes84tests
(one existing manual benchmark ignored). Nothing from PR32 is included in this
main-based runtime PR, and no PR/main merge was performed.

A constant-true pre-cancel probe wins over oversized/invalid input and all zero
budgets, returning the typed `DecodeCancelled` with **zero measured allocations**.
A constant-false probe preserves the input-limit error with3allocations/82bytes,
largest42. Arbitrary caller cancellation probes can allocate their own memory;
these measurements use trivial probes and do not promise a cancellation latency.

Reproduce in an isolated worktree, merge PR32 locally without committing, append
the following two tests to `wml2/tests/jpeg_xl_slice_admission.rs`, then run the
matrix command below. This uses that file's existing allocation helper; source
hashes of the tested composition are in the JSON evidence.

```rust

// Append-only composition checks; requires PR32 and the slice-admission tests.
#[cfg(feature="color-management")]
#[test]
fn composed_precancel_precedes_copy_and_all_invalid_budgets(){
 let input=oversized(false);
 let limits=DecodeLimits{input_bytes:0,pixels:0,expanded_bytes:0,metadata_bytes:0,frames:0,animation_bytes:0};
 for bytes in [&input[..],&b"invalid"[..]] {
  let (result,stats)=measure(||wml2::jpegxl::image_from_srgb_with_limits_and_cancel(bytes,limits,&||true));
  let error=result.err().unwrap();assert!(error.is::<wml2::jpegxl::DecodeCancelled>(),"{error}");
  assert!(stats.largest<4096&&stats.total<8192,"{stats:?}");eprintln!("composed pre-cancel: {stats:?}");
 }
}
#[cfg(feature="color-management")]
#[test]
fn composed_non_cancelled_probe_preserves_allocation_bounded_refusal(){
 let input=oversized(true);
 bounded_refusal("composed non-cancelled managed",||wml2::jpegxl::image_from_srgb_with_limits_and_cancel(&input,DecodeLimits::default(),&||false));
}
```

```sh
cargo test -p wml2 --locked --no-default-features \
  --features jpeg-xl,color-management,image-buffer-ex,avif --lib \
  --test highres_icc --test avif_decode --test jpeg_xl \
  --test jpeg_xl_additional --test jpeg_xl_layers --test jpeg_xl_slice_admission
```

## Existing valid-path timing and requested heap

Rust1.91.0 release, root repository dependency lock with `--locked`, Linux
x86_64 AMD EPYC9V74, CPU0. Three alternating baseline/candidate process pairs
for each input/route, three warmups plus150timed calls for small cases or7for
606×500. Source loading, hashing and PPM export are outside timing. Returned
pixels are held through the additional requested-live-heap sample. The counting
System allocator excludes stack, allocator bookkeeping, caller source and
realloc-internal transient overlap; this is not RSS. No concurrent local builds
or tests ran during the benchmark. Both versions use the same committed helper.

| Case/route | Before median µs | After median µs |
| --- | ---: | ---: |
| RGB7×5 ordinary | 43.636 | 43.886 |
| RGB7×5 EX16 | 45.349 | 44.988 |
| RGB7×5 CMS | 153.541 | 151.369 |
| ICC7×5 ordinary | 66.671 | 65.619 |
| ICC7×5 EX16 | 65.689 | 65.339 |
| ICC7×5 CMS | 151.058 | 144.528 |
| 606×500 ordinary | 16547.210 | 17022.776 |
| 606×500 EX16 | 21363.790 | 21465.482 |
| 606×500 CMS | 1096205.732 | 1102718.492 |

These are medians of process medians, not general performance/zero-overhead
claims. The larger ordinary case is nominally2.87% slower, with overlapping
process ranges (before16507.720–17324.241, after17021.836–17300.966µs). EX/CMS
large-case changes are+0.48%/+0.59%; smaller cases also vary. No decoder algorithm
or colour conversion was changed to optimize this measurement.

For606×500, requested-live peaks decrease ordinary6178973→5994632,
EX166453719→6269378 and CMS7391595→7207254 bytes. Every full output hash matches
baseline in all nine case/route combinations. Exported606×500 output is readable
and both source-coordinate and managed previews were visually inspected; managed output is independently checked by the WASM
and native oracle suites. Raw process records, hashes, source/candidate IDs and
WASM results are in [the evidence JSON](jpeg-xl-input-admission.json).

Build baseline and candidate in separate target directories to avoid artifact
reuse across worktrees, then compile the helper against each rlib:

```sh
cargo build -p wml2 --locked --release --no-default-features \
  --features jpeg-xl,image-buffer-ex,color-management --target-dir .test-target
rustc --edition 2024 -C opt-level=3 wml2-test/scripts/jxl_input_admission_bench.rs \
  --extern wml2=.test-target/release/libwml2.rlib \
  -L dependency=.test-target/release/deps -o .test-bench
taskset -c 0 ./.test-bench wml2/tests/fixtures/review/jxl-jbrd-bench.jxl ordinary 7
```

Pass `ordinary`, `ex16` or `cms` and the iteration count as separate arguments
(e.g. `ordinary 7`). For WASM, build the same feature set with
`--target wasm32-unknown-unknown`; compile the fixed-input helper as a cdylib with
that rlib, target dependency directory **and host** release/deps search path.
Run four fresh instances with Node `--wasm-max-mem-pages=2048`, requiring
`oversized_refusal(0..3)==1`. Source/EX verification is reproducible through the
existing `verify_jpegxl_wasm.py`; managed checks reuse `jpegxl_browser.rs` in Node
and do not claim new browser UI execution. Other-platform runtime evidence is
limited to the CI jobs that actually execute relevant tests.
