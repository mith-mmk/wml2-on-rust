# TIFF SampleFormat review fixtures

These six one-row classic TIFFs were written independently of WML2. LibTIFF
4.7.0 `TIFFReadScanline` confirmed the sample type and the native values below.
This oracle result is distinct from the WML2 decoder result: the four non-unsigned
images must be rejected, and the two unsigned images must preserve their bits.

| File suffix | Bits | SampleFormat | Native scanline values |
|---|---:|---:|---|
| signed8 | 8 | 2 | -128, -1, 0, 127 |
| signed16 | 16 | 2 | -32768, -1, 0, 32767 |
| float16 | 16 | 3 | -1, -0, 0, 65504 |
| float32 | 32 | 3 | -1, -0, 0, 1 |
| unsigned16le | 16 | 1 | 32768, 65535, 0, 32767 |
| unsigned16be | 16 | 1 | 32768, 65535, 0, 32767 |

All filenames have the prefix `ex-review-sampleformat-` and suffix `.tif`.
The unsigned fixtures cover both TIFF byte orders; output EX words are little
endian. No compression or FillOrder variation is involved in these fixtures.

Reproduce with a system LibTIFF shared library:

```sh
python3 wml2-test/scripts/ex-review-sampleformat-oracle.py --output-dir .test-sampleformat-oracle
cargo test -p wml2 --no-default-features --features tiff,image-buffer-ex,high-bit-depth --test tiff_ex_sample_format
```

[TIFF 6.0 §19, p.80](https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf)
defines SHORT values, one per channel, with default 1 (unsigned). WML2 also
retains its scalar uniform value convention, like
[LibTIFF 4.7.0](https://github.com/libsdl-org/libtiff/blob/v4.7.0/libtiff/tif_dirread.c#L4400).
Tests cover both representations, tag type/count errors, and a non-unsigned
value in every RGB channel. Signed, floating point and undefined samples are
outside this decoder's unsigned-only profile.
