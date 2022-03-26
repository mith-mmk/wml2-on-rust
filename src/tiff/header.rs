/*
 * tiff/header.rs  Mith@mmk (C) 2022
 * use MIT License
 */

type Error = Box<dyn std::error::Error>;

/* for EXIF */
use crate::color::RGBA;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use super::tags::gps_mapper;
use super::tags::tag_mapper;
use bin_rs::reader::BinaryReader;
use bin_rs::Endian;
use std::io::SeekFrom;

#[derive(Debug)]
pub struct Rational {
    pub n: u32,
    pub d: u32,
}

#[derive(Debug)]
pub struct RationalU64 {
    pub n: u64,
    pub d: u64,
}

#[derive(Debug)]
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
#[derive(Debug)]
pub struct TiffHeader {
    pub tagid: usize,
    pub data: DataPack,
}

#[derive(Debug)]
pub enum Compression {
    NoneCompression = 1,
    CCITTHuffmanRE = 2,
    CCITTGroup3Fax = 3,
    CCITTGroup4Fax = 4,
    LZW = 5,
    OldJpeg = 6,
    JPeg = 7,
    AdobeDeflate = 8,
    Next = 32766,
    CcittrleW = 32771,
    Packbits = 32773,
    ThunderScan = 32809,
    IT8CTPad = 32895,
    IT8LW = 32896,
    IT8MP = 32897,
    IT8BL = 32898,
    PIXARFILM = 32908,
    PIXARLOG = 32909,
    DEFLATE = 32946,
    DCS = 32947,
    JBIG = 34661,
    SGILOG = 34676,
    SGILOG24 = 34677,
    Jpeg2000 = 34712,
}

/// This struct is not use embed tiff tag,also EXIF.
/// Use only tiff encoder/decoder
#[derive(Debug)]
pub struct TiffBaseline {
    //must

    pub newsubfiletype:u32,             // 0x00fe  also 0
    pub subfiletype:u32,                // 0x00ff  also 1              
    pub width: u32,                     // 0x0100
    pub height: u32,                    // 0x0101
    pub bitperpixel: u32,               // 0x0102 data takes 1..N. if data count is 1>0,also bitperpixel =24
    pub photometric_interpretation: u32,// 0x0106
    // 0 = White is zero white based grayscale
    // 1 = black is zero black based grayscale
    // 2 = RGB888
    // 3 = Palette color
    // 4 = Transparecy Mask
    //
    // 5 = alos CMYK
    // 6 = YCbCr
    // 8 = CieLaB
    // 9 = ICCLab
    //10 = ITULab
    //32844 = Logl
    //32845 = logluv
    //32803 = Color filter array
    //34892 = Linear Raw
    //51177 = depth

    pub fill_order:u32,                 // 0x010A 1 = msb ,2 = lsb. usualy msb but lzw also use lsb
    pub strip_offsets: u32,             // 0x0111 image data offsets, data number also 1, but it may be exist mutli offsets images.
    pub orientation: u32,               // 0x0112 also 1  0
    // 1 = TOPLEFT (LEFT,TOP) Image end (RIGHT,BOTTOM)
    // 2 = TOPRIGHT right-left reverce Image
    // 3 = BOTTOMRIGHT top-bottom and right-left reverce Image
    // 4 = BOTTOMLEFT also same Windows Bitmap
    // 5 = LEFTTOP rotate 90 TOPLEFT (TOP,LEFT) - (BOTTOM,RIGHT)
    // 6 = RIGHTTOP rotate 90 TOPRIGHT
    // 7 = RIGHTBOTTOM rotate 90 BOTTOMRIGHT
    // 8 = LEFTBOTTOM  rotate 90 BOTTOMLEFT
    pub samples_per_pixel:u16,          // 0x0115
    pub rows_per_strip: u32,            // 0x0116 also width * BitPerSample /8  <= row_per_strip
    pub strip_byte_counts :u32,         // 0x0117 For each strip, the number of bytes in the strip after compression.
    pub min_sample_value:u16,           // 0x0118 also no use
    pub max_sample_value:u16,           // 0x0119 default 2**(BitsPerSample) - 1
    pub planar_config: u32,             // 0x011c also 1
    pub compression: Compression,       // 0x0103 see enum Compression

    // may
    pub x_resolution:u32,               // 0x011A also for DTP
    pub y_resolution:u32,               // 0x0112 also for DTP
    pub color_table: Option<Vec<RGBA>>, // 0x0140 use only bitperpixel <= 8
                                        // if color_table is empty,
                                        // you use standard color pallte or grayscale

    pub threshholding: u32, // 0x0107
    pub cell_width:u32, //0x0108
    pub cell_height:u32, //0x0109
    pub free_offsets:u32, //288	0120	
    pub free_byte_counts:u32, //289	0121	
    pub gray_response_unit:u32, //290	0122
    pub gray_response_curve:u32, //291	0123

    // comments
    pub document_name:String, //0x010d
    pub make:String, //0x010f
    pub model:String, //0x0110
    pub software:String, //0x131
    pub datetime:String, //0x132
    pub artist:String, //0x13b
    pub host_computer:String, //0x13c

    // no baseline
    pub startx: u32,                // 0x011E
    pub starty: u32,                // 0x011F

}


#[allow(unused)]
#[derive(Debug)]
pub struct TiffHeaders {
    pub version :u16,
    pub headers :Vec<TiffHeader>,
    pub standard: Option<TiffBaseline>,
    pub exif: Option<Vec<TiffHeader>>,
    pub gps: Option<Vec<TiffHeader>>,
    pub endian: Endian,
}
/*
impl TiffHeaders {
    pub fn new(reader: &dyn BinaryReader) -> Self{




    }
}
*/

pub fn read_tags(reader: &mut dyn bin_rs::reader::BinaryReader) -> Result<TiffHeaders,Error>{
    let b0 = reader.read_byte()?;
    let b1 = reader.read_byte()?;

    if b0 != b1 {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::IlligalData,"not Tiff".to_string())));
    }

    if b0 == 'I' as u8 { // Little Endian
        reader.set_endian(Endian::LittleEndian);
    } else if b0 == 'M' as u8 {      // Big Eindian
        reader.set_endian(Endian::BigEndian);
    } else {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::IlligalData,"not Tiff".to_string())));
    }

    // version
    let ver = reader.read_u16()?;
    let offset_ifd  = reader.read_u32()? as usize;
    read_tiff(ver,reader,offset_ifd)
}

fn get_data (reader: &mut dyn BinaryReader,datatype: usize, datalen: usize) -> Result<DataPack,Error> {
    let data :DataPack;
    match datatype {
        1  => {  // 1.BYTE(u8)
            let mut d: Vec<u8> = Vec::with_capacity(datalen);
            if datalen <=4 {
                let buf = reader.read_bytes_as_vec(4)?;
                for i in 0.. datalen { 
                    d.push(buf[i]);
                }

            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_byte()?);
                }
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::Bytes(d);
        },
        2 => {  // 2. ASCII(u8)
            let string;
            if datalen <=4 {
                string = reader.read_ascii_string(4)?;
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                string = reader.read_ascii_string(datalen)?;
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::Ascii(string);    
        }
        3 => {  // SHORT (u16)
            let mut d: Vec<u16> = Vec::with_capacity(datalen);
            if datalen*2 <= 4 {
                d.push(reader.read_u16()?);
                d.push(reader.read_u16()?);
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_u16()?);
                }
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::Short(d);
        },
        4 => {  // LONG (u32)
            let mut d :Vec<u32> = Vec::with_capacity(datalen);
            if datalen*4 <= 4 {
                d.push(reader.read_u32()?);
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_u32()?);
                }
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::Long(d);
        },
        5 => {  //RATIONAL u32/u32
            let mut d :Vec<Rational> = Vec::with_capacity(datalen);
            let offset = reader.read_u32()? as u64;
            let current = reader.offset()?;
            reader.seek(SeekFrom::Start(offset))?;
            for _ in 0.. datalen { 
                let n  = reader.read_u32()?;
                let denomi = reader.read_u32()?;
                d.push(Rational{n:n,d:denomi});
            }
            reader.seek(SeekFrom::Start(current))?;
            data = DataPack::Rational(d);
        },
        6 => {  // 6 i8 
            let mut d: Vec<i8> = Vec::with_capacity(datalen);
            let buf = reader.read_bytes_as_vec(4)?;
            if datalen <=4 {
                for i in 0.. datalen { 
                    d.push(buf[i] as i8);
                }
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_i8()?);
                }
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::SByte(d);
        },
        7 => {  // 1.undef
            let mut d: Vec<u8> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for _ in 0.. datalen { 
                    d = reader.read_bytes_as_vec(4)?;
                }
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_byte()?);
                }
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::Undef(d);

        },
        8 => {  // 6 i8 
            let mut d: Vec<i16> = Vec::with_capacity(datalen);
            if datalen <=4 {
                d.push(reader.read_i16()?);
                d.push(reader.read_i16()?);
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_i16()?);

                }
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::SShort(d);
        },
        9 => {  // i32 
            let mut d: Vec<i32> = Vec::with_capacity(datalen);
            if datalen*4 <=4 {
                for _ in 0.. datalen { 
                    d.push(reader.read_i32()?);
                }
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_i32()?);
                }
                reader.seek(SeekFrom::Start(current))?;

            }
            data = DataPack::SLong(d);
        },
        // 7 undefined 8 s16 9 s32 10 srational u64/u64 11 float 12 double
        10 => {  //RATIONAL u64/u64
            let mut d :Vec<RationalU64> = Vec::with_capacity(datalen);
            let offset = reader.read_u32()? as u64;
            let current = reader.offset()?;
            reader.seek(SeekFrom::Start(offset))?;
            for _ in 0.. datalen { 
                let n_u64 = reader.read_u64()?;
                let d_u64 = reader.read_u64()?;
                d.push(RationalU64{n:n_u64,d:d_u64});
            }
            reader.seek(SeekFrom::Start(current))?;
            data = DataPack::RationalU64(d);

        },
        11 => {  // f32 
            let mut d: Vec<f32> = Vec::with_capacity(datalen);
            if datalen*4 <=4 {
                for _ in 0.. datalen { 
                    d.push(reader.read_f32()?);
                }
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_f32()?);

                }
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::Float(d);
        },
        12 => {  // f64 
            let mut d: Vec<f64> = Vec::with_capacity(datalen);
            let offset = reader.read_u32()? as u64;
            let current = reader.offset()?;
            reader.seek(SeekFrom::Start(offset))?;
            for _ in 0.. datalen { 
                d.push(reader.read_f64()?);

            }
            reader.seek(SeekFrom::Start(current))?;
            data = DataPack::Double(d);
        },
        _ => {
            let mut d: Vec<u8> = Vec::with_capacity(datalen);
            if datalen <=4 {
                for _ in 0.. datalen { 
                    d = reader.read_bytes_as_vec(4)?;
                }
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                for _ in 0.. datalen { 
                    d.push(reader.read_byte()?);
                }
                reader.seek(SeekFrom::Start(current))?;
            };
            data = DataPack::Unkown(d);
        }
    }
    Ok(data)
}

fn read_tiff (version:u16,reader: &mut dyn BinaryReader, offset_ifd: usize) -> Result<TiffHeaders,Error>{
    read_tag(version,reader,offset_ifd,0)
}

fn read_gps (version:u16,reader: &mut dyn BinaryReader, offset_ifd: usize) -> Result<TiffHeaders,Error> {
    read_tag(version,reader,offset_ifd,2)
}

fn read_tag (version:u16,reader: &mut dyn BinaryReader, offset_ifd: usize,mode: usize) -> Result<TiffHeaders,Error>{
    let endian = reader.endian();
    let mut headers :TiffHeaders = TiffHeaders{version,headers:Vec::new(),standard:None,exif:None,gps:None,endian};
    let mut ptr = offset_ifd;
    let tag = reader.read_u16()?; 


    for _i in 0..tag {
        let tagid = reader.read_u16()?;
        if tagid == 0x0 {break}
        let datatype = reader.read_u16()? as usize;
        let datalen = reader.read_u32()? as usize;
        ptr = ptr + 8;
        let data :DataPack = get_data(reader, datatype, datalen)?;
        ptr = ptr + 4;
        if mode != 2 {
            match tagid {
                0x8769 => {
                    match &data {
                        DataPack::Long(d) => {
                            let current = reader.offset()?;
                            reader.seek(SeekFrom::Start(d[0] as u64))?;
                            let r = read_tag(version,reader, d[0] as usize,1)?; // read exif
                            headers.exif = Some(r.headers);
                            reader.seek(SeekFrom::Start(current))?;
                        },
                        _  => {
                        }
                    }
                },
                0x8825 => {
                    match &data {
                        DataPack::Long(d) => {
                            let current = reader.offset()?;
                            reader.seek(SeekFrom::Start(d[0] as u64))?;
                            let r = read_gps(version,reader, d[0] as usize)?; // read exif
                            reader.seek(SeekFrom::Start(current))?;
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

    Ok(headers)
}
