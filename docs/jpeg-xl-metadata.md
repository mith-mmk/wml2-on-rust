# Structured JPEG XL EXIF and XMP

The existing ordinary, EX and explicit sRGB APIs interpret bounded `Exif` and
`xml ` auxiliary payloads, including the decoded contents of `brob` boxes. All
original indexed Source/Decoded Raw records remain available unchanged. The
pinned `jxl =0.7.4`, image APIs, pixel behaviour and existing limits remain.

| Common key | Type and selection |
| --- | --- |
| `EXIF` | Existing `DataMap::Exif(TiffHeaders)`, only with `exif` enabled; first valid Exif payload in file order |
| `XMP` | Existing `DataMap::Ascii(String)` containing the exact UTF-8 packet, with `jpeg-xl`; first valid supported XMP packet in file order |

A brob's effective decoded record has the original box's position. Compressed
source bytes are never parsed as TIFF/XML. Later records remain raw even after
selection. A malformed or unsupported optional packet stays raw and the next
candidate is considered. Resource/allocation failures are fatal before image
initialization, draw or metadata publication, rather than silently becoming a
successful raw-only decode. Existing begin-image/Abort behaviour is unchanged.
This is not the header-only image information API and adds no dimension API.

## EXIF and orientation

Read the box's first four bytes as a big-endian offset **from the end of that
four-byte field** to the TIFF header. Checked offsets and bounded TIFF slices
use the existing Classic TIFF/BigTIFF directory parser, including byte order,
value extents, tag types, cycle and nested-IFD checks. The independently generated
Classic TIFF fixtures cover both endiannesses, a nonzero offset, malformed and
truncated input, duplicate boxes and brob wrapping. Supported TIFF values use
the existing DataPack contract; this does not promise interpretation of maker
notes or every EXIF schema extension.

JXL codestream orientation remains authoritative for pixels. In the common
EXIF primary directory, every Orientation tag is normalized to SHORT 1 (added
if missing). All other parsed values are retained under existing TIFF semantics.
Original EXIF orientation remains in indexed Raw, and original coded JXL
orientation remains in `JPEG XL Source Orientation`. `orientation_applied=true`
continues to mean consumers must not apply source orientation again. No EXIF
rotation or pixel transform is added. Other source tags, such as dimensions,
are not rewritten to describe the oriented output.

## UTF-8 XML/XMP scope

Use pinned pure-Rust `roxmltree =0.21.2` with default features disabled. Accept
well-formed UTF-8 XML 1.0 (optional BOM/declaration; declared UTF-8 or ASCII when
actually ASCII). Recognized document roots are `xmpmeta`/`xapmeta` in
`adobe:ns:meta/`, or `RDF` in the W3C RDF namespace. Namespace prefixes are not
significant. The returned String preserves the original bytes: entities are
validated, not rewritten in the packet. This is packet extraction, not an RDF
property map, semantic XMP validation or metadata editing/encoding API.

DTD/DOCTYPE, unknown entities, XInclude elements, and processing instructions
other than `xpacket` are unsupported. Other XML roots and declared encodings
stay raw; no transcoding. The parser has no entity resolver or I/O hook and
never performs file/network access. Namespace URIs and RDF resource attributes
are data and never fetched. Predefined/numeric entities and ordinary CDATA are
supported. XMP size/complexity failures remain fatal even for `unlimited()`.

## Admission and retained structures

The existing raw/decompressed metadata and pixel/profile workspaces are admitted
first. Additional parser work is checked before constructing either parser:

- XML input: fixed 4 MiB maximum, parser nodes limit 65,536. Retained String and
  key count against remaining `metadata_bytes`. Temporary admission is
  `128 * input_bytes + 64 KiB + 4 * ("xmlns" occurrences + 1) * ("<" bytes + 1)`.
  The final term conservatively reserves doubled u16 namespace-inheritance
  indexes; repeated declarations/descendants cannot bypass the estimate. The
  initial node/attribute capacity guesses happen before the parser's node limit,
  so that limit alone is not used as an allocation bound.
- EXIF parser charges directory-entry structures, every materialized field
  payload (including aliased offsets), and value-range records before allocation.
  Its charged metadata is bounded by the minimum of fixed 8 MiB, one quarter of
  remaining retained metadata after 4 KiB overhead/key, and one sixteenth of
  remaining workspace after 64 KiB overhead. A separate fixed maximum of 64
  primary IFDs bounds empty-directory overhead; this does not change the image
  frame limit. Sub-IFD policies remain those of the shared TIFF reader.
- Before materializing TiffHeaders, reserve `4 * parser_charge + 4 KiB` of
  retained metadata and `16 * parser_charge + 64 KiB` temporary workspace.
  After materialization, count TiffHeaders, actual directory Vec capacities and
  every owned DataPack buffer/String capacity. Structured EXIF is never priced
  as an eight-byte scalar. Each retained addition contributes its key and value
  to metadata and twice that amount to combined workspace for callback copies.

These are conservative known-buffer/request admission rules, not process RSS,
allocator-overhead or arbitrary downstream callback allocation guarantees.
Malformed data that first exceeds a resource budget is a resource failure. After
first valid selection, later candidates are retained raw but not interpreted.
No security caps are relaxed and no cancellation API is added in this branch.

## Reproduce validation

```sh
cargo test -p wml2 --locked --no-default-features \
  --features jpeg-xl,exif,image-buffer-ex,color-management \
  --lib --test jpeg_xl --test jpeg_xl_additional --test jpeg_xl_brob \
  --test jpeg_xl_metadata --test jpeg_xl_layers
python -B wml2-test/scripts/generate_jpegxl_metadata_fixtures.py
python -B wml2-test/scripts/verify_jpegxl_wasm.py --output .test-jxl-meta-wasm
python -B wml2-test/scripts/verify_jpegxl_wasm.py --exif --output .test-jxl-meta-wasm-exif
```

Fixture provenance is in `wml2/tests/fixtures/review/jxl-meta-fixtures.json`:
Pillow verifies EXIF values; Python ElementTree verifies XMP namespaces/UTF-8;
C libbrotli generates compressed streams; C libjxl checks every decompressed
metadata payload and all RGBA8 bytes against the existing independent djxl
reference. Conflicting-EXIF tests for all eight JXL orientations use the saved
independent djxl pixel references. No decoder-self comparison is a colour oracle.
Tests verify resource failures leave ImageBuffer uninitialized and unpublished.
See [machine-readable evidence](jpeg-xl-metadata-validation.json) for exact
commands, counts, source hashes, WASM and performance/display checks.

## Local results and performance limits (2026-10-30)

Rust 1.91.0 / Linux x86_64: expanded native matrix 78 passed, minimal JXL
without EXIF/EX/CMS 59 passed, disabled-feature detection 1 passed. The two
pre-admission unit tests check parser-call counters stay zero on denial. Actual
Node 24.19.0 WASM passes 73 cases with `exif` and 73 without it, with no host
imports. The five generated containers include a brob-wrapped malformed-then-
valid sequence; the four well-formed variants also run in WASM. CI now runs
brob/structured metadata on Linux compatible-feature profiles, macOS, Windows,
32-bit Windows, and both actual WASM feature configurations.

Scoped Clippy output equals the parent branch's 35 existing warnings after
normalizing paths/build lines; no new helper/test diagnostic. The 35 repository
CI selection/release tests also pass. No C oracle library is a runtime dependency.
A 606x500 bench with independently compressed metadata matches every saved
reference RGBA byte within 1/255; the PNG export was visually inspected without
corruption. That source's coded orientation is preserved.

Release measurements compare parent #56 `b3c52a5` against the candidate with the
same `jpeg-xl,exif,image-buffer-ex,color-management` features, resident input,
three alternating process pairs pinned to CPU 0, three warmups per process.
Small cases use 150 timed iterations; large ordinary/managed use 40/5. Values
below are medians of process medians, not disk I/O or general throughput.

| Case/route | Parent µs | Candidate µs |
| --- | ---: | ---: |
| RGBA16 / ordinary | 53.872 | 55.048 |
| Small ICC / sRGB | 141.604 | 150.803 |
| 606x500 / ordinary | 15917.163 | 15588.156 |
| 606x500 / sRGB | 1074989.414 | 1048978.536 |
| New plain EXIF/XMP / ordinary | 81.013 | 92.520 |
| New brob EXIF/XMP / ordinary | 99.876 | 113.607 |

Every full-buffer hash matches. The new packet interpretation adds about
11.5/13.7 µs in these tiny fixtures. Legacy small-case measurements are unstable:
a five-pair, 1000-iteration CPU-0 recheck produced RGBA16 53.641→53.651 µs and
ICC 138.574→192.976 µs, while the equivalent CPU-2 recheck produced
86.536→81.533 µs and 195.014→150.668 µs. Individual before/after process medians
span both timing clusters. Both rechecks, including unfavourable results, are
retained in the evidence. These runs cannot isolate a reliable small-path speed
change; they are not evidence for a general speedup or guaranteed zero overhead.
The first recheck's baseline Clippy run overlapped its beginning, an additional
reason not to infer a throughput regression from that sample. Large managed
results also have only five timed iterations per process. No RSS claim is made.
