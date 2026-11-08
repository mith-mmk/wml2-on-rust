# Errors and warnings

This additive API keeps the existing `Box<dyn Error>` return types, callback
signatures, `ImgErrorKind` variants and warning marker trait. No codec, recovery,
cancellation or pixel-decoding policy is changed. It is intended for consumers
such as viewers and painting tools which need diagnostics without parsing text.

## Inspect errors without assuming their concrete type

`ImgError::kind()` returns the producer's stored category. `ImgErrorKind` now
supports `Copy`, `Clone`, equality and hashing. `Error::source()` exposes the same
wrapped error as `get_ref()`; `new_const` still has no source. `into_inner()` is
unchanged. A message passed to `ImgError::new` can itself be an opaque string error,
so having a source does not necessarily provide a more specific category.

```rust
use std::error::Error;
use wml2::error::{ImgError, ImgErrorKind};
use wml2::limits::DecodeLimitExceeded;

fn describe(error: &(dyn Error + 'static)) {
    if let Some(image_error) = error.downcast_ref::<ImgError>() {
        match image_error.kind() {
            ImgErrorKind::UnsupportedFeature | ImgErrorKind::NoSupportFormat => {
                eprintln!("This decoder route does not support the input: {error}");
            }
            _ => eprintln!("{error}"),
        }
    } else {
        // I/O, caller-defined callback errors and other error types remain valid.
        eprintln!("{error}");
    }
    let mut cause = Some(error);
    while let Some(current) = cause {
        if let Some(limit) = current.downcast_ref::<DecodeLimitExceeded>() {
            eprintln!("{}: requested {}, limit {}",
                limit.resource(), limit.requested(), limit.limit());
        }
        cause = current.source();
    }
}
```

Shared resource checks attach `DecodeLimitExceeded` while retaining their legacy
`ImgErrorKind::OutOfMemory` and direct display text. The source carries requested
and maximum counts as `u64`, including input lengths above 4 GiB on 32-bit hosts.
The resource label is human-readable, not a stable identifier. Actual allocation
failures may be a `TryReserveError`, or a codec-specific error, and do not acquire
this source. Absence of this source does **not** prove an actual out-of-memory
condition. No new enum variant is added, preserving existing exhaustive matches.

This is deliberately not a universal classifier:

- Readers, filesystem operations, allocation, integer conversion and callbacks
  can propagate their original error types. Boxed errors are not newly required
  to be `Send + Sync`. Existing `ImgError::new` bounds are unchanged.
- Codec adapters that already stringify causes still do so. `source()` cannot
  reconstruct a cause after it has been erased.
- Some codec-specific bounds use other errors. A shared-check source can also
  disappear through existing translation or recovery, described below.
- `IllegalData`, `DecodeError`, `UnexpectedEof`, and unsupported categories are
  producer classifications, not a consistent whole-file validation verdict.
- Callback `Abort` remains a normal early stop: guarded decodes terminate and
  return `Ok(None)`. It is not newly reported as an error or warning. Legacy
  per-format callback coverage is unchanged. JPEG XL cancellable native APIs
  separately retain the public `jpegxl::DecodeCancelled` error.
- `ImageInfoError`, high-resolution APIs and public low-level codec error types
  remain separate contracts.

## Retain warnings and display them individually

`ImgWarnings::len`, `is_empty` and `iter` borrow the original ordered collection.
The iterator is double-ended and exact-size. `ImgWarning` remains the existing
`Display + Debug` marker trait, including caller implementations; there is no
new severity or common typed-category promise. Legacy collection formatting is
unchanged and concatenates messages without separators.

```rust
use wml2::draw::image_from_with_limits_and_warnings;
use wml2::limits::DecodeLimits;

fn load(bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let (image, warnings) = image_from_with_limits_and_warnings(bytes, DecodeLimits::default())?;
    if let Some(warnings) = warnings {
        for warning in warnings.iter() {
            eprintln!("{warning}");
        }
    }
    // Use image normally, even if the codec reported a recoverable condition.
    Ok(())
}
```

`image_load_with_warnings` provides the same result with default limits.
Existing `image_from`, `image_load`, `image_from_with_limits`, `image_from_file`
and EX image-returning helpers still discard warnings and retain their return
types. Callback entry points (`image_loader`, `image_reader`,
`image_decoder_with_limits`, and EX variants) already return the warning
collection. File/EX consumers can use those with their own image buffer; new
file/EX convenience variants are not added here. `image_writer` currently returns
`Ok(None)` after successful encoding and writing.

No warnings means only that the selected route reported none. It does not mean
all metadata was interpreted or the input was fully validated. Warnings are only
returned on successful/recovered decodes; there is no new partial-output report
on fatal failure.

## Source-level codec audit

This inventory covers all feature-gated codecs in the 0.0.42 baseline. It is a
source review, not a claim that every malformed input or feature combination was
executed. Reader/callback errors can occur in every callback route.

| Codec | Error observations | Warning behavior |
| --- | --- | --- |
| BMP | `ImgError`, raw I/O and allocation errors | Forwards embedded JPEG/PNG warnings |
| GIF | `ImgError`, raw I/O and allocation errors | No native warning generation |
| ICO | `ImgError` and embedded decoder errors | Forwards BMP/PNG warnings |
| JPEG | `ImgError`, I/O and allocation errors | Invalid EXIF, damaged data, unexpected markers/format |
| PNG/APNG | `ImgError`, I/O, helper errors; inflate errors stringified | Post-image chunk-read recovery branch has a warning |
| TIFF | `ImgError`, I/O, allocation and embedded decoder errors | Embedded JPEG warnings; later-page failures become warnings |
| WebP | Adapter stringifies codec errors; low-level APIs retain external types | Invalid EXIF/XMP warnings, raw metadata retained |
| AVIF/AVIF encoder | Adapter mappings, raw I/O, allocation and color errors | No WML2 warning generation |
| PSD | `ImgError`, allocator/reader/callback errors; ZIP errors collapsed | No native warning generation |
| JPEG2000 | `ImgError`, allocator and mapped color errors | No native warning generation |
| JPEG XL | `ImgError`, allocator/conversion/color errors, `DecodeCancelled` | No native warning generation |
| MAG | Validation often uses raw I/O errors | No native warning generation |
| MAKI/PCD/PI/PIC/VSP | Shared retro `ImgError`, reader/conversion/callback errors | No native warning generation |
| PIC2/TGA | `ImgError`, reader/callback errors | No native warning generation |
| PCX/Q4 | `ImgError`, allocation/reader/callback errors | No native warning generation |
| DDS | `ImgError`, reader/callback errors; encoder reserve maps to `EncodeError` | No native warning generation |

Existing places where diagnostics are reduced or omitted, left unchanged:

- `png/decoder.rs::inflate_image` stringifies the shared inflate error; legacy
  frame handling wraps it again. `psd/compression.rs::decompress_planar` replaces failed
  shared inflation with a generic `DecodeError`.
- PNG EXIF parsing in `png/utils.rs` ignores failed structured parsing. WebP EXIF
  parsing in `webp/utils.rs` converts it to a warning. Both retain raw EXIF but
  can lose the typed cause, including a TIFF parser metadata-budget error.
- `tiff/decoder/mod.rs` converts **any** later-page decode failure to a string
  warning, potentially including I/O, callback or limit errors.
- AVIF's compatibility decode adapter applies its stringifying `map_error` to
  errors returned by the dependency, which can include callback errors.
- JPEG XL structured XMP/EXIF extraction may ignore invalid optional metadata
  while retaining indexed raw bytes. It does not report an `ImgWarning` for this.

Changing these recovery or adapter policies needs separate codec-specific tests
and consumer decisions. Consumers should keep a fallback for unclassified
errors and avoid treating warning absence as a clean validation result.

## Consumer integration

Viewer/paintcore changes are separate from this library patch: update their WML2
dependency deliberately, downcast `ImgError` only when present, retain their own
error types, and enumerate warnings instead of splitting display text. A WASM
bridge can serialize explicitly selected diagnostics for its UI without changing
the core callback error type or requiring every caller error to be thread-safe.

## Validation (2026-11-08)

Verified on Windows x86-64 with Rust/Cargo 1.99.0 against baseline
`2a195f6ee18d4b55294be081710cb00533750d6c`:

- `cargo fmt -p wml2 --check`: passed.
- `cargo test -p wml2 --test diagnostics --no-default-features`: 6 passed.
- `cargo test -p wml2 --test diagnostics`: 7 passed.
- `cargo test -p wml2 --test diagnostics --no-default-features --features jpeg,idct_llm`:
  7 passed, including exact pixel equality with recoverable EXIF warnings.
- `cargo test -p wml2 --lib --tests`: 308 passed, 5 ignored, no failures
  across 77 library/integration targets (including feature-gated empty targets).
- `cargo test -p wml2 --lib --tests --profile ci --features image-buffer-ex,high-bit-depth,color-management,jpeg2000,jpeg-xl,avif,avifenc,psd,tga,pcx,dds,pic2,q4`:
  799 passed, 18 ignored, no failures across 89 targets. Ignored tests are
  existing manual benchmarks/display artifacts, external-corpus checks, and
  an opt-in PNG mutation campaign; they were not run.
- `cargo test -p wml2 --doc` and
  `cargo test -p wml2 --doc --no-default-features`: 12 passed each.

The existing `review_bench` example was built in release mode on baseline and
patched source with `--no-default-features --features jpeg,idct_llm`, then run
as `review_bench jpeg 9`. Each case ran nine times; iteration zero was excluded
from the reported median. Baseline then head were measured on the same machine:

| JPEG case | Baseline median | Patched median | Allocations (both) | Allocated bytes (both) | Pixel hash (both) |
| --- | ---: | ---: | ---: | ---: | ---: |
| 16 x 16 | 21,300 ns | 19,950 ns | 154 | 31,101 | 4969434112955904360 |
| 1024 x 768 | 22,632,800 ns | 20,857,700 ns | 73,858 | 18,512,421 | 1543857408883073829 |

No regression was observed in these two samples; this single-machine,
non-randomized measurement is not a general performance claim. Pixel hashes,
allocation counts and allocated bytes matched exactly. Decoder/encoder pixel
arithmetic was not changed. CI completion was intentionally not awaited.
