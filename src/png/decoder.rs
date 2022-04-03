use crate::color::RGBA;
use std::cmp::min;
use crate::png::warning::PngWarning;
use crate::png::header::PngHeader;
use crate::warning::*;
use crate::draw::DecodeOptions;
use crate::error::*;
use bin_rs::reader::BinaryReader;
type Error = Box<dyn std::error::Error>;

fn load_grayscale(header:&PngHeader,buffer:&[u8] ,option:&mut DecodeOptions) 
-> Result<Option<ImgWarnings>,Error> {
    let is_alpha = if header.color_type == 4 {1} else {0};
    let raw_length = (header.width * ((header.bitpersample as u32 +7 / 8) + is_alpha) + 1) as usize;
    let mut prev_buf:Vec<u8> = Vec::new();

    for y in 0..header.height as usize{
        let mut ptr = raw_length * y;
        let flag = buffer[ptr];
        let mut outbuf:Vec<u8> = (0..header.width * 4).map(|_| 0).collect();
        ptr += 1;
        let mut outptr = 0;
        for _ in 0..header.width as usize {
            let (mut grey, mut alpha) = (0,0xff);
            match header.bitpersample {
                16 => {
                    grey = buffer[ptr];ptr += 2;
                    if is_alpha == 1 {                
                        alpha = buffer[ptr];ptr += 2;
                    } else {
                        alpha = 0xff;
                    }
                },
                8 => {
                    grey = buffer[ptr];ptr += 1;
                    if is_alpha == 1 {                
                        alpha = buffer[ptr];ptr += 1;
                    } else {
                        alpha = 0xff;
                    }
                },
                _ => {},
            }
            match flag {
                1 => { // Sub
                    if outptr > 0 {
                        grey   += outbuf[outptr -4];
                    }
                    if is_alpha == 1 {
                        outbuf[outptr - 1] = alpha;
                    } else {
                        outbuf[outptr+3] = 0xff;
                    }
                },
                2 => { // Up
                    if prev_buf.len() > outptr +4 {
                        grey   += prev_buf[outptr];
                    }
                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                }
                3 => { // Avalage
                    let (mut grey_,mut alpha_);
                    if outptr >= 4 {
                        grey_  = outbuf[outptr -4];
                        alpha_ = outbuf[outptr -1];
                    } else {
                        grey_  = 0;
                        alpha_ = 0;
                    }
                    if prev_buf.len() > outptr +4 {
                        grey_   += prev_buf[outptr];
                        alpha_ += prev_buf[outptr+3];
                    } else {
                        grey_   += 0;
                        alpha_ += 0;
                    }
                    grey  += grey_  / 2;
                    alpha += alpha_ / 2;

                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                },
                4 => { // Pease
                    let (grey_a, alpha_a);
                    if outptr >= 4 {
                        grey_a  = outbuf[outptr -4];
                        alpha_a = outbuf[outptr -1];
                    } else {
                        grey_a   = 0;
                        alpha_a  = 0;
                    }
                    let (grey_b, alpha_b);
                    if prev_buf.len() > outptr +4 {
                        grey_b  = prev_buf[outptr];
                        alpha_b = outbuf[outptr -1];
                    } else {
                        grey_b  = 0;
                        alpha_b = 0;
                    }
                    let (grey_c, alpha_c);
                    if prev_buf.len() > outptr +4 && outptr >=4 {
                        grey_c  = prev_buf[outptr-4];
                        alpha_c = prev_buf[outptr-1];
                    } else {
                        grey_c  = 0;
                        alpha_c = 0;
                    }

                    grey   += min(min(grey_a,grey_b),grey_c);
                    alpha += min(min(alpha_a,alpha_b),alpha_c);

                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                }
                _ => {}  // None
            }
            outbuf[outptr] = grey;
            outbuf[outptr+1] = grey;
            outbuf[outptr+2] = grey;
            outbuf[outptr+3] = alpha;
            outptr += 4;
        }
        option.drawer.draw(0,y,header.width as usize,1,buffer,None)?;
        prev_buf = outbuf;
    }
    return Ok(None)
}

fn load_truecolor(header:&PngHeader,buffer:&[u8] ,option:&mut DecodeOptions) 
-> Result<Option<ImgWarnings>,Error> {

    let is_alpha = if header.color_type == 6 {1} else {0};
    let raw_length = (header.width * (header.bitpersample as u32 / 8 + is_alpha) + 1) as usize;
    let mut prev_buf:Vec<u8> = Vec::new();

    for y in 0..header.height as usize{
        let mut ptr = raw_length * y;
        let flag = buffer[ptr];
        let mut outbuf:Vec<u8> = (0..header.width * 4).map(|_| 0).collect();
        ptr += 1;
        let mut outptr = 0;
        for _ in 0..header.width as usize {
            let (mut red, mut green, mut blue, mut alpha);
            if header.bitpersample == 16 {
                red = buffer[ptr];ptr += 2;
                green = buffer[ptr];ptr += 2;
                blue = buffer[ptr];ptr += 2;
                if is_alpha == 1 {                
                    alpha = buffer[ptr];ptr += 2;
                } else {
                    alpha = 0xff;
                }        
            } else {
                red = buffer[ptr];ptr += 1;
                green = buffer[ptr];ptr += 1;
                blue = buffer[ptr];ptr += 1;
                if is_alpha == 1 {                
                    alpha = buffer[ptr];ptr += 1;
                } else {
                    alpha = 0xff;
                }
            }
            match flag {
                1 => { // Sub
                    if outptr >= 4 {
                        red   += outbuf[outptr -4];
                        green += outbuf[outptr -3];
                        blue  += outbuf[outptr -2];
                        alpha += outbuf[outptr -1];
                    }
                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                },
                2 => { // Up
                    if prev_buf.len() > outptr +4 {
                        red   += prev_buf[outptr];
                        green += prev_buf[outptr+1];
                        blue  += prev_buf[outptr+2];
                        alpha += prev_buf[outptr+3];
                    }
                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                }
                3 => { // Avalage
                    let (mut red_, mut green_, mut blue_, mut alpha_);
                    if outptr >= 4 {
                        red_   = outbuf[outptr -4];
                        green_ = outbuf[outptr -3];
                        blue_  = outbuf[outptr -2];
                        alpha_ = outbuf[outptr -1];
                    } else {
                        red_   = 0;
                        green_ = 0;
                        blue_  = 0;
                        alpha_  = 0;
                    }
                    if prev_buf.len() > outptr +4 {
                        red_   += prev_buf[outptr];
                        green_ += prev_buf[outptr+1];
                        blue_  += prev_buf[outptr+2];
                        alpha_ += prev_buf[outptr+3];
                    } else {
                        red_   += 0;
                        green_ += 0;
                        blue_  += 0;
                        alpha_ += 0;
                    }
                    red   += red_   / 2;
                    green += green_ / 2;
                    blue  += blue_  / 2;
                    alpha += alpha_ / 2;

                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                },
                4 => { // Pease
                    let (red_a, green_a, blue_a, alpha_a);
                    if outptr >= 4 {
                        red_a   = outbuf[outptr -4];
                        green_a = outbuf[outptr -3];
                        blue_a  = outbuf[outptr -2];
                        alpha_a = outbuf[outptr -1];
                    } else {
                        red_a   = 0;
                        green_a = 0;
                        blue_a  = 0;
                        alpha_a  = 0;
                    }
                    let (red_b, green_b, blue_b, alpha_b);
                    if prev_buf.len() > outptr +4 {
                        red_b   = prev_buf[outptr];
                        green_b = prev_buf[outptr+1];
                        blue_b  = prev_buf[outptr+2];
                        alpha_b = prev_buf[outptr+3];
                    } else {
                        red_b   = 0;
                        green_b = 0;
                        blue_b  = 0;
                        alpha_b = 0;
                    }
                    let (red_c, green_c, blue_c, alpha_c);
                    if prev_buf.len() > outptr +4 && outptr >=4 {
                        red_c   = prev_buf[outptr-4];
                        green_c = prev_buf[outptr-3];
                        blue_c  = prev_buf[outptr-2];
                        alpha_c = prev_buf[outptr-1];
                    } else {
                        red_c   = 0;
                        green_c = 0;
                        blue_c  = 0;
                        alpha_c = 0;
                    }

                    red   += min(min(red_a,red_b),red_c);
                    green += min(min(green_a,green_b),green_c);
                    blue  += min(min(blue_a,blue_b),blue_c);
                    alpha += min(min(alpha_a,alpha_b),alpha_c);

                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                }
                _ => {}  // None
            }
            outbuf[outptr] = red;
            outbuf[outptr+1] = green;
            outbuf[outptr+2] = blue;
            outbuf[outptr+3] = alpha;
            outptr += 4;
        } 
        option.drawer.draw(0,y,header.width as usize,1,buffer,None)?;
        prev_buf = outbuf;
    }
    return Ok(None)
}

fn load_index_color(header:&PngHeader,buffer:&[u8] ,option:&mut DecodeOptions) 
-> Result<Option<ImgWarnings>,Error> {
    if header.pallete.is_none() {
        let string = "Pallte data is nothing.";
        return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, string.to_string()))) 
    }
    let pallet = header.pallete.as_ref().unwrap();
    let raw_length = (header.width * ((header.bitpersample as u32 +7 / 8)) + 1) as usize;

    let mut outbuf:Vec<u8> = (0..header.width * 4).map(|_| 0).collect();
    for y in 0..header.height as usize{
        let mut ptr = raw_length * y;
        ptr += 1;
        let mut outptr = 0;
        for x in 0..header.width as usize {
            let mut color = 0;
            match header.bitpersample {
                8 => {
                    color = buffer[ptr];
                    ptr += 1;
                },
                4 => {
                    if x % 2 == 0 {
                        color = buffer[ptr] >> 4;
                    } else {
                        color = buffer[ptr] & 0xf;
                        ptr += 1;
                    }
                },
                2 => {
                    let shift = 6 - (x % 4) * 2;
                    color = (buffer[ptr] >> shift) & 0x3;
                    if shift == 0 {
                        ptr += 1;
                    }
                },
                1 => {
                    let shift = 7 - (x % 8);
                    color = (buffer[ptr] >> shift) & 0x1;
                    if shift == 0 {
                        ptr += 1;
                    }
                },
                _ => {},
            }
            // index color also no use filter
            let color = color as usize;
            outbuf[outptr] = pallet[color].red;
            outbuf[outptr+1] = pallet[color].green;
            outbuf[outptr+2] = pallet[color].blue;
            outbuf[outptr+3] = 0xff;
            outptr += 4;
        }
        option.drawer.draw(0,y,header.width as usize,1,buffer,None)?;
    }
    return Ok(None)
}

fn load(header:&mut PngHeader,buffer:&[u8] ,option:&mut DecodeOptions) -> Result<Option<ImgWarnings>,Error> {
    match header.color_type {
        0 => {
            if header.bitpersample >= 8 {
                return load_grayscale(&header,&buffer,option)
            } else {
                let color_max = 1 << header.bitpersample;
                let mut pallet :Vec<RGBA> = Vec::new();
                for i in 1..color_max {
                    let grey = (i * 255 / (color_max - 1)) as u8;
                    pallet.push(RGBA{red:grey,green:grey,blue:grey,alpha:0xff});
                }
                header.pallete = Some(pallet);
                return load_index_color(header, buffer, option)
            }
        },
        2 => {
            return load_truecolor(&header,&buffer,option)
        },
        3 => {
            return load_index_color(&header,&buffer,option)
        },
        4 => {
            return load_grayscale(&header,&buffer,option)
        },
        6 => {
            return load_truecolor(&header,&buffer,option)
        },
        _ => {
            let string = format!("Color type {} is unknown",header.color_type);
            return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, string))) 
        }
    }
}

pub fn decode<'decode, B: BinaryReader>(reader:&mut B ,option:&mut DecodeOptions) 
                    -> Result<Option<ImgWarnings>,Error> {

    let mut header = PngHeader::new(reader,option.debug_flag)?;
    if option.debug_flag > 0 {
        let string = format!("{:?}",&header);
        option.drawer.verbose(&string,None)?;
    }
    option.drawer.init(header.width as usize,header.height as usize,None)?;

    let mut buffer: Vec<u8> = Vec::new();

    loop {
        let length = reader.read_u32_be()?;
        let ret_chunck = reader.read_bytes_as_vec(4);
        match ret_chunck {
            Ok(chunck) => {
                if chunck == super::header::IMAGE_END {
                    let decomressed = miniz_oxide::inflate::decompress_to_vec_zlib(&buffer);
                    match decomressed {
                        Ok(buffer) => {
                            load(&mut header, &buffer, option)?;
                        },
                        Err(err) => {
                            let message = format!("Uncompressed Error {:?}",err);
                            return Err(
                                Box::new(ImgError::new_const(ImgErrorKind::DecodeError,message))
                            )
                        }
                    }
                    break;
                } else if chunck == super::header::IMAGE_DATA {
                    if option.debug_flag > 1 {
                        let string = format!("read compressed image data {} bytes",length);
                        option.drawer.verbose(&string,None)?;
                    }
                    let mut buf = reader.read_bytes_as_vec(length as usize)?;
                    buffer.append(&mut buf);
                    let _crc = reader.read_u32_be()?;

//                    imagelen += length as usize;
                } else {
                    let length = reader.read_u32_be()?;
                    reader.skip_ptr(length as usize)?;
                    let _crc = reader.read_u32_be()?;
                }
            },
            Err(_) => {
                let warnings = ImgWarnings::add(None,Box::new(
                        PngWarning::new("Data crruption after image datas".to_string())));
                return Ok(warnings)
            }
        }

    }
    Ok(None)

}

