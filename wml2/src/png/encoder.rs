use crate::draw::*;
use crate::error::*;
use crate::png::header::*;
use crate::png::utils::*;
use bin_rs::io::*;
type Error = Box<dyn std::error::Error>;

pub fn encode(image: &mut EncodeOptions<'_>) -> Result<Vec<u8>,Error> {
    let profile = image.drawer.encode_start(None)?;
    let width;
    let height;
    let background;
    if let Some(profile) = profile {
        width = profile.width as u32;
        height= profile.height as u32;
        background = profile.background;
    } else {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::OutboundIndex,"Image profiles nothing".to_string())))
    }

    let crc32 = CRC32::new();
    let mut write_buffer:Vec<u8> = Vec::new();
    write_bytes(&SIGNATURE,&mut write_buffer);

    write_u32_be(13,&mut write_buffer);  //alway 13

    let mut temp_buffer:Vec<u8> = Vec::with_capacity(20);
    write_bytes(&IMAGE_HEADER,&mut temp_buffer);
    write_u32_be(width,&mut temp_buffer);
    write_u32_be(height,&mut temp_buffer);
    write_byte(8,&mut temp_buffer); // 8bit depth
    write_byte(6,&mut temp_buffer); // True Color + alpha
    write_byte(0,&mut temp_buffer); // only zero deflate
    write_byte(0,&mut temp_buffer); // only zero filter
    write_byte(0,&mut temp_buffer); // no interlace
   
    //    write_header;
    write_bytes(&temp_buffer, &mut write_buffer);
    let crc = crc32.crc32(&temp_buffer);
    write_u32_be(crc,&mut write_buffer);

    if let Some(background) = background {
        let red = background >> 24;
        let green = background >> 16;
        let blue = background >> 8;
        write_u32_be(6,&mut write_buffer); // 2byte * 3;
        let mut temp_buffer:Vec<u8> = Vec::with_capacity(10);
        write_bytes(&BACKGROUND_COLOR,&mut temp_buffer);
        write_u32_be(red,&mut temp_buffer);
        write_u32_be(green,&mut temp_buffer);
        write_u32_be(blue,&mut temp_buffer);
        write_bytes(&temp_buffer, &mut write_buffer);
        let crc = crc32.crc32(&temp_buffer);
        write_u32_be(crc,&mut write_buffer);
    }

    let mut prev_buf:Vec<u8> = Vec::new();

    let mut data = vec![];

    for y in 0..height {
        let buf = image.drawer.encode_pick(0,y as usize,width as usize,1,None)?.unwrap_or(vec![0]);
        let mut inptr = 0;
        if buf.len() < width as usize * 4 {
            let boxstr = format!("data shotage width {} but {}",width,buf.len());
            return Err(Box::new(ImgError::new_const(ImgErrorKind::EncodeError, boxstr))) 
        }
        data.push(4_u8);
        for _ in 0..width {
            let mut red   = buf[inptr];
            let mut green = buf[inptr+1];
            let mut blue  = buf[inptr+2];
            let mut alpha = buf[inptr+3];
            let (red_a, green_a, blue_a, alpha_a);
            if inptr > 0 {
                red_a   = buf[inptr -4] as i32;
                green_a = buf[inptr -3] as i32;
                blue_a  = buf[inptr -2] as i32;
                alpha_a = buf[inptr -1] as i32;
            } else {
                red_a   = 0;
                green_a = 0;
                blue_a  = 0;
                alpha_a  = 0;
            }
            let (red_b, green_b, blue_b, alpha_b);
            if prev_buf.len() > 0 {
                red_b   = prev_buf[inptr] as i32;
                green_b = prev_buf[inptr+1] as i32;
                blue_b  = prev_buf[inptr+2] as i32;
                alpha_b = prev_buf[inptr+3] as i32;
            } else {
                red_b   = 0;
                green_b = 0;
                blue_b  = 0;
                alpha_b = 0;
            }
            let (red_c, green_c, blue_c, alpha_c);
            if prev_buf.len() > 0 && inptr > 0 {
                red_c   = prev_buf[inptr-4] as i32;
                green_c = prev_buf[inptr-3] as i32;
                blue_c  = prev_buf[inptr-2] as i32;
                alpha_c = prev_buf[inptr-1] as i32;
            } else {
                red_c   = 0;
                green_c = 0;
                blue_c  = 0;
                alpha_c = 0;
            }
            red   = paeth_enc(red,red_a,red_b,red_c);
            green = paeth_enc(green,green_a,green_b,green_c);
            blue  = paeth_enc(blue,blue_a,blue_b,blue_c);
            alpha = paeth_enc(alpha,alpha_a,alpha_b,alpha_c);

            data.push(red);
            data.push(green);
            data.push(blue);
            data.push(alpha);
            inptr += 4;
        }
        prev_buf = buf;
    }

    let idat = miniz_oxide::deflate::compress_to_vec_zlib(&data, 8);
    let mut temp_buffer = vec![];
    write_bytes(&IMAGE_DATA, &mut temp_buffer);
    write_bytes(&idat, &mut temp_buffer);
    write_u32_be(temp_buffer.len()as u32 - 4 ,&mut write_buffer);
    write_bytes(&temp_buffer,&mut write_buffer);
    let crc = crc32.crc32(&temp_buffer);
    write_u32_be(crc,&mut write_buffer);

    write_u32_be(0,&mut write_buffer);
    write_bytes(&IMAGE_END,&mut write_buffer);
    let crc = crc32.crc32(&IMAGE_END);
    write_u32_be(crc,&mut write_buffer);
    image.drawer.encode_end(None)?;
    Ok(write_buffer)
}
