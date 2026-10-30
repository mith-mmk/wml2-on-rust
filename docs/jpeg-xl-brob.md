# Bounded JPEG XL compressed metadata

The `jpeg-xl` adapter accepts metadata `brob` boxes through its existing still
ordinary/EX/sRGB entry points. It uses published pure-Rust
`brotli-decompressor =6.2.0`, default features disabled, with the streaming API
and custom fallible budgeted allocators. No C library, FFI, unsafe Brotli feature,
external dictionary or new public WML2 API is enabled. `jxl =0.7.4` stays pinned.

The first four payload bytes identify the underlying box type; the remainder
must be one complete standard RFC 7932 Brotli stream. A valid compressed empty
payload is accepted. Missing type/stream, truncation, malformed streams,
nonstandard large-window extensions, trailing bytes and concatenated streams
are rejected. Outer extended-size and final to-EOF boxes use the existing checked
container rules. Metadata may appear before/between/after codestream chunks.

## Type and metadata contract

`Exif`, `xml `, `jumb` and unknown auxiliary types remain **opaque bytes**.
They do not replace the codestream ICC or orientation and are not structured
EXIF/XMP/JUMBF parsing. In particular, raw EXIF orientation must not be applied
to an already-oriented raster. Each outer box retains its original index:

| Metadata key | Raw value |
| --- | --- |
| `JPEG XL Source Box <index> 62726f62` | Exact original `brob` payload: four underlying-type bytes plus compressed stream |
| `JPEG XL Decoded Box <index> <underlying-type-hex>` | Decompressed opaque content only |

Uncompressed source keys remain unchanged. Duplicate types at different indexes
remain separate. Both retained values and both key strings count against the
aggregate metadata quota; decompressed bytes are not silently substituted for
the source bytes. Unknown type bytes are represented as hex, not interpolated
as user-controlled metadata names.

A compressed underlying type starting with `jxl`, or equal to `jbrd` or `brob`,
is forbidden. See the reference implementation's
[`JxlEncoderAddBox` checks](https://github.com/libjxl/libjxl/blob/v0.11.2/lib/jxl/encode.cc#L2484).
The adapter additionally refuses `ftyp` and `JXL*` as unsupported compressed
control types. A wrapper cannot replace the mandatory first outer `ftyp`.
No recursive decompression occurs, including inside opaque JUMBF content.

**Uncompressed `jbrd` retention is unchanged.** A `brob`-wrapped `jbrd` is rejected
before decompression; accepting compressed metadata does not parse reconstruction
payloads, validate opaque JPEG side data or reconstruct a JPEG bitstream.

## Separate resource checks

| Resource | Admission |
| --- | --- |
| Compressed input | Entire input, including brob bytes, obeys `DecodeLimits.input_bytes` and the existing fixed 64 MiB file cap before assembly; retained compressed payloads also count as metadata |
| Expanded and aggregate metadata | Per-stream output is limited to the remaining aggregate `metadata_bytes` after existing data, source payload and both keys; additionally all decoded brob output together has a fixed 64 MiB cap even with caller `unlimited()` |
| Brotli codec heap requests | Shared fallible allocators for u8/u32/Huffman buffers check each request before allocation; their combined live requests have a fixed 32 MiB cap |
| Whole assembly/decompression workspace | `expanded_bytes` checks the existing input assembly, retained metadata/copies, bounded state/stack scratch, codec allocations and output growth together before each new request |
| Container complexity | Existing metadata insertion/index limit, codestream chunk rules and all pixel/frame/internal-layer/profile caps remain enabled |

The decompressor uses a 16 KiB output chunk, allowing one excess byte to detect
an output-quota violation before retaining that byte. Output growth budgets the
new complete request while the old output request is still live, covering a
moving realloc. Codec allocation denial returns an empty allocation through the
dependency's fallible interface, then surfaces as a decode-limit error rather
than being mislabeled malformed compressed data. The constructor's allocation
status is checked before processing input. Real reserve failure is an allocation
error. Invalid/truncated streams retain decode-error classification.

Assembly reserves three input lengths and twice the pending retained metadata;
state construction/moves, chunk and control overhead are also reserved before
starting Brotli. Dynamic codec/output requests share the remaining workspace.
After Brotli state is dropped, existing preflight rechecks all retained metadata
with the normal pixel-decoding workspace estimate. The fixed expansion cap is
separate from the input cap, rather than being implicitly changed by a future
input-policy adjustment. This does not change the resource-policy work in #37.

These are bounds on admitted/requested buffers and a conservative workspace
estimate, **not process RSS**, allocator bookkeeping or a general bound on every
codec/stack implementation detail. Non-brob inputs do not construct Brotli state.
No cancellation API is added here; the independent #32 contract identifies
container assembly/preflight as non-interruptible work.

All container/decompression errors are discovered before image initialization,
draw or metadata publication. Temporary partially decoded metadata is dropped;
it is never returned as successful output. Existing begin-image lifecycle and
callback Abort handling are unchanged.

## Independent fixtures and verification

[Fixture provenance](../wml2/tests/fixtures/review/jxl-brob-fixtures.json) records
C libbrotli 1.1.0 encoding (quality 5, window 22) and C libjxl 0.11.2 validation.
The generator independently decodes both compressed/plain containers through
libjxl with box decompression enabled, checking every effective metadata payload
and all RGBA8 pixels against the existing `djxl` oracle. Empty, XML, EXIF,
repeated 65,659-byte and incompressible 16,415-byte streams exercise distinct
paths. A C-encoded window-25 extension is independently rejected by the C strict
decoder and is a negative fixture. No WML2/Rust Brotli encoder creates an oracle.

```console
python3 wml2-test/scripts/generate_jpegxl_brob_fixtures.py
cargo test -p wml2 --locked --no-default-features --features jpeg-xl,image-buffer-ex,color-management --lib --test jpeg_xl --test jpeg_xl_additional --test jpeg_xl_brob --test jpeg_xl_layers
cargo test -p wml2 --locked --no-default-features --test jpeg_xl --test jpeg_xl_brob
python3 wml2-test/scripts/verify_jpegxl_wasm.py --output .test-jxl-brob-wasm
```

The generator requires development C libbrotli/libjxl shared libraries; these
are not runtime dependencies. Native tests cover exact output/metadata limits,
workspace denials across constructor/decoder/output growth, mutated headers,
forbidden types, all truncated prefixes of the XML stream, placement/duplicates/
chunking, and no publication on failure. A compact compressed input expanding
past the cumulative 64 MiB cap is rejected with caller `unlimited()`.
The Node harness adds eight actual WASM brob checks (69 total) for pixels,
raw/decoded metadata, EX precision, input/metadata/workspace rejection, truncation,
forbidden reconstruction type and trailing bytes. Prior 61-case results remain
historical; the new run must be identified by its tested source and artifact hash.

## Local validation checkpoint (2026-10-30)

On Linux x86_64 cloud / Rust 1.91.0, code checkpoint
`773007296fbb878d6ad468ed3a501cd366523bb6` passes 63 scoped native tests:
29 unit, 22 original JPEG XL, 3 additional, 6 brob and 3 layer tests. Feature-off
passes its detection test without compiling the optional Brotli dependency.
Scoped Clippy completes with 11 existing library warnings and no new brob-test
warning. Formatting, local documentation links and Python syntax are checked.
The [recorded evidence](jpeg-xl-brob-validation.json) includes exact source and
fixture hashes and the actual 69-case Node WASM artifact hash (no host imports).
These are new local runs, separate from earlier JPEG XL historical records.

The 606x500 JPEG-derived bench input was also wrapped with the independently
encoded XML metadata. Managed output is within 1/255 of the saved independent
libjxl/LittleCMS reference for every RGBA byte. Its exported PNG was visually
inspected without corruption. No external viewer/plugin was exercised.

Release comparison uses unchanged main runtime `6a48db9` and the checkpoint
above, three alternating process pairs on CPU 0 with no concurrent local
builds/tests. Small cases use 150 timed iterations, the ordinary bench 33,
and the managed bench 5, plus 3 warmups each. Values are medians of process
medians; all output hashes match. The managed large case's small sample count
and these finite inputs do not establish general throughput.

| Existing path | Before µs | After µs | Change |
| --- | ---: | ---: | ---: |
| RGBA16 ordinary | 55.879 | 53.701 | -3.90% |
| 606x500 ordinary | 16168.812 | 15800.076 | -2.28% |
| ICC lossless sRGB | 139.556 | 135.390 | -2.99% |
| 606x500 sRGB | 1096988.440 | 1097700.257 | +0.06% |

No material existing-path change is observed in this local sample. For the new
7x5 metadata fixture, one candidate process measured 106.791 µs with brob and
79.751 µs with uncompressed metadata (150 timed + 3 warmups). This includes both
extra raw retention and decompression, not just codec time. The old decoder did
not support brob, so no old brob success or speedup is claimed. Workspace
admission and allocator-denial tests do not measure whole-process RSS.
