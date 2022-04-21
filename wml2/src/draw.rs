//! draw.rs  Image Load to Buffer Library
//! This library uses callback and decoder callbacks response initialize, draw, next, terminate function.
//! Drawer will use callback, flexisble drawing.
type Error = Box<dyn std::error::Error>;
use std::collections::HashMap;
use std::io::Write;
use crate::util::ImageFormat;
use std::io::BufReader;
use std::io::Seek;
use std::io::BufRead;
use bin_rs::reader::*;
use crate::util::format_check;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use crate::warning::ImgWarnings;
/* Dynamic Select Callback System */

pub enum DrawNextOptions {
    Continue,
    NextImage,
    ClearNext,
    WaitTime(usize),
    None,
}

pub type Response =  Result<Option<CallbackResponse>,Error>;

pub trait DrawCallback: Sync + Send {
/// DrawCallback trait is callback template
    fn init(&mut self,width: usize,height: usize,option: Option<InitOptions>) -> Result<Option<CallbackResponse>,Error>;
    fn draw(&mut self,start_x: usize, start_y: usize, width: usize, height: usize, data: &[u8],option: Option<DrawOptions>)
             -> Result<Option<CallbackResponse>,Error>;
    fn terminate(&mut self, _term: Option<TerminateOptions>) -> Result<Option<CallbackResponse>,Error>;
    fn next(&mut self, _next: Option<NextOptions>) -> Result<Option<CallbackResponse>,Error>;
    fn verbose(&mut self, _verbose: &str,_: Option<VerboseOptions>  ) -> Result<Option<CallbackResponse>,Error>;
}

pub trait PickCallback: Sync + Send {
    /// DrawCallback trait is callback template
    fn encode_start(&mut self,option: Option<EncoderOptions>) -> Result<Option<ImageProfiles>,Error>;
    fn encode_pick(&mut self,start_x: usize, start_y: usize, width: usize, height: usize,option: Option<PickOptions>)
                 -> Result<Option<Vec<u8>>,Error>;
    fn encode_end(&mut self, _: Option<EndOptions>) -> Result<(),Error>;
}

pub struct EncoderOptions {}
pub struct PickOptions {}
pub struct EndOptions{}

#[allow(unused)]
/// InitOptions added infomations send for drawer allback function
/// loop_count uses animation images (not impl)
/// backgroud if uses backgroud with alpha channel images;
pub struct InitOptions {
    pub loop_count: u32,
    pub background: Option<u32>, // RGBA
    pub animation: bool,
}

impl InitOptions {
    pub fn new() -> Option<Self> {
        Some(Self {
            loop_count: 1,
            background: None,
            animation: false
        })
    }
}

pub struct DrawOptions {
    
}

pub struct TerminateOptions {

}

pub struct VerboseOptions {

}

pub enum NextOption {
    Continue,
    Next,
    Dispose,
    ClearAbort,
    Terminate,
}

pub enum NextDispose {
    None,
    Override,
    Background,
    Previous,
}

pub enum NextBlend {
    Source,
    Override,
}

#[allow(unused)]
pub struct ImageRect {
    pub start_x: i32,
    pub start_y: i32,
    pub width: usize,
    pub height: usize,
}

#[allow(unused)]
pub struct NextOptions {
    pub flag: NextOption,
    pub await_time: u64,
    pub image_rect: Option<ImageRect>,
    pub dispose_option: Option<NextDispose>,
    pub blend: Option<NextBlend>
}

impl NextOptions {
    pub fn new() -> Self {
        NextOptions{
            flag: NextOption::Continue,
            await_time: 0,
            image_rect: None,
            dispose_option: None,
            blend: None,
        }
    }

    pub fn wait(ms_time: u64) -> Self {
        NextOptions{
            flag: NextOption::Continue,
            await_time: ms_time,
            image_rect: None,
            dispose_option: None,
            blend: None,
        }
    }
}

#[derive(std::cmp::PartialEq)]
pub enum ResposeCommand {
    Abort,
    Continue,
}

pub struct CallbackResponse {
    pub response: ResposeCommand,
}


impl CallbackResponse {
    pub fn abort() -> Self {
        Self {
            response: ResposeCommand::Abort,
        }
    }

    pub fn cont() -> Self {
        Self {
            response: ResposeCommand::Continue,
        }

    }
}

pub struct ImageProfiles {
    pub width: usize,
    pub height: usize,
    pub background: Option<u32>,
}

pub struct AnimationLayer {
    pub width: usize,
    pub height: usize,
    pub start_x: i32,
    pub start_y: i32,
    pub buffer: Vec<u8>,
    pub control: NextOptions,
}

#[allow(unused)]
pub struct ImageBuffer {
    /// ImageBuffer is default drawer
    /// Excample callback impliment
    /// ImageBuffer use only RGBA8888 color space model
    /// buffersize = width * height * 4 (8+8+8+8)/8
    /// other color space is implement feature.

    pub width: usize,
    pub height: usize,
    pub backgroud_color: Option<u32>,
    pub buffer: Option<Vec<u8>>,
    pub animation: Option<Vec<AnimationLayer>>,
    pub current: Option<usize>,
    pub loop_count: Option<u32>,
    pub first_wait_time: Option<u64>,
    fnverbose: fn(&str) -> Result<Option<CallbackResponse>,Error>,
}

fn default_verbose(_ :&str) -> Result<Option<CallbackResponse>, Error>{
    Ok(None)
}

impl ImageBuffer {
    pub fn new () -> Self {
        Self {
            width: 0,
            height: 0,
            backgroud_color: None,
            buffer: None,
            animation: None,
            current: None,
            loop_count: None,
            first_wait_time: None,
            fnverbose: default_verbose,
        }
    }

    pub fn set_animation(&mut self,flag: bool) {
        if flag {
            self.animation = Some(Vec::new())
        } else {
            self.animation = None
        }

    }

    /// A funtion set_verbose uses also debug.
    pub fn set_verbose(&mut self,verbose:fn(&str) -> Result<Option<CallbackResponse>,Error>) {
        self.fnverbose = verbose;
    }
}

impl DrawCallback for ImageBuffer {
    fn init(&mut self, width: usize, height: usize,option: Option<InitOptions>) -> Result<Option<CallbackResponse>, Error> {
        let buffersize = width * height * 4;
        self.width = width;
        self.height = height;
        self.buffer = Some((0 .. buffersize).map(|_| 0).collect());
        if let Some(option) = option {
            self.backgroud_color = option.background;
            if option.animation {
                self.set_animation(true);
            }
            self.loop_count = Some(option.loop_count);
        }
        Ok(None)
    }

    fn draw(&mut self, start_x: usize, start_y: usize, width: usize, height: usize, data: &[u8],_: Option<DrawOptions>)
                -> Result<Option<CallbackResponse>,Error>  {
        if self.buffer.is_none() {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::NotInitializedImageBuffer,"in draw".to_string())))
        }
        let buffer;
        let (w,h,raws);
        if self.current.is_none() {
            if start_x >= self.width || start_y >= self.height {return Ok(None);}
            w = if self.width < width + start_x {self.width - start_x} else { width };
            h = if self.height < height + start_y {self.height - start_y} else { height };
            raws = self.width;
            buffer =  self.buffer.as_deref_mut().unwrap();
        } else if let Some(animation) = &mut self.animation {
            let current = self.current.unwrap();
            if start_x >= animation[current].width || start_y >= animation[current].height {return Ok(None);}
            w = if animation[current].width < width + start_x {animation[current].width - start_x} else { width };
            h = if animation[current].height < height + start_y {animation[current].height - start_y} else { height };
            raws = animation[current].width;
            buffer = &mut animation[current].buffer;
        } else {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::NotInitializedImageBuffer,"in animation".to_string())))
        }

        for y in 0..h {
            let scanline_src =  y * width * 4;
            let scanline_dest= (start_y + y) * raws * 4;
            for x in 0..w {
                let offset_src = scanline_src + x * 4;
                let offset_dest = scanline_dest + (x + start_x) * 4;
                if offset_src + 3 >= data.len() {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::OutboundIndex,"decoder buffer in draw".to_string())))
                }
                buffer[offset_dest    ] = data[offset_src];
                buffer[offset_dest + 1] = data[offset_src + 1];
                buffer[offset_dest + 2] = data[offset_src + 2];
                buffer[offset_dest + 3] = data[offset_src + 3];
            }
        }
        Ok(None)
    }

    fn terminate(&mut self,_: Option<TerminateOptions>) -> Result<Option<CallbackResponse>, Error> {
        Ok(None)
    }

    fn next(&mut self, opt: Option<NextOptions>) -> Result<Option<CallbackResponse>, Error> {
        if  self.animation.is_some() {
            if let Some(opt) = opt  {
                if self.current.is_none() {
                    self.current = Some(0);
                    self.first_wait_time = Some(opt.await_time);
                } else {
                    self.current = Some(self.current.unwrap() + 1);
                }
                let (width,height,start_x,start_y);
                if let Some(ref rect) = opt.image_rect {
                    width = rect.width;
                    height= rect.height;
                    start_x=rect.start_x;
                    start_y=rect.start_y;
                } else {
                    width = self.width;
                    height= self.height;
                    start_x=0;
                    start_y=0;
                }
                let buffersize = width * height * 4;
                let buffer: Vec<u8> = (0..buffersize).map(|_| 0).collect();
                let layer = AnimationLayer {
                    width: width,
                    height: height,
                    start_x: start_x,
                    start_y: start_y,
                    buffer: buffer,
                    control: opt,
                };

                self.animation.as_mut().unwrap().push(layer);

                return Ok(Some(CallbackResponse::cont()))
            }
        }
        Ok(Some(CallbackResponse::abort()))
    }

    fn verbose(&mut self, str: &str,_: Option<VerboseOptions>) -> Result<Option<CallbackResponse>, Error> { 
        return (self.fnverbose)(str);
    }
}

impl PickCallback for ImageBuffer {
    
    fn encode_start(&mut self, _: Option<EncoderOptions>) -> Result<Option<ImageProfiles>, Error> {
        let init = ImageProfiles {
            width: self.width,
            height: self.height,
            background: self.backgroud_color,
        };
        Ok(Some(init))
    }

    fn encode_pick(&mut self,start_x: usize, start_y: usize, width: usize, height: usize, _: Option<PickOptions>) -> Result<Option<Vec<u8>>,Error> {
        if self.buffer.is_none() {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::NotInitializedImageBuffer,"in pick".to_string())))
        }
        let buffersize = width * height * 4;
        let mut data = Vec::with_capacity(buffersize);
        let buffer = self.buffer.as_ref().unwrap();

        if start_x >= self.width || start_y >= self.height {return Ok(None);}
        let w = if self.width < width + start_x {self.width - start_x} else { width };
        let h = if self.height < height + start_y {self.height - start_y} else { height };

        for y in 0..h {
            let scanline_src =  (start_y + y) * width * 4;
            for x in 0..w {
                let offset_src = scanline_src + (start_x + x) * 4;
                if offset_src + 3 >= buffer.len() {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::OutboundIndex,"Image buffer in pick".to_string())))
                }
                data.push(buffer[offset_src]);
                data.push(buffer[offset_src + 1]);
                data.push(buffer[offset_src + 2]);
                data.push(buffer[offset_src + 3]);
            }
        }
        Ok(Some(data))
    }

    fn encode_end(&mut self, _: Option<EndOptions>) -> Result<(), Error> {
        Ok(())
    }
}

pub struct DecodeOptions<'a> {
    pub debug_flag: usize,
    pub drawer: &'a mut dyn DrawCallback,
}

pub enum DataPack {
    UInt(u64),
    SInt(i64),
    Float(f64),
    String(String),    
}

pub struct EncodeOptions<'a> {
    pub debug_flag: usize,
    pub drawer: &'a mut dyn PickCallback,
    pub options: Option<HashMap<String,DataPack>>,
}

pub fn image_from(buffer: &[u8]) -> Result<ImageBuffer,Error> {
    image_load(buffer)
}

pub fn image_from_file(filename: String) -> Result<ImageBuffer,Error> {
    let f = std::fs::File::open(&filename)?;
    let reader = BufReader::new(f);
    let mut image = ImageBuffer::new();
    let mut option = DecodeOptions {
        debug_flag: 0x00,
        drawer: &mut image,
    };
    let _ = image_reader(reader, &mut option)?;
    Ok(image)
}

pub fn image_to_file(filename: String,image:&mut dyn PickCallback,format:ImageFormat) -> Result<(),Error> {
    let f = std::fs::File::create(&filename)?;
    let mut option = EncodeOptions {
        debug_flag: 0x00,
        drawer: image,
        options: None,
    };
    image_writer(f,&mut option,format)?;
    Ok(())
}


pub fn image_load(buffer: &[u8]) -> Result<ImageBuffer,Error> {    
    let mut ib = ImageBuffer::new();
    let mut option = DecodeOptions {
        debug_flag: 0,
        drawer: &mut ib
    };
    let mut reader = BytesReader::new(buffer);

    image_decoder(&mut reader,&mut option)?;
    Ok(ib)
}

pub fn image_loader(buffer: &[u8],option:&mut DecodeOptions) -> Result<Option<ImgWarnings>,Error> {    
    let mut reader = BytesReader::new(buffer);

    let r =image_decoder(&mut reader,option)?;
    Ok(r)
}

pub fn image_reader<R:BufRead + Seek>(reader: R,option:&mut DecodeOptions) -> Result<Option<ImgWarnings>,Error> {    

    let mut reader = StreamReader::new(reader);

    let r =image_decoder(&mut reader,option)?;
    Ok(r)
}


pub fn image_writer<W:Write>(mut writer: W,option:&mut EncodeOptions,format:ImageFormat) -> Result<Option<ImgWarnings>,Error> {    
    let buffer =image_encoder(option,format)?;
    writer.write_all(&buffer)?;
    writer.flush()?;
    Ok(None)
}


pub fn image_decoder<B: BinaryReader>(reader:&mut B ,option:&mut DecodeOptions) -> Result<Option<ImgWarnings>,Error> {
    let buffer = reader.read_bytes_no_move(128)?;
    let format = format_check(&buffer);

    use crate::util::ImageFormat::*;
    match format {
        Jpeg => {
            return crate::jpeg::decoder::decode(reader, option);

        },
        Bmp => {
            return crate::bmp::decoder::decode(reader, option);

        },
        Gif => {
            return crate::gif::decoder::decode(reader, option);
        },
        Png => {
            return crate::png::decoder::decode(reader, option);
        },
        _ => {
            return Err(
                Box::new(ImgError::new_const(ImgErrorKind::NoSupportFormat, "This buffer can not decode".to_string())))
        }
    }
}


pub fn image_encoder(option:&mut EncodeOptions,format:ImageFormat) -> Result<Vec<u8>,Error> {
    use crate::util::ImageFormat::*;

    match format {
        Bmp => {
            return crate::bmp::encoder::encode(option);
        },
        _ => {
            return Err(
                Box::new(ImgError::new_const(ImgErrorKind::NoSupportFormat, "This encoder no impl".to_string())))
        }
    }
}
