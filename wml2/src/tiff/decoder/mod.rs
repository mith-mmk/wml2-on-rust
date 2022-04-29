//!
//! TIFF Decoder No test
//! 

type Error = Box<dyn std::error::Error>;
use crate::draw::CallbackResponse;
use crate::draw::ImageBuffer;
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
mod packbits;

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
    if data.len() == 0 {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Data empty.".to_string())));
    }
    if header.photometric_interpretation >= 4{
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"This decoder is not support this color modelBuffer overrun in draw.".to_string())));
    }
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
    if header.bitpersample <= 8 {
        if color_table.is_none() {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"This is an index color image,but A color table is empty.".to_string())));
        }
    }
    option.drawer.init(header.width as usize,header.height as usize,None)?;
    let mut i = 0;

    for y in 0..header.height as usize {
        let mut buf = vec![];
        for _ in 0..header.width as usize {
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
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"This bit per sample is not support.".to_string())));
                }
            }
        }
        option.drawer.draw(0,y,header.width as usize,1,&buf,None)?;
        if i > data.len() {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Buffer overrun in draw.".to_string())));
        }
    }
    Ok(None)
}

fn read_strips<'decode,B: BinaryReader>(reader:&mut B,header: &Tiff) -> Result<Vec<u8>,Error> {
    let mut data = vec![];
    if header.strip_offsets.len() != header.strip_byte_counts.len() {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Mismach length, image strip offsets and strib byte counts.".to_string())));
    }
    for (i,offset) in header.strip_offsets.iter().enumerate() {
        reader.seek(std::io::SeekFrom::Start(*offset as u64))?;
        let mut buf = reader.read_bytes_as_vec(header.strip_byte_counts[i] as usize)?;
        data.append(&mut buf);
    }
    Ok(data)
}

pub fn decode_lzw_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {
    let data = read_strips(reader, header)?;

    let is_lsb = if header.fill_order == 2 { true } else {false}; // 1: MSB 2: LSB
    let mut decoder = Lzwdecode::tiff(is_lsb);

    let data = decoder.decode(&data)?;
    let warnings = draw(&data,option,header)?;
    Ok(warnings)
}


pub fn decode_packbits_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {
    let data = read_strips(reader, header)?;
    let data = packbits::decode(&data)?;
    let warnings = draw(&data,option,header)?;
    Ok(warnings)
}

pub fn decode_none_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {

    let data = read_strips(reader, header)?;
    let warnings = draw(&data,option,header)?;
    Ok(warnings)
}

fn verbose(str:&str) -> Result<Option<CallbackResponse>,Error> {
    println!("{}", str);
    Ok(None)
}

pub fn decode_jpeg_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {

    let mut jpeg_tables:Option<Vec<u8>> = None;
    for header in &header.tiff_headers.headers {
        if header.tagid == 0x015b {
            if let DataPack::Undef(data) = &header.data {
                jpeg_tables = Some(data.to_vec())
            }
        }
        if header.tagid == 0x8773 {
            if let DataPack::Undef(data) = &header.data {

                let mut f = std::fs::File::create("/tools/icc.icc").unwrap();
                use std::io::Write;
                f.write_all(data).unwrap();
                f.flush().unwrap();
            }

        }
    }
    let len = jpeg_tables.as_ref().unwrap().len() - 2;
    let metadata;
    if jpeg_tables.is_none() {
        metadata = vec![];
    } else {
        metadata = (&jpeg_tables.as_ref().unwrap()[..len]).to_vec();  // remove EOI
    }
    let mut warnings:Option<ImgWarnings> = None;
    let mut y = 0;
    option.drawer.init(header.width as usize,header.height as usize,None)?;
    for (i,offset) in header.strip_offsets.iter().enumerate() {
        reader.seek(std::io::SeekFrom::Start(*offset as u64))?;
        let mut data = vec![];
        data.append(&mut metadata.to_vec());
        let buf = reader.read_bytes_as_vec(header.strip_byte_counts[i] as usize)?;
        data.append(&mut buf[2..].to_vec());    // remove SOI

        let mut image = ImageBuffer::new();
        image.set_verbose(verbose);
        let mut part_option = DecodeOptions{
            debug_flag: 0x01,
            drawer: &mut image,
        };
        let mut reader = bin_rs::reader::BytesReader::from_vec(data);
        let ws = crate::jpeg::decoder::decode(&mut reader,&mut part_option)?;
        let width = image.width;
        let height = image.height;


        if image.buffer.is_some() {
            option.drawer.draw(0,y,width,height,&image.buffer.unwrap(),None)?;
        }

        y += height;

        if let Some(ws) = ws {
            for w in ws.warnings {
                warnings = ImgWarnings::add(warnings, w);
            }        
        }
    }

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
    option.drawer.set_metadata("compression",DataMap::Ascii(header.compression.to_string()))?;

    match header.compression { 
        Compression::NoneCompression => {
            return decode_none_compresson(reader,option,&header);
        },
        Compression::LZW => {
            return decode_lzw_compresson(reader,option,&header);
        },
        Compression::Jpeg => {
            return decode_jpeg_compresson(reader,option,&header);
        },
        Compression::Packbits => {
            return decode_packbits_compresson(reader,option,&header);
        }
        _ => {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Not suport compression".to_string())));
        }
    }
}