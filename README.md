Notice: Specification of this library is not decision.

# WML2 - Web graphic Multi format Library To Rust
- Rust writing graphic loader for WASM
- only on memory use
- callback system
- not use multithreding
- No need to force use - becouse You can use Javascript Image.

# run example
See `wml2-test/examples`
## decode bmp reader
```
$ cargo run -p wml2-test --example to_bmp --release -- <inputfile> <output_dir>
```
## metadata reader

```
$ cargo run -p wml2-test --example metadata --release -- <inputfile>
```

## converter

```
$ cargo run -p wml2-test --example converter -- <inputfiles...> -o <output_dir> [-f png|jpeg|bmp|webp] [-q <quality>] [-z <0-9>] [--split]
```

# Support Format 0.0.18

|format|enc|dec|  |
|------|---|---|--|
|BMP|O|O|encode only no compress|
|JPEG|O|O|Baseline encode / Baseline and huffman progressive decode|
|GIF|O|O|with Animation GIF|
|PNG|O|O|encode Truecolor + alpha only|
|TIFF|O|O|encode: no compression/LZW/JPEG(new), decode: no compression/LZW/PackBits/JPEG(new)/Adobe Deflate/CCITT Huffman RLE/CCITT Group 3 Fax/CCITT Group 4 Fax|
|WEBP|O|O|pure Rust still/animated decoder, still/animated encoder, lossless/lossy output|
|MAG|x|O|Japanese legasy image format|
|MAKI|x|O|Japanese legacy image format, disabled by `noretoro`|
|PI|x|O|Japanese legacy image format, disabled by `noretoro`|
|PIC|x|O|Japanese legacy image format, disabled by `noretoro`|
|PCD|x|O|Photo CD base4 decode, disabled by `noretoro`|

# Features

- default: retro decoders are enabled
- `noretoro`: disable legacy decoders (`MAKI`, `PI`, `PIC`, `VSP/DAT`, `PCD`)

```toml
[dependencies]
wml2 = "0.0.18"
```

```toml
[dependencies]
wml2 = { version = "0.0.18", features = ["noretoro"] }
```

# Test samples

- integration tests use generic filenames such as `sample.mki`, `sample.pi`, `sample.pic`, `sample.dat`
- the original sample filenames are intentionally not referenced in public-facing test code
- optional external sample paths can be configured in `wml2/tests/test_samples.txt`
- `wml2/tests/test_samples.txt` is ignored by git; use `wml2/tests/test_samples.example.txt` as the template

# Encode options

- JPEG: `quality`
- WebP: `optimize` (`0..=9`) and `quality` (`0..=100`, lossy only)
- `draw::image_to()` encodes an `ImageBuffer` directly into a `Vec<u8>`
- `draw::convert()` selects the encoder from the output extension, including `.webp`
- `wml2-test/examples/converter` supports `-z` for WebP optimize and `--split` for PNG/WebP animation frame export

# TIFF

- Encode supports these TIFF compression formats:
  - no compression
  - LZW
  - JPEG (new-style TIFF JPEG, RGB only)
- Decode supports these TIFF compression formats:
  - no compression
  - LZW
  - PackBits
  - JPEG (new-style TIFF JPEG)
  - Adobe Deflate
  - CCITT Huffman RLE
  - CCITT Group 3 Fax
  - CCITT Group 4 Fax

# using loader
- an on-memory compress buffered image or an image file 
- output memory buffer and callback (defalt use ImageBuffer)

```rust
  let image = new ImageBuffer();
  let verbose = 0;
  let mut option = DecodeOptions{
    debug_flag: verbose,  // depended decoder
    drawer: image,
  };

  let r = wml2::jpeg::decoder::decode(data, &mut option);
```

## Symple Reader

```rust
use std::error::Error;
use wml2::draw::ImageBuffer;

pub fn main()-> Result<(),Box<dyn Error>> {
    println!("read");
    let filename = "foo.bmp";
    let mut image = image_from_file(filename.to_string())?;
    let _metadata = image.metadata()?.unwrap();

    Ok(())
}


```
 ImageBuffer impl DrawCallback trait, 5 function.

 - init -> callback initialize.Decoder return width and height (and more...).You must get buffers or resours for use.
 - draw -> callback draw.Decoder return a part of image.You must draw or other process.
 - verbose -> callback verbose. for debug use
 - next -> If the image has more frame or rewrite(ex.progressive jpeg),next function return value DrawNextOptions,
    - Continue,             -- continue image draw 
    - NextImage,            -- this image has other images
    - ClearNext,            -- You may next image draw before clear
    - WaitTime(usize),      -- You may wait time(ms)
    - None,                 -- none option
    - and other...(no impl)
   You may impl animation,slice images and more.

 - terminate -> You can impl terminate process
 - set_metadata -> 0.0.10 after 

# using saver
```rust
use wml2::draw::{ImageBuffer, image_to};
use wml2::util::ImageFormat;

let mut image = ImageBuffer::from_buffer(1, 1, vec![255, 0, 0, 255]);
let png = image_to(&mut image, ImageFormat::Png, None)?;
```

Use `draw::image_encoder()` when you want to encode a custom `PickCallback`
implementation instead of `ImageBuffer`.

 ImageBuffer encoder impl PickCallback trait, 4 function.

- encode_start encoder start
- encode_pick  encoder pick image data from Image Buffer
- encode_end   terminate encode
- metadata // 0.0.10 after

# Metadata
 Metadatas is had by (Key Value) HashMap.
 Metadata key is String.value is into DataMap.

```rust

// Get ICC Profile
    let filename = "foo.bmp";
    let mut image = image_from_file(filename.to_string())?;
    let metadata = image.metadata()?.unwrap();

    if let Some(icc_profile_data) = metadata.get(&"ICC Profile") {
      if let DataMap::ICCProfile(icc_profile) = icc_profile_data {
        // Read ICC Profile
      }
    }

  

```

# debug flag
## JPEG
-  0x01 basic header
-  0x02 with Huffman Table
-  0x04 with Extract Huffman Table 
-  0x08 with Define Quatization Table
-  0x10 with Exif
-  0x20 with IIC Profile(header)
-  0x40 with IIC Profile(more infomation)
-  0x60 with IIC Profile(all)
-  0x80 ...
# update
- 0.0.1 baseline jpeg
- 0.0.2 bmp OS2/Windows RGB/RLE4/RLE8/bit fields/baseline JPEG
- 0.0.3 add GIF
- 0.0.4 change error message
- 0.0.5 reader change/Error delagation change
- 0.0.6 issue RST maker read bug fix
- 0.0.7 add PNG
- 0.0.8 add Jpeg multithread(pipelined),Progressive Jpeg has bugs(4,1,1) / BMP saver / Animation GIF(alpha)
- 0.0.9 Png Saver
- 0.0.10 Progressive Bug(4,1,1) fix
- 0.0.11  2022/05/25 fix
  - obsolete ICCProfile parse in verbose -> use metadata - see https://github.com/mith-mmk/icc_profile Tiff G3 Fax
  - TIFF 3G/4G FAX and multi page tiff decode support, Tiled image is support,but new Jpeg Tiff only.
- 0.0.12 encode option change
- 0.0.13 MAG Format supppot
- 0.0.14 add MAKI/PI/PIC/VSP(DAT)/PCD decoders, add `noretoro` feature to disable legacy decoders
- 0.0.15 add JPEG encoder(only baseline)
- 0.0.16 add pure rust Webp decoder, APNG encoder
- 0.0.17 add pure rust Webp encoder / animated WebP encode / converter WebP options
- 0.0.18 add GIF encoder / add TIFF encoder / add Exif writer

# todo
- Formated Header writer
- other decoder
- color translation

#　License
 MIT License (C) 2022-2026

# Author
 MITH@mmk https://mith-mmk.github.io/
