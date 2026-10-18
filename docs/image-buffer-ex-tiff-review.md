# ImageBufferEx TIFF review follow-up

The FillOrder finding is reproduced and fixed in the shared unsigned native
TIFF path. The SampleFormat finding is not reproduced: the existing TIFF page
preflight already rejects signed, floating-point and malformed declarations
before any EX callback. This follow-up starts from `a25cb29` on the independent
EX continuation branch. Runtime changes are committed at `93ebfe1`; the
measurement helper and clean WASM execution checkpoint is `637dec1`.

## SampleFormat

The public EX call path is `image_decoder_ex → decode_ex_inner → Tiff::new →
page::read_pages → init_ex → decode_unsigned_samples`. The page parser checks
SHORT type, nonzero count, count of one or SamplesPerPixel, and every value
equal to unsigned (1). The decoder-local absence of another check is therefore
not a public bypass. At the exact reviewed commit
[`a25cb29e653bb968824cd4e6671464448eba430a`](https://github.com/mith-mmk/wml2-on-rust/commit/a25cb29e653bb968824cd4e6671464448eba430a),
[`ex.rs:15`](https://github.com/mith-mmk/wml2-on-rust/blob/a25cb29e653bb968824cd4e6671464448eba430a/wml2/src/tiff/decoder/ex.rs#L15)
calls Tiff::new before init_ex at line65 or sample reading at line78;
[`header.rs:454–455`](https://github.com/mith-mmk/wml2-on-rust/blob/a25cb29e653bb968824cd4e6671464448eba430a/wml2/src/tiff/header.rs#L454)
delegates to read_pages. The actual non-unsigned rejection is
[`page.rs:184–196`](https://github.com/mith-mmk/wml2-on-rust/blob/a25cb29e653bb968824cd4e6671464448eba430a/wml2/src/tiff/page.rs#L184),
with SHORT type checked at lines112–117. The review's decoder-local argument
misses this earlier shared page-validation position. The five baseline source
hashes in the independent review were rechecked against `git show` of this
full commit, so the evidence concerns the same source SHA.

No SampleFormat runtime change was needed. Comments and
four regression groups document the shared preflight contract.

[TIFF 6.0, section 19, page 80](https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf)
defines unsigned as the default and distinguishes signed, IEEE float and
undefined samples. Although its nominal count is SamplesPerPixel,
[LibTIFF 4.7.0's parser](https://github.com/libsdl-org/libtiff/blob/v4.7.0/libtiff/tif_dirread.c)
also accepts the scalar uniform vendor convention. The existing compatibility
is retained.

The [independent baseline review](image-buffer-ex-tiff-sampleformat-review.json)
records 24 generated unsigned accepts and 66 generated rejects, plus six
LibTIFF scanline oracle fixtures (signed8/16, float16/32 and unsigned16 LE/BE).
Rejected inputs invoke zero EX callbacks; both public native highres entry
points also reject them. Budget scope restoration is checked. Baseline source
hashes in this record describe the pre-fix review, not later comment changes.
The [baseline test log](image-buffer-ex-tiff-sampleformat-baseline.log) records
four passing groups at a25: the signed8, signed16 and float16 independent
fixtures all return `TIFF non-unsigned SampleFormat is unsupported` and zero
callbacks through the same public EX entry point. Float32 is also rejected;
unsigned16 LE/BE are accepted with exact pixels. These are equivalent
independently generated fixtures, not the six unavailable original ZIP samples;
this evidence does not fully prove the original supplied review wrong.

## FillOrder

The old native path treated stored FillOrder2 bytes as canonical bytes. Raw
samples were wrong; PackBits controls, LZW codes and Deflate streams could be
misread or rejected. This is an inherited shared-native defect affecting EX
and highres unsigned8/16 decoding.

`decompress_native_block` now reverses each stored byte in place before the
selected raw/PackBits/LZW/Deflate decompressor. Predictor and sample endian
conversion follow decompression. Endianness and FillOrder remain separate.
This follows [LibTIFF's read boundary](https://github.com/libsdl-org/libtiff/blob/v4.7.0/libtiff/tif_read.c)
and its codec flags. Legacy RGBA8/packed decoding, JPEG and CCITT retain their
existing behavior. Unknown/unsupported codecs are rejected without mutating
their input. FillOrder tag validation itself already existed.

The existing natural-byte old-LSB LZW convention needs explicit compatibility:
its prefix can collide with a canonical bit-reversed old-LSB stream. The
normalized stream is decoded strictly first. Where the raw prefix also matches
the compatibility convention, the original-byte candidate is checked with the
existing strict early/late-width handling, exact output size and EOI validation.
Different successful results are rejected as ambiguous. If the normalized
result is live, the three-candidate expanded-byte budget is checked before the
original-byte candidates allocate. A confirmed minimal stream with two valid
but different results was not found; that rejection branch was reviewed
statically. Canonical-prefix collision and exact candidate-budget boundaries
are covered dynamically.

The [independent FillOrder review](image-buffer-ex-tiff-fillorder-review.json)
records baseline failures and the final eight passing regression groups.
[Fixture manifest](../wml2/tests/fixtures/ex-review-fill-order.json) contains
54 LibTIFF4.7.0 canonical oracle cases: 48 combinations of depth8/16, LE/BE,
FillOrder1/2, raw/PackBits/LZW/Deflate and applicable predictor1/2, plus six
canonical old-LSB prefix cases. Eight additional natural-LSB compatibility
conditions are separate from canonical oracle claims. Invalid tags and
highres callers are also tested. LibTIFF is an oracle only; Rust production
decoding adds no external codec dependency.

## Executed checks

All local regression gates use Rust/Cargo1.91.0, offline dependency resolution
and the release profile where applicable. [Aggregate record](image-buffer-ex-tiff-validation.json)
contains source and log hashes.

| Configuration | Passed | Failed | Existing ignored |
| --- | ---: | ---: | ---: |
| Workspace, EX/high-bit-depth/optional retro codecs, 50 targets | 408 | 0 | 13 |
| Minimal EX, no default codecs, 32 targets | 70 | 0 | 0 |
| Minimal feature-off, 32 targets | 39 | 0 | 0 |
| TIFF/high-bit-depth without EX, 7 targets | 91 | 0 | 2 |
| Focused library and two new TIFF test targets | 56 | 0 | 0 |

Format and diff checks pass. Clippy all-targets passes with inherited warnings;
this is not a `-Dwarnings` gate. No dependency, default feature, public API
signature or WML2 0.0.35 version change is introduced.

[Actual WASM execution](image-buffer-ex-tiff-wasm.json) passes 82 groups from
a clean tree at `637dec1`, using Nodev24.19.0 with no host imports. It includes
54 canonical FillOrder fixtures, six SampleFormat fixtures, eight compatibility
conditions and candidate-budget restoration, in addition to the original
13 EX foundation groups. The resulting linear memory is1,572,864bytes for this
bounded suite; this is not a total-memory promise for other inputs.

The public CLI exported [54 native RGBA16 images](image-buffer-ex-tiff-native-export.json).
Every source sample matches exactly (Gray8 uses the defined257 scaling;
Gray16 retains all low bits; alpha is opaque). New review-image viewing was
explicitly unnecessary for this task; numeric checks are recorded and the
previous EX display evidence remains available. Other OS, browser UI and the
complete CI matrix were not executed.

## Paired performance evidence

[Paired measurements](image-buffer-ex-tiff-benchmark.json) compare immutable
a25/current release binaries with the same measurement helper, fixed CPU,
B/C/C/B/C/B, three processes per version, ten warmups and1,000timed calls per
process. Cases are existing FillOrder1 paths: native EX Gray16 LZW, Gray8
PackBits, RGBA16 raw, and legacy Gray16 LZW. Every output digest and allocation
byte count matches. FillOrder2 has no valid baseline output to compare; its
correctness is covered by oracle/pixel tests.

| Case | Baseline median µs | Current median µs | Baseline/current allocated bytes |
| --- | ---: | ---: | ---: |
| EX Gray16 LZW | 218.797 | 220.442 | 247,938 /247,938 |
| EX Gray8 PackBits | 37.995 | 37.941 | 26,954 /26,954 |
| EX RGBA16 raw | 131.192 | 131.450 | 175,803 /175,803 |
| Legacy Gray16 LZW | 203.527 | 191.737 | 247,322 /247,322 |

Means show substantial shared-host variation, including a5.65% PackBits mean
increase with essentially unchanged median and one slower process. These
samples show no consistent slowdown; they do not establish guaranteed
speedups or all-workload throughput. All24,240raw rows are retained in each
linked compressed artifact. The [initial run](image-buffer-ex-tiff-benchmark-initial.json)
is retained separately: it briefly overlapped native export work and is not
used as the final isolated timing result.

## Supplied archive and boundaries

The supplied `wml2_ex_review_a25cb29.zip` was located through Library, but the
official download helper failed with `hosted apps tools/list request failed:
network`. No archive bytes were acquired. Its original six tests/fixtures
are therefore not claimed executed. The parent authorized proceeding with
independent reproduction, oracle fixtures and public workspace tests.

Only this EX continuation branch is saved through normal Git push. Main,
JPEG2000 PR#10 and the old EX branch are preserved. No GitHub-app write retry,
permission/configuration change, merge, release, version change, new PR or CI
wait is part of this follow-up.

```sh
cargo test --workspace --offline --release --features wml2/image-buffer-ex,wml2/high-bit-depth,wml2/tga,wml2/pcx,wml2/dds,wml2/q4
cargo test -p wml2 --offline --release --no-default-features --features image-buffer-ex
cargo test -p wml2 --offline --release --no-default-features
cargo test -p wml2 --offline --release --no-default-features --features tiff,image-buffer-ex,high-bit-depth --lib --test tiff_ex_fill_order --test tiff_ex_sample_format
python3 wml2-test/scripts/verify_image_buffer_ex_wasm.py --offline --output .test-ex-tiff-wasm
cargo run -p wml2 --offline --release --features image-buffer-ex,high-bit-depth,png,tiff --example review_bench -- decode-ex-file 1010 wml2/tests/fixtures/ex-gray16-lzw.tif
```
