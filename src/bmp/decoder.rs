

use crate::io::read_byte;
use crate::io::read_u16le;
use crate::bmp::worning::BMPWorning;
use crate::bmp::worning::WorningKind;
use crate::error::ImgError::SimpleAddMessage;
use crate::error::{ImgError,ErrorKind};
use crate::DecodeOptions;

use crate::bmp::header::BitmapHeader;
use crate::bmp::header::Compressions;


fn decode_rgb(buffer: &[u8],header:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<BMPWorning>,ImgError>  {
    option.drawer.init(header.width,header.height)?;
    let mut line :Vec<u8> = (0..header.width*4).map(|i| if i%4==3 {0xff} else {0}).collect();
    if header.bit_count <= 8 && header.color_table.is_none() {
        return  Err(SimpleAddMessage(ErrorKind::NoSupportFormat,"Not Support under 255 color and no color table".to_string()))
    }
    for y_ in  0..header.height{
        let mut offset = y_ * header.width as usize * ((header.bit_count + 31) / 32 * 4);
        let y = header.height - y_;
        match header.bit_count {
            32 => { // bgra
                for x in  0..header.width{
                    offset += 4;
                    let b = buffer[offset];
                    let g = buffer[offset + 1];
                    let r = buffer[offset + 2];
                    line[x*4] = r;
                    line[x*4+1] = g;
                    line[x*4+2] = b;
                }       
            },
            24 => { // bgra
                for x in  0..header.width{
                    offset += 3;
                    let b = buffer[offset];
                    let g = buffer[offset + 1];
                    let r = buffer[offset + 2];
                    line[x*4] = r;
                    line[x*4+1] = g;
                    line[x*4+2] = b;
                }       
            },
            16 => { // rgb555
                for x in  0..header.width{
                    offset += 2;
                    let color = read_u16le(buffer,offset);
                    let r = (color & 0x7c00 >> 10) as u8;
                    let g = (color & 0x03e0 >> 5) as u8;
                    let b = (color & 0x001f) as u8;
                    line[x*4] = r;
                    line[x*4+1] = g;
                    line[x*4+2] = b;
                }       
            },
            8 => { 
                for x in  0..header.width{
                    let offset = offset + 1;
                    let color = read_byte(buffer,offset)  as usize;
                    let r = header.color_table.as_ref().unwrap()[color].red.clone();
                    let g = header.color_table.as_ref().unwrap()[color].green.clone();
                    let b = header.color_table.as_ref().unwrap()[color].blue.clone();
                    line[x*4] = r;
                    line[x*4+1] = g;
                    line[x*4+2] = b;
                }       
            },
            4 => { 
                for x_ in  0..(header.width + 1) /2{
                    let mut x = x_ * 2;
                    let offset = offset + 1;
                    let color_ = read_byte(buffer,offset)  as usize;
                    let color = color_ >> 4;
                    let r = header.color_table.as_ref().unwrap()[color].red.clone();
                    let g = header.color_table.as_ref().unwrap()[color].green.clone();
                    let b = header.color_table.as_ref().unwrap()[color].blue.clone();
                    line[x*4] = r;
                    line[x*4+1] = g;
                    line[x*4+2] = b;
                    x += 1;
                    let color = color_ >> 0xf;
                    let r = header.color_table.as_ref().unwrap()[color].red.clone();
                    let g = header.color_table.as_ref().unwrap()[color].green.clone();
                    let b = header.color_table.as_ref().unwrap()[color].blue.clone();
                    line[x*4] = r;
                    line[x*4+1] = g;
                    line[x*4+2] = b;
                }       
            },

            1 => { 
                for x_ in  0..(header.width + 7) /8{
                    let mut x = x_ * 8;
                    let offset = offset + 1;
                    let color_ = read_byte(buffer,offset)  as usize;
                    for i in [7,6,5,4,3,2,1] {
                        let color = ((color_ >> i) & 0x1) as usize;
                        let r = header.color_table.as_ref().unwrap()[color].red.clone();
                        let g = header.color_table.as_ref().unwrap()[color].green.clone();
                        let b = header.color_table.as_ref().unwrap()[color].blue.clone();
                        line[x*4] = r;
                        line[x*4+1] = g;
                        line[x*4+2] = b;
                        x += 1;
                    }
                }       
            },
            _ => {
                return Err(SimpleAddMessage(ErrorKind::NoSupportFormat,"Not Support bit count".to_string()))         
            }
        }
        option.drawer.draw(0,y,header.width,y,&line)?;
    }
    option.drawer.terminate()?;
    Ok(None)
}

fn decode_rle(buffer: &[u8],header:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<BMPWorning>,ImgError>  {
//    option.drawer.init(header.width,header.height);
    return Err(SimpleAddMessage(ErrorKind::NoSupportFormat,"RLE bitmap not support".to_string()))
}

fn decode_jpeg(buffer: &[u8],_:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<BMPWorning>,ImgError>  {
    let ret = crate::jpeg::decoder::decode(buffer,option);
    match ret {
        Err(error) => {
            return Err(error);
        },
        Ok(some) => {
            if let Some(worning) = some {
                return Ok(Some(BMPWorning::Simple(WorningKind::JpegWorning(worning))))
            } else {
                Ok(None)
            }
        }
    }
}

fn decode_png(buffer: &[u8],header:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<BMPWorning>,ImgError>  {
    return Err(SimpleAddMessage(ErrorKind::NoSupportFormat,"PNG bitmap not support".to_string()))
}


pub fn decode<'decode>(buffer: &[u8],option:&mut DecodeOptions) 
                    -> Result<Option<BMPWorning>,ImgError> {
    
    let header = BitmapHeader::new(&buffer,option.debug_flag)?;
    let offset = header.image_offset;
    let buffer = &buffer[offset..];
    if let Some(ref compression) = header.compression {
        match compression {
            Compressions::BiRGB => {
                return decode_rgb(buffer,&header,option);
            },
            Compressions::BiRLE8 => {
                return decode_rle(buffer,&header,option);
            },
            Compressions::BiRLE4 => {
                return decode_rle(buffer,&header,option);
            },
            Compressions::BiBitFileds => {
                return decode_rle(buffer,&header,option);
            },
            Compressions::BiJpeg => {
                return decode_jpeg(buffer,&header,option);
            },
            Compressions::BiPng => {
                return decode_png(buffer,&header,option);
            },
        }

    } else {
        // error
    }

    Ok(None)
}