
use crate::io::read_bytes;
use crate::warning::ImgWarning;
use crate::io::read_byte;
use crate::io::read_u16le;
use crate::error::{ImgError,*};
use crate::DecodeOptions;

use crate::bmp::header::BitmapHeader;
use crate::bmp::header::Compressions;

fn covert_rgba32(buffer:&[u8],line: &mut Vec<u8>,header:&BitmapHeader) -> Result<(),ImgError> {
    let mut offset = 0;
    match header.bit_count {
        32 => { // bgra
            for x in  0..header.width{
                let b = buffer[offset];
                let g = buffer[offset + 1];
                let r = buffer[offset + 2];
                line[x*4] = r;
                line[x*4+1] = g;
                line[x*4+2] = b;
                offset += 4;
            }       
        },
        24 => { // bgra
            for x in  0..header.width{
                let b = buffer[offset];
                let g = buffer[offset + 1];
                let r = buffer[offset + 2];
                line[x*4] = r;
                line[x*4+1] = g;
                line[x*4+2] = b;
                offset += 3;
            }       
        },
        16 => { // rgb555
            for x in  0..header.width{
                let color = read_u16le(buffer,offset);
                let r = ((color & 0x7c00) >> 10) as u8;
                let g = ((color & 0x03e0) >> 5) as u8;
                let b = (color & 0x001f) as u8;
                line[x*4] = r << 3 | r >> 2;
                line[x*4+1] = g << 3 | g >> 2;
                line[x*4+2] = b << 3 | b >> 2;
                offset += 2;
            }       
        },
        8 => { 
            for x in  0..header.width{
                let color = read_byte(buffer,offset)  as usize;
                let r = header.color_table.as_ref().unwrap()[color].red.clone();
                let g = header.color_table.as_ref().unwrap()[color].green.clone();
                let b = header.color_table.as_ref().unwrap()[color].blue.clone();
                line[x*4] = r;
                line[x*4+1] = g;
                line[x*4+2] = b;
                offset += 1;
            }       
        },
        4 => { 
            for x_ in  0..(header.width + 1) /2{
                let mut x = x_ * 2;
                let color_ = read_byte(buffer,offset)  as usize;
                let color = color_ >> 4;
                let r = header.color_table.as_ref().unwrap()[color].red.clone();
                let g = header.color_table.as_ref().unwrap()[color].green.clone();
                let b = header.color_table.as_ref().unwrap()[color].blue.clone();
                line[x*4] = r;
                line[x*4+1] = g;
                line[x*4+2] = b;
                x += 1;
                let color = color_ & 0xf;
                let r = header.color_table.as_ref().unwrap()[color].red.clone();
                let g = header.color_table.as_ref().unwrap()[color].green.clone();
                let b = header.color_table.as_ref().unwrap()[color].blue.clone();
                line[x*4] = r;
                line[x*4+1] = g;
                line[x*4+2] = b;
                offset += 1;
            }       
        },

        1 => { 
            for x_ in  0..(header.width + 7) /8{
                let mut x = x_ * 8;
                let color_ = read_byte(buffer,offset)  as usize;
                for i in [7,6,5,4,3,2,1,0] {
                    let color = ((color_ >> i) & 0x1) as usize;
                    let r = header.color_table.as_ref().unwrap()[color].red.clone();
                    let g = header.color_table.as_ref().unwrap()[color].green.clone();
                    let b = header.color_table.as_ref().unwrap()[color].blue.clone();
                    line[x*4] = r;
                    line[x*4+1] = g;
                    line[x*4+2] = b;
                    x += 1;
                }
                offset += 1;
            }       
        },
        _ => {
            return Err(ImgError::new_const(ImgErrorKind::NoSupportFormat,&"Not Support bit count"))
        }
    }
    Ok(())
}


fn decode_rgb(buffer: &[u8],header:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<ImgWarning>,ImgError>  {
    option.drawer.init(header.width,header.height)?;
    let mut line :Vec<u8> = (0..header.width*4).map(|i| if i%4==3 {0xff} else {0}).collect();
    if header.bit_count <= 8 && header.color_table.is_none() {
        return  Err(
            ImgError::new_const(ImgErrorKind::NoSupportFormat,
                &"Not Support under 255 color and no color table")
        )
    }

    let line_size =  ((header.width as usize * header.bit_count + 31) / 32) * 4;
    for y_ in  0..header.height {
        let y = header.height -1 - y_ ;
        let offset = y_ * line_size;
        covert_rgba32(&buffer[offset..],&mut line,header)?;
        option.drawer.draw(0,y,header.width,1,&line)?;
    }
    option.drawer.terminate()?;
    Ok(None)
}

fn decode_rle(buffer: &[u8],header:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<ImgWarning>,ImgError>  {
    option.drawer.init(header.width,header.height)?;
    let mut line :Vec<u8> = (0..header.width*4).map(|i| if i%4==3 {0xff} else {0}).collect();
    let mut ptr = 0;
    let mut y:usize = header.height - 1;
    let rev_bytes = (8 / header.bit_count) as usize;
    loop{
        let mut x:usize = 0;
        let mut buf :Vec<u8> = (0..((header.width + rev_bytes -1) / rev_bytes)).map(|_| 0).collect();
        let mut is_eob = false;
        loop {
            if ptr >= buffer.len() {
                break;
            }
            let data0 = read_byte(buffer, ptr);
            let data1 = read_byte(buffer, ptr+1);
            ptr += 2;
            if data0 == 0 {
                if data1==0 {break}    // EOL
                if data1==1 {
                    is_eob = true;
                    break
                }    // EOB
                if data1 == 2 {         // Jump
                    let data0 = read_byte(buffer, ptr);
                    let data1 = read_byte(buffer, ptr+1);
                    ptr += 2;
                    if data1 == 0 {
                        x += data0 as usize;
                    } else {
                        covert_rgba32(&buf, &mut line, header)?;
                        option.drawer.draw(0,y,header.width,1,&line)?;
                        if y == 0 {break;}
                        y -= 1;
                        buf  = (0..((header.width + rev_bytes -1) / rev_bytes)).map(|_| 0).collect();
                        for _ in 0..data1 as usize {
                            covert_rgba32(&buf, &mut line, header)?;
                            option.drawer.draw(0,y,header.width,1,&line)?;
                            if y == 0 {break;}
                            y -= 1;
                        }
                    }
                }

                let bytes = (data1 as usize + rev_bytes -1) / rev_bytes;   // pixel
                let rbytes = (bytes + 1) /2 * 2;        // even bytes
                let rbuf = read_bytes(buffer,ptr,rbytes);
                ptr += rbytes;
            
                for i in 0..bytes {
                    buf[x] = rbuf[i];
                    x += 1;
                }    
            } else {
                for _ in 0..data0 as usize / rev_bytes{
                    buf[x] = data1;
                    x += 1;
                }
            }
        }
        covert_rgba32(&buf, &mut line, header)?;
        option.drawer.draw(0,y,header.width,1,&line)?;
        if y == 0 || ptr >= buffer.len() || is_eob {
            break;
        }
        y -= 1;
    }
    option.drawer.terminate()?;
    return Ok(None)
}


fn decode_jpeg(buffer: &[u8],_:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<ImgWarning>,ImgError>  {
    let ret = crate::jpeg::decoder::decode(buffer,option);
    match ret {
        Err(error) => {
            return Err(error);
        },
        Ok(some) => {
            if let Some(warning) = some {
                return Ok(Some(warning))
            } else {
                Ok(None)
            }
        }
    }
}

fn decode_png(buffer: &[u8],header:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<ImgWarning>,ImgError>  {
    return Err(ImgError::new_const(ImgErrorKind::NoSupportFormat,&"PNG bitmap not support"))
}

pub fn decode<'decode>(buffer: &[u8],option:&mut DecodeOptions) 
                    -> Result<Option<ImgWarning>,ImgError> {
    
    let header = BitmapHeader::new(&buffer,option.debug_flag)?;

    if cfg!(debug_assertions) {
        println!("{:?}", header);
    }

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