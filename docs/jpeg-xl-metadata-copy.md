# Fallible JPEG XL adapter metadata copies

This small implementation stage depends on draft [#72](https://github.com/mith-mmk/wml2-on-rust/pull/72), base `9ce284030367f5812b26d256b9efbedbd89d01b6`. It preserves the 0.0.41 API and all existing fixed resource ceilings. It does not admit 2560×1440 images or establish a codec-wide heap/RSS bound.

## Implementation and boundary

Raw source/reconstruction/output ICC payloads and the retained source-header bytes now use `try_reserve_exact` before copying. The managed output's fixed sRGB ICC uses the same path. An allocation failure returns `TryReserveError` before any image or metadata is published. Output-profile selection borrows the codec profile; only the bounded simple Gray-to-RGB color-encoding value needs ownership. Arbitrary Gray ICC expansion still requires the existing explicit CMS path.

For the finite resource APIs introduced in #72, a provisional ledger admits each retained key plus payload before requesting its raw copy. It checks cumulative container-plus-added metadata and `preflight_workspace + 2 * added_metadata`, using checked arithmetic. Failed claims leave the subtotal unchanged. After all metadata is assembled, the final retained-metadata estimate replaces this subtotal before the output/CMS ledger starts: the payloads are not charged twice. Legacy APIs retain their existing quota ordering while gaining fallible raw copies.

The codec's simple-profile factory still allocates internally. Its returned retained size is accounted before subsequent copies, but the factory itself has not become fallible. Small metadata keys, metadata vector growth, codec state and all dependency allocations are outside this patch's fallible-copy claim. The previous conservative preflight estimates and refusal checks remain in force.

## Validation

The accompanying [validation snapshot](jpeg-xl-metadata-copy-validation.json) records the local checks and measurements. The source-level tests inject actual allocator null returns immediately before each selected raw copy: three legacy callback sites and four managed owned-API sites. They assert `TryReserveError`, no published canvas/buffer/metadata, and a successful subsequent decode. Separate tests verify cumulative quota denial before a copy is requested, checked overflow, failed-claim rollback, accounting handoff and pointer-identical borrowing.

Native JXL coverage passed 131 tests; the minimal JXL build passed 43 and the disabled-feature build passed one. The inherited WASM verifier passed 12 case groups under each of 32 MiB and 64 MiB linear-memory ceilings, including actual CMS and cancellation. Local i686-musl validation is compilation only: this host cannot execute ELF32. CI supplies the executable Windows 32-bit coverage.

A 606×500 managed decode was displayed and compared with the independent frozen sRGB oracle: maximum RGB error 1, alpha exact. Three alternating release benchmark process pairs used two warmups and eight measured decodes each, including pixel hashing. All hashes matched #72. Median time ratios ranged 0.973–1.037; this short run shows ordinary noise, not a statistically established speedup. Raw ICC cases remove one allocation: 588 bytes on the small ICC fixture, 1,049,176 bytes on a generated large-profile case, and 2,712 bytes on the photo. Peak requested heap is unchanged on the large-profile case and 2,712 bytes lower on the photo; these are allocator request metrics, not RSS bounds.

The optional large-profile benchmark is a 7×5 constant RGB(64,128,192) image encoded with independent C libjxl 0.11.2. Starting from the frozen gamma2 ICC, append a private `zzzz` tag of 1 MiB (`text`, four zero bytes, ASCII x padding, terminal zero), inserting a 12-byte tag-table entry and adjusting existing offsets and declared profile length. This scratch input is measurement evidence, not a new compatibility fixture. Its profile and codestream hashes are in the snapshot.

## Next dependency stage

No dependency is changed here. Before extending admission, a qualified shared-budget implementation needs these concrete paths:

| Area | Remaining allocation work |
| --- | --- |
| jxl 0.7.4 entropy context maps | Replace infallible `Vec::with_capacity`, `vec!`, `HashSet` and `collect` paths with admitted fallible storage. |
| Entropy SymbolReader/LZ77 window | The window constructor is already fallible, but needs the same live-byte budget as other decoder storage. |
| Frame/modular processing | Account group, transform, palette and scratch buffers, growth overlap, clones and reference-frame lifetimes. |
| Float/render planes, sections and TOC | Tie admission and release to actual simultaneous ownership, including section buffering and reference layers. |
| Preflight and generated readers | WML2 directly uses FileHeader, Histograms and IncrementalTocReader before constructing the decoder. Decoder options alone cannot cover them; inspect jxl_macros 0.7.4 generated dynamic readers too. |
| Metadata/ICC/CMS | Preserve bounded brob and ICC expansion, integrate retained profiles with parsing and the existing icc-profile 0.0.7 compiler budget, and avoid counting traffic as live storage. |

A first dependency patch could introduce an optional shared budget and connect entropy context maps plus the LZ77 window, while keeping all current caps. Every reservation must precede growth, use checked byte arithmetic, handle allocation refusal and release only after the last owner dies. Reference sharing must not cause undercounting or duplicate release. Parallel code needs safe shared accounting; 32-bit and WASM need executable boundary tests. Cancellation and existing error precedence must remain stable. A product-wide global allocator or thread-local test injector is not a substitute. Only after all applicable paths are qualified can padded large-image admission be reconsidered.
