use crate::io::*;
use crate::error::ImgError;
use crate::error::ImgErrorKind;

// not support bitmap header V4/V5 yet

#[allow(unused)]
#[derive(Debug)]
pub struct BitmapHeader {
    pub filesize: usize,
    pub image_offset: usize,
    pub width: usize,
    pub height: usize,
    pub bit_count: usize,
    pub compression: Option<Compressions>,
    pub color_table: Option<Vec<ColorTable>>,
    pub bitmap_file_header: BitmapFileHeader,
    pub bitmap_info: BitmapInfo,
}

#[derive(Debug)]
pub struct BitmapFileHeader {
    pub bf_type : u16,
    pub bf_size : u32,
    pub bf_reserved1:u16,
    pub bf_reserved2:u16,
    pub bf_offbits:u32,
}

#[allow(unused)]
#[derive(Debug)]
pub struct BitmapWindowsInfo {
	pub bi_size : u32,
	pub bi_width : u32,
	pub bi_height : u32,
	pub bi_plane : u16,
	pub bi_bit_count : u16,
	pub bi_compression : u32,
	pub bi_size_image : u32,
	pub bi_xpels_per_meter : u32,
	pub bi_ypels_per_meter : u32,
	pub bi_clr_used : u32,
	pub bi_clr_importation : u32,
}

#[derive(Debug)]
pub struct BitmapCore{
	pub bc_size : u32 ,
	pub bc_width : u16,
	pub bc_height : u16,
	pub bc_plane : u16,
	pub bc_bit_count : u16,
}

#[derive(Debug)]
pub enum BitmapInfo {
    Windows(BitmapWindowsInfo),
    Os2(BitmapCore),
}

#[derive(Debug)]
pub struct ColorTable {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub reserved: u8
}

#[derive(Debug)]
pub enum Compressions {
    BiRGB = 0,
    BiRLE8 = 1,
    BiRLE4 = 2,
    BiBitFileds = 3,
    BiJpeg = 4,
    BiPng =5,
}

impl BitmapHeader {
    pub fn new(buffer :&[u8],_opt :usize) -> Result<Self,ImgError> {
        if buffer[0] != b'B' || buffer[1] != b'M' {
            return Err(ImgError::new_const(ImgErrorKind::UnknownFormat,&"Not Bitmap"))
        }

        let bitmap_file_header = BitmapFileHeader {
            bf_type : read_u16le(buffer,0),
            bf_size  :read_u32le(buffer,2),
            bf_reserved1: read_u16le(buffer,6),
            bf_reserved2: read_u16le(buffer,8),
            bf_offbits: read_u32le(buffer,10),
    
        };

        let bi_size = read_u32le(buffer,14);
        let mut ptr :usize = bi_size as usize + 14;
        let width;
        let height;
        let bit_per_count;
        let bitmap_info;
        let compression;
        let mut clut_size;

        if bi_size == 12 {
            let os2header = BitmapCore {
                bc_size : read_u32le(buffer,14),
                bc_width : read_u16le(buffer,18),
                bc_height : read_u16le(buffer,20),
                bc_plane : read_u16le(buffer,22),
                bc_bit_count : read_u16le(buffer,24),
            };
            width = os2header.bc_width as usize;
            height = os2header.bc_height as usize;
            compression = None;
            bit_per_count = os2header.bc_bit_count as usize;
            clut_size = 0;
            bitmap_info = BitmapInfo::Os2(os2header);
        } else {
            let info_header = BitmapWindowsInfo {
                bi_size : read_u32le(buffer,14),
                bi_width : read_u32le(buffer,18),
                bi_height : read_u32le(buffer,22),
                bi_plane : read_u16le(buffer,26),
                bi_bit_count : read_u16le(buffer,28),
                bi_compression : read_u32le(buffer,30),
                bi_size_image : read_u32le(buffer,34),
                bi_xpels_per_meter : read_u32le(buffer,38),
                bi_ypels_per_meter : read_u32le(buffer,42),
                bi_clr_used : read_u32le(buffer,46),
                bi_clr_importation : read_u32le(buffer,50), 
            };
            width = info_header.bi_width as usize;
            height = info_header.bi_height as usize;
            compression = match info_header.bi_compression {
                0 => {Some(Compressions::BiRGB)},
                1 => {Some(Compressions::BiRLE8)},
                2 => {Some(Compressions::BiRLE4)},
                3 => {Some(Compressions::BiBitFileds)},
                4 => {Some(Compressions::BiJpeg)},
                5 => {Some(Compressions::BiPng)},
                _ => {None}
            };
            bit_per_count = info_header.bi_bit_count as usize;
            clut_size = if bit_per_count <= 8 {info_header.bi_clr_used as usize} else {0};
            bitmap_info = BitmapInfo::Windows(info_header);
        }
        if clut_size == 0 && bit_per_count <=8 {
            clut_size = 1 << bit_per_count;
        }

        let mut color_table :Vec<ColorTable> = Vec::with_capacity(clut_size);

        if clut_size > 0 {
            for _ in 0..clut_size {
                match bitmap_info {
                    BitmapInfo::Windows(..) => {
                        color_table.push(ColorTable{
                            blue: read_byte(buffer,ptr),
                            green: read_byte(buffer,ptr+1),
                            red: read_byte(buffer,ptr+2),
                            reserved: read_byte(buffer,ptr+3),
                        });
                        ptr += 4;
                    },
                    BitmapInfo::Os2(..) => {
                        color_table.push(ColorTable{
                            blue: read_byte(buffer,ptr),
                            green: read_byte(buffer,ptr+1),
                            red: read_byte(buffer,ptr+2),
                            reserved: 0,
                        });
                        ptr += 3;
                    },
                }
            }
        }

        let color_table = if color_table.len() > 0 {Some(color_table)} else {None};

        Ok(BitmapHeader {
            filesize: bitmap_file_header.bf_size as usize,
            image_offset: bitmap_file_header.bf_offbits as usize,
            width,
            height,
            bit_count: bit_per_count,
            compression,
            color_table,
            bitmap_file_header,
            bitmap_info,
        })
    }
}

