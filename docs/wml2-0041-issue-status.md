# WML2 0.0.41 acceptance and remaining work

Reconciled on 2026-10-31 against published main
`f2151b2408e9d04f1bc7b4e9e29679e078a57209`. This map separates shipping
implementation from unmerged evaluation reports. The release is described in
[0.0.41 notes](releases/0.0.41.md). No runtime, dependency or version change is
part of this documentation update.

## Completed acceptance scopes

The following issues have implementation and regression evidence for their
stated initial scopes. Closing them does not imply broader format support.

| Issue | Shipped behavior and acceptance evidence | Explicit limits |
| --- | --- | --- |
| [#31](https://github.com/mith-mmk/wml2-on-rust/issues/31) | `jpegxl::image_from_srgb_with_limits_and_cancel`, typed `DecodeCancelled`, independent ICC/alpha output and every-checkpoint no-publication tests in [jpeg_xl_additional.rs](../wml2/tests/jpeg_xl_additional.rs); composition tests in [composition.rs](../wml2/tests/jpeg_xl/composition.rs) | Cooperative safe points; preflight, allocations, serial work, each runner task and ICC compilation remain non-interruptible; no latency bound |
| [#33](https://github.com/mith-mmk/wml2-on-rust/issues/33) | Static PNG EX Gray8/16 and RGBA8/16; independent png-crate decoding verifies lower bits, transparent RGB/alpha, odd stride, filters, ICC and Exact/Round in [png_encode_ex.rs](../wml2/tests/png_encode_ex.rs) | No EX animation or 12-bit PNG output; explicit precision conversion remains required |
| [#34](https://github.com/mith-mmk/wml2-on-rust/issues/34) | Single-page Gray16/RGBA16 EX TIFF, raw/LZW/Deflate and horizontal prediction; independent tiff-crate pixel/tag/ICC/alpha checks across endian/Classic/BigTIFF in [tiff_encode_ex.rs](../wml2/tests/tiff_encode_ex.rs) | Initial single-page/single-strip straight-alpha subset; legacy 8-bit APIs remain separate |
| [#35](https://github.com/mith-mmk/wml2-on-rust/issues/35) | Enabled JPEG, PNG/APNG, GIF, TIFF/BigTIFF, PSD v1 and dimension-only JXL basic queries; [image_info.rs](../wml2/tests/image_info.rs) forbids payload reads and checks malformed/truncated streams, unknown counts and restored reader positions | Unsupported formats return explicit errors; JXL sample/color/alpha and unqueried page/layer counts remain Unknown; basic info is not whole-file validation |
| [#36](https://github.com/mith-mmk/wml2-on-rust/issues/36) | Baseline JPEG Half/Quarter/Eighth reduced IDCT; [jpeg_scaled.rs](../wml2/tests/jpeg_scaled.rs) checks independent libjpeg pixels, subsampling, odd geometry, restarts, malformed data and allocation limits | Initial 8-bit Gray/interleaved RGB/YCbCr subset; no progressive, multi-scan color, CMYK or full-decode fallback. Existing real-image timing/requested-heap/RSS evidence is in [jpeg-scaled.md](jpeg-scaled.md) |
| [#38](https://github.com/mith-mmk/wml2-on-rust/issues/38) | `util::encoder_supports_format(Avif)` follows `avifenc`; [avif_encode.rs](../wml2/tests/avif_encode.rs) asserts capability/extensions and actually saves/decodes; [avif_feature_off.rs](../wml2/tests/avif_feature_off.rs) checks decoder-only refusal | The decoder feature alone does not enable saving |
| [#41](https://github.com/mith-mmk/wml2-on-rust/issues/41) | Bounded Brotli auxiliary expansion with exact outer source payload retention; [jpeg_xl_brob.rs](../wml2/tests/jpeg_xl_brob.rs) checks plain/compressed independent pixels, duplicates/chunking, malformed/trailing data, forbidden control boxes and cumulative budgets | Fixed cumulative expansion cap remains under caller unlimited; no recursive control-box expansion |
| [#45](https://github.com/mith-mmk/wml2-on-rust/issues/45) | Distinct coded/output/intrinsic geometry with all eight orientations, no resampling; [intrinsic.rs](../wml2/tests/jpeg_xl/intrinsic.rs) and [geometry query tests](../wml2/tests/image_info/jpegxl.rs) verify independent pixels, oversized sizes, malformed prefixes and actual-pixel budgets | Intrinsic dimensions remain bounded metadata; query success does not admit the later image |
| [#50](https://github.com/mith-mmk/wml2-on-rust/issues/50) | Common EXIF/XMP plus indexed raw boxes; [jpeg_xl_metadata.rs](../wml2/tests/jpeg_xl_metadata.rs) checks endian/offsets, first-valid semantics, malformed payloads, XML refusal/complexity, budgets and orientation | Common EXIF requires `exif`; raw remains with it disabled. No XML external resolution; codestream orientation wins and common EXIF Orientation becomes 1 |

## Partial and evaluated work

| Tracking | Product state after 0.0.41 | Remaining acceptance |
| --- | --- | --- |
| [#42](https://github.com/mith-mmk/wml2-on-rust/issues/42) | Mixed-depth full-resolution straight integer alpha is shipped. The original issue's broader acceptance is incomplete and is reopened | Associated/subsampled alpha normalization; typed retention/selection of non-alpha or multiple extras, dimensions, precision and cumulative budgets. [Refusal regressions](../wml2/tests/jpeg_xl/alpha.rs) intentionally reject these; [core evaluation](jpeg-xl-alpha.md) is not implementation |
| [#44](https://github.com/mith-mmk/wml2-on-rust/issues/44) | Bounded terminal-visible preview skipping returns the main image; stays open | Explicit preview extraction API, separate geometry/layout/color/alpha and individual/combined resource contract. [Preview tests](../wml2/tests/jpeg_xl/preview.rs) validate main pixels, structural bounds and opaque skipped entropy, not extracted preview pixels |
| [#37 / PR #63](https://github.com/mith-mmk/wml2-on-rust/pull/63) | Resource evaluation only; existing hard caps remain | Allocation-aware larger-image admission, cumulative internal/reference/entropy/metadata/CMS accounting and finite configuration. 2560x1440 is still refused by the padded sample screen; a trusted-core probe is not production admission |
| [#40 / PR #54](https://github.com/mith-mmk/wml2-on-rust/pull/54) | jxl 0.8 evaluation only; runtime stays pinned to 0.7.4 | Qualified pre-allocation ICC/internal-layer/TOC admission with the changed/private parser API |
| [#43 / PR #67](https://github.com/mith-mmk/wml2-on-rust/pull/67) | Animation oracle/transport evaluation only | Production companion API, timing/publication, cumulative frame/reference/snapshot admission and real cancellation/Abort/late-error tests |
| [#46 / PR #68](https://github.com/mith-mmk/wml2-on-rust/pull/68) | Native float/HDR evaluation only | F32/precision/alpha/finite-sample storage contract, bounded admission and separately qualified tone mapping |
| [#47 / PR #69](https://github.com/mith-mmk/wml2-on-rust/pull/69) | Progressive resumption/provisional output evaluation only | Incremental preflight, accepted-input accounting, acknowledged snapshots and sealed-EOF publication |
| [#48 / PR #60](https://github.com/mith-mmk/wml2-on-rust/pull/60) | Backend feasibility report; no original-JPEG reconstruction API | Bounded jbrd/frame reconstruction and original-byte/hash identity qualification |
| [#49 / PR #61](https://github.com/mith-mmk/wml2-on-rust/pull/61) | Candidate encoder evaluated; no JXL save support | Candidate 16-bit-alpha mismatches, one-pixel refusal and panic cases exclude direct adoption; pre-allocation resource/cancellation contracts also remain |
| [#39 / PR #51](https://github.com/mith-mmk/wml2-on-rust/pull/51) | Documentation correction remains draft until merged | This reconciliation updates the existing author's branch by ordinary fast-forward push, preserving its original commit and merging published main into it |

The fixed JXL caps remain active with `DecodeLimits::unlimited()`. PR #66's
borrowed-input admission is already shipped and removes the input-sized copy
before recognized-JXL rejection; it does not solve internal reconstruction
workspace. Main-image preview skipping is separate from progressive snapshots.

## Replayed native evidence

Linux x86_64 managed cloud, Rust 1.91.0, production source at the published main
SHA above. Isolated source/target directories were used. These 131 passing test
invocations replay existing frozen independent oracle fixtures; three manual
benchmark cases were skipped. There was no new independent-oracle generation,
browser/WASM/Windows execution, decoder change, display artifact or benchmark in
this documentation stage. Historical measurements retain their own revisions.

| Feature selection / targets | Passed | Manual skipped |
| --- | ---: | ---: |
| `jpeg-xl,color-management,image-buffer-ex,high-bit-depth,tiff,png,jpeg,idct_llm,gif,psd`: `image_info`, `jpeg_scaled`, `jpeg_xl`, `jpeg_xl_additional`, `jpeg_xl_brob`, `jpeg_xl_metadata`, `jpeg_xl_layers`, `jpeg_xl_slice_admission`, `png_encode_ex` | 108 | 2 |
| `tiff,image-buffer-ex,high-bit-depth`: `tiff_encode_ex` | 6 | 1 |
| `avifenc`: `avif_encode::avif_encoder_round_trips_through_wml2_decoder` | 1 | 0 |
| `avif` without encoder: `avif_feature_off` | 1 | 0 |
| No default features: `avif_feature_off`, `image_info`, `jpeg_xl` | 3 | 0 |
| `jpeg-xl` without `exif`: `jpeg_xl_brob`, `jpeg_xl_metadata` | 12 | 0 |

Reproduce using `CARGO_TARGET_DIR=.test-target cargo test -p wml2
--no-default-features --features <selection> --test <target>` for each row.
Omit `--features` for the disabled row. The AVIF encoder row selects the named
test after `--test avif_encode`; it does not require the separate FFmpeg tests.

## Next resource-contract implementation

Start #37 from published main, separately from the evaluation branches. Reuse
#63's pinned 2048x1536 and 2560x1440 evidence as reproduction inputs. Distinguish
output pixels, actual padded color/extra sample planes, internal/reference
retention, entropy/TOC state, metadata/ICC, conversion buffers and collector
storage. Share a checked cumulative admission plan before codec allocation,
with finite caller-tightened settings and versioned diagnostics. Estimates,
allocator requests and process/linear-memory ceilings must stay distinct.

Do not simply raise MAX_PIXELS or remove the 4/3 sample screen. Qualify the
replacement against independent pixels, exact/one-under cumulative budgets,
malformed TOCs/ICC/layers, allocation failure, existing cancellation precedence,
32-bit arithmetic and WASM memory ceilings before enabling a larger canvas.
If upstream allocations cannot be admitted conservatively, obtain a bounded
allocator/parser contract or keep the affected mode refused. No universal RSS
bound or cancellation-latency guarantee follows from sample counts.

## Separate TIFF follow-up

[Draft PR #71](https://github.com/mith-mmk/wml2-on-rust/pull/71) follows 0.0.41;
its TIFF compatibility changes are not treated as released main behavior here.
The public smallliz sample and generated RGB missing-tag pattern were verified
there. The private `XING_T24.tiff` file remains unavailable and untested.
