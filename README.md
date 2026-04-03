[![crates.io](https://img.shields.io/crates/v/wml2)](https://crates.io/crates/wml2) ![license](https://img.shields.io/crates/l/wml2)
[Japanese](./README.ja.md)

# WML2 - Web graphic Multi format Library To Rust

`wml2` is a callback-based image I/O library for Rust.

- decodes and encodes images in memory
- works with the built-in RGBA `ImageBuffer` or custom callbacks
- supports still images and selected animation / multi-page formats
- includes native file helpers on non-WASM targets

## Examples

See `wml2-test/examples`.

### Decode to BMP

```console
$ cargo run -p wml2-test --example to_bmp --release -- <inputfile> <output_dir>
```

### Read metadata

```console
$ cargo run -p wml2-test --example metadata --release -- <inputfile>
```

### Convert formats

```console
$ cargo run -p wml2-test --example converter -- <inputfiles...> -o <output_dir> [-f gif|png|jpeg|bmp|tiff|webp] [-q <quality>] [-z <0-9>] [-c <none|lzw|lzw_msb|lzw_lsb|jpeg|lossy|lossless>] [--exif copy] [--split]
```

## Supported formats (`0.0.19`)

| format | enc | dec | notes |
| --- | --- | --- | --- |
| BMP | O | O | encoder writes uncompressed BMP |
| JPEG | O | O | baseline encoder; baseline and Huffman progressive decoder |
| GIF | O | O | palette/LZW encoder, animation supported |
| ICO | x | O | decoder for BMP/PNG embedded icon images |
| PNG | O | O | PNG/APNG; encoder writes RGBA truecolor |
| TIFF | O | O | encode: none/LZW/JPEG(new); decode: none/LZW/PackBits/JPEG(new)/Adobe Deflate/CCITT Huffman RLE/CCITT Group 3/4 Fax |
| WEBP | O | O | pure Rust still/animated decoder and still/animated encoder; lossless/lossy output |
| MAG | x | O | Japanese legacy image format, disabled by `noretoro` |
| MAKI | x | O | Japanese legacy image format, disabled by `noretoro` |
| PI | x | O | Japanese legacy image format, disabled by `noretoro` |
| PIC | x | O | Japanese legacy image format, disabled by `noretoro` |
| VSP/DAT | x | O | Japanese legacy image format/container, disabled by `noretoro` |
| PCD | x | O | Photo CD base4 decode, disabled by `noretoro` |

## Features

- `default`: enables the standard decoders/encoders, EXIF support, embedded-format bridges, and `idct_llm`
- format features: `bmp`, `gif`, `ico`, `jpeg`, `png`, `tiff`, `webp`, `mag`, `maki`, `pcd`, `pi`, `pic`, `vsp`
- metadata feature: `exif`
- embedded-format bridge features: `bmp-jpeg`, `bmp-png`, `tiff-jpeg`, `ico-bmp`, `ico-png`
- JPEG IDCT features: choose exactly one of `idct_llm` (default), `idct_aan`, or `idct_slower`
- JPEG encoder toggle: `fdct_slower`
- miscellaneous toggles: `multithread`, `SJIS`, `noretoro`
- `noretoro`: disables all retro format decoders gated by it: `MAG`, `MAKI`, `PCD`, `PI`, `PIC`, and `VSP/DAT`

```toml
[dependencies]
wml2 = "0.0.19"
```

```toml
[dependencies]
wml2 = { version = "0.0.19", features = ["noretoro"] }
```

```toml
[dependencies]
wml2 = { version = "0.0.19", default-features = false, features = ["jpeg", "png", "exif", "idct_aan"] }
```

## Encode and convert options

`draw::image_to()` encodes an `ImageBuffer` directly into a `Vec<u8>`.

`draw::convert()` chooses the encoder from the output extension:
`.gif`, `.png`, `.apng`, `.jpg`, `.jpeg`, `.bmp`, `.tif`, `.tiff`, `.webp`.

Supported option keys in `EncodeOptions::options` / `draw::convert(..., options)`:

- JPEG: `quality`
- TIFF: `compression = none|lzw|lzw_msb|lzw_lsb|jpeg`
- TIFF with `compression=jpeg`: `quality`
- WebP: `optimize` (`0..=9`)
- WebP lossy: `quality`
- PNG/JPEG/TIFF/WebP: `exif`
  - raw EXIF bytes via `DataMap::Raw`
  - TIFF-style EXIF via `DataMap::Exif`
  - `DataMap::Ascii("copy".to_string())` to preserve decoded EXIF during `convert()`

`wml2-test/examples/converter` options:

- `-q`: JPEG quality, WebP lossy quality, or TIFF JPEG quality
- `-z`: WebP optimize level
- `-c`: TIFF compression or WebP `lossy|lossless`
- `--exif copy`: copy source EXIF to PNG/JPEG/TIFF/WebP output
- `--split`: split animation / multi-frame output for GIF/PNG/TIFF/WebP

## TIFF compression details

Encode supports:

- no compression
- LZW
- JPEG (new-style TIFF JPEG, RGB only)

Decode supports:

- no compression
- LZW
- PackBits
- JPEG (new-style TIFF JPEG)
- Adobe Deflate
- CCITT Huffman RLE
- CCITT Group 3 Fax
- CCITT Group 4 Fax

## Basic decoding

For simple in-memory decoding, use `draw::image_load()`. For native file I/O,
use `draw::image_from_file()`.

```rust
use std::error::Error;
use wml2::draw::{PickCallback, image_from_file};

fn main() -> Result<(), Box<dyn Error>> {
    let mut image = image_from_file("foo.webp".to_string())?;
    println!("{}x{}", image.width, image.height);

    if let Some(metadata) = image.metadata()? {
        if let Some(format) = metadata.get("Format") {
            println!("format: {:?}", format);
        }
    }

    Ok(())
}
```

Use `draw::image_loader()` or `draw::image_reader()` when you want decoders to
write into a custom `DrawCallback`.

`ImageBuffer` implements `DrawCallback` and receives:

- `init`: initialize the canvas
- `draw`: write RGBA pixels into a rectangle
- `next`: receive `NextOptions` for animation or multi-image transitions
- `terminate`: finalize decoding
- `verbose`: decoder-specific debug output
- `set_metadata`: decoded metadata

## Basic encoding

Use `draw::image_to()` for `ImageBuffer`, or `draw::image_encoder()` /
`draw::image_writer()` when you want to encode a custom `PickCallback`.

```rust
use std::collections::HashMap;
use std::error::Error;
use wml2::draw::{ImageBuffer, image_to};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

fn main() -> Result<(), Box<dyn Error>> {
    let mut image = ImageBuffer::from_buffer(1, 1, vec![255, 0, 0, 255]);
    let mut options = HashMap::new();
    options.insert("quality".to_string(), DataMap::UInt(90));

    let jpeg = image_to(&mut image, ImageFormat::Jpeg, Some(options))?;
    assert!(jpeg.starts_with(&[0xff, 0xd8]));
    Ok(())
}
```

`ImageBuffer` implements `PickCallback` and provides:

- `encode_start`
- `encode_pick`
- `encode_end`
- `metadata`

## Metadata

Metadata is stored as `HashMap<String, DataMap>`.

```rust
use std::error::Error;
use wml2::draw::{PickCallback, image_from_file};
use wml2::metadata::DataMap;

fn main() -> Result<(), Box<dyn Error>> {
    let mut image = image_from_file("foo.jpg".to_string())?;
    let metadata = image.metadata()?.unwrap_or_default();

    if let Some(DataMap::ICCProfile(profile)) = metadata.get("ICC Profile") {
        println!("ICC profile: {} bytes", profile.len());
    }

    Ok(())
}
```

## Test samples

- integration tests use generic names such as `sample.mki`, `sample.pi`, `sample.pic`, `sample.dat`
- original sample filenames are intentionally not referenced in public test code
- optional external sample paths can be configured in `wml2/tests/test_samples.txt`
- `wml2/tests/test_samples.txt` is ignored by git; use `wml2/tests/test_samples.example.txt` as a template

## Debug flags

### JPEG

- `0x01`: basic header
- `0x02`: Huffman table
- `0x04`: extracted Huffman table
- `0x08`: quantization table
- `0x10`: EXIF
- `0x20`: ICC profile header
- `0x40`: ICC profile detail
- `0x60`: ICC profile all

## Release history

- `0.0.1`: baseline JPEG
- `0.0.2`: BMP OS/2 and Windows RGB/RLE4/RLE8/bit fields, baseline JPEG
- `0.0.3`: GIF decoder
- `0.0.4`: error message updates
- `0.0.5`: reader and error propagation changes
- `0.0.6`: RST marker read bug fix
- `0.0.7`: PNG decoder
- `0.0.8`: pipelined JPEG, BMP saver, animation GIF work
- `0.0.9`: PNG saver
- `0.0.10`: progressive JPEG fix
- `0.0.11`: TIFF Group 3/4 Fax and multipage TIFF decode improvements
- `0.0.12`: encode option changes
- `0.0.13`: MAG decoder
- `0.0.14`: MAKI/PI/PIC/VSP(DAT)/PCD decoders and `noretoro`
- `0.0.15`: baseline JPEG encoder
- `0.0.16`: pure Rust WebP decoder and APNG encoder
- `0.0.17`: pure Rust WebP encoder and animated WebP encode
- `0.0.18`: GIF encoder, TIFF encoder, EXIF writer
- `0.0.19`: ICO decoder, feature restructuring, and LL&M as the default JPEG decoder IDCT


## License

MIT License (C) 2022-2026

## Author

MITH@mmk https://mith-mmk.github.io/
