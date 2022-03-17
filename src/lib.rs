/*
 * WML2 - Web graphic Multi format Library To Rust
 *  (C)Mith@mmk 2022
 * 
 *  use MIT Licnce
 */

pub mod io;
pub mod jpeg;
pub mod tiff;
pub mod error;
pub mod util;
pub mod iccprofile;

use crate::ImgError::SimpleAddMessage;
use crate::error::ImgError;
use crate::error::ErrorKind;

/* Dynamic Select Callback System */

pub enum DrawNextOptions {
    Continue,
    NextImage,
    ClearNext,
    WaitTime(usize),
    None,
}

pub trait DrawCallback {
    fn init(&mut self,width: usize,height: usize) -> Result<Option<isize>,ImgError>;
    fn draw(&mut self,start_x: usize, start_y: usize, width: usize, height: usize, data: &[u8])
             -> Result<Option<isize>,ImgError>;
    fn terminate(&mut self) -> Result<Option<isize>,ImgError>;
    fn next(&mut self, _next: Vec<u8>) -> Result<Option<isize>,ImgError>;
    fn verbose(&mut self, _verbose: &str ) -> Result<Option<isize>,ImgError>;
}

#[allow(unused)]
pub struct ImageBuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Option<Vec<u8>>,
    fnverbose: fn(&str) -> Result<Option<isize>,ImgError>,
}

fn default_verbose(_ :&str) -> Result<Option<isize>, ImgError>{
    Ok(None)
}

impl ImageBuffer {
    pub fn new () -> Self {
        Self {
            width: 0,
            height: 0,
            buffer: None,
            fnverbose: default_verbose,
        }
    }

    pub fn set_verbose(&mut self,verbose:fn(&str) -> Result<Option<isize>,ImgError>) {
        self.fnverbose = verbose;
    }
}

impl DrawCallback for ImageBuffer {
    fn init(&mut self, width: usize, height: usize) -> Result<Option<isize>, ImgError> {
        let buffersize = width * height * 4;
        self.width = width;
        self.height = height;
        self.buffer = Some((0 .. buffersize).map(|_| 0).collect());
        Ok(None)
    }

    fn draw(&mut self, start_x: usize, start_y: usize, width: usize, height: usize, data: &[u8])
                -> Result<Option<isize>,ImgError>  {
        if self.buffer.is_none() {
            return Err(ImgError::Simple(ErrorKind::NotInitializedImageBuffer))
        }
        let buffer =  self.buffer.as_deref_mut().unwrap();
        if start_x >= self.width || start_y >= self.height {return Ok(None);}
        let w = if self.width < width + start_x {self.width - start_x} else { width };
        let h = if self.height < height + start_y {self.height - start_y} else { height };
        for y in 0..h {
            let scanline_src =  y * width * 4;
            let scanline_dest= (start_y + y) * self.width * 4;
            for x in 0..w {
                let offset_src = scanline_src + x * 4;
                let offset_dest = scanline_dest + (x + start_x) * 4;
                if offset_src + 3 >= data.len() {
                    return Err(SimpleAddMessage(ErrorKind::OutboundIndex,format!("decoder buffer in draw {}",data.len())))
                }
                buffer[offset_dest    ] = data[offset_src];
                buffer[offset_dest + 1] = data[offset_src + 1];
                buffer[offset_dest + 2] = data[offset_src + 2];
                buffer[offset_dest + 3] = data[offset_src + 3];
            }
        }
        Ok(None)
    }

    fn terminate(&mut self) -> Result<Option<isize>, ImgError> {
        Ok(None)
    }

    fn next(&mut self, _: std::vec::Vec<u8>) -> Result<Option<isize>, ImgError> {
        Ok(None)
    }

    fn verbose(&mut self, str: &str) -> Result<Option<isize>, ImgError> { 
        return (self.fnverbose)(str);
    }
}


pub struct DecodeOptions<'a> {
    pub debug_flag: usize,
    pub drawer: &'a mut dyn DrawCallback,
}