/*
 * tiff/header.rs  Mith@mmk (C) 2022
 * use MIT License
 */

type Error = Box<dyn std::error::Error>;

/* for EXIF */
use crate::tiff::util::print_tags;
use crate::color::RGBA;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use super::tags::gps_mapper;
use super::tags::tag_mapper;
use bin_rs::reader::BinaryReader;
use bin_rs::Endian;
use std::io::SeekFrom;


trait RationalNumber {
    fn as_f32(&self) -> f32;
    fn as_f64(&self) -> f64;
    fn denominator(&self) -> u64;
    fn numerator(&self) -> u64;
}

#[derive(Debug,Clone,PartialEq)]
pub struct Rational {
    pub n: u32,
    pub d: u32,
}

impl RationalNumber for Rational {
    fn as_f32(&self) -> f32 {
        let n = self.n as f32;
        let d = self.d as f32;
        n/d
    }

    fn as_f64(&self) -> f64 {
        let n = self.n as f64;
        let d = self.d as f64;
        n/d
    }

    fn denominator(&self) -> u64 {
        self.d as u64
    }

    fn numerator(&self) -> u64 {
        self.n as u64
    }
}

#[derive(Debug,Clone,PartialEq)]
pub struct RationalU64 {
    pub n: u64,
    pub d: u64,
}

impl RationalNumber for RationalU64 {
    fn as_f32(&self) -> f32 {
        let n = self.n as f64;
        let d = self.d as f64;
        (n/d) as f32
    }

    fn as_f64(&self) -> f64 {
        let n = self.n as f64;
        let d = self.d as f64;
        n/d
    }

    fn denominator(&self) -> u64 {
        self.d as u64
    }

    fn numerator(&self) -> u64 {
        self.n as u64
    }
}

#[derive(Debug,Clone,PartialEq)]
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
#[derive(Debug,Clone,PartialEq)]
pub struct TiffHeader {
    pub tagid: usize,
    pub data: DataPack,
    pub length: usize
}


#[allow(unused)]
#[derive(Debug,Clone,PartialEq)]
pub struct TiffHeaders {
    pub version :u16,
    pub headers :Vec<TiffHeader>,
    pub exif: Option<Vec<TiffHeader>>,
    pub gps: Option<Vec<TiffHeader>>,
    pub endian: Endian,
}

impl TiffHeaders {
    pub fn to_string(&self) -> String {
        print_tags(self)
    }
}

#[derive(Debug)]
#[derive(std::cmp::PartialEq)]
enum IfdMode {
    BaseTiff,
    Exif,
    Gps,
}


#[derive(Debug)]
pub enum Compression {
    NoneCompression = 1,
    CCITTHuffmanRLE = 2,
    CCITTGroup3Fax = 3,
    CCITTGroup4Fax = 4,
    LZW = 5,
    OldJpeg = 6,
    Jpeg = 7,
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
pub struct Tiff {
    /// 0x00fe  also 0
    pub newsubfiletype:u32,
    /// 0x00ff  also 1
    pub subfiletype:u32,                              
    /// These tag must need decode image.
    /// 0x0100
    pub width: u32,
    /// 0x0101
    pub height: u32,
    /// 0x0102 data takes 1..N. but only use one data.
    /// N = SamplesPerPixel if N > 1 also RGB888
    pub bitpersample: u16,        
    /// 0x0106 PhotometricInterpretation for color space.

    /// 0 = White is zero white based grayscale
    /// 1 = black is zero black based grayscale
    /// 2 = RGB888
    /// 3 = Palette color
    /// 4 = Transparecy Mask
    /// 
    /// Extention use
    /// 5 = alos CMYK
    /// 6 = YCbCr
    /// 8 = CieLaB
    /// 9 = ICCLab
    /// 10 = ITULab
    /// 32844 = Logl
    /// 32845 = logluv
    /// 
    /// DNG
    /// 32803 = Color filter array
    /// 34892 = Linear Raw
    /// 51177 = depth
    pub photometric_interpretation: u16,

    /// 0x010A FillOrder
    /// 1 = msb ,2 = lsb. usualy msb but lzw may use lsb(GIF like)
    pub fill_order:u16,
    /// 0x0111 Strip Offsets count 1 or 2
    /// Image data start offset, data number also 1, but it may be exist mutli offsets images.
    pub strip_offsets:Vec<u32>,             
    /// 0x0112 also 1  0
    /// 1 = TOPLEFT (LEFT,TOP) Image end (RIGHT,BOTTOM)
    /// 2 = TOPRIGHT right-left reverce Image
    /// 3 = BOTTOMRIGHT top-bottom and right-left reverce Image
    /// 4 = BOTTOMLEFT also same Windows Bitmap
    /// 5 = LEFTTOP rotate 90 TOPLEFT (TOP,LEFT) - (BOTTOM,RIGHT)
    /// 6 = RIGHTTOP rotate 90 TOPRIGHT
    /// 7 = RIGHTBOTTOM rotate 90 BOTTOMRIGHT
    /// 8 = LEFTBOTTOM  rotate 90 BOTTOMLEFT
    pub orientation: u32,               
    /// 0x0115
    /// 1 grayscale or Index color
    /// 3 RGB Image
    pub samples_per_pixel:u16,          
    /// 0x0116 also width * BitPerSample /8  <= rows_per_strip
    pub rows_per_strip: u32,
    /// 0x0117 For each strip(width), the number of bytes in the strip after compression.           
    pub strip_byte_counts :Vec<u32>,
    /// 0x0118 also no use         
    pub min_sample_value:u16,           
    pub max_sample_value:u16,           // 0x0119 default 2**(BitsPerSample) - 1
    // 0x011C

    pub planar_config: u16,
    // 0x0103 Compression
    /// NoneCompression = 1,
    /// CCITTHuffmanRE = 2,
    /// CCITTGroup3Fax = 3,
    /// CCITTGroup4Fax = 4,
    /// LZW = 5,
    /// OldJpeg = 6,
    /// JPeg = 7,
    /// AdobeDeflate = 8,
    /// Next = 32766,
    /// CcittrleW = 32771,
    /// Packbits = 32773,
    /// ThunderScan = 32809,
    /// IT8CTPad = 32895,
    /// IT8LW = 32896,
    /// IT8MP = 32897,
    /// IT8BL = 32898,
    /// PIXARFILM = 32908,
    /// PIXARLOG = 32909,
    /// DEFLATE = 32946,
    /// DCS = 32947,
    /// JBIG = 34661,
    /// SGILOG = 34676,
    /// SGILOG24 = 34677,
    /// Jpeg2000 = 34712,
    pub compression: Compression,

    // may
    /// 0x011A x dot per inch
    pub x_resolution:f32,
    /// 0x011B y dot per inch
    pub y_resolution:f32,

    // 0x0140 use only bitperpixel <= 8
    // if color_table is empty,
    // use standard color pallte or grayscale
    pub color_table: Option<Vec<RGBA>>, 

    // no baseline
    pub startx: u32,                // 0x011E
    pub starty: u32,                // 0x011F
    pub tiff_headers: TiffHeaders,

}

impl Tiff {
    pub fn new(reader: &mut dyn BinaryReader) -> Result<Self,Error>{
        let tiff_headers = read_tags(reader)?;
        //m ay        
        let  mut newsubfiletype:u32 = 0;            
        let  mut subfiletype:u32 = 1;               // 0x00ff  also 1              

        // must
        let  mut width: u32 = 0;                    // 0x0100
        let  mut height: u32 = 0;                   // 0x0101
        let  mut bitpersample: u16 = 24;             // 0x0102 data takes 1..N. if data count is 1>0;also bitperpixel =24
        let  mut photometric_interpretation: u16 = 2;// 0x0106
        let  mut fill_order:u16 = 1;
        let  mut strip_offsets = vec![];                 // 0x111
        let  orientation: u32 = 1;
        let  mut samples_per_pixel:u16  = 0;        // 0x0115
        let  mut rows_per_strip = 0_u32;           // 0x0116 also width * BitPerSample /8  <= row_per_strip
        let  mut strip_byte_counts = vec![];        // 0x0117 For each strip;the number of bytes in the strip after compression.
        let  min_sample_value:u16 = 0;          // 0x0118 also no use
        let  mut max_sample_value:u16 = 0;          // 0x0119 default 2**(BitsPerSample) - 1
        let  mut planar_config: u16 = 1;            // 0x011c also 1
        let  mut compression: Compression =Compression::NoneCompression;      // 0x0103 see enum Compression

        // may
        let  x_resolution:f32 = 0.0;              // 0x011A also for DTP
        let  y_resolution:f32 = 0.0;              // 0x0112 also for DTP
        let  mut color_table: Option<Vec<RGBA>> = None; // 0x0140 use only bitperpixel <= 8

        // no baseline
        let  startx: u32 = 0;               // 0x011E
        let  starty: u32 = 0;               // 0x011F

        for header in &tiff_headers.headers {
            match header.tagid {
                0xff => {
                    if let DataPack::Long(d) = &header.data {
                        subfiletype = d[0]
                    }
                },
			    0xfe =>{
                    if let DataPack::Long(d) = &header.data {
                        newsubfiletype = d[0]
                    }
                },
                0x100 => {
                    if let DataPack::Long(d) = &header.data {
                        width = d[0]
                    } else if let DataPack::Short(d) = &header.data {
                        width = d[0] as u32;
                    }
                },
                0x101 => {
                    if let DataPack::Long(d) = &header.data {
                        height = d[0]
                    } else if let DataPack::Short(d) = &header.data {
                        height = d[0] as u32;
                    }
                },
                0x102 => {
                    if header.length == 1{
                        if let DataPack::Short(d) = &header.data {
                            bitpersample = d[0] as u16;
                        } 
                    } else {
                        bitpersample = 24;
                        // alos d[0] + .. + d[n-1]
                    }
                },
                0x103 => {
    				if let DataPack::Short(d) = &header.data {
                        compression =
                            match d[0] {
                                1 => {
                                    Compression::NoneCompression
                                },
                                2 => {
                                    Compression::CCITTHuffmanRLE
                                },
                                3 => {
                                    Compression::CCITTGroup3Fax
                                },
                                4 => {
                                    Compression::CCITTGroup4Fax
                                },
                                5 => {
                                    Compression::LZW
                                },
                                6 => {
                                    Compression::OldJpeg
                                }
                                7 => {
                                    Compression::Jpeg
                                },
                                8 => {
                                    Compression::AdobeDeflate
                                },
                                _ => {
                                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData,
                                        "Unknown or Never Support Compression".to_string())));
                                }                            
                        };
                    }
                }, 
                0x106 => {
                    if let DataPack::Short(d) = &header.data {
                        photometric_interpretation =  d[0];
                    } 
                },
                0x10A => {
                    if let DataPack::Short(d) = &header.data {
                        fill_order =  d[0];
                    }
                },
                0x115 =>{
                    if let DataPack::Short(d) = &header.data {
                        samples_per_pixel =  d[0];
                    }
                },
                0x111 => {
                    if let DataPack::Short(d) = &header.data {
                        for v in d {
                            strip_offsets.push(*v as u32);
                        }
                    } else if let DataPack::Long(d) = &header.data {
                        for v in d {
                            strip_offsets.push(*v as u32);
                        }
                    }
                },
                0x116 => {
                    if let DataPack::Short(d) = &header.data {
                        rows_per_strip = d[0] as u32;
                    } else if let DataPack::Long(d) = &header.data {
                        rows_per_strip = d[0] as u32;
                    }
                },
                0x117 => {
                    if let DataPack::Short(d) = &header.data {
                        for v in d {
                            strip_byte_counts.push(*v as u32);
                        }
                    } else if let DataPack::Long(d) = &header.data {
                        for v in d {
                            strip_byte_counts.push(*v as u32);
                        }
                    }
                },
                0x119 => {
                    if header.length == 1 {
                        if let DataPack::Short(d) = &header.data {
                            max_sample_value =  d[0];
                        }
                    } else {
                        max_sample_value = 255;
                    }
                },
                0x11c => {
                    if let DataPack::Short(d) = &header.data {
                        planar_config =  d[0];
                    }
                },
                0x140 => {
                    if let DataPack::Short(d) = &header.data {
                        let mut table :Vec<RGBA> = Vec::new();
                        let offset = header.length / 3;
                        for i in 0..offset {
                            let red   = (d[i] >> 8) as u8;
                            let green = (d[i + offset] >> 8) as u8;
                            let blue  = (d[i + offset*2] >> 8) as u8;
                            let alpha = 0xff;
                            let color = RGBA{
                                red,
                                green,
                                blue,
                                alpha,
                            };
                            table.push(color);
                        }
                        color_table = Some(table)
                    }
                },
                _ => {},
            }

        }


        Ok(Self{
            newsubfiletype,
            subfiletype,
            width,
            height,
            bitpersample,
            photometric_interpretation,
            fill_order,
            strip_offsets,
            orientation,
            samples_per_pixel,
            rows_per_strip,
            strip_byte_counts ,
            min_sample_value,
            max_sample_value,
            planar_config,
            compression,
            x_resolution,
            y_resolution,
            color_table,
            startx,
            starty,
            tiff_headers,
        })
    }
}


pub fn read_tags(reader: &mut dyn bin_rs::reader::BinaryReader) -> Result<TiffHeaders,Error>{
    let b0 = reader.read_byte()?;
    let b1 = reader.read_byte()?;

    if b0 != b1 {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData,"not Tiff".to_string())));
    }

    if b0 == 'I' as u8 { // Little Endian
        reader.set_endian(Endian::LittleEndian);
    } else if b0 == 'M' as u8 {      // Big Eindian
        reader.set_endian(Endian::BigEndian);
    } else {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData,"not Tiff".to_string())));
    }

    // version
    let ver = reader.read_u16()?;
    if ver != 42 {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData,"not Tiff".to_string())));
    }
    let offset_ifd  = reader.read_u32()? as usize;
    read_tag(reader,offset_ifd,IfdMode::BaseTiff)
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
            let s;
            if datalen <=4 {
                s = reader.read_ascii_string(4)?;
            } else {
                let offset = reader.read_u32()? as u64;
                let current = reader.offset()?;
                reader.seek(SeekFrom::Start(offset))?;
                s = reader.read_ascii_string(datalen)?;
                reader.seek(SeekFrom::Start(current))?;
            }
            data = DataPack::Ascii(s);    
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



fn read_tag (reader: &mut dyn BinaryReader, mut offset_ifd: usize,mode: IfdMode) -> Result<TiffHeaders,Error>{
    let endian = reader.endian();
    let mut headers :TiffHeaders = TiffHeaders{version:42,headers:Vec::new(),exif:None,gps:None,endian};

    loop {
        reader.seek(SeekFrom::Start(offset_ifd as u64))?;
        let tag = reader.read_u16()? as usize; 
        let buf = reader.read_bytes_no_move(tag *12 + 4)?;
        let next_ifd = bin_rs::io::read_u32(&buf,tag*12,reader.endian());

        if cfg!(debug_assertions) {
            println!("Tiff ifd {} {} {}",tag,offset_ifd,next_ifd);
        }
    
        for _i in 0..tag {
            let tagid = reader.read_u16()?;
            if tagid == 0x0 {break}
            let datatype = reader.read_u16()? as usize;
            let datalen = reader.read_u32()? as usize;
            if cfg!(debug_assertions) {
                println!("tag {:04x} {} {}",tagid,datatype,datalen);
            }

            let data :DataPack = get_data(reader, datatype, datalen)?;
            if mode == IfdMode::BaseTiff {
                match tagid {
                    0x8769 => { // Exif
                        match &data {
                            DataPack::Long(d) => {
                                let current = reader.offset()?;
                                reader.seek(SeekFrom::Start(d[0] as u64))?;
                                let r = read_tag(reader, d[0] as usize,IfdMode::Exif)?; // read exif
                                headers.exif = Some(r.headers);
                                reader.seek(SeekFrom::Start(current))?;
                            },
                            _  => {
                            }
                        }
                    },
                    0x8825 => { // GPS
                        match &data {
                            DataPack::Long(d) => {
                                let current = reader.offset()?;
                                reader.seek(SeekFrom::Start(d[0] as u64))?;
                                let r = read_tag(reader, d[0] as usize,IfdMode::Gps)?; // read gps
                                reader.seek(SeekFrom::Start(current))?;
                                headers.gps = Some(r.headers);
                        },
                        _  => {
                        }
                        }
                    },
                    _ => {
                        #[cfg(debug_assertions)]
                        tag_mapper(tagid ,&data,datalen);
                    }
                }
            } else {
                #[cfg(debug_assertions)]
                gps_mapper(tagid ,&data,datalen);
            }
            headers.headers.push(TiffHeader{tagid: tagid as usize,data: data,length: datalen});
        }
        if next_ifd == 0 || mode != IfdMode::BaseTiff {
            break;
        }
        offset_ifd = next_ifd as usize;
    }

    Ok(headers)
}
