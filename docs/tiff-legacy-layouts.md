# Bounded legacy TIFF layouts

This change follows published 0.0.41, main
`f2151b2408e9d04f1bc7b4e9e29679e078a57209`. It fixes two independently
reproduced compatibility failures without changing public APIs or limits.

## Complete old JPEG interchange

Some Compression=6 files store a complete JPEG at JPEGInterchangeFormat.
Their strips point back into that JPEG rather than supplying additional bytes.
The previous assembler required the first strip to begin with SOS and appended
it, duplicating bytes already present in the complete image.

The bounded interchange reader now walks marker lengths from SOI to SOS. When
the interchange ends in EOI and contains SOS, it is decoded once. Every strip
must have a nonzero count, fit the input and lie within the complete scan region
(including its SOS header, excluding EOI). Tiles and outside ranges reject.
Header-only interchange, with or without EOI, still appends SOS-led strips.
The nested JPEG decoder retains its pixel and expanded-byte checks; the
interchange read remains subject to the assembly limit before allocation.

The public `smallliz.tif` at the existing pinned
[codec-corpus revision](https://github.com/imazen/codec-corpus/tree/8e10d4d765667c1c49d74413878fc4bfb46dcf8d/tiff-conformance)
has interchange offset/length 8/4608 and strip offset/count 610/3447.
SOS is at 599 and EOI at 4614. The strip begins inside the SOS header and covers
only part of the entropy data. Its bytes must not be appended or treated as the
authoritative JPEG length. The corpus attribution identifies libtiffpic/CC0;
the binary remains external. Its SHA256 and size are pinned in the
[interoperability manifest](../wml2/tests/fixtures/tiff_interop/manifest.json).
This public file was actually tested in the cloud; identity with any private
NERO file is not claimed.

## Missing raw RGB strip counts

Infer `width * height * 3` only for a single uncompressed, contiguous RGB8 strip,
with three 8-bit components, Predictor=1 and absent/full-height RowsPerStrip.
Checked arithmetic rejects overflow. Existing range, geometry, allocation and
decode limits remain enforced. EOF, subsequent offsets and trailing directories
are never used to guess the length. Compressed, planar, multiple-strip,
subsampled and partial-height cases retain rejection when counts are absent.
Explicit counts retain their existing validation.

`XING_T24.tiff` was not provided to this environment. Its reported missing-tag
pattern is covered by generated files; its actual pixels are not verified.

## Validation

Both new positive reproductions fail on isolated 0.0.41 sources with their
reported errors and pass with this change. Independent target directories avoid
reusing build artifacts between revisions.

- TIFF/JPEG/EX and library regression targets cover complete and header-only
  interchange, valid table forms, truncation, bad marker lengths, out-of-input
  and out-of-interchange ranges, zero counts, arithmetic overflow, nested JPEG
  pixel limits and existing allocation limits.
- Generated 240x164 gradients with explicit and omitted counts/RowsPerStrip,
  in both byte orders, match Pillow and prescribed RGB bytes exactly.
- Public smallliz decodes to 160x160 RGBA, with maximum channel difference 3,
  mean difference 0.34548828125 against Pillow 12.3.0; alpha is exact.
- All seven pinned external corpus hashes pass. Existing old-JPEG, YCbCr and
  unsupported-CCITT behavior passes the external integration tests.
- Output PNGs were visually inspected in the cloud. No private corpus or
  downstream viewer validation is claimed.

[Recorded oracle and benchmark data](tiff-legacy-layouts-validation.json) uses
Rust 1.91.0 on Linux x86_64. Six alternating release process pairs each use five
warmups and 100 timed decodes. Baseline and changed pixel hashes, allocation
counts, total requested bytes and requested live peaks match on all three
existing accepted files:

| File | Baseline median | Changed median | Change |
| --- | ---: | ---: | ---: |
| RGB8 explicit counts 240x164 | 0.501 ms | 0.510 ms | +1.8% |
| Old JPEG Chewey multi-strip | 1.985 ms | 1.963 ms | -1.1% |
| Old JPEG single-strip | 1.317 ms | 1.367 ms | +3.8% |

These are local snapshots with small timing variation, not a speedup or a
universal memory bound. Newly accepted layouts have no successful baseline
decode to compare.

## Reproduction

Use `.test`-prefixed temporary/build directories per AGENTS.md. With the pinned
external corpus and Pillow installed:

```sh
CARGO_TARGET_DIR=.test-target cargo build -p wml2-test --release --example to_png
python3 wml2-test/scripts/verify_tiff_legacy_layouts.py \
  --converter .test-target/release/examples/to_png \
  --output .test-tiff-legacy --smallliz "$WML2_TIFF_CORPUS/smallliz.tif"
CARGO_TARGET_DIR=.test-target cargo test -p wml2 --no-default-features \
  --features tiff-jpeg,idct_llm,image-buffer-ex,high-bit-depth \
  --lib --test tiff_jpeg_extend --test tiff_review_regressions
CARGO_TARGET_DIR=.test-target cargo test -p wml2 --no-default-features \
  --features tiff-jpeg,idct_llm,image-buffer-ex --test tiff_interop -- --ignored
```

The existing TIFF oracle CI job runs the independent verifier with the pinned
corpus. Generated Rust regressions also run without external files on the
existing feature/platform matrix.
