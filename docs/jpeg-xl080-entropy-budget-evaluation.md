# JPEG XL 0.8 entropy budget evaluation

This is an isolated evaluation for #37 / #63, based on draft #95
(`721025a0ccbd19faeaaf3b68a5bcc8d7d4132e26`). It does not change WML2's
product manifest, lockfile, dependencies, fixed caps or accepted canvas sizes.
2560×1440 is still refused. No decoder-wide recoverable-heap guarantee is made.

## Source and reproduction

The published `jxl` 0.8.0 archive SHA256 is
`b8379fc83d79477900c9aed78cb8370a556d834081bebcb771c6010381c4e230`.
The [validation snapshot](jpeg-xl080-entropy-budget-validation.json) pins the
patch and each original/result source file. The BSD license accompanies the
patch. The replay verifies all hashes before building a separate local crate;
its two frozen dependency graphs use only registry versions/checksums already
in the product lockfile.

```sh
rustup target add wasm32-unknown-unknown
python3 -B wml2-test/scripts/evaluate_jxl080_entropy_budget.py \
  --output .test-entropy080 --wasm --wml-regression
```

Use `--offline` with cached dependencies. Native ownership/failure tests always
run; WASM and the 65 existing WML regressions are optional switches. The latter
exercise the patched codec's **unbudgeted legacy path**, not product quota
propagation. CI runs native, compile-fail and actual WASM checks in one stable
JPEG XL feature job. CI completion is reported separately.

## Ownership contract

The evaluation adapts the bounded context-map primitives explored for 0.7.4 in
#74, but implements 0.8's different reusable LZ77 scratch lifetime explicitly.
A feature-gated facade exposes otherwise private low-level types only in the
isolated evaluation. No public upstream budget-injection API currently exists.

A shared atomic ledger counts requested context-map and LZ77 payload capacities.
Reservations precede fallible allocation, reject overflow, and roll back on
failure. Growth admits old plus replacement storage simultaneously. Leases are
non-cloneable and remain charged until their storage is deallocated; cloning a
budget or bit reader does not create an independent quota. `BudgetedHistograms`
provides fallible resize and no mutable dereference to the legacy resize API.

Budgeted LZ77 readers preallocate a 4 MiB ring because the existing scalar read
API cannot report later allocation failure. This can reject earlier or reserve
more than lazy legacy decoding. Cached scratch retains its charge after a
reader finishes. Reuse moves both buffer and lease; finishing consumes the
reader, checks final input/ANS state, and publishes scratch only on success.
Cross-budget reuse is rejected. Replacement frees the old cached allocation
before releasing its reservation. Raw legacy scratch reclamation cannot export
a budgeted window, and a naked legacy Vec is never silently adopted as charged.
The unbudgeted legacy signatures and lazy allocation behavior are preserved.

This requested-payload ledger excludes allocator rounding/overhead, setup's Arc
allocation, ANS/Huffman tables and configuration vectors, verification HashSets,
frame/modular/reference storage, TOC, metadata/ICC/CMS and the static palette
cache. Legacy entry points and WML's actual decode still do not propagate it.
It is not an RSS bound or proof that the whole decoder handles OOM recoverably.

## Local evidence

On Linux x86_64, Rust 1.91.0: 10 native tests and two expected compile failures
passed. Tests cover shared/cloned readers, nested maps with and without LZ77
and MTF, resize overlap, multiple live windows and cached reuse, mismatched
roots, malformed/final-state failure, real allocator-null injection and
rollback, deallocation ordering, eight-thread shared admission, and ring wrap.
Compile failures prevent legacy resize through the budgeted wrapper and reader
reuse after consuming finish.

Actual wasm32 execution under Node 24.19.0 passed six groups, with no imports,
a 64 MiB linear-memory ceiling and explicit growth rejection. The snapshot
records the module hash and memory size. Native i686 execution and upstream
unit tests were not run; the registry archive lacks upstream test resources.
The isolated WML consumer passed 65 existing pixel/ICC/metadata/timing,
resource and cancellation regressions using the unbudgeted legacy route.

Two 7×5 fixtures, ordinary and managed output, were compared with identical
release features/build settings: three alternating paired blocks of 100 samples
per case, three warmups per process. Patched/base median ratios were 0.897,
0.960, 0.990 and 1.010; block medians and variation are in the snapshot. These
small-fixture measurements show no consistent slowdown here, and do not
qualify broader inputs or the cost of full quota propagation. All paired output
bytes matched. The managed ICC fixture matched the independent C sRGB oracle
exactly, including alpha; a magnified side-by-side display was inspected.

## Remaining integration work

Product integration requires an upstream/public API or an explicitly chosen
dependency distribution strategy, then propagation through all codec entry
points and actual simultaneously live storage. No external fork or upstream
write is included. The 0.7.4 preflight parser remains necessary for private
ICC/TOC/layer inspection; removing it requires suitable public APIs rather than
copying private parsers or dropping safety checks.

Frame/modular/reference buffers, entropy tables, TOC and metadata/color paths
still need ownership and fallible-allocation coverage before increasing the
resource envelope. Existing finite adapter estimates and first-use palette
reservation remain in force. #37 / #63 remain open; this evaluation is not a
product implementation or completion of those issues.
