use crate::color::RGBA;
use crate::png::warning::PngWarning;
use crate::png::header::PngHeader;
use crate::warning::*;
use crate::draw::DecodeOptions;
use crate::error::*;
use bin_rs::reader::BinaryReader;
type Error = Box<dyn std::error::Error>;

#[inline]
fn paeth(d:u8,a:i32,b:i32,c:i32) -> u8 {
    let pa = (b - c).abs();
    let pb = (a - c).abs();
    let pc = (b + a - c - c).abs();
    let d = d as i32;
    if pa <= pb && pa <= pc {
        ((d + a) & 0xff) as u8
    } else if pb <= pc {
        ((d + b) & 0xff) as u8
    } else {
        ((d + c) & 0xff) as u8
    }
}

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
            let (mut gray, mut alpha) = (0,0xff);
            match header.bitpersample {
                16 => {
                    gray = buffer[ptr];ptr += 2;
                    if is_alpha == 1 {                
                        alpha = buffer[ptr];ptr += 2;
                    } else {
                        alpha = 0xff;
                    }
                },
                8 => {
                    gray = buffer[ptr];ptr += 1;
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
                        gray   += outbuf[outptr -4];
                    }
                    if is_alpha == 1 {
                        outbuf[outptr - 1] = alpha;
                    } else {
                        outbuf[outptr+3] = 0xff;
                    }
                },
                2 => { // Up
                    if prev_buf.len() > 0 {
                        gray   += prev_buf[outptr];
                    }
                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                }
                3 => { // Avalage
                    let (mut gray_,mut alpha_);
                    if outptr > 0 {
                        gray_  = outbuf[outptr -4] as u32;
                        alpha_ = outbuf[outptr -1] as u32;
                    } else {
                        gray_  = 0;
                        alpha_ = 0;
                    }
                    if prev_buf.len() > 0 {
                        gray_   += prev_buf[outptr] as u32;
                        alpha_ += prev_buf[outptr+3] as u32;
                    } else {
                        gray_   += 0;
                        alpha_ += 0;
                    }
                    gray_ /= 2;
                    alpha_ /= 2;
                    gray  += gray_ as u8;
                    alpha += alpha_ as u8;

                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                },
                4 => { // Pease
                    let (gray_a, alpha_a);
                    if outptr > 0 {
                        gray_a  = outbuf[outptr -4] as i32;
                        alpha_a = outbuf[outptr -1] as i32;
                    } else {
                        gray_a   = 0;
                        alpha_a  = 0;
                    }
                    let (gray_b, alpha_b);
                    if prev_buf.len() > 0 {
                        gray_b  = prev_buf[outptr] as i32;
                        alpha_b = outbuf[outptr -1] as i32;
                    } else {
                        gray_b  = 0;
                        alpha_b = 0;
                    }
                    let (gray_c, alpha_c);
                    if prev_buf.len() > 0 && outptr > 0 {
                        gray_c  = prev_buf[outptr-4] as i32;
                        alpha_c = prev_buf[outptr-1] as i32;
                    } else {
                        gray_c  = 0;
                        alpha_c = 0;
                    }


                    gray  = paeth(gray,gray_a,gray_b,gray_c);
                    alpha = paeth(alpha,alpha_a,alpha_b,alpha_c);

                    if is_alpha == 0 {
                        alpha = 0xff;
                    }
                }
                _ => {}  // None
            }
            outbuf[outptr] = gray;
            outbuf[outptr+1] = gray;
            outbuf[outptr+2] = gray;
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
    let raw_length = (header.width * (header.bitpersample as u32 / 8 * (3 + is_alpha)) + 1) as usize;
    let mut prev_buf:Vec<u8> = Vec::new();

    for y in 0..header.height as usize{
        let mut ptr = raw_length * y;
        let flag = buffer[ptr];
        if option.debug_flag & 0x4 == 0x4 {
            let string = format!("Y:{} filter is {} ",y,flag);
            option.drawer.verbose(&string,None)?;
        }

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
                    if outptr > 0 {
                        red   += outbuf[outptr -4];
                        green += outbuf[outptr -3];
                        blue  += outbuf[outptr -2];
                        alpha += outbuf[outptr -1];
                    }
                },
                2 => { // Up
                    if prev_buf.len() > 0 {
                        red   += prev_buf[outptr];
                        green += prev_buf[outptr+1];
                        blue  += prev_buf[outptr+2];
                        alpha += prev_buf[outptr+3];
                    }

                }
                3 => { // Avalage
                    let (mut red_, mut green_, mut blue_, mut alpha_);
                    if outptr > 0 {
                        red_   = outbuf[outptr -4] as u32;
                        green_ = outbuf[outptr -3] as u32;
                        blue_  = outbuf[outptr -2] as u32;
                        alpha_ = outbuf[outptr -1] as u32;
                    } else {
                        red_   = 0;
                        green_ = 0;
                        blue_  = 0;
                        alpha_  = 0;
                    }
                    if prev_buf.len() > 0 {
                        red_   += prev_buf[outptr] as u32;
                        green_ += prev_buf[outptr+1] as u32;
                        blue_  += prev_buf[outptr+2] as u32;
                        alpha_ += prev_buf[outptr+3] as u32;
                    } else {
                        red_   += 0;
                        green_ += 0;
                        blue_  += 0;
                        alpha_ += 0;
                    }
                    red_ /=2;
                    green_ /= 2;
                    blue_ /=2;
                    alpha_ /=2;

                    red   += red_ as u8;
                    green += green_ as u8;
                    blue  += blue_ as u8;
                    alpha += alpha_ as u8;

                },
                4 => { // Pease
                    let (red_a, green_a, blue_a, alpha_a);
                    if outptr > 0 {
                        red_a   = outbuf[outptr -4] as i32;
                        green_a = outbuf[outptr -3] as i32;
                        blue_a  = outbuf[outptr -2] as i32;
                        alpha_a = outbuf[outptr -1] as i32;
                    } else {
                        red_a   = 0;
                        green_a = 0;
                        blue_a  = 0;
                        alpha_a  = 0;
                    }
                    let (red_b, green_b, blue_b, alpha_b);
                    if prev_buf.len() > 0 {
                        red_b   = prev_buf[outptr] as i32;
                        green_b = prev_buf[outptr+1] as i32;
                        blue_b  = prev_buf[outptr+2] as i32;
                        alpha_b = prev_buf[outptr+3] as i32;
                    } else {
                        red_b   = 0;
                        green_b = 0;
                        blue_b  = 0;
                        alpha_b = 0;
                    }
                    let (red_c, green_c, blue_c, alpha_c);
                    if prev_buf.len() > 0 && outptr > 0 {
                        red_c   = prev_buf[outptr-4] as i32;
                        green_c = prev_buf[outptr-3] as i32;
                        blue_c  = prev_buf[outptr-2] as i32;
                        alpha_c = prev_buf[outptr-1] as i32;
                    } else {
                        red_c   = 0;
                        green_c = 0;
                        blue_c  = 0;
                        alpha_c = 0;
                    }

                    red   = paeth(red,red_a,red_b,red_c);
                    green = paeth(green,green_a,green_b,green_c);
                    blue  = paeth(blue,blue_a,blue_b,blue_c);
                    alpha = paeth(alpha,alpha_a,alpha_b,alpha_c);

                }
                _ => {}  // None
            }
            outbuf[outptr] = red;
            outbuf[outptr+1] = green;
            outbuf[outptr+2] = blue;            
            if is_alpha == 0 {
                alpha = 0xff;
            }
            outbuf[outptr+3] = alpha;
            outptr += 4;
        } 
        option.drawer.draw(0,y,header.width as usize,1,&outbuf,None)?;
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
        option.drawer.draw(0,y,header.width as usize,1,&outbuf,None)?;
    }
    return Ok(None)
}

fn load(header:&mut PngHeader,buffer:&[u8] ,option:&mut DecodeOptions) -> Result<Option<ImgWarnings>,Error> {
    match header.color_type {
        0 => {
            option.drawer.verbose("Glayscale",None)?;
            if header.bitpersample >= 8 {
                return load_grayscale(&header,&buffer,option)
            } else {
                let color_max = 1 << header.bitpersample;
                let mut pallet :Vec<RGBA> = Vec::new();
                for i in 1..color_max {
                    let gray = (i * 255 / (color_max - 1)) as u8;
                    pallet.push(RGBA{red:gray,green:gray,blue:gray,alpha:0xff});
                }
                header.pallete = Some(pallet);
                return load_index_color(header, buffer, option)
            }
        },
        2 => {
            option.drawer.verbose("True Color",None)?;
            return load_truecolor(&header,&buffer,option)
        },
        3 => {
            option.drawer.verbose("Index Color",None)?;
            return load_index_color(&header,&buffer,option)
        },
        4 => {
            option.drawer.verbose("Grayscale with alpha",None)?;
            return load_grayscale(&header,&buffer,option)
        },
        6 => {
            option.drawer.verbose("Truecolor with alpha",None)?;
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
    if option.debug_flag > 1 {
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
                        Ok(debuffer) => {
                            load(&mut header, &debuffer, option)?;
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
                } else {
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

