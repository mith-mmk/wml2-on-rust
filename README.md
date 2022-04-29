Notice: Specification of this library is not decision.

# WML2 - Web graphic Multi format Library To Rust
- Rust writing graphic loader for WASM
- only on memory use
- callback system
- not use multithreding
- No need to force use - becouse You can use Javascript Image.

# run example
See wml-test/examples
## decode bmp reader
$ cargo run --example to_bmp --release <inputfile> <output_dir>

## metadata reader
$ cargo run --example metadata --release <inputfile>


# Support Format 0.0.10

|format|enc|dec|  |
|------|---|---|--|
|BMP|O|O|encode only no compress|
|JPEG|x|O|Baseline and huffman progressive|
|GIF|x|O|with Animation GIF|
|PNG|O|O|APNG not supprt/encode Truecolor + alpha only|
|TIFF|x|o|no compression/LZW/Packbits/Jpeg(new)|
|WEBP|x|x|not support|

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
  let image = new ImageBuffer(width,height);

  if let Some(image) = image {
      let option = EncodeOptions {
          debug_flag: 0,
          drawer: &mut image,    
      };
      let data = wml2::bmp::encoder(option);
      if let Ok(data) = data {
          let filename = format!("{}.bmp",filename);
          let f = File::create(&filename).unwrap();
          f.write_all(data).unwrap();
          f.flush().unwrap();
      }
  }
```

 ImageBuffer encoder impl PickCallback trait, 4 function.

- encode_start encoder start
- encode_pick  encoder pick image data from Image Buffer
- encode_end   terminate encode
- set_metadata // 0.0.10 after

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

# todo
- TIFF
- animation image buffer
- ICCProfile Reader
- Formated Header writer
- other decoder
- color translation
- encoder
