//!
//! TIFF Decoder is support No Compress,LZW,Pack bits,JPEG(new style),Adobe Deflate images.
//! 
//! 

type Error = Box<dyn std::error::Error>;
use bin_rs::io::read_u32;
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
use self::jpeg::decode_jpeg_compresson;
mod packbits;
mod jpeg;
mod ccitt;

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

fn planar_to_chuncky(data:&[u8],header: &Tiff) -> Result<Vec<u8>,Error> {
    let mut buf = vec![];
    let length = header.height as usize * header.width as usize * header.bitspersamples[0] as usize / 8;
    if buf.len() < length * header.samples_per_pixel as usize {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Data shotage.".to_string())));
    }

    for i in 0..length {
        for j in 0..header.samples_per_pixel as usize {
            buf.push(data[i + j * length]);
        }
    }

    Ok(buf)
}

pub fn draw(data:&[u8],option:&mut DecodeOptions,header: &Tiff) -> Result<Option<ImgWarnings>,Error> {
    if data.len() == 0 {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Data empty.".to_string())));
    }

    let mut data = data.to_owned();

    // no debug
    if header.planar_config == 2 && header.samples_per_pixel > 1 {
        data = planar_to_chuncky(&data,header)?;
    }
    
    let color_table: Option<Vec<RGBA>> = 
        if let Some(color_table) = header.color_table.as_ref() {
            Some(color_table.to_vec())
        } else {
            let bitspersample = if header.bitspersample >= 8 { 8 } else { header.bitspersample };
            match header.photometric_interpretation {
                0 => {  // WhiteIsZero
                    Some(create_pallet(bitspersample as usize, false))
                },
                1 => {  // BlackIsZero
                    Some(create_pallet(bitspersample as usize, true))
                },
                2 => {
                    if header.samples_per_pixel < 3 {
                        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"RGB image needs Sample per pixel >=3.".to_string())));
                    } else {
                        None
                    }
                },
                3 => {  // RGB Palette
                    Some(create_pallet(bitspersample as usize, true))
                },
                5 => {
                    if header.samples_per_pixel < 4 {
                        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"YMCK image needs Sample per pixel >=4.".to_string())));
                    } else {
                        None
                    }
                },
                6 => {
                    if header.samples_per_pixel < 3 {
                        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"YCbCr image needs Sample per pixel >=3.".to_string())));
                    } else {
                        None
                    }
                },
                _ => {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Not Supprt Color model.".to_string())));
                }
            }
        };
    if header.bitspersample <= 8 {
        if color_table.is_none() {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"This is an index color image,but A color table is empty.".to_string())));
        }
    }
    option.drawer.init(header.width as usize,header.height as usize,None)?;
    let mut i = 0;

    for y in 0..header.height as usize {
        let mut buf = vec![];
        let mut prevs = vec![0_u8;header.samples_per_pixel as usize];
        for _ in 0..header.width as usize {

            match header.photometric_interpretation {
                0 | 1 | 3 => {
                    match header.bitspersamples[0] {
                        16 => { // Illegal tiff(TOWNS TIFF)
                            if header.max_sample_values[0] == 32767 && header.tiff_headers.endian == bin_rs::Endian::LittleEndian {
                                let color = read_u16(&data,i,header.tiff_headers.endian) >> 8;
                                let temp_r = (color >> 5 & 0x1f) as u8;
                                let r = (temp_r <<3 | temp_r >>2) as u8;
                                let temp_g = (color >> 10 & 0x1f) as u8;
                                let g = (temp_g <<3 | temp_g>>2) as u8;
                                let temp_b = (color & 0x1f) as u8;
                                let b = (temp_b <<3 | temp_b>>2) as u8;
                                buf.push(r);
                                buf.push(g);
                                buf.push(b);
                                buf.push(0xff);
                                i += 2;
                            } else {    // 16 bit glayscale
                                let mut color = data[i];
                                if header.predictor == 2 {
                                    color += prevs[0];
                                    prevs[0] = color; 
                                }
                                buf.push(color);
                                buf.push(color);
                                buf.push(color);
                                buf.push(color);
                                i += header.samples_per_pixel as usize;
                            }
                        },
                        8 => {
                            let mut color = data[i];
                            if header.predictor == 2 {
                                color += prevs[0];
                                prevs[0] = color; 
                            }
        
                            let rgba = &color_table.as_ref().unwrap()[color as usize];
        
                            buf.push(rgba.red);
                            buf.push(rgba.green);
                            buf.push(rgba.blue);
                            buf.push(rgba.alpha);
                            i += header.samples_per_pixel as usize;
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
                            i += 1; //  i += header.samples_per_pixel as usize; ?
                        },
                        2 => {  // usually illegal
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
                
                },
                2 => {  //RGB
                    let (mut r, mut g, mut b, mut a) = (0,0,0,0xff);
                    match header.bitspersamples[0] {  //bit per samples same (8,8,8), but also (8,16,8) pattern 
                        8 => {
                            r = data[i];
                            g = data[i+1];
                            b = data[i+2];
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 3 {
                                data[i+3] } else { 0xff };
                            i += header.samples_per_pixel as usize;
                        },
                        16 => {
                            if header.samples_per_pixel >= 3 {
                                r = (read_u16(&data,i,header.tiff_headers.endian) >> 8) as u8;
                                g = (read_u16(&data,i+2,header.tiff_headers.endian) >> 8) as u8;
                                b = (read_u16(&data,i+4,header.tiff_headers.endian) >> 8) as u8;
                                a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                            && header.samples_per_pixel > 3 {
                                        (read_u16(&data,i+6,header.tiff_headers.endian) >> 8) as u8 } else { 0xff };
                                i += header.samples_per_pixel as usize * 2;
                            }
                        },
                        32 => {
                            r = (read_u32(&data,i,header.tiff_headers.endian) >> 24) as u8;
                            g = (read_u32(&data,i+4,header.tiff_headers.endian) >> 24) as u8;
                            b = (read_u32(&data,i+8,header.tiff_headers.endian) >> 24) as u8;
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 3 {
                                    (read_u32(&data,i+12,header.tiff_headers.endian) >> 24) as u8 } else { 0xff };
                            i += header.samples_per_pixel as usize * 4;
                        },
                        _ => {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"This bit per sample is not support.".to_string())));
                        }
                    }

                    if header.predictor == 2 {
                        r += prevs[0];
                        prevs[0] = r; 
                        g += prevs[1];
                        prevs[1] = g; 
                        b += prevs[2];
                        prevs[2] = b;
                        if header.extra_samples.len() > 0 && header.extra_samples[0]  == 2 {
                            a += prevs[3];
                            prevs[3] = a;
                        }
                    }
                    buf.push(r);
                    buf.push(g);
                    buf.push(b);
                    buf.push(a);
 
                }
                // 4 : Transparentary musk is not support
                5 => {  //CMYK
                    let (mut c, mut m, mut y, mut k,mut a);
                    match header.bitspersamples[0] {  //bit per samples same (8,8,8), but also (8,16,8) pattern 
                        8 => {
                            c = data[i];
                            m = data[i+1];
                            y = data[i+2];
                            k = data[i+3];
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                && header.samples_per_pixel > 4 { data[i+4] } else { 0xff };
                            i += header.samples_per_pixel as usize;
                        },
                        16 => {
                            c = (read_u16(&data,i,header.tiff_headers.endian) >> 8) as u8;
                            m = (read_u16(&data,i+2,header.tiff_headers.endian) >> 8) as u8;
                            y = (read_u16(&data,i+4,header.tiff_headers.endian) >> 8) as u8;
                            k = (read_u16(&data,i+6,header.tiff_headers.endian) >> 8) as u8;
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 4 {
                                    (read_u16(&data,i+8,header.tiff_headers.endian) >> 8) as u8 } else { 0xff };
                            i += header.samples_per_pixel as usize * 2;
                        },
                        32 => {
                            c = (read_u32(&data,i,header.tiff_headers.endian) >> 24) as u8;
                            m = (read_u32(&data,i+4,header.tiff_headers.endian) >> 24) as u8;
                            y = (read_u32(&data,i+8,header.tiff_headers.endian) >> 24) as u8;
                            k = (read_u32(&data,i+12,header.tiff_headers.endian) >> 24) as u8;
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 4 {
                                    (read_u32(&data,i+16,header.tiff_headers.endian) >> 24) as u8 } else { 0xff };
                            i += header.samples_per_pixel as usize * 4;
                        },
                        _ => {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"This bit per sample is not support.".to_string())));
                        }
                    }

                    if header.predictor == 2 {
                        y += prevs[0];
                        prevs[0] = y; 
                        m += prevs[1];
                        prevs[1] = m; 
                        c += prevs[2];
                        prevs[2] = c;
                        k += prevs[3];
                        prevs[3] = k;
                        if header.extra_samples.len() > 0 && header.extra_samples[0]  == 2 {
                            a += prevs[4];
                            prevs[4] = a;
                        }
                    }
                    let r = 255 - c;
                    let g = 255 - m;
                    let b = 255 - y;

                    buf.push(r);
                    buf.push(g);
                    buf.push(b);
                    buf.push(a);
                },
                // not support 
                /*
                6 => {  // YCbCr ... this function is not suport YCbCrCoficients,positioning...,yet.  ....4:1:1 sampling
                    let (mut y, mut cb, mut cr, mut a);
                    match header.bitspersamples[0] {  //bit per samples same (8,8,8), but also (8,16,8) pattern 
                        8 => {
                            y = data[i];
                            cb = data[i+1];
                            cr = data[i+2];
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 3 {
                                data[i+3] } else { 0xff };
                            i += header.samples_per_pixel as usize;
                        },
                        16 => {
                            y = (read_u16(&data,i,header.tiff_headers.endian) >> 8) as u8;
                            cb = (read_u16(&data,i+2,header.tiff_headers.endian) >> 8) as u8;
                            cr = (read_u16(&data,i+4,header.tiff_headers.endian) >> 8) as u8;
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 3 {
                                    (read_u16(&data,i+6,header.tiff_headers.endian) >> 8) as u8 } else { 0xff };
                            i += header.samples_per_pixel as usize * 2;
                        },
                        32 => {
                            y = (read_u32(&data,i,header.tiff_headers.endian) >> 24) as u8;
                            cb = (read_u32(&data,i+4,header.tiff_headers.endian) >> 24) as u8;
                            cr = (read_u32(&data,i+8,header.tiff_headers.endian) >> 24) as u8;
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 3 {
                                    (read_u32(&data,i+12,header.tiff_headers.endian) >> 24) as u8 } else { 0xff };
                            i += header.samples_per_pixel as usize * 4;
                        },
                        _ => {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"This bit per sample is not support.".to_string())));
                        }
                    }

                    if header.predictor == 2 {
                        y += prevs[0];
                        prevs[0] = y; 
                        cb += prevs[1];
                        prevs[1] = cb; 
                        cr += prevs[2];
                        prevs[2] = cr;
                        if header.extra_samples.len() > 0 && header.extra_samples[0]  == 2 {
                            a += prevs[3];
                            prevs[3] = a;
                        }
                    }
                    // Bt601.1
                    let lr = 0.299;
                    let lg = 0.587;
                    let lb = 0.114;

                    let r = ((y as f32 + (2.0 - 2.0 * lr ) * cr as f32) as i16).clamp(0,255) as u8;
                    let b = ((y as f32 + (2.0 - 2.0 * lb ) * cr as f32) as i16).clamp(0,255) as u8;
                    let g = (((y as f32 - lb * b as f32 - lr * r as f32) / lg) as i16).clamp(0,255) as u8;

                    buf.push(r);
                    buf.push(g);
                    buf.push(b);
                    buf.push(a);
 
                }
                */
                _ => {
//                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Not support color space.".to_string())));
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

pub fn decode_deflate_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {
    let data = read_strips(reader, header)?;
    let res = miniz_oxide::inflate::decompress_to_vec_zlib(&data);
    match res {
        Ok(data) => {
            let warnings = draw(&data,option,header)?;
            Ok(warnings)    
        },
        Err(err) => {
            let deflate_err = format!("{:?}",err);
            Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,deflate_err)))
        }
    }
}

pub fn decode_none_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {

    let data = read_strips(reader, header)?;
    let warnings = draw(&data,option,header)?;
    Ok(warnings)
}

pub fn decode_ccitt_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {
    let buf = read_strips(reader, header)?;
    let data= ccitt::decode(&buf, header.width as  usize, header.height as usize
            ,header.photometric_interpretation)?;
    let warnings = draw(&data,option,header)?;
    Ok(warnings)
}

pub fn decode<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions) -> Result<Option<ImgWarnings>,Error> {

    let header = Tiff::new(reader)?;

    option.drawer.set_metadata("Format",DataMap::Ascii("Tiff".to_owned()))?;
//    let mut map = super::util::make_metadata(&header.tiff_headers);

    option.drawer.set_metadata("width",DataMap::UInt(header.width as u64))?;
    option.drawer.set_metadata("height",DataMap::UInt(header.height as u64))?;
    option.drawer.set_metadata("bits per pixel",DataMap::UInt(header.bitspersample as u64))?;
    option.drawer.set_metadata("Tiff headers",DataMap::Exif(header.tiff_headers.clone()))?;
    option.drawer.set_metadata("compression",DataMap::Ascii(header.compression.to_string()))?;
    if let Some(ref icc_profile) = header.icc_profile {
        option.drawer.set_metadata("ICC Profile",DataMap::ICCProfile(icc_profile.to_vec()))?;
    }

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
        },
        Compression::AdobeDeflate => {
            return decode_deflate_compresson(reader,option,&header);
        },
        // no debug
        Compression::CCITTHuffmanRLE => {
            return decode_ccitt_compresson(reader, option, &header);
        },

        _ => {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Not suport compression".to_string())));
        }
    }
}