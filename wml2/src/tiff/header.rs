/*
 * tiff/header.rs  Mith@mmk (C) 2022
 * use MIT License
 */

type Error = Box<dyn std::error::Error>;

/* for EXIF */

use bin_rs::io::*;
use crate::tiff::util::print_tags;
use crate::color::RGBA;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
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
pub struct SRational {
    pub n: i32,
    pub d: i32,
}

impl RationalNumber for SRational {
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
    SRational(Vec<SRational>),
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

    pub fn empty(endian:Endian) -> Self {
        Self {
            version: 42,
            headers: Vec::new(),
            exif : None,
            gps: None,
            endian
        }
    }
}

#[derive(Debug)]
#[derive(std::cmp::PartialEq)]
enum IfdMode {
    BaseTiff,
    Exif,
    Gps,
}


#[derive(Debug,PartialEq,Clone)]
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
    Unknown = 0,
}

impl Compression {
    pub fn to_string(&self) -> String {
        match self {
            Compression::NoneCompression => {
                "None".to_string()
            },
            Compression::CCITTHuffmanRLE => {
                "CCITT Huffman RLE".to_string()
            },
            Compression::CCITTGroup3Fax => {
                "CCITT Group3 Fax".to_string()
            },
            Compression::CCITTGroup4Fax => {
                "CCITT Group4 Fax".to_string()
            },
            Compression::LZW => {
                "LZW(Tiff)".to_string()
            },
            Compression::OldJpeg => {
                "Jpeg(old)".to_string()
            },
            Compression::Jpeg => {
                "Jpeg".to_string()
            },
            Compression::AdobeDeflate => {
                "Adobe Deflate".to_string()
            },
            Compression::Next => {
                "Next".to_string()
            },
            Compression::CcittrleW => {
                "CCITT RLEW".to_string()
            },
            Compression::Packbits => {
                "Apple Macintosh Packbits".to_string()
            },
            Compression::ThunderScan => {
                "Thuder Scan".to_string()
            },
            Compression::IT8CTPad => {
                "IT8 CTPad".to_string()
            },
            Compression::IT8LW => {
                "IT8 LW".to_string()
            },
            Compression::IT8MP => {
                "IT8 MP".to_string()
            }
            Compression::IT8BL => {
                "IT8 BL".to_string()
            }
            Compression::PIXARFILM => {
                "Pixar Film".to_string()
            }
            Compression::PIXARLOG => {
                "Pixar Log".to_string()
            }
            Compression::DEFLATE => {
                "Deflate".to_string()
            }
            Compression::DCS => {
                "DCS".to_string()
            }
            Compression::JBIG => {
                "JBIG".to_string()
            }
            Compression::SGILOG => {
                "SGI LOG".to_string()
            }
            Compression::SGILOG24 => {
                "SGI LOG24".to_string()
            }
            Compression::Jpeg2000 => {
                "JPEG2000".to_string()
            }
            Compression::Unknown => {
                "Unknown".to_string()
            }
        }

    }
}

/// This struct is not use embed tiff tag,also EXIF.
/// Use only tiff encoder/decoder
#[derive(Debug,Clone)]
pub struct Tiff {
    /// 0x00fe  also 0
    pub newsubfiletype:u32,
    /// 0x00ff  also 1, but 2,3 is multi part image,This decoder is not support.
    pub subfiletype:u32,
    /// These tag must need decode image.
    /// 0x0100
    pub width: u32,
    /// 0x0101
    pub height: u32,
    /// 0x0102 data takes 1..N. but only use one data.
    /// N = SamplesPerPixel if N =2 GA88 , N = 3 also RGB888, N = 4 also YMCK8888/RGBA8888
    pub bitspersample: u16,
    pub bitspersamples: Vec<u16>,
    /// 0x0106 PhotometricInterpretation for color space.

    /// 0 = White is zero white based grayscale
    /// 1 = black is zero black based grayscale
    /// 2 = RGB888
    /// 3 = Palette color
    /// Transparency Mask is not support
    /// 
    /// 4 = Transparecy Mask
    /// 
    /// Extention use - This color space need color management modules.
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
    /// 1 = msb ,2 = lsb. usualy msb. lsb is using FAX G3/G4 compressions.
    pub fill_order:u16,
    /// Image data start offsets. strip_offsets length need equal strip_byte_counts length
    pub strip_offsets:Vec<u32>,             
    /// 0x0112 also 1  0    this parameter is not use.
    /// 1 = TOPLEFT (LEFT,TOP) Image end (RIGHT,BOTTOM)
    /// 2 = TOPRIGHT right-left reverce Image
    /// 3 = BOTTOMRIGHT top-bottom and right-left reverce Image
    /// 4 = BOTTOMLEFT also same Windows Bitmap
    /// 5 = LEFTTOP rotate 90 TOPLEFT (TOP,LEFT) - (BOTTOM,RIGHT)
    /// 6 = RIGHTTOP rotate 90 TOPRIGHT
    /// 7 = RIGHTBOTTOM rotate 90 BOTTOMRIGHT
    /// 8 = LEFTBOTTOM  rotate 90 BOTTOMLEFT
    pub orientation: u32,               
    /// 0x0115 samples per pixel
    /// 1 grayscale or Index color
    /// 2 also gralscale + alpha channel
    /// 3 RGB Image/YCbCr Image...
    /// 4 RGB + alpha/YMCK...
    pub samples_per_pixel:u16,          
    /// 0x0116 also width * Bitspersample /8
    pub rows_per_strip: u32,
    /// 0x0117 For each strip(width), the number of bytes in the strip after compression.       
    pub strip_byte_counts :Vec<u32>,
    /// 0x0118 also this decoder does not use.
    pub min_sample_values:Vec<u16>,           
    /// 0x0119 also this decoder does not use. default 2**(BitsPerSample) - 1
    pub max_sample_values:Vec<u16>,

    // 0x011C support only 1
    pub planar_config: u16,
    // 0x0103 Compression
    /// NoneCompression = 1,    supported
    /// CCITTHuffmanRE = 2,
    /// CCITTGroup3Fax = 3, suppoted
    /// CCITTGroup4Fax = 4, suppoted
    /// LZW = 5,    supported
    /// OldJpeg = 6,
    /// Jpeg = 7,   supported
    /// AdobeDeflate = 8,   supported
    /// Next = 32766,
    /// CcittrleW = 32771,
    /// Packbits = 32773,   supported
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

    /// 0x0140 use only bitperpixel <= 8
    /// if color_table is empty,
    /// use standard color pallte or grayscale
    pub color_table: Option<Vec<RGBA>>, 

    // no baseline
    pub startx: u32,                // 0x011E
    pub starty: u32,                // 0x011F
    pub predictor: u16,             // 0x013D
    pub extra_samples:Vec<u16>,     // 0x0152

    /// TileWidth/TileLength/TileOffsets/TileByteCOunts are using tiled image
    /// TIFF 6.0 Section 15
    pub tile_width: u32,            // 0x0142
    pub tile_length:u32,            // 0x0143
    pub tile_offsets:Vec<u32>,      // 0x0144
    pub tile_byte_counts:Vec<u32>,  // 0x0145

    // compression option 
    // t4_options // G3
    pub t4_options:u32,
    pub t6_options:u32,             // G4
    pub jpeg_tables:Vec<u8>,        // jpeg(new)

    // metadata
    pub tiff_headers: TiffHeaders,
    pub icc_profile: Option<Vec<u8>>,       

    // multi page tiff
    pub multi_page: Box<Vec<Tiff>>,
    // pub predictor // if predictor is 2,pixels are horizontal differencing. not support
}

impl Tiff {
    pub fn empty() -> Self {
        Self{
            newsubfiletype: 0,
            subfiletype: 0,
            width: 0,
            height: 0,
            bitspersample: 1,
            bitspersamples: vec![],
            photometric_interpretation: 2,
            fill_order: 1,
            strip_offsets: vec![],
            orientation: 1,
            samples_per_pixel: 1,
            rows_per_strip: 0,
            strip_byte_counts: vec![] ,
            min_sample_values: vec![],
            max_sample_values: vec![],
            planar_config: 1,
            compression: Compression::NoneCompression,
            x_resolution: 0.0,
            y_resolution: 0.0,
            color_table: None,
            startx: 0,
            starty: 0,
            predictor: 1,
            extra_samples: vec![],
            tile_width: 0,
            tile_length:0,
            tile_offsets:vec![],
            tile_byte_counts:vec![],
            t4_options: 0,
            t6_options: 0,
            jpeg_tables: vec![],
            tiff_headers: TiffHeaders::empty(Endian::LittleEndian),
            icc_profile: None,
            multi_page: Box::new(Vec::new()),
        }
    }

    pub fn new(reader: &mut dyn BinaryReader) -> Result<Self,Error>{
        let tiff_headers = read_tags(reader)?;

        let mut max_id = 0;

        let mut this = Self::empty();
        let mut current = &mut this;
        let mut append = vec![];

        for header in &tiff_headers.headers {
            // skip thumbnail or multi page images

            if max_id < header.tagid {
                max_id = header.tagid;
            } else {
                max_id = header.tagid;
                if current.bitspersamples.len() == 0 {
                    current.bitspersamples.push(current.bitspersample);
                }        
                append.push(Self::empty());
                current = append.last_mut().unwrap();
                current.tiff_headers.endian = tiff_headers.endian;
            }
            match header.tagid {
                0xff => {
                    if let DataPack::Long(d) = &header.data {
                        current.subfiletype = d[0]
                    }
                },
			    0xfe =>{
                    if let DataPack::Long(d) = &header.data {
                        current.newsubfiletype = d[0];
                    }
                },
                0x100 => {
                    if let DataPack::Long(d) = &header.data {
                        current.width = d[0]
                    } else if let DataPack::Short(d) = &header.data {
                        current.width = d[0] as u32;
                    }
                },
                0x101 => {
                    if let DataPack::Long(d) = &header.data {
                        current.height = d[0]
                    } else if let DataPack::Short(d) = &header.data {
                        current.height = d[0] as u32;
                    }
                },
                0x102 => {
                    if header.length == 1 {
                        if let DataPack::Short(d) = &header.data {
                            current.bitspersample = d[0] as u16;
                            current.bitspersamples.push(d[0] as u16);
                        } 
                    } else {
                        let mut bpm = 0;
                        for i in 0..header.length {
                            if let DataPack::Short(d) = &header.data {
                                bpm += d[i] as u16;
                                current.bitspersamples.push(d[i] as u16);
                            } 
                        }
                        current.bitspersample = bpm;
                    }
                },
                0x103 => {
    				if let DataPack::Short(d) = &header.data {
                        current.compression =
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
                                // obsolete
                                6 => {
                                    Compression::OldJpeg
                                },
                                7 => {
                                    Compression::Jpeg
                                },
                                8 => {
                                    Compression::AdobeDeflate
                                },
                                32766 => {
                                    Compression::Next
                                },
                                32771 => {
                                    Compression::CcittrleW
                                },
                                32773 => {
                                    Compression::Packbits
                                },
                                32809 => {
                                    Compression::ThunderScan
                                },
                                32895 => {
                                    Compression::IT8CTPad
                                },
                                32896 => {
                                    Compression::IT8LW
                                },
                                32897 => {
                                    Compression::IT8MP
                                },
                                32898 => {
                                    Compression::IT8BL
                                },
                                32908 => {
                                    Compression::PIXARFILM
                                },
                                32909 => {
                                    Compression::PIXARLOG
                                },
                                32946 => {
                                    Compression::DEFLATE
                                },
                                32947 => {
                                    Compression::DCS
                                },
                                34661 => {
                                    Compression::JBIG
                                },
                                34676 => {
                                    Compression::SGILOG
                                },
                                34677 => {
                                    Compression::SGILOG24
                                },
                                34712 => {
                                    Compression::Jpeg2000
                                }
                                _ => {
                                    Compression::Unknown                                    
                                }                            
                        };
                    }
                }, 
                0x106 => {
                    if let DataPack::Short(d) = &header.data {
                        current.photometric_interpretation =  d[0];
                    } 
                },
                0x10A => {
                    if let DataPack::Short(d) = &header.data {
                        current.fill_order =  d[0];
                    }
                },
                0x115 =>{
                    if let DataPack::Short(d) = &header.data {
                        current.samples_per_pixel =  d[0];
                    }
                },
                0x111 => {
                    let mut strip_offsets = vec![];
                    if let DataPack::Short(d) = &header.data {
                        for i in 0..d.len() {
                            strip_offsets.push(d[i] as u32);
                        }
                    } else if let DataPack::Long(d) = &header.data {
                        for i in 0..d.len() {
                            strip_offsets.push(d[i] as u32);
                        }
                    }
                    current.strip_offsets = strip_offsets;
                },
                0x116 => {
                    if let DataPack::Short(d) = &header.data {
                        current.rows_per_strip = d[0] as u32;
                    } else if let DataPack::Long(d) = &header.data {
                        current.rows_per_strip = d[0] as u32;
                    }
                },
                0x117 => {
                    let mut strip_byte_counts = vec![];
                    if let DataPack::Short(d) = &header.data {
                        for i in 0..d.len() {
                            strip_byte_counts.push(d[i] as u32);
                        }
                    } else if let DataPack::Long(d) = &header.data {
                        for i in 0..d.len() {
                            strip_byte_counts.push(d[i] as u32);
                        }
                    }
                    current.strip_byte_counts = strip_byte_counts;
                },
                0x118 => {
                    for i in 0..header.length {
                        if let DataPack::Short(d) = &header.data {
                            current.min_sample_values.push(d[i]);
                        }
                    }
                },
                0x119 => {
                    for i in 0..header.length {
                        if let DataPack::Short(d) = &header.data {
                            current.max_sample_values.push(d[i]);
                        }
                    }
                },
                0x11c => {
                    if let DataPack::Short(d) = &header.data {
                        current.planar_config =  d[0];
                    }
                },
                0x013d => { // predictor
                    if let DataPack::Short(d) = &header.data {
                        current.predictor = d[0];
                    }                    
                },
                0x0124 => { // T4Options
                    if let DataPack::Long(d) = &header.data {
                        current.t4_options = d[0];
                    }
                },
                0x0125 => { // T6Options
                    if let DataPack::Long(d) = &header.data {
                        current.t6_options = d[0];
                    }
                },
                0x0140 => {  // color table
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
                        current.color_table = Some(table)
                    }
                },
                0x142 => { // TileWidth
                    if let DataPack::Short(d) = &header.data {
                        current.tile_width = d[0] as u32;
                    } else if let DataPack::Long(d) = &header.data {
                        current.tile_width = d[0] as u32;
                    }
                },
                0x143 => { // TileWidth
                    if let DataPack::Short(d) = &header.data {
                        current.tile_length = d[0] as u32;
                    } else if let DataPack::Long(d) = &header.data {
                        current.tile_length = d[0] as u32;
                    }
                },
                0x144 => { // TileOffsets
                   let mut tile_offsets = vec![];
                    if let DataPack::Long(d) = &header.data {
                        for i in 0..d.len() {
                            tile_offsets.push(d[i] as u32);
                        }
                    }
                    current.tile_offsets = tile_offsets;
                },
                0x145 => {
                    let mut tile_byte_counts = vec![];
                    if let DataPack::Short(d) = &header.data {
                        for i in 0..d.len() {
                            tile_byte_counts.push(d[i] as u32);
                        }
                    } else if let DataPack::Long(d) = &header.data {
                        for i in 0..d.len() {
                            tile_byte_counts.push(d[i] as u32);
                        }
                    }
                    current.tile_byte_counts = tile_byte_counts;
                },
                0x0152 => { // ExtraSamples
                    if let DataPack::Short(d) = &header.data {
                        current.extra_samples = d.to_vec();
                    }                    
                },
                0x015b => { //Jpeg Tables
                    if let DataPack::Undef(d) = &header.data {
                        current.jpeg_tables = d.to_vec();
                    }
                },
                0x8773 => { //ICC Profile
                    if let DataPack::Undef(d) = &header.data {
                        current.icc_profile = Some(d.to_vec());
                    }
                }
                _ => {},
            }
        }

        if current.bitspersamples.len() == 0 {
            current.bitspersamples.push(current.bitspersample);
        }
        this.tiff_headers = tiff_headers;
        this.multi_page = Box::new(append);
        Ok(this)
    }
}

pub fn write_tag(buf:&mut Vec<u8>,append:&mut Vec<u8>,tag:&TiffHeader,last_offset:&mut usize,endian:&Endian) -> Result<(),Error> {
    let endian = *endian;
    write_u16(tag.tagid as u16, buf , endian);
    match &tag.data {
        DataPack::Bytes(data) => {
            write_u16(1,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 4 {
                write_u32(*last_offset as u32, buf,endian);
                write_bytes(data, append);
                *last_offset += data.len();
            } else {
                write_bytes(data, buf);
                for _ in 0..(4 - data.len() as i32) {
                    write_byte(0,buf);
                }
            }
        },
        DataPack::Ascii(data) => {
            write_u16(2,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 4 {
                write_u32(*last_offset as u32, buf,endian);
                write_string(data.to_string(), append);
                *last_offset += data.len();
            } else {
                write_string(data.to_string(), buf);
                for _ in 0..(4 - data.len() as i32) {
                    write_byte(0,buf);
                }
            }
        },
        DataPack::Short(data) => {
            write_u16(3,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 2 {
                write_u32(*last_offset as u32, buf,endian);
                for d in data {
                    write_u16(*d, append,endian);
                }
                *last_offset += data.len() * 2;
            } else {
                for d in data {
                    write_u16(*d, buf,endian);
                }
                for _ in 0..(2 - data.len() as i32) {
                    write_u16(0,buf,endian);
                }
            }
        },
        DataPack::Long(data) => {
            write_u16(4,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 1 {
                write_u32(*last_offset as u32, buf,endian);
                for d in data {
                    write_u32(*d, append,endian);
                }
                *last_offset += data.len() * 4;
            } else {
                write_u32(data[0], buf,endian);
            }
        },
        DataPack::Rational(data) => {
            write_u16(5,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            write_u32(*last_offset as u32, buf,endian);
            for d in data {
                write_u32(d.n ,append, endian);
                write_u32(d.d ,append, endian);
            }
            *last_offset += data.len() * 4;
        },
        DataPack::SRational(data) => {
            write_u32(*last_offset as u32, buf,endian);
            for d in data {
                write_i32(d.n ,append, endian);
                write_i32(d.d ,append, endian);
            }
            *last_offset += data.len() * 4;

        },
        DataPack::Undef(data) => {
            write_u16(7,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 4 {
                write_u32(*last_offset as u32, buf,endian);
                write_bytes(data, append);
                *last_offset += data.len();
            } else {
                write_bytes(data, buf);
                for _ in 0..(4 - data.len() as i32) {
                    write_byte(0,buf);
                }
            }
        },
        DataPack::Unkown(data) => {
            write_u16(7,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 4 {
                write_u32(*last_offset as u32, buf,endian);
                write_bytes(data,append);
                *last_offset += data.len();
            } else {
                write_bytes(data, buf);
                for _ in 0..(4 - data.len() as i32) {
                    write_byte(0,buf);
                }
            }
        },
        DataPack::Float(data) => {
            write_u16(11,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 1 {
                write_u32(*last_offset as u32, buf,endian);
                for d in data {
                    write_f32(*d, append, endian);
                }
                *last_offset += data.len() * 4;
            } else {
                write_f32(data[0], buf,endian);
            }
        },
        DataPack::Double(data) => {
            write_u16(12,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            write_u32(*last_offset as u32, buf,endian);
            for d in data {
                write_f64(*d, append, endian);
            }
        },
        DataPack::SByte(data) => {
            write_u16(6,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 4 {
                write_u32(*last_offset as u32, buf,endian);
                for d in data {
                    write_i8(*d, append);
                }
                *last_offset += data.len();
            } else {
                for d in data {
                    write_i8(*d, append);
                }
                for _ in 0..(4 - data.len() as i32) {
                    write_byte(0,buf);
                }
            }
        },
        DataPack::SShort(data) => {
            write_u16(8,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 2 {
                write_u32(*last_offset as u32, buf,endian);
                for d in data {
                    write_i16(*d, append,endian);
                }
                *last_offset += data.len() * 2;
            } else {
                for d in data {
                    write_i16(*d, buf,endian);
                }
                for _ in 0..(2 - data.len() as i32) {
                    write_u16(0,buf,endian);
                }
            }
        },
        DataPack::SLong(data) => {
            write_u16(9,buf,endian);
            write_u32(data.len() as u32,buf,endian);
            if data.len() > 1 {
                write_u32(*last_offset as u32, buf,endian);
                for d in data {
                    write_i32(*d, append,endian);
                }
                *last_offset += data.len() * 4;
            } else {
                write_i32(data[0], buf,endian);
            }
        },
    }
    Ok(())
}

pub fn write_header(buf: &mut Vec<u8>,tags: &TiffHeaders) -> Result<(),Error> {
    let endian = tags.endian;
    // First 2bytes are endian flag,If big endian is "MM",little endian is "II".
    match endian {
        Endian::BigEndian => {
            write_string("MM".to_string(), buf)
        },
        Endian::LittleEndian => {
            write_string("II".to_string(), buf)
        },
    }
    // Version number is 42
    write_u16(42, buf, endian);
    // IFD offsets is unknown yet.
    write_u32(8, buf, endian);
    Ok(())
}

/* An IFD offset is after or before image data */

pub fn write_ifd(buf: &mut Vec<u8>,tags: &TiffHeaders) -> Result<usize,Error> {
    let endian = tags.endian;
    let mut offset = buf.len();

    let bytes = if endian == Endian::BigEndian {
            (offset as u32).to_be_bytes() } else {
            (offset as u32).to_le_bytes() };

    // Set IFD offset
    buf[4] = bytes[0];
    buf[5] = bytes[1];
    buf[6] = bytes[2];
    buf[7] = bytes[3];

    let mut add_num = if tags.exif.is_some() {1} else {0};
    add_num += if tags.gps.is_some() {1} else {0};

    let mut last_offset = offset + (tags.headers.len() + add_num) * 12 + 4;
    let mut append :Vec<u8> = vec![];
    write_u16(tags.headers.len() as u16, buf, endian);
    let mut image_offset = 0;
    let mut exif_write = false;
    let mut gps_write = false;

    for tag in &tags.headers {
        if tag.tagid == 0x0111 {    // StripOffsets
            image_offset = offset + 8; // 2(tag) + 2(type) + 4(count)
        }

        // Exif
        if tag.tagid > 0x8769 && ! exif_write {
            if let Some(exif) = &tags.exif {
                let extra_offset = last_offset.clone();
                let mut buf_extra = vec![];
                let mut append_extra = vec![];
                write_u16(exif.len() as u16, &mut buf_extra, endian);
                for tag in exif {
                    write_tag(&mut buf_extra,&mut append_extra,tag,&mut last_offset,&endian)?;
                }
                last_offset = extra_offset + buf.len() + append_extra.len() + 2;
                buf_extra.append(&mut append_extra);
                let length = buf_extra.len();
                let tag = TiffHeader {
                    tagid: 0x8769,
                    data: DataPack::Undef(buf_extra),
                    length
                };
                write_tag(buf,&mut append,&tag,&mut last_offset,&endian)?;
                offset += 12;
            }
            exif_write = true;
        }

        // GPS
        if tag.tagid > 0x8825 && ! gps_write {
            if let Some(exif) = &tags.exif {
                let extra_offset = last_offset.clone();
                let mut buf_extra = vec![];
                let mut append_extra = vec![];
                write_u16(exif.len() as u16, &mut buf_extra, endian);
                for tag in exif {
                    write_tag(&mut buf_extra,&mut append_extra,tag,&mut last_offset,&endian)?;
                }
                last_offset = extra_offset + buf.len() + append_extra.len() + 2;
                buf_extra.append(&mut append_extra);
                let length = buf_extra.len();
                let tag = TiffHeader {
                    tagid: 0x8769,
                    data: DataPack::Undef(buf_extra),
                    length
                };
                write_tag(buf,&mut append,&tag,&mut last_offset,&endian)?;
                offset += 12;
            }
            gps_write = true;
        }

        // append tag
        if tag.tagid != 0x8825 || tag.tagid != 0x8769 {
            write_tag(buf,&mut append,tag,&mut last_offset,&endian)?;
        }
        offset += 12;
    }
    write_u32(0,buf,endian);   //IFD end
    Ok(image_offset)
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
                s = reader.read_ascii_string(datalen)?;
                reader.skip_ptr(4 - datalen)?;
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
                if datalen == 2 {
                    d.push(reader.read_u16()?);
                } else {
                    reader.skip_ptr(2)?;
                }
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
            if datalen <=4 {
                let buf = reader.read_bytes_as_vec(4)?;
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
            data = DataPack::Undef(d);

        },
        8 => {  // 6 i16
            let mut d: Vec<i16> = Vec::with_capacity(datalen);
            if datalen <=2 {
                d.push(reader.read_i16()?);
                if datalen == 2 {
                    d.push(reader.read_i16()?);
                } else {
                    reader.skip_ptr(2)?;
                }
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
                d.push(reader.read_i32()?);
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
            let mut d :Vec<SRational> = Vec::with_capacity(datalen);
            let offset = reader.read_u32()? as u64;
            let current = reader.offset()?;
            reader.seek(SeekFrom::Start(offset))?;
            for _ in 0.. datalen { 
                let n_i32 = reader.read_i32()?;
                let d_i32 = reader.read_i32()?;
                d.push(SRational{n:n_i32,d:d_i32});
            }
            reader.seek(SeekFrom::Start(current))?;
            data = DataPack::SRational(d);

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
                let buf = reader.read_bytes_as_vec(4)?;
                for i in 0.. datalen { 
                    d.push(buf[i])
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
                        super::tags::tag_mapper(tagid ,&data,datalen);
                    }
                }
            } else {
                #[cfg(debug_assertions)]
                super::tags::gps_mapper(tagid ,&data,datalen);
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
