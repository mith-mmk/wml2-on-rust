type Error = Box<dyn std::error::Error>;
use crate::metadata::DataMap;
use bin_rs::io::*;
use bin_rs::Endian;
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
    let mut buf = Vec::with_capacity(0x80 + width as usize * height as usize * 3);
    
    let mut endian = Endian::LittleEndian;

    if let Some(metadata) = &image.options {
        let meta_endian = metadata.get("endian");
        if let Some(meta_endian) = meta_endian {
            if meta_endian == &DataMap::Ascii("Big Endian".to_string()) {
                endian = Endian::BigEndian;
            }
        }
    }

    if endian == Endian::LittleEndian {
        buf.push(b'I');
        buf.push(b'I');
    } else {
        buf.push(b'M');
        buf.push(b'M');
    }

    write_u16(42,&mut buf,endian);       // 0002 version
    write_u32(8,&mut buf,endian);        // 0004 IFD Offset

    write_u16(9,&mut buf,endian);      // 0008 Tag number 

    write_u16(0x0100,&mut buf,endian);   // Tag ImageWidth
    write_u16(3,&mut buf,endian);        // data type Long
    write_u32(1,&mut buf,endian);        // count
    write_u16(width as u16,&mut buf,endian);    // Value or offset
    write_u16(0,&mut buf,endian);       // padding

    write_u16(0x0101,&mut buf,endian);   // Tag ImageHeight
    write_u16(3,&mut buf,endian);        // data type Long
    write_u32(1,&mut buf,endian);        // count
    write_u16(height as u16,&mut buf,endian);    // Value or offset
    write_u16(0,&mut buf,endian);       // padding

    write_u16(0x0102,&mut buf,endian);   // BitsPerSample
    write_u16(3,&mut buf,endian);        // data type Short
    write_u32(3,&mut buf,endian);        // count
    write_u32(0x007a,&mut buf,endian);      // offset

    write_u16(0x0103,&mut buf,endian);   // Compression
    write_u16(3,&mut buf,endian);        // data type Short
    write_u32(1,&mut buf,endian);        // count
    write_u16(1,&mut buf,endian);        // Compression None
    write_u16(0,&mut buf,endian);        // Padding

    write_u16(0x0106,&mut buf,endian);   // PhotometricInterpretation
    write_u16(3,&mut buf,endian);        // data type Short
    write_u32(1,&mut buf,endian);        // count
    write_u16(2,&mut buf,endian);        // RGB Full color
    write_u16(0,&mut buf,endian);        // Padding

    write_u16(0x0111,&mut buf,endian);   // StripOffsets
    write_u16(3,&mut buf,endian);        // data type Short
    write_u32(1,&mut buf,endian);        // count
    write_u32(0x80,&mut buf,endian);      // offset

    write_u16(0x0115,&mut buf,endian);   // SamplesPerPixel
    write_u16(3,&mut buf,endian);        // data type Short
    write_u32(1,&mut buf,endian);        // 
    write_u32(3,&mut buf,endian);        // 3

    write_u16(0x0116,&mut buf,endian);   // RowsPerStrip
    write_u16(4,&mut buf,endian);        // data type Long
    write_u32(1,&mut buf,endian);        // 
    write_u32(height,&mut buf,endian);   // height

    write_u16(0x0117,&mut buf,endian);   // StripsByCount
    write_u16(4,&mut buf,endian);        // data type Long
    write_u32(1,&mut buf,endian);        // 
    let size = height * width * 3;
    write_u32(size,&mut buf,endian);     // height * width * 3

    write_u32(0,&mut buf,endian);        // idf end

    // offset +007A
    write_u16(8,&mut buf,endian);        // BitPerSample[0]
    write_u16(8,&mut buf,endian);        // BitPerSample[1]
    write_u16(8,&mut buf,endian);        // BitPerSample[2]
    
    // offset +0080
    
    for y in 0..height {
        let data = image.drawer.encode_pick(0,y as usize ,width as usize,1,None)?.unwrap_or(vec![0;width as usize *3]);
        let mut ptr = 0;
        for _ in 0..width {
            let red  = data[ptr];
            let green= data[ptr+1];
            let blue = data[ptr+2];
    //        let alpha = buf[ptr+3];
    
            buf.push(red);
            buf.push(green);
            buf.push(blue);
    //        data.push(alpha);
            ptr += 4;
        }    
    }
    image.drawer.encode_end(None)?;
    Ok(buf)
}
