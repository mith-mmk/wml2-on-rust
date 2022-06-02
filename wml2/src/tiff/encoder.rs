type Error = Box<dyn std::error::Error>;
use bin_rs::Endian;
use crate::tiff::header::*;
use crate::metadata::DataMap;
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
    let mut meta_tiff:Option<&TiffHeaders> = None;

    if let Some(metadata) = &image.options {
        let meta = metadata.get("endian");
        if let Some(meta_endian) = meta {
            if meta_endian == &DataMap::Ascii("Big Endian".to_string()) {
                endian = Endian::BigEndian;
            }
        }
        let meta = metadata.get("EXIF");
        if let Some(meta_exif) = meta {
            if let DataMap::Exif(tiff) = meta_exif {
                meta_tiff = Some(tiff);
            }
        }

    }

    let mut tiff =TiffHeaders{version:42,headers:Vec::new(),exif:None,gps:None,endian};

    if let Some(meta_tiff) = meta_tiff {
        tiff.exif = meta_tiff.exif.clone();
        tiff.gps = meta_tiff.gps.clone();
    }

    write_header(&mut buf,&tiff)?;

    let header = TiffHeader{
        tagid: 0x0100,
        data: DataPack::Long([width as u32].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0101,
        data: DataPack::Long([height as u32].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0102,
        data: DataPack::Short([8,8,8].to_vec()),
        length: 3,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0103,
        data: DataPack::Short([1].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0106,
        data: DataPack::Short([2].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0111,
        data: DataPack::Short([0].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0115,
        data: DataPack::Short([3].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0116,
        data: DataPack::Long([height as u32].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0117,
        data: DataPack::Long([(height * width * 3) as u32].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x011a,
        data: DataPack::Rational([Rational{n:72,d:1}].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x011b,
        data: DataPack::Rational([Rational{n:72,d:1}].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    let header = TiffHeader{
        tagid: 0x0128,
        data: DataPack::Short([2].to_vec()),
        length: 1,
    };
    tiff.headers.push(header);

    write_ifd(&mut buf,&tiff)?;
    
    for y in 0..height {
        let data = image.drawer.encode_pick(0,y as usize ,width as usize,1,None)?.unwrap_or(vec![0;width as usize *3]);
        let mut ptr = 0;
        for _ in 0..width {
            let red  = data[ptr];
            let green= data[ptr+1];
            let blue = data[ptr+2];
    
            buf.push(red);
            buf.push(green);
            buf.push(blue);
            ptr += 4;
        }    
    }
    image.drawer.encode_end(None)?;
    Ok(buf)
}
