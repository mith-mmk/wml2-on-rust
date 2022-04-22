Notice: Specification of this library is not decision.

# WML2 - Web graphic Multi format Library To Rust
- Rust writing graphic loader for WASM
- only on memory use
- callback system
- not use multithreding
- No need to force use - becouse You can use Javascript Image.

# Support Format 0.0.9

|format|enc|dec|  |
|------|---|---|--|
|BMP|O|O|encode only no compress|
|JPEG|x|O|Baseline and huffman progressive|
|GIF|x|O|with Animation GIF|
|PNG|O|O|APNG not supprt/encode Truecolor + alpha only|
|TIFF|x|x|header reader only|
|WEBP|x|x|not support|

# using loader
- on memory compress image buffer (now only baseline jpg)
- output memory buffer and callback (defalt use ImageBuffer)

```rust
  let image = new ImageBuffer();
  let verbose = 0;
  let mut option = DecodeOptions{
    debug_flag: verbose,
    drawer: image,
  };

  let r = wml2::jpeg::decoder::decode(data, &mut option);
```

Symple Reader

```rust
use std::io::BufReader;
use std::error::Error;
use wml2::draw::DecodeOptions;
use wml2::draw::*;
use wml2::draw::ImageBuffer;
use wml2::draw::CallbackResponse;

use std::fs::File;
let f = File::open(file)?;
let reader = BufReader::new(f);

/* Callback verbose info */
fn write_log(str: &str) -> Result<Option<CallbackResponse>,Box<dyn Error>> {
    println!("{}", str);
    Ok(None)
}

pub fn main()-> Result<(),Box<dyn Error>> {
    println!("read");
    let f = File::open("foo.bmp")?;
    let reader = BufReader::new(f);
    let mut image = ImageBuffer::new();
    image.set_verbose(write_log);
    let mut option = DecodeOptions {
        debug_flag: 0xff, // All message
        drawer: &mut image,
    };
    image_reader(reader, &mut option)?;
    /*
      // for on memory image
      let mut buf :Vec<u8> = Vec::new();
      f.read_to_end(&mut buf)?;
      image_loader(&buf,&mut option)?;

    */

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

 ImageBuffer encoder impl PickCallback trait, 3 function.

```
    fn encode_start(&mut self,option: Option<EncoderOptions>) -> Result<Option<ImageProfiles>,Error>;
    fn encode_pick(&mut self,start_x: usize, start_y: usize, width: usize, height: usize,option: Option<PickOptions>)
                 -> Result<Option<Vec<u8>>,Error>;
    fn encode_end(&mut self, _: Option<EndOptions>) -> Result<(),Error>;
```
# update
- 0.0.1 baseline jpeg
- 0.0.2 bmp OS2/Windows RGB/RLE4/RLE8/bit fields/baseline JPEG
- 0.0.3 add GIF
- 0.0.4 change error message
- 0.0.5 reader change/Error delagation change
- 0.0.6 issue RST maker read bug fix
- 0.0.7 add PNG
- 0.0.8 add Jpeg multithread(pipelined),Progressive Jpeg has bugs(4,1,1) / BMP saver / Animation GIF(alpha)

# todo
- TIFF
- animation image buffer
- ICCProfile Reader
- Formated Header writer
- other decoder
- color translation
- encoder
