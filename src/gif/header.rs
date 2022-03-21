use crate::color::RGBA;
use crate::io::*;
use crate::error::ImgError;
use crate::error::ImgErrorKind;

#[derive(Debug)]
pub struct GifHeader {
    pub width: usize,
    pub height: usize,
    pub color_table: Vec<RGBA>,
    pub scd: GifScd,
    pub header_size: usize,
    pub transparent_color: Option<u8>,
    pub comment: Option<String>,
}

#[derive(Debug)]
pub struct GifScd {
    pub width: u16,
    pub height: u16,
    pub field: u8,
    pub color_index: u8,
    pub aspect: u8,
}

#[derive(Debug)]
pub struct GifLscd {
    pub xstart: u16,
    pub ystart: u16,
    pub xsize: u16,
    pub ysize: u16,
    pub field: u8,
}

#[derive(Debug)]
pub struct GifExtend {
    pub indent: u8,
    pub code: u8,
    pub length: u8,
    pub data: [u8;256]
}

impl GifHeader {
    pub fn new(buffer :&[u8],_opt :usize) -> Result<Self,ImgError> {
        let mut ptr = 0;
        let gif = read_bytes(buffer,ptr,6);

        if gif[0] != b'G' || gif[1] != b'I' || gif[2] != b'F' {
            return Err(ImgError::new_const(ImgErrorKind::UnknownFormat,&"not Gif"))
        }
        if gif[3] != b'8' || (gif[4] != b'7' && gif[4] != b'9') || gif[5] != b'a' {
            return Err(ImgError::new_const(ImgErrorKind::UnknownFormat,&"not Gif"))
        }
        ptr += 6;

        let scd = GifScd{
            width: read_u16le(buffer,ptr + 0),
            height: read_u16le(buffer,ptr + 2),
            field: read_byte(buffer,ptr + 4),
            color_index: read_byte(buffer,ptr + 5),
            aspect: read_byte(buffer,ptr + 6),
        };
        ptr += 7;

        let mut color_table :Vec<RGBA> = Vec::new();

        if (scd.field & 0x80) == 0x80 {
            println!("Read color table");
            let table_size = 1 << ((scd.field&0x07) +1);

            for _ in 0..table_size {
                let palette = RGBA {
                    red: read_byte(buffer, ptr),
                    green: read_byte(buffer, ptr+1),
                    blue: read_byte(buffer, ptr+2),
                    alpha: 0xff,
                };
                color_table.push(palette);
                ptr +=3;
            }
        }
    
        return Ok(GifHeader {
            width: scd.width as usize,
            height: scd.height as usize,
            color_table: color_table,
            scd: scd,
            header_size: ptr,
            transparent_color: None,
            comment: None,
        })
    }
}

impl GifLscd {
    pub fn new(buffer :&[u8],ptr: usize) -> Self{
        Self {
            xstart: read_u16le(buffer,ptr),
            ystart: read_u16le(buffer,ptr + 2),
            xsize: read_u16le(buffer,ptr + 4),
            ysize: read_u16le(buffer,ptr + 6),
            field: read_byte(buffer,ptr + 8),
        }
    }
}