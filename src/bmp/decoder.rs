/*
 *  bmp/decorder.rs (C) 2022 Mith@mmk
 *  
 * 
 */


use crate::bmp::header::BitmapInfo::Windows;
use crate::error::{ImgError,ImgErrorKind};
use crate::draw::*;
use crate::io::{read_byte, read_bytes, read_u16le, read_u32le};
use crate::warning::ImgWarning;

use crate::bmp::header::BitmapHeader;
use crate::bmp::header::Compressions;

fn covert_rgba32(buffer:&[u8],line: &mut Vec<u8>,header:&BitmapHeader,bit_count: usize) -> Result<(),ImgError> {
    let mut offset = 0;
    let width = header.width.abs() as usize;
    match bit_count {
        32 => { // bgra
            for x in  0..width{
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
            for x in  0..width{
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
            for x in  0..width{
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
            for x in  0..width{
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
            for x_ in  0..(width + 1) /2{
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
            for x_ in  0..(width + 7) /8{
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
    let width = header.width.abs() as usize;
    let height = header.height.abs() as usize;
    option.drawer.init(width,height,InitOptions::new())?;
    let mut line :Vec<u8> = (0..width*4).map(|i| if i%4==3 {0xff} else {0}).collect();
    if header.bit_count <= 8 && header.color_table.is_none() {
        return  Err(
            ImgError::new_const(ImgErrorKind::NoSupportFormat,
                &"Not Support under 255 color and no color table")
        )
    }

    let line_size =  ((width as usize * header.bit_count + 31) / 32) * 4;
    for y_ in  0..height {
        let y = height -1 - y_ ;
        let offset = y_ * line_size;
        covert_rgba32(&buffer[offset..],&mut line,header,header.bit_count)?;
        if header.height > 0 {
            option.drawer.draw(0,y,width,1,&line,None)?;
        } else {
            option.drawer.draw(0,y_,width,1,&line,None)?;
        }
    }
    option.drawer.terminate(None)?;
    Ok(None)
}

fn decode_rle(buffer: &[u8],header:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<ImgWarning>,ImgError>  {
    let width = header.width.abs() as usize;
    let height = header.height.abs() as usize;
    option.drawer.init(width,height,InitOptions::new())?;
    let mut line :Vec<u8> = (0..header.width*4).map(|i| if i%4==3 {0xff} else {0}).collect();
    let mut ptr = 0;
    let mut y:usize = height - 1;
    let rev_bytes = (8 / header.bit_count) as usize;
    'y: loop{
        let mut x:usize = 0;
        let mut buf :Vec<u8> = (0..(width + 1)).map(|_| 0).collect();
        'x:  loop {
            if ptr >= buffer.len() {
                break;
            }
            let data0 = read_byte(buffer, ptr);
            let data1 = read_byte(buffer, ptr+1);
            ptr += 2;
            if data0 == 0 {
                if data1==0 {
                    break
                }    // EOL
                if data1==1 {
                    break 'y
                }    // EOB
                if data1 == 2 {         // Jump
                    let data0 = read_byte(buffer, ptr);
                    let data1 = read_byte(buffer, ptr+1);
                    ptr += 2;
                    if data1 == 0 {
                        x += data0 as usize;
                    } else {
                        covert_rgba32(&buf, &mut line, header,8)?;
                        option.drawer.draw(0,y,width,1,&line,None)?;
                        if y == 0 {break;}
                        y -= 1;
                        buf  = (0..((width + rev_bytes -1) / rev_bytes)).map(|_| 0).collect();
                        for _ in 0..data1 as usize {
                            covert_rgba32(&buf, &mut line, header,8)?;
                            option.drawer.draw(0,y,width,1,&line,None)?;
                            if y == 0 {break;}
                            y -= 1;
                        }
                        x = data0 as usize;
                        continue 'x
                    }
                }

                let bytes = (data1 as usize + rev_bytes -1) / rev_bytes;   // pixel
                let rbytes = (bytes + 1) /2 * 2;                            // even bytes
                let rbuf = read_bytes(buffer,ptr,rbytes);
                ptr += rbytes;
            
                if header.bit_count == 8 {
                    for i in 0..bytes {
                        buf[x] = rbuf[i];
                        x += 1;
                    }
                } else if header.bit_count == 4{
                    for i in 0..bytes {
                        buf[x  ] = rbuf[i] >> 4;
                        buf[x+1] = rbuf[i] & 0xf;
                        x += 2;
                    }
                } else {
                    return Err(ImgError::new_const(ImgErrorKind::NoSupportFormat,&"Unknwon"))
                } 
            } else {

                if header.bit_count == 8 {
                    for _ in 0..data0{
                        buf[x] = data1;
                        x += 1;
                        if x >= buf.len() {
                            break 'x;
                        }
                    }
                } else if header.bit_count == 4 {
                    for _ in 0..data0 as usize / rev_bytes {
                        buf[x] = data1 >> 4;
                        x +=1;
                        if x >= buf.len() {
                            break 'x;
                        }
                        buf[x] = data1 & 0xf;
                        x +=1;
                        if x >= buf.len() {
                            break 'x;
                        }
                    }
                    if data0 % 2 == 1 {
                        buf[x] = data1 >> 4;
                        x +=1;
                    }
                } else {
                    return Err(ImgError::new_const(ImgErrorKind::NoSupportFormat,&"Unknwon"))
                }
            }
        }
        covert_rgba32(&buf, &mut line, header,8)?;
        if header.height > 0 {
            option.drawer.draw(0,y,width,1,&line,None)?;
        } else {
            option.drawer.draw(0,height - 1 - y,width,1,&line,None)?;
        }
        if y == 0 || ptr >= buffer.len()  {
            break;
        }
        y -= 1;
    }
    option.drawer.terminate(None)?;
    return Ok(None)
}

fn get_shift(mask :u32) -> (u32,u32) {
    let mut temp = mask;
    let mut shift = 0;
    while temp & 0x1 == 0 {
        temp >>= 1;
        shift += 1;
        if shift > 32 {
            return (0,8);
        }
    }
    let mut bits = 0;
    while temp & 0x1 == 1 {
        temp >>= 1;
        bits += 1;
        if bits + shift > 32 {
            return (0,8);
        }
    }
    if bits >= 8 {
        shift += bits - 8;
        bits = 0;
    }
    (shift,bits)
}

fn decode_bit_fileds(buffer: &[u8],header:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<ImgWarning>,ImgError>  {
    let width = header.width.abs() as usize;
    let height = header.height.abs() as usize;

    let info;

    if header.bit_count != 16 && header.bit_count != 32 {
        return Err(ImgError::new_const(ImgErrorKind::NoSupportFormat,
            &"Illigal bit field / bit count")) 
    }
    if let Windows(info_) = &header.bitmap_info {
        info = info_;
    } else {
        return Err(ImgError::new_const(ImgErrorKind::NoSupportFormat,
            &"Illigal bit field / not Windows Bitmap"))        
    }

    if info.b_v4_header.is_none() {
        return Err(ImgError::new_const(ImgErrorKind::NoSupportFormat,
            &"Illigal bit field / no V4 Header"))
    }
    let v4 = info.b_v4_header.as_ref().unwrap();

    let red_mask = v4.b_v4_red_mask;
    let (red_shift,red_bits) = get_shift(red_mask);

    let green_mask = v4.b_v4_green_mask;
    let (green_shift,green_bits) = get_shift(green_mask);

    let blue_mask = v4.b_v4_blue_mask;
    let (blue_shift,blue_bits) = get_shift(blue_mask);

    let alpha_mask = v4.b_v4_alpha_mask;
    let (alpha_shift,alpha_bits) = get_shift(alpha_mask);

    
    if cfg!(debug_assertions) {
        println!("{:>04x} {:>032b} >>{} {}",red_mask,red_mask,red_shift,red_bits);
        println!("{:>04x} {:>032b} >>{} {}",green_mask,green_mask,green_shift,green_bits);
        println!("{:>04x} {:>032b} >>{} {}",blue_mask,blue_mask,blue_shift,blue_bits);
        println!("{:>04x} {:>032b} >>{} {}",alpha_mask,alpha_mask,alpha_shift,alpha_bits);
    }

    option.drawer.init(width,height,InitOptions::new())?;
    let mut line :Vec<u8> = (0..width*4).map(|i| if i%4==3 {0xff} else {0}).collect();

    let line_size =  ((width as usize * header.bit_count + 31) / 32) * 4;
    for y_ in  0..height {
        let y = height -1 - y_ ;
        let offset = y_ * line_size;

        for x in 0..width {
            let color = if header.bit_count == 32 {
                read_u32le(buffer, offset + x * 4) as u32
            } else {
                read_u16le(buffer, offset + x * 2) as u32
            };
            let red   = ((color & red_mask) >> red_shift) as u32;
            let green = ((color & green_mask) >> green_shift) as u32;
            let blue  = ((color & blue_mask) >> blue_shift) as u32;

            let alpha = if alpha_mask != 0 {
                ((color & alpha_mask) >> alpha_shift) as u32
             } else {0xff};
            line[x*4  ] = (red << (8 - red_bits) | red >> red_bits) as u8;
            line[x*4+1] = (green << (8 - green_bits) | green >> green_bits) as u8;
            line[x*4+2] = (blue << (8 - blue_bits) | blue >> blue_bits) as u8;
            line[x*4+3] = (alpha << (8 - alpha_bits) | alpha >> alpha_bits) as u8;
        }
        if header.height > 0 {
            option.drawer.draw(0,y,width,1,&line,None)?;
        } else {
            option.drawer.draw(0,y_,width,1,&line,None)?;
        }
    }
    option.drawer.terminate(None)?;
    Ok(None)
}

fn decode_jpeg(buffer: &[u8],_:&BitmapHeader,option:&mut  DecodeOptions) -> Result<Option<ImgWarning>,ImgError>  {
    return crate::jpeg::decoder::decode(buffer,option);
}

fn decode_png(_buffer: &[u8],_header:&BitmapHeader,_option:&mut  DecodeOptions) -> Result<Option<ImgWarning>,ImgError>  {
    return Err(ImgError::new_const(ImgErrorKind::NoSupportFormat,&"PNG bitmap not support"))
}

pub fn decode<'decode>(buffer: &[u8],option:&mut DecodeOptions) 
                    -> Result<Option<ImgWarning>,ImgError> {
    
    let header = BitmapHeader::new(&buffer,option.debug_flag)?;

    if option.debug_flag > 0 {
        let s = format!("{:?}",&header);
        option.drawer.verbose(&s,None)?;
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
                return decode_bit_fileds(buffer,&header,option);
            },
            Compressions::BiJpeg => {
                return decode_jpeg(buffer,&header,option);
            },
            Compressions::BiPng => {
                return decode_png(buffer,&header,option);
            },
        }
    } else {
        return decode_rgb(buffer,&header,option);
    }
}