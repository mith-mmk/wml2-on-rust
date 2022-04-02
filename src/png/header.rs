use crate::error::*;
use crate::iccprofile::ICCProfile;
use crate::color::RGBA;
use bin_rs::reader::BinaryReader;
type Error = Box<dyn std::error::Error>;

const SIGNATURE: [u8; 8] = [0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A];
const IMAGE_HEADER:[u8;4] = [b'I',b'H',b'D',b'R'];
const PALLET:[u8;4] = [b'P',b'L',b'T',b'E'];
const IMAGE_DATA:[u8;4] = [b'I',b'D',b'A',b'T'];
const IMAG_EEND:[u8;4] = [b'I',b'E',b'N',b'D'];
const TRANNCEPEARENCY:[u8;4] = [b't',b'R',b'N',b'S'];
const GAMMA:[u8;4] =  [b'g',b'A',b'M',b'A'];
const COLOR_HMR:[u8;4] = [b'c',b'H',b'R',b'M'];
const SRPG:[u8;4] = [b's',b'R',b'P',b'G'];
const ICC_PROFILE:[u8;4] = [b'i',b'C',b'C',b'P'];
const TEXTDATA:[u8;4] = [b't',b'E',b'X',b't'];
const COMPRESSED_TEXTUAL_DATA:[u8;4] = [b'z',b'T',b'X',b't'];
const I18N_TEXT:[u8;4] = [b'i',b'T',b'X',b't'];
const BACKGROUND_COLOR:[u8;4] = [b'b',b'K',b'G',b'D'];
const PHYSICAL_PIXEL_DIMENSION:[u8;4] = [b'p',b'H',b'Y',b's'];
const SIGNIFICANT_BITS:[u8;4] = [b's',b'B',b'I',b'T'];
const STANDARD_PALLET:[u8;4] = [b's',b'P',b'L',b'T'];
const PALLTE_HISTGRAM:[u8;4] = [b'h',b'I',b'S',b'T'];
const MODIFIED_TIME:[u8;4] = [b't',b'I',b'M',b'E'];
/*
// no impl
const FRACTALIMAGE:[u8;4] = [b'f',b'R',b'A',b'c'];
const GIFGRAPHICEXTENTION:[u8;4] = [b'g',b'I',b'F',b'g'];
const GIFTEXTEXTENTION:[u8;4] = [b'g',b'I',b'F',b't'];
const GIFEXTENTION:[u8;4] = [b'g',b'I',b'F',b'x'];
const IMAGEOFFSETS:[u8;4] = [b'o',b'F',b'F',b's'];
const PIXELCALC:[u8;4] = [b'p',b'C',b'A',b'l'];
const SCAL:[u8;4] = [b's',b'C',b'A',b'L'];
*/

// APNG
const ANIMATION_CONTROLE:[u8;4] = [b'a',b'c',b'T',b'L'];
const FRAME_CONTROLE:[u8;4] = [b'f',b'c',b'T',b'L'];
const FRAME_DATA:[u8;4] = [b'f',b'd',b'A',b'T'];


struct PngHeader {
    width: u32,
    height: u32,
    bitpersample: u8,
    color_type: u8,
    compression: u8,
    filter_method: u8,
    interace_method: u8,
    crc: u32,
    image_lenghth: u32,
    pallete: Option<Vec<RGBA>>,
    gamma: Option<u32>,
    transparencey: Option<Vec<u8>>,
    iccprofile: Option<ICCProfile>,
    backgroud_color: Option<u8>,
    sbit: Option<Vec<u8>>,
    modified_time:Option<String>,
    animation:Option<()>,
}

impl PngHeader {
    pub fn new<B:BinaryReader>(&self, reader:&mut B,_opt :usize) -> Result<Self,Error> {
        let signature = reader.read_bytes_as_vec(8)?;
        if signature != SIGNATURE {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::NoSupportFormat, "not PNG".to_string())))
        }

        let mut header = Self {
            width: 0,
            height: 0,
            bitpersample: 32,
            color_type: 2,
            compression: 1,
            filter_method: 0,
            interace_method: 0,
            crc: 0,
            image_lenghth: 0,
            pallete: None,
            gamma: None,
            transparencey: None,
            iccprofile: None,
            backgroud_color: None,
            sbit:None,
            modified_time: None,
            animation: None,
        };

        let mut pallete_size = 0;

        loop {
            let length = reader.read_u32_be()?;
            let chunck = reader.read_bytes_as_vec(4)?;
            if chunck == IMAGE_DATA {
                header.image_lenghth = length;
                break;
            } else if chunck == IMAGE_HEADER {
                if length != 13  {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, "Illegal size IHDR".to_string())))   
                }
                header.width = reader.read_u32_be()?;
                header.height = reader.read_u32_be()?;
                header.bitpersample = reader.read_byte()?;
                header.color_type = reader.read_byte()?;

                match header.color_type {
                    0 => {
                        if header.bitpersample != 1  && header.bitpersample != 2 && header.bitpersample != 4 &&
                            header.bitpersample !=8 && header.bitpersample != 16 {
                                let string = format!("Glayscale must be bitpersample 1,2,4,8,16 but {}",header.bitpersample);
                                return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, string))) 
                        }
                    },
                    2 => {
                        if header.bitpersample !=8 && header.bitpersample != 16 {
                                let string = format!("True colors must be bitpersample 8,16 but {}",header.bitpersample);
                                return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, string))) 
                        }

                    },
                    3 => {
                        if header.bitpersample != 1  && header.bitpersample != 2 && header.bitpersample != 4 &&
                            header.bitpersample !=8 {
                                let string = format!("Index colors must be bitpersample 8,16 but {}",header.bitpersample);
                                return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, string))) 
                        }

                    },
                    4 => {
                        if header.bitpersample !=8 && header.bitpersample != 16 {
                            let string = format!("Glayscale with alpha must be bitpersample 8,16 but {}",header.bitpersample);
                                return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, string))) 
                        }

                    },
                    6 => {
                        if header.bitpersample !=8 && header.bitpersample != 16 {
                            let string = format!("True colors with alpha must be bitpersample 8,16 but {}",header.bitpersample);
                                return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, string))) 
                        }

                    },
                    _ => {
                        let string = format!("Color type {} is unknown",header.color_type);
                        return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, string))) 
                    }
                }

                header.compression = reader.read_byte()?;
                header.filter_method = reader.read_byte()?;
                header.interace_method = reader.read_byte()?;
                let _crc = reader.read_u32_be();
            } else if chunck == PALLET {
                if length % 3 != 0  {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, "Illegal size PLTE".to_string())))   
                }
                pallete_size = length as usize /3;
                let mut pallet:Vec<RGBA> = Vec::with_capacity(pallete_size);
                for _ in 0..pallete_size {
                    let color = RGBA {
                        red: reader.read_byte()?,
                        green: reader.read_byte()?,
                        blue: reader.read_byte()?,
                        alpha: 0xff
                    };
                    pallet.push(color);
                }
                header.pallete = Some(pallet);
            } else if chunck == TRANNCEPEARENCY {
                if header.pallete.is_none() {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, "Illegal format tRNS before PLTE".to_string())))   
                } else if length as usize > pallete_size {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData, "Illegal format tRNS too big,PLTE size".to_string())))   
                }
                if let Some(pallete) = &mut header.pallete {
                    for i in 0..length as usize {
                        pallete[i].alpha = reader.read_byte()?;
                    }  
                }
            } else {
                reader.skip_ptr(length as usize)?;
                let _crc = reader.read_u32_be();
            }
        }
        Ok(
            header
        )
    }
}