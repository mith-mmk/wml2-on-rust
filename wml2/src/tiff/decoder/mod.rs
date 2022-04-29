//!
//! TIFF Decoder No test
//! 

type Error = Box<dyn std::error::Error>;
use bin_rs::io::read_u16;
use crate::color::RGBA;
use crate::decoder::lzw::Lzwdecode;
use crate::metadata::DataMap;
use crate::tiff::header::*;
use crate::warning::ImgWarnings;
use crate::draw::DecodeOptions;
use bin_rs::reader::BinaryReader;
use crate::error::ImgError;
use crate::error::ImgErrorKind;

fn create_pallet(bits:usize,is_black_zero:bool) -> Vec<RGBA>{
    let color_max = 1 << bits;
    let mut pallet = Vec::with_capacity(color_max);

    if is_black_zero {
        for i in 0..color_max {
            let gray = (i * 255 / (color_max - 1)) as u8;
            pallet.push(RGBA{red:gray,green:gray,blue:gray,alpha:0xff});
        }
    } else {
        for i in 0..color_max {
            let gray = 255 - ((i * 255 / (color_max - 1)) as u8);
            pallet.push(RGBA{red:gray,green:gray,blue:gray,alpha:0xff});
        }
    }
    pallet
}

pub fn draw(data:&[u8],option:&mut DecodeOptions,header: &Tiff) -> Result<Option<ImgWarnings>,Error> {
    if header.photometric_interpretation >= 4{
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Decoder is not support this color modelBuffer overrun in draw.".to_string())));
    }
    option.drawer.init(header.width as usize,header.height as usize,None)?;
    let mut i = 0;
    let color_table: Option<Vec<RGBA>> = 
        if let Some(color_table) = header.color_table.as_ref() {
            Some(color_table.to_vec())
        } else {
            match header.photometric_interpretation {
                0 => {  // WhiteIsZero
                    Some(create_pallet(header.bitpersample as usize, false))
                },
                1 => {  // BlackIsZero
                    Some(create_pallet(header.bitpersample as usize, true))
                },
                3 => {  // RGB Palette
                    Some(create_pallet(header.bitpersample as usize, true))
                },
                _ => {
                    None
                }
            }
        };

    for y in 0..header.height as usize {
        let mut buf = vec![];
        for _ in 0..header.width as usize {
            if i >= data.len() {
                return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Buffer overrun in draw.".to_string())));
            }
            match header.bitpersample {
                24 => {
                    let r = data[i];
                    let g = data[i+1];
                    let b = data[i+2];
                    buf.push(r);
                    buf.push(g);
                    buf.push(b);
                    buf.push(0xff);
                    i += 3;
                },
                15 => {
                    let color = read_u16(data,i,header.tiff_headers.endian);
                    let r = (color >> 10) & 0x1f;
                    let g = (color >> 10) & 0x1f;
                    let b = (color >> 10) & 0x1f;
                    let r = ((r << 3) | (r >> 2)) as u8;
                    let g = ((g << 3) | (g >> 2)) as u8;
                    let b = ((b << 3) | (b >> 2)) as u8;
                    buf.push(r);
                    buf.push(g);
                    buf.push(b);
                    buf.push(0xff);
                    i += 2;
                },
                8 => {
                    let color = data[i];
                    let rgba = &color_table.as_ref().unwrap()[color as usize];

                    buf.push(rgba.red);
                    buf.push(rgba.green);
                    buf.push(rgba.blue);
                    buf.push(rgba.alpha);
                    i += 1;
                }
                4 => {
                    let c;
                    let color = data[i/2];
                    if i % 2 == 0 {
                        if header.fill_order == 1 {
                            c = (color >> 4) as usize;
                        } else {
                            c = (color & 0xf) as usize;
                        }
                    } else {
                        if header.fill_order == 1 {
                            c = (color & 0xf) as usize;
                        } else {
                            c = (color >> 4) as usize;
                        }
                    }
                    
                    let rgba = &color_table.as_ref().unwrap()[c];

                    buf.push(rgba.red);
                    buf.push(rgba.green);
                    buf.push(rgba.blue);
                    buf.push(rgba.alpha);
                    i += 1;
                },
                2 => {
                    let c;
                    let color = data[i/4];
                    let shift = (i % 4) * 2;
                    if header.fill_order == 1 {
                            c = ((color >> (6 - shift)) & 0x3) as usize;
                    } else {
                            c = ((color >> shift)& 0x3) as usize;
                    }
                    
                    let rgba = &color_table.as_ref().unwrap()[c];

                    buf.push(rgba.red);
                    buf.push(rgba.green);
                    buf.push(rgba.blue);
                    buf.push(rgba.alpha);
                    i += 1;
                },
                1 => {
                    let c;
                    let color = data[i/8];
                    let shift = i % 8;
                    if header.fill_order == 1 {
                            c = ((color >> (7 - shift)) & 0x1) as usize;
                    } else {
                            c = ((color >> shift)& 0x1) as usize;
                    }
                    
                    let rgba = &color_table.as_ref().unwrap()[c];

                    buf.push(rgba.red);
                    buf.push(rgba.green);
                    buf.push(rgba.blue);
                    buf.push(rgba.alpha);
                    i += 1;
                },

                _ => {

                }
            }
        }
        option.drawer.draw(0,y,header.width as usize,1,&buf,None)?;
    }
    Ok(None)
}

// has bug Lzwdecode::tiff
pub fn decode_lzw_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {
//    let buf = reader.read_bytes_as_vec()
    let is_lsb = if header.fill_order == 2 { true } else {false};
    let mut data = vec![];
    for (i,offset) in header.strip_offsets.iter().enumerate() {
        reader.seek(std::io::SeekFrom::Start(*offset as u64))?;
        let mut buf = reader.read_bytes_as_vec(header.strip_byte_counts[i] as usize)?;
        data.append(&mut buf);
    }

    let mut decoder = Lzwdecode::tiff(8,is_lsb);

    let data = decoder.decode(&data)?;
    let warnings = draw(&data,option,header)?;
    Ok(warnings)
}

pub fn decode_none_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {
    if header.bitpersample <=8 {
        if header.color_table.is_none() {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Not suport index image without color table.".to_string())));
        } else {
            let colors = 1 << header.bitpersample;
            if header.color_table.as_ref().unwrap().len() < colors {
                return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"A color table is shortage.".to_string())));

            }
        }
    }
    let mut data = vec![];
    for (i,offset) in header.strip_offsets.iter().enumerate() {
        reader.seek(std::io::SeekFrom::Start(*offset as u64))?;
        let mut buf = reader.read_bytes_as_vec(header.strip_byte_counts[i] as usize)?;
        data.append(&mut buf);
    }

    let warnings = draw(&data,option,header)?;

    Ok(warnings)
}

pub fn decode<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions) -> Result<Option<ImgWarnings>,Error> {

    let header = Tiff::new(reader)?;

    option.drawer.set_metadata("Format",DataMap::Ascii("Tiff".to_owned()))?;
//    let mut map = super::util::make_metadata(&header.tiff_headers);

    option.drawer.set_metadata("width",DataMap::UInt(header.width as u64))?;
    option.drawer.set_metadata("height",DataMap::UInt(header.height as u64))?;
    option.drawer.set_metadata("bits per pixel",DataMap::UInt(header.bitpersample as u64))?;
    option.drawer.set_metadata("Tiff headers",DataMap::Exif(header.tiff_headers.clone()))?;

    match header.compression { 
        Compression::NoneCompression => {
            option.drawer.set_metadata("compression",DataMap::Ascii("None".to_owned()))?;
            return decode_none_compresson(reader,option,&header);
        },
        Compression::LZW => {
            option.drawer.set_metadata("compression",DataMap::Ascii("LZW".to_owned()))?;
            return decode_lzw_compresson(reader,option,&header);
        },
        Compression::Jpeg => {
            option.drawer.set_metadata("compression",DataMap::Ascii("JPEG".to_owned()))?;
            return crate::jpeg::decoder::decode(reader,option)
        },

        _ => {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Not suport compression".to_string())));
        }
    }
}