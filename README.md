Notice: Specification of this library is not decision.

# WML2 - Web graphic Multi format Library To Rust
- Rust writing graphic loader for WASM
- only on memory use
- callback system
- not use multithreding
- No need to force use - becouse You can use Javascript Image.

# using
- on memory compress image buffer (now only baseline jpg)
- output memory buffer and callback (defalt use ImageBuffer)

```
  let image = new ImageBuffer();
  let verbose = 0;
  let mut option = DecodeOptions{
    debug_flag: verbose,
    drawer: image,
  };

  let r = wml2::jpeg::decoder::decode(data, &mut option);
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
