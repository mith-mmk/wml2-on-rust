type Error = Box<dyn std::error::Error>;
use bin_rs::io::*;
use super::header::*;
use crate::error::*;
use crate::draw::*;

pub fn encode(image: &mut EncodeOptions<'_>) -> Result<Vec<u8>,Error> {
    let profile = image.drawer.encode_start(None)?;
    let width;
    let height;
    let _background;
    if let Some(profile) = profile {
        width = profile.width as u32;
        height= profile.height as u32;
        _background = profile.background;
    } else {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::OutboundIndex,"Image profiles nothing".to_string())))
    }
    let bit_count = 24;
    let raw_samples = (width * bit_count + 31) / 32 * 4;
    let gap = raw_samples - (width * (bit_count / 8));
    let buffersize = raw_samples * height;

    let bitmap_file_header = BitmapFileHeader {
        bf_type : 0x4d42,
        bf_size  :buffersize + 54,
        bf_reserved1: 0,
        bf_reserved2: 0,
        bf_offbits: 54,
    };

    let header = BitmapWindowsInfo {
        bi_size : 40,
        bi_width : width,
        bi_height : height,
        bi_plane : 1,
        bi_bit_count : bit_count as u16,
        bi_compression : Compressions::BiRGB as u32,
        bi_size_image : buffersize,
        bi_xpels_per_meter : 0,
        bi_ypels_per_meter : 0,
        bi_clr_used : 0,
        bi_clr_importation: 0,
        b_v4_header : None,
        b_v5_header : None,
    };

    let mut data: Vec<u8> = Vec::with_capacity((bitmap_file_header.bf_offbits + header.bi_size_image) as usize);
    write_u16_le(bitmap_file_header.bf_type,&mut data);
    write_u32_le(bitmap_file_header.bf_size,&mut data);
    write_u16_le(bitmap_file_header.bf_reserved1,&mut data);
    write_u16_le(bitmap_file_header.bf_reserved2,&mut data);
    write_u32_le(bitmap_file_header.bf_offbits,&mut data);

    write_u32_le(header.bi_size,&mut data);
    write_u32_le(header.bi_width,&mut data);
    write_u32_le(header.bi_height,&mut data);
    write_u16_le(header.bi_plane,&mut data);
    write_u16_le(header.bi_bit_count,&mut data);
    write_u32_le(header.bi_compression,&mut data);
    write_u32_le(header.bi_size_image,&mut data);
    write_u32_le(header.bi_xpels_per_meter,&mut data);
    write_u32_le(header.bi_ypels_per_meter,&mut data);
    write_u32_le(header.bi_clr_used,&mut data);
    write_u32_le(header.bi_clr_importation,&mut data);

    
    for y in 0..height {
        let bmp_y = height - y - 1;
        let buf = image.drawer.encode_pick(0,bmp_y as usize,width as usize,1,None)?.unwrap_or(vec![0]);
        let mut ptr = 0;
        for _ in 0..width {
            let blue = buf[ptr];
            let green= buf[ptr+1];
            let red  = buf[ptr+2];
//            let alpha = buf[ptr+3];

            data.push(red);
            data.push(green);
            data.push(blue);
//            data.push(alpha);
            ptr += 4;
        }
        for _ in 0..gap {
            data.push(0);
        }
    }
    Ok(data)
}
