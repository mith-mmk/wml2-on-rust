# JPEG EXIF parsing and optional metadata failures

JPEG APP1 parsing checks the complete `Exif\0\0` signature and passes only the
bounded TIFF payload to the EXIF reader. TIFF offsets start at its byte-order
marker, independent of the JPEG stream position.

The EXIF reader has one compatibility exception: a metadata sub-IFD's trailing
next-IFD slot may overlap an explicitly referenced out-of-line tag value. In
that case the value bytes are not interpreted as an IFD link. Some writers omit
that slot and place resolution or other values immediately after the entries.
This is limited to EXIF metadata sub-IFDs; regular TIFF image parsing remains
strict. Non-overlapping nonzero links, nested metadata pointers, cycles, and
out-of-bounds values are still rejected by the EXIF parser.

If bounded optional EXIF content is invalid or unsupported, JPEG decoding keeps
the raw APP1 bytes as `EXIF Raw`, returns an `InvalidExif` warning through the
callback decode APIs, and continues to decode the image. Valid EXIF remains
`DataMap::Exif`, including Orientation values 1 through 8 for callers that apply
display orientation. WML2 itself continues to return encoded-coordinate pixels.
Convenience APIs that discard decoder warnings still retain the raw metadata.

Segment-length/read failures, resource/allocation limits, callbacks and JPEG
coding errors are not handled by this fallback. EXIF payloads and parsed
allocations count against the aggregate metadata budget. The public EXIF parser
still returns errors for other malformed or unsupported structures; tolerance
of optional EXIF failures belongs specifically to the JPEG APP1 boundary.

`wml2/tests/jpeg_exif.rs` generates synthetic pixels and EXIF in both byte
orders. It checks overlapping value/footer layouts, retained Orientation 1–8,
unchanged decoded pixels, malformed signatures and TIFF headers, cyclic/nested
and out-of-bounds directories, resource limits, and fatal JPEG structure errors.
No user image or extracted private payload is included.

```sh
cargo test --locked -p wml2 --no-default-features --features jpeg,idct_llm --test jpeg_exif
cargo test --locked -p wml2 --lib --tests
```
