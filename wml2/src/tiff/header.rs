//! TIFF header and IFD parsing structures.

/*
 * tiff/header.rs  Mith@mmk (C) 2022
 * use MIT License
 */

type Error = Box<dyn std::error::Error>;

/* for EXIF */

use crate::color::RGBA;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use crate::tiff::util::print_tags;
use bin_rs::Endian;
use bin_rs::io::*;
use bin_rs::reader::BinaryReader;

#[derive(Debug, Clone, PartialEq)]
pub struct Rational {
    pub n: u32,
    pub d: u32,
}

impl Rational {
    pub fn as_f32(&self) -> f32 {
        let n = self.n as f32;
        let d = self.d as f32;
        n / d
    }

    pub fn as_f64(&self) -> f64 {
        let n = self.n as f64;
        let d = self.d as f64;
        n / d
    }

    pub fn denominator(&self) -> u64 {
        self.d as u64
    }

    pub fn numerator(&self) -> u64 {
        self.n as u64
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SRational {
    pub n: i32,
    pub d: i32,
}

impl SRational {
    pub fn as_f32(&self) -> f32 {
        let n = self.n as f64;
        let d = self.d as f64;
        (n / d) as f32
    }

    pub fn as_f64(&self) -> f64 {
        let n = self.n as f64;
        let d = self.d as f64;
        n / d
    }

    pub fn denominator(&self) -> u64 {
        self.d as u64
    }

    pub fn numerator(&self) -> u64 {
        self.n as u64
    }
}

#[derive(Debug, Clone, PartialEq)]
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

impl DataPack {
    pub fn to_string(&self) -> String {
        match self {
            DataPack::Bytes(d) => {
                format!("{:?}", d)
            }
            DataPack::Ascii(d) => d.clone(),
            DataPack::SByte(d) => {
                format!("{:?}", d)
            }
            DataPack::Short(d) => {
                format!("{:?}", d)
            }
            DataPack::Long(d) => {
                format!("{:?}", d)
            }
            DataPack::Rational(d) => {
                format!("{:?}", d)
            }
            DataPack::SRational(d) => {
                format!("{:?}", d)
            }
            DataPack::Float(d) => {
                format!("{:?}", d)
            }
            DataPack::Double(d) => {
                format!("{:?}", d)
            }
            DataPack::SShort(d) => {
                format!("{:?}", d)
            }
            DataPack::SLong(d) => {
                format!("{:?}", d)
            }
            DataPack::Unkown(d) => {
                format!("{:?}", d)
            }
            DataPack::Undef(d) => {
                format!("{:?}", d)
            }
        }
    }

    pub fn get_bytes(&self) -> Option<&Vec<u8>> {
        match self {
            DataPack::Bytes(d) => Some(d),
            DataPack::Undef(d) => Some(d),

            _ => None,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct TiffHeader {
    pub tagid: usize,
    pub data: DataPack,
    pub length: usize,
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct TiffHeaders {
    pub version: u16,
    pub headers: Vec<TiffHeader>,
    pub exif: Option<Vec<TiffHeader>>,
    pub gps: Option<Vec<TiffHeader>>,
    pub endian: Endian,
}

impl TiffHeaders {
    pub fn to_string(&self) -> String {
        print_tags(self)
    }

    pub fn empty(endian: Endian) -> Self {
        Self {
            version: 42,
            headers: Vec::new(),
            exif: None,
            gps: None,
            endian,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
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
            Compression::NoneCompression => "None".to_string(),
            Compression::CCITTHuffmanRLE => "CCITT Huffman RLE".to_string(),
            Compression::CCITTGroup3Fax => "CCITT Group3 Fax".to_string(),
            Compression::CCITTGroup4Fax => "CCITT Group4 Fax".to_string(),
            Compression::LZW => "LZW(Tiff)".to_string(),
            Compression::OldJpeg => "Jpeg(old)".to_string(),
            Compression::Jpeg => "Jpeg".to_string(),
            Compression::AdobeDeflate => "Adobe Deflate".to_string(),
            Compression::Next => "Next".to_string(),
            Compression::CcittrleW => "CCITT RLEW".to_string(),
            Compression::Packbits => "Apple Macintosh Packbits".to_string(),
            Compression::ThunderScan => "Thuder Scan".to_string(),
            Compression::IT8CTPad => "IT8 CTPad".to_string(),
            Compression::IT8LW => "IT8 LW".to_string(),
            Compression::IT8MP => "IT8 MP".to_string(),
            Compression::IT8BL => "IT8 BL".to_string(),
            Compression::PIXARFILM => "Pixar Film".to_string(),
            Compression::PIXARLOG => "Pixar Log".to_string(),
            Compression::DEFLATE => "Deflate".to_string(),
            Compression::DCS => "DCS".to_string(),
            Compression::JBIG => "JBIG".to_string(),
            Compression::SGILOG => "SGI LOG".to_string(),
            Compression::SGILOG24 => "SGI LOG24".to_string(),
            Compression::Jpeg2000 => "JPEG2000".to_string(),
            Compression::Unknown => "Unknown".to_string(),
        }
    }
}

/// This struct is not use embed tiff tag,also EXIF.
/// Use only tiff encoder/decoder
#[derive(Debug, Clone)]
pub struct Tiff {
    /// 0x00fe  also 0
    pub newsubfiletype: u32,
    /// 0x00ff  also 1, but 2,3 is multi part image,This decoder is not support.
    pub subfiletype: u32,
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
    pub fill_order: u16,
    /// Image data start offsets. strip_offsets length need equal strip_byte_counts length
    pub strip_offsets: Vec<u64>,
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
    pub samples_per_pixel: u16,
    /// 0x0116 also width * Bitspersample /8
    pub rows_per_strip: u32,
    /// 0x0117 For each strip(width), the number of bytes in the strip after compression.       
    pub strip_byte_counts: Vec<u64>,
    /// 0x0118 also this decoder does not use.
    pub min_sample_values: Vec<u16>,
    /// 0x0119 also this decoder does not use. default 2**(BitsPerSample) - 1
    pub max_sample_values: Vec<u16>,

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
    pub x_resolution: f32,
    /// 0x011B y dot per inch
    pub y_resolution: f32,

    /// 0x0140 use only bitperpixel <= 8
    /// if color_table is empty,
    /// use standard color pallte or grayscale
    pub color_table: Option<Vec<RGBA>>,

    // no baseline
    pub startx: u32,             // 0x011E
    pub starty: u32,             // 0x011F
    pub predictor: u16,          // 0x013D
    pub extra_samples: Vec<u16>, // 0x0152

    /// TileWidth/TileLength/TileOffsets/TileByteCOunts are using tiled image
    /// TIFF 6.0 Section 15
    pub tile_width: u32, // 0x0142
    pub tile_length: u32,           // 0x0143
    pub tile_offsets: Vec<u64>,     // 0x0144
    pub tile_byte_counts: Vec<u64>, // 0x0145

    // compression option
    // t4_options // G3
    pub t4_options: u32,
    pub t6_options: u32,      // G4
    pub jpeg_tables: Vec<u8>, // jpeg(new)
    pub ycbcr_coefficients: Vec<Rational>,
    pub ycbcr_sub_sampling: Vec<u16>,
    pub ycbcr_positioning: u16,
    pub reference_black_white: Vec<Rational>,
    pub jpeg_proc: u16,
    pub jpeg_restart_interval: u16,
    pub jpeg_interchange_format: Option<u64>,
    pub jpeg_interchange_format_length: Option<u64>,
    pub jpeg_q_tables: Vec<u64>,
    pub jpeg_dc_tables: Vec<u64>,
    pub jpeg_ac_tables: Vec<u64>,

    // metadata
    pub tiff_headers: TiffHeaders,
    pub icc_profile: Option<Vec<u8>>,

    // multi page tiff
    pub multi_page: Box<Vec<Tiff>>,
    // pub predictor // if predictor is 2,pixels are horizontal differencing. not support
}

impl Tiff {
    pub fn empty() -> Self {
        Self {
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
            strip_byte_counts: vec![],
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
            tile_length: 0,
            tile_offsets: vec![],
            tile_byte_counts: vec![],
            t4_options: 0,
            t6_options: 0,
            jpeg_tables: vec![],
            ycbcr_coefficients: vec![
                Rational { n: 299, d: 1000 },
                Rational { n: 587, d: 1000 },
                Rational { n: 114, d: 1000 },
            ],
            ycbcr_sub_sampling: vec![2, 2],
            ycbcr_positioning: 1,
            reference_black_white: vec![
                Rational { n: 0, d: 1 },
                Rational { n: 255, d: 1 },
                Rational { n: 128, d: 1 },
                Rational { n: 255, d: 1 },
                Rational { n: 128, d: 1 },
                Rational { n: 255, d: 1 },
            ],
            jpeg_proc: 1,
            jpeg_restart_interval: 0,
            jpeg_interchange_format: None,
            jpeg_interchange_format_length: None,
            jpeg_q_tables: vec![],
            jpeg_dc_tables: vec![],
            jpeg_ac_tables: vec![],
            tiff_headers: TiffHeaders::empty(Endian::LittleEndian),
            icc_profile: None,
            multi_page: Box::<Vec<Tiff>>::default(),
        }
    }

    pub fn new(reader: &mut dyn BinaryReader) -> Result<Self, Error> {
        super::page::read_pages(reader)
    }

    pub(crate) fn from_headers(tiff_headers: TiffHeaders) -> Result<Self, Error> {
        let mut this = Self::empty();
        let current = &mut this;
        for header in &tiff_headers.headers {
            if header.length == 0 {
                continue;
            }
            match header.tagid {
                0xff => {
                    if let DataPack::Long(d) = &header.data {
                        current.subfiletype = d[0]
                    }
                }
                0xfe => {
                    if let DataPack::Long(d) = &header.data {
                        current.newsubfiletype = d[0];
                    }
                }
                0x100 => {
                    if let DataPack::Long(d) = &header.data {
                        current.width = d[0]
                    } else if let DataPack::Short(d) = &header.data {
                        current.width = d[0] as u32;
                    }
                }
                0x101 => {
                    if let DataPack::Long(d) = &header.data {
                        current.height = d[0]
                    } else if let DataPack::Short(d) = &header.data {
                        current.height = d[0] as u32;
                    }
                }
                0x102 => {
                    if header.length == 1 {
                        if let DataPack::Short(d) = &header.data {
                            current.bitspersample = d[0];
                            current.bitspersamples.push(d[0]);
                        }
                    } else {
                        let mut bpm = 0;
                        for i in 0..header.length {
                            if let DataPack::Short(d) = &header.data {
                                bpm = u16::checked_add(bpm, d[i]).ok_or_else(|| {
                                    ImgError::new_const(
                                        ImgErrorKind::IllegalData,
                                        "BitsPerSample overflow".into(),
                                    )
                                })?;
                                current.bitspersamples.push(d[i]);
                            }
                        }
                        current.bitspersample = bpm;
                    }
                }
                0x103 => {
                    if let DataPack::Short(d) = &header.data {
                        current.compression = match d[0] {
                            1 => Compression::NoneCompression,
                            2 => Compression::CCITTHuffmanRLE,
                            3 => Compression::CCITTGroup3Fax,
                            4 => Compression::CCITTGroup4Fax,
                            5 => Compression::LZW,
                            // obsolete
                            6 => Compression::OldJpeg,
                            7 => Compression::Jpeg,
                            8 => Compression::AdobeDeflate,
                            32766 => Compression::Next,
                            32771 => Compression::CcittrleW,
                            32773 => Compression::Packbits,
                            32809 => Compression::ThunderScan,
                            32895 => Compression::IT8CTPad,
                            32896 => Compression::IT8LW,
                            32897 => Compression::IT8MP,
                            32898 => Compression::IT8BL,
                            32908 => Compression::PIXARFILM,
                            32909 => Compression::PIXARLOG,
                            32946 => Compression::DEFLATE,
                            32947 => Compression::DCS,
                            34661 => Compression::JBIG,
                            34676 => Compression::SGILOG,
                            34677 => Compression::SGILOG24,
                            34712 => Compression::Jpeg2000,
                            _ => Compression::Unknown,
                        };
                    }
                }
                0x106 => {
                    if let DataPack::Short(d) = &header.data {
                        current.photometric_interpretation = d[0];
                    }
                }
                0x10A => {
                    if let DataPack::Short(d) = &header.data {
                        current.fill_order = d[0];
                    }
                }
                0x115 => {
                    if let DataPack::Short(d) = &header.data {
                        current.samples_per_pixel = d[0];
                    }
                }
                0x111 => {
                    let mut strip_offsets = vec![];
                    if let DataPack::Short(d) = &header.data {
                        for i in 0..d.len() {
                            strip_offsets.push(u64::from(d[i]));
                        }
                    } else if let DataPack::Long(d) = &header.data {
                        for i in 0..d.len() {
                            strip_offsets.push(u64::from(d[i]));
                        }
                    }
                    current.strip_offsets = strip_offsets;
                }
                0x116 => {
                    if let DataPack::Short(d) = &header.data {
                        current.rows_per_strip = d[0] as u32;
                    } else if let DataPack::Long(d) = &header.data {
                        current.rows_per_strip = d[0];
                    }
                }
                0x117 => {
                    let mut strip_byte_counts = vec![];
                    if let DataPack::Short(d) = &header.data {
                        for i in 0..d.len() {
                            strip_byte_counts.push(u64::from(d[i]));
                        }
                    } else if let DataPack::Long(d) = &header.data {
                        for i in 0..d.len() {
                            strip_byte_counts.push(u64::from(d[i]));
                        }
                    }
                    current.strip_byte_counts = strip_byte_counts;
                }
                0x118 => {
                    for i in 0..header.length {
                        if let DataPack::Short(d) = &header.data {
                            current.min_sample_values.push(d[i]);
                        }
                    }
                }
                0x119 => {
                    for i in 0..header.length {
                        if let DataPack::Short(d) = &header.data {
                            current.max_sample_values.push(d[i]);
                        }
                    }
                }
                0x11c => {
                    if let DataPack::Short(d) = &header.data {
                        current.planar_config = d[0];
                    }
                }
                0x013d => {
                    // predictor
                    if let DataPack::Short(d) = &header.data {
                        current.predictor = d[0];
                    }
                }
                0x0124 => {
                    // T4Options
                    if let DataPack::Long(d) = &header.data {
                        current.t4_options = d[0];
                    }
                }
                0x0125 => {
                    // T6Options
                    if let DataPack::Long(d) = &header.data {
                        current.t6_options = d[0];
                    }
                }
                0x0140 => {
                    // color table
                    if let DataPack::Short(d) = &header.data {
                        let mut table: Vec<RGBA> = Vec::new();
                        let offset = header.length / 3;
                        for i in 0..offset {
                            let red = (d[i] >> 8) as u8;
                            let green = (d[i + offset] >> 8) as u8;
                            let blue = (d[i + offset * 2] >> 8) as u8;
                            let alpha = 0xff;
                            let color = RGBA {
                                red,
                                green,
                                blue,
                                alpha,
                            };
                            table.push(color);
                        }
                        current.color_table = Some(table)
                    }
                }
                0x142 => {
                    // TileWidth
                    if let DataPack::Short(d) = &header.data {
                        current.tile_width = d[0] as u32;
                    } else if let DataPack::Long(d) = &header.data {
                        current.tile_width = d[0];
                    }
                }
                0x143 => {
                    // TileWidth
                    if let DataPack::Short(d) = &header.data {
                        current.tile_length = d[0] as u32;
                    } else if let DataPack::Long(d) = &header.data {
                        current.tile_length = d[0];
                    }
                }
                0x144 => {
                    // TileOffsets
                    let mut tile_offsets = vec![];
                    if let DataPack::Long(d) = &header.data {
                        for i in 0..d.len() {
                            tile_offsets.push(u64::from(d[i]));
                        }
                    }
                    current.tile_offsets = tile_offsets;
                }
                0x145 => {
                    let mut tile_byte_counts = vec![];
                    if let DataPack::Short(d) = &header.data {
                        for i in 0..d.len() {
                            tile_byte_counts.push(u64::from(d[i]));
                        }
                    } else if let DataPack::Long(d) = &header.data {
                        for i in 0..d.len() {
                            tile_byte_counts.push(u64::from(d[i]));
                        }
                    }
                    current.tile_byte_counts = tile_byte_counts;
                }
                0x0152 => {
                    // ExtraSamples
                    if let DataPack::Short(d) = &header.data {
                        current.extra_samples = d.to_vec();
                    }
                }
                0x015b => {
                    //Jpeg Tables
                    if let DataPack::Undef(d) = &header.data {
                        current.jpeg_tables = d.to_vec();
                    }
                }
                0x0211 => {
                    if let DataPack::Rational(d) = &header.data {
                        current.ycbcr_coefficients = d.to_vec();
                    }
                }
                0x0212 => {
                    if let DataPack::Short(d) = &header.data {
                        current.ycbcr_sub_sampling = d.to_vec();
                    }
                }
                0x0213 => {
                    if let DataPack::Short(d) = &header.data {
                        current.ycbcr_positioning = d[0];
                    }
                }
                0x0214 => {
                    if let DataPack::Rational(d) = &header.data {
                        current.reference_black_white = d.to_vec();
                    } else if let DataPack::Long(d) = &header.data {
                        current.reference_black_white =
                            d.iter().map(|value| Rational { n: *value, d: 1 }).collect();
                    }
                }
                0x0200 => {
                    if let DataPack::Short(d) = &header.data {
                        current.jpeg_proc = d[0];
                    }
                }
                0x0201 => {
                    if let DataPack::Long(d) = &header.data {
                        current.jpeg_interchange_format = Some(u64::from(d[0]));
                    }
                }
                0x0202 => {
                    if let DataPack::Long(d) = &header.data {
                        current.jpeg_interchange_format_length = Some(u64::from(d[0]));
                    }
                }
                0x0203 => {
                    if let DataPack::Short(d) = &header.data {
                        current.jpeg_restart_interval = d[0];
                    }
                }
                0x0207 => {
                    if let DataPack::Long(d) = &header.data {
                        current.jpeg_q_tables = d.iter().map(|value| u64::from(*value)).collect();
                    }
                }
                0x0208 => {
                    if let DataPack::Long(d) = &header.data {
                        current.jpeg_dc_tables = d.iter().map(|value| u64::from(*value)).collect();
                    }
                }
                0x0209 => {
                    if let DataPack::Long(d) = &header.data {
                        current.jpeg_ac_tables = d.iter().map(|value| u64::from(*value)).collect();
                    }
                }
                0x8773 => {
                    //ICC Profile
                    if let DataPack::Undef(d) | DataPack::Bytes(d) = &header.data {
                        current.icc_profile = Some(d.to_vec());
                    }
                }
                _ => {}
            }
        }

        if current.bitspersamples.is_empty() {
            current.bitspersamples.push(current.bitspersample);
        }
        this.tiff_headers = tiff_headers;

        Ok(this)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EncodedTag {
    pub(crate) tagid: u16,
    pub(crate) type_id: u16,
    pub(crate) count: u32,
    pub(crate) payload: Vec<u8>,
}

fn even_padded_len(length: usize) -> usize {
    length + (length & 1)
}

fn append_even_padding(buf: &mut Vec<u8>) {
    if buf.len() & 1 == 1 {
        buf.push(0);
    }
}

fn validate_count(count: usize) -> Result<u32, Error> {
    u32::try_from(count).map_err(|_| {
        Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "TIFF tag count is too large".to_string(),
        )) as Error
    })
}

fn encode_ascii_bytes(data: &str) -> Vec<u8> {
    let mut payload = data.as_bytes().to_vec();
    if payload.last().copied() != Some(0) {
        payload.push(0);
    }
    payload
}

pub(crate) fn encode_tag_data(tag: &TiffHeader, endian: Endian) -> Result<EncodedTag, Error> {
    let tagid = tag.tagid as u16;
    match &tag.data {
        DataPack::Bytes(data) => Ok(EncodedTag {
            tagid,
            type_id: 1,
            count: validate_count(data.len())?,
            payload: data.clone(),
        }),
        DataPack::Ascii(data) => {
            let payload = encode_ascii_bytes(data);
            Ok(EncodedTag {
                tagid,
                type_id: 2,
                count: validate_count(payload.len())?,
                payload,
            })
        }
        DataPack::Short(data) => {
            let mut payload = Vec::with_capacity(data.len() * 2);
            for value in data {
                write_u16(*value, &mut payload, endian);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 3,
                count: validate_count(data.len())?,
                payload,
            })
        }
        DataPack::Long(data) => {
            let mut payload = Vec::with_capacity(data.len() * 4);
            for value in data {
                write_u32(*value, &mut payload, endian);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 4,
                count: validate_count(data.len())?,
                payload,
            })
        }
        DataPack::Rational(data) => {
            let mut payload = Vec::with_capacity(data.len() * 8);
            for value in data {
                write_u32(value.n, &mut payload, endian);
                write_u32(value.d, &mut payload, endian);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 5,
                count: validate_count(data.len())?,
                payload,
            })
        }
        DataPack::SByte(data) => {
            let mut payload = Vec::with_capacity(data.len());
            for value in data {
                write_i8(*value, &mut payload);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 6,
                count: validate_count(data.len())?,
                payload,
            })
        }
        DataPack::Undef(data) | DataPack::Unkown(data) => Ok(EncodedTag {
            tagid,
            type_id: 7,
            count: validate_count(data.len())?,
            payload: data.clone(),
        }),
        DataPack::SShort(data) => {
            let mut payload = Vec::with_capacity(data.len() * 2);
            for value in data {
                write_i16(*value, &mut payload, endian);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 8,
                count: validate_count(data.len())?,
                payload,
            })
        }
        DataPack::SLong(data) => {
            let mut payload = Vec::with_capacity(data.len() * 4);
            for value in data {
                write_i32(*value, &mut payload, endian);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 9,
                count: validate_count(data.len())?,
                payload,
            })
        }
        DataPack::SRational(data) => {
            let mut payload = Vec::with_capacity(data.len() * 8);
            for value in data {
                write_i32(value.n, &mut payload, endian);
                write_i32(value.d, &mut payload, endian);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 10,
                count: validate_count(data.len())?,
                payload,
            })
        }
        DataPack::Float(data) => {
            let mut payload = Vec::with_capacity(data.len() * 4);
            for value in data {
                write_f32(*value, &mut payload, endian);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 11,
                count: validate_count(data.len())?,
                payload,
            })
        }
        DataPack::Double(data) => {
            let mut payload = Vec::with_capacity(data.len() * 8);
            for value in data {
                write_f64(*value, &mut payload, endian);
            }
            Ok(EncodedTag {
                tagid,
                type_id: 12,
                count: validate_count(data.len())?,
                payload,
            })
        }
    }
}

fn serialize_encoded_tag(
    buf: &mut Vec<u8>,
    append: &mut Vec<u8>,
    tag: &EncodedTag,
    next_data_offset: &mut usize,
    endian: Endian,
) -> Result<(), Error> {
    write_u16(tag.tagid, buf, endian);
    write_u16(tag.type_id, buf, endian);
    write_u32(tag.count, buf, endian);

    if tag.payload.len() <= 4 {
        write_bytes(&tag.payload, buf);
        for _ in tag.payload.len()..4 {
            write_byte(0, buf);
        }
        return Ok(());
    }

    let data_offset = u32::try_from(*next_data_offset).map_err(|_| {
        Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "TIFF data offset exceeds 32-bit range".to_string(),
        )) as Error
    })?;
    write_u32(data_offset, buf, endian);
    append.extend_from_slice(&tag.payload);
    append_even_padding(append);
    *next_data_offset += even_padded_len(tag.payload.len());
    Ok(())
}

fn clone_ifd_tags(tags: &[TiffHeader]) -> Vec<TiffHeader> {
    tags.to_vec()
}

fn filter_base_ifd_tags(tags: &[TiffHeader], has_exif: bool, has_gps: bool) -> Vec<TiffHeader> {
    let mut tags = clone_ifd_tags(tags);
    if has_exif {
        tags.retain(|tag| tag.tagid != 0x8769);
    }
    if has_gps {
        tags.retain(|tag| tag.tagid != 0x8825);
    }
    tags.sort_by_key(|tag| tag.tagid);
    tags
}

fn serialize_ifd(
    tags: &[TiffHeader],
    exif: Option<&Vec<TiffHeader>>,
    gps: Option<&Vec<TiffHeader>>,
    absolute_offset: usize,
    next_ifd_offset: u32,
    endian: Endian,
) -> Result<Vec<u8>, Error> {
    let has_exif = exif.map(|tags| !tags.is_empty()).unwrap_or(false);
    let has_gps = gps.map(|tags| !tags.is_empty()).unwrap_or(false);
    let tags = filter_base_ifd_tags(tags, has_exif, has_gps);

    let mut encoded = Vec::with_capacity(tags.len());
    let mut local_payload_len = 0usize;
    for tag in &tags {
        let encoded_tag = encode_tag_data(tag, endian)?;
        if encoded_tag.payload.len() > 4 {
            local_payload_len += even_padded_len(encoded_tag.payload.len());
        }
        encoded.push(encoded_tag);
    }

    let entry_count = encoded.len() + has_exif as usize + has_gps as usize;
    let ifd_size = 2usize
        .checked_add(entry_count * 12)
        .and_then(|size| size.checked_add(4))
        .and_then(|size| size.checked_add(local_payload_len))
        .ok_or_else(|| {
            Box::new(ImgError::new_const(
                ImgErrorKind::InvalidParameter,
                "TIFF IFD is too large".to_string(),
            )) as Error
        })?;

    let exif_offset = if has_exif {
        Some(absolute_offset + ifd_size)
    } else {
        None
    };
    let exif_bytes = if let Some(exif) = exif {
        if exif.is_empty() {
            None
        } else {
            Some(serialize_ifd(
                exif,
                None,
                None,
                exif_offset.ok_or_else(|| {
                    Box::new(ImgError::new_const(
                        ImgErrorKind::InvalidParameter,
                        "TIFF EXIF offset is missing".to_string(),
                    )) as Error
                })?,
                0,
                endian,
            )?)
        }
    } else {
        None
    };
    let gps_offset = if has_gps {
        Some(absolute_offset + ifd_size + exif_bytes.as_ref().map(|buf| buf.len()).unwrap_or(0))
    } else {
        None
    };
    let gps_bytes = if let Some(gps) = gps {
        if gps.is_empty() {
            None
        } else {
            Some(serialize_ifd(
                gps,
                None,
                None,
                gps_offset.ok_or_else(|| {
                    Box::new(ImgError::new_const(
                        ImgErrorKind::InvalidParameter,
                        "TIFF GPS offset is missing".to_string(),
                    )) as Error
                })?,
                0,
                endian,
            )?)
        }
    } else {
        None
    };

    if let Some(offset) = exif_offset {
        encoded.push(EncodedTag {
            tagid: 0x8769,
            type_id: 4,
            count: 1,
            payload: u32::to_ne_bytes(u32::try_from(offset).map_err(|_| {
                Box::new(ImgError::new_const(
                    ImgErrorKind::InvalidParameter,
                    "TIFF EXIF offset exceeds 32-bit range".to_string(),
                )) as Error
            })?)
            .to_vec(),
        });
    }
    if let Some(offset) = gps_offset {
        encoded.push(EncodedTag {
            tagid: 0x8825,
            type_id: 4,
            count: 1,
            payload: u32::to_ne_bytes(u32::try_from(offset).map_err(|_| {
                Box::new(ImgError::new_const(
                    ImgErrorKind::InvalidParameter,
                    "TIFF GPS offset exceeds 32-bit range".to_string(),
                )) as Error
            })?)
            .to_vec(),
        });
    }
    encoded.sort_by_key(|tag| tag.tagid);

    let mut buf = Vec::with_capacity(ifd_size);
    let mut append = Vec::with_capacity(local_payload_len);
    let mut next_data_offset = absolute_offset + 2 + entry_count * 12 + 4;

    write_u16(entry_count as u16, &mut buf, endian);
    for tag in &encoded {
        if tag.tagid == 0x8769 || tag.tagid == 0x8825 {
            write_u16(tag.tagid, &mut buf, endian);
            write_u16(4, &mut buf, endian);
            write_u32(1, &mut buf, endian);
            let offset = if tag.tagid == 0x8769 {
                exif_offset.ok_or_else(|| {
                    Box::new(ImgError::new_const(
                        ImgErrorKind::InvalidParameter,
                        "TIFF EXIF offset is missing".to_string(),
                    )) as Error
                })?
            } else {
                gps_offset.ok_or_else(|| {
                    Box::new(ImgError::new_const(
                        ImgErrorKind::InvalidParameter,
                        "TIFF GPS offset is missing".to_string(),
                    )) as Error
                })?
            };
            write_u32(offset as u32, &mut buf, endian);
            continue;
        }

        serialize_encoded_tag(&mut buf, &mut append, tag, &mut next_data_offset, endian)?;
    }
    write_u32(next_ifd_offset, &mut buf, endian);
    buf.append(&mut append);
    if let Some(mut exif_bytes) = exif_bytes {
        buf.append(&mut exif_bytes);
    }
    if let Some(mut gps_bytes) = gps_bytes {
        buf.append(&mut gps_bytes);
    }
    Ok(buf)
}

/// Serializes one TIFF/EXIF tag into an IFD entry plus optional out-of-line
/// payload bytes.
pub fn write_tag(
    buf: &mut Vec<u8>,
    append: &mut Vec<u8>,
    tag: &TiffHeader,
    last_offset: &mut usize,
    endian: &Endian,
) -> Result<(), Error> {
    let encoded = encode_tag_data(tag, *endian)?;
    serialize_encoded_tag(buf, append, &encoded, last_offset, *endian)
}

/// Serializes a [`TiffHeaders`] tree to a complete TIFF byte stream.
pub fn tiff_headers_to_bytes(tags: &TiffHeaders) -> Result<Vec<u8>, Error> {
    tiff_pages_to_bytes(std::slice::from_ref(tags))
}

pub(crate) fn tiff_pages_to_bytes(pages: &[TiffHeaders]) -> Result<Vec<u8>, Error> {
    let Some(first) = pages.first() else {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "TIFF requires at least one page".to_string(),
        )));
    };

    let endian = first.endian;
    let version = 42;
    let mut offsets = Vec::with_capacity(pages.len());
    let mut next_offset = 8usize;
    for page in pages {
        if page.endian != endian {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::InvalidParameter,
                "all TIFF pages must use the same endian".to_string(),
            )));
        }
        if page.version != first.version {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::InvalidParameter,
                "all TIFF pages must use the same version".to_string(),
            )));
        }

        offsets.push(next_offset);
        let ifd = serialize_ifd(
            &page.headers,
            page.exif.as_ref(),
            page.gps.as_ref(),
            next_offset,
            0,
            endian,
        )?;
        next_offset = next_offset.checked_add(ifd.len()).ok_or_else(|| {
            Box::new(ImgError::new_const(
                ImgErrorKind::InvalidParameter,
                "TIFF header size overflow".to_string(),
            )) as Error
        })?;
    }

    let mut buf = Vec::new();
    match endian {
        Endian::BigEndian => write_bytes(b"MM", &mut buf),
        Endian::LittleEndian => write_bytes(b"II", &mut buf),
    }
    write_u16(version, &mut buf, endian);
    write_u32(8, &mut buf, endian);

    for (index, page) in pages.iter().enumerate() {
        let next_ifd_offset = if let Some(next) = offsets.get(index + 1) {
            u32::try_from(*next).map_err(|_| {
                Box::new(ImgError::new_const(
                    ImgErrorKind::InvalidParameter,
                    "TIFF next IFD offset exceeds 32-bit range".to_string(),
                )) as Error
            })?
        } else {
            0
        };
        let mut ifd = serialize_ifd(
            &page.headers,
            page.exif.as_ref(),
            page.gps.as_ref(),
            offsets[index],
            next_ifd_offset,
            endian,
        )?;
        buf.append(&mut ifd);
    }

    Ok(buf)
}

/// Serializes a [`TiffHeaders`] tree to an EXIF-compatible TIFF byte stream.
pub fn exif_to_bytes(tags: &TiffHeaders) -> Result<Vec<u8>, Error> {
    tiff_headers_to_bytes(tags)
}

/// Serializes a [`TiffHeaders`] tree into `buf` and returns the first
/// `StripOffsets` value when present.
pub fn write_tags(buf: &mut Vec<u8>, tags: &TiffHeaders) -> Result<usize, Error> {
    let image_offset = tags
        .headers
        .iter()
        .find(|tag| tag.tagid == 0x0111)
        .and_then(|tag| match &tag.data {
            DataPack::Short(values) => values.first().map(|value| *value as usize),
            DataPack::Long(values) => values.first().map(|value| *value as usize),
            _ => None,
        })
        .unwrap_or(0);

    let mut bytes = tiff_headers_to_bytes(tags)?;
    buf.append(&mut bytes);
    Ok(image_offset)
}

pub fn read_tags(reader: &mut dyn BinaryReader) -> Result<TiffHeaders, Error> {
    super::ifd::TiffDocument::read(reader)?.headers()
}
