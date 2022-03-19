/*
 * tiff/header.rs  Mith@mmk (C) 2022
 * use MIT License
 */

/* for EXIF */
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use super::tags::gps_mapper;
use super::tags::tag_mapper;
use super::super::io::*;


pub struct Rational {
    pub n: u32,
    pub d: u32,
}

pub struct RationalU64 {
    pub n: u64,
    pub d: u64,
}

pub enum DataPack {
    Bytes(Vec<u8>),
    Ascii(String),
    SByte(Vec<i8>),
    Short(Vec<u16>),
    Long(Vec<u32>),
    Rational(Vec<Rational>),
    RationalU64(Vec<RationalU64>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    SShort(Vec<i16>),
    SLong(Vec<i32>),
    Unkown(Vec<u8>),
    Undef(Vec<u8>),
}


#[allow(unused)]
pub struct TiffHeader {
    pub tagid: usize,
    pub data: DataPack,
}

#[allow(unused)]
pub struct TiffHeaders {
    pub version :u16,
    pub headers :Vec<TiffHeader>,
    pub exif: Option<Vec<TiffHeader>>,
    pub gps: Option<Vec<TiffHeader>>,
    pub little_endian: bool,
}

pub fn read_tags( buffer: &Vec<u8>) -> Result<TiffHeaders,ImgError>{
    let endian :bool;
    if buffer[0] != buffer [1] {
        return Err(ImgError::new_const(ImgErrorKind::IlligalData,&"not Tiff"));
    }

    if buffer[0] == 'I' as u8 { // Little Endian
        endian = true;
    } else if buffer[0] == 'M' as u8 {      // Big Eindian
        endian = false;
    } else {
        return Err(ImgError::new_const(ImgErrorKind::IlligalData,&"not Tiff"));
    }

    let mut ptr = 2 as usize;
    // version
    let ver = read_u16(buffer,ptr,endian);
    ptr = ptr + 2;
    let offset_ifd  = read_u32(buffer,ptr,endian) as usize;
    read_tiff(ver,buffer,offset_ifd,endian)
}

fn get_data (buffer: &[u8], ptr :usize ,datatype:usize, datalen: usize, endian: bool) -> DataPack {
    let data :DataPack;
    match datatype {
        1  => {  // 1.BYTE(u8)
            let mut d: Vec<u8> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(read_byte(buffer,ptr + i));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_byte(buffer,offset + i));
                }
            }
            data = DataPack::Bytes(d);
        },
        2 => {  // 2. ASCII(u8)
            let string;
            if datalen <=4 {
                string = read_string(buffer,ptr,datalen);

            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                string = read_string(buffer,offset,datalen);
            }
            data = DataPack::Ascii(string);    
        }
        3 => {  // SHORT (u16)
            let mut d: Vec<u16> = Vec::with_capacity(datalen);
            if datalen*2 <= 4 {
                if datalen == 1 {
                    d.push(read_u16(buffer,ptr,endian));
                } else if datalen == 2{
                    d.push(read_u16(buffer,ptr,endian));
                    d.push(read_u16(buffer,ptr + 2,endian));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_u16(buffer,offset + i*2,endian));
                }
            }
            data = DataPack::Short(d);
        },
        4 => {  // LONG (u32)
            let mut d :Vec<u32> = Vec::with_capacity(datalen);
            if datalen*4 <= 4 {
                d.push(read_u32(buffer,ptr,endian));
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_u32(buffer,offset + i*4,endian));
                }
            }
            data = DataPack::Long(d);
        },
        5 => {  //RATIONAL u32/u32
            let mut d :Vec<Rational> = Vec::with_capacity(datalen);
            let offset = read_u32(buffer,ptr,endian) as usize;
            for i in 0.. datalen { 
                let n  = read_u32(buffer,offset + i*8,endian);
                let denomi = read_u32(buffer,offset + i*8+4,endian);
                d.push(Rational{n:n,d:denomi});

            }
            data = DataPack::Rational(d);
        },
        6 => {  // 6 i8 
            let mut d: Vec<i8> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(read_i8(buffer,ptr + i));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_i8(buffer,offset + i));

                }
            }
            data = DataPack::SByte(d);
        },
        7 => {  // 1.undef
            let mut d: Vec<u8> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(read_byte(buffer,ptr + i));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_byte(buffer,offset + i));
                }
            }
            data = DataPack::Undef(d);
        },
        8 => {  // 6 i8 
            let mut d: Vec<i16> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(read_i16(buffer,ptr + i,endian));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_i16(buffer,offset + i,endian));

                }
            }
            data = DataPack::SShort(d);
        },
        9 => {  // i32 
            let mut d: Vec<i32> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(read_i32(buffer,ptr + i,endian));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_i32(buffer,offset + i,endian));

                }
            }
            data = DataPack::SLong(d);
        },
        // 7 undefined 8 s16 9 s32 10 srational u64/u64 11 float 12 double
        10 => {  //RATIONAL u64/u64
            let mut d :Vec<RationalU64> = Vec::with_capacity(datalen);
            let offset = read_u32(buffer,ptr,endian) as usize;
            for i in 0.. datalen { 
                let n_u64 = read_u64(buffer,offset + i*8,endian);
                let d_u64 =read_u64(buffer,offset + i*8+4,endian);
                d.push(RationalU64{n:n_u64,d:d_u64});
            }
            data = DataPack::RationalU64(d);

        },
        11 => {  // f32 
            let mut d: Vec<f32> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(read_f32(buffer,ptr + i,endian));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_f32(buffer,offset + i,endian));

                }
            }
            data = DataPack::Float(d);
        },
        12 => {  // f64 
            let mut d: Vec<f64> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(read_f64(buffer,ptr + i,endian));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_f64(buffer,offset + i,endian));

                }
            }
            data = DataPack::Double(d);
        },
        _ => {
            let mut d: Vec<u8> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(read_byte(buffer,ptr + i));
                }
            } else {
                let offset = read_u32(buffer,ptr,endian) as usize;
                for i in 0.. datalen { 
                    d.push(read_byte(buffer,offset + i));
                }
            };
            data = DataPack::Unkown(d);
        }
    }
    data
}

fn read_tiff (version:u16,buffer: &[u8], offset_ifd: usize,endian: bool) -> Result<TiffHeaders,ImgError>{
    read_tag(version,buffer,offset_ifd,endian,0)
}

fn read_gps (version:u16,buffer: &[u8], offset_ifd: usize,endian: bool) -> Result<TiffHeaders,ImgError> {
    read_tag(version,buffer,offset_ifd,endian,2)
}

fn read_tag (version:u16,buffer: &[u8], mut offset_ifd: usize,endian: bool,mode: usize) -> Result<TiffHeaders,ImgError>{
    let mut ifd = 0;
    let mut headers :TiffHeaders = TiffHeaders{version,headers:Vec::new(),exif:None,gps:None,little_endian: endian};
    loop {
        let mut ptr = offset_ifd;
        let tag = read_u16(buffer,ptr,endian);
        ptr = ptr + 2;
 
        for _ in 0..tag {
            let tagid = read_u16(buffer,ptr,endian);
            let datatype = read_u16(buffer,ptr + 2,endian) as usize;
            let datalen = read_u32(buffer,ptr + 4,endian) as usize;
            ptr = ptr + 8;
            let data :DataPack = get_data(buffer, ptr, datatype, datalen, endian);
            ptr = ptr + 4;

    
            if mode != 2 {
                match tagid {
                    0x8769 => {
                        match &data {
                            DataPack::Long(d) => {
                                let r = read_tag(version,buffer, d[0] as usize, endian,1)?; // read exif
                                headers.exif = Some(r.headers);

                            },
                            _  => {
                            }
                        }
                    },
                    0x8825 => {
                        match &data {
                            DataPack::Long(d) => {
                                let r = read_gps(version,buffer, d[0] as usize, endian)?; // read exif
                                headers.gps = Some(r.headers);
                        },
                        _  => {
                        }
                        }
                    },
                    _ => {
                        #[cfg(debug_assertions)]
                        tag_mapper(tagid ,&data);
                    }
                }
            } else {
                #[cfg(debug_assertions)]
                gps_mapper(tagid ,&data);
            }
            headers.headers.push(TiffHeader{tagid: tagid as usize,data: data});
        }
        offset_ifd  = read_u32(buffer,ptr,endian) as usize;
        if offset_ifd == 0 {break ;}
        ifd = ifd + 1;
    }
    Ok(headers)
}
