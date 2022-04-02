type Error = Box<dyn std::error::Error>;
use crate::warning::ImgWarnings;
use bin_rs::reader::BinaryReader;
use super::header::*;
use crate::draw::*;
use crate::decoder::lzw::Lzwdecode;
use crate::color::RGBA;
use crate::error::ImgError;
use crate::error::ImgErrorKind;


const SEPARATER: u8 = b',';     // 0x2c
const EXTEND_BLOCK:u8 = b'!';   // 0x21
const COMMENT_LABEL:u8 = 0xfe;
const GRAPHIC_CONTROLE:u8 = 0xf9;
const END_MARKER:u8 = b';';     // 0x3c
const END:u8 = 0x00;



pub fn decode<'decode, B: BinaryReader>(reader:&mut B ,option:&mut DecodeOptions) 
                    -> Result<Option<ImgWarnings>,Error> {
    let mut header = GifHeader::new(reader,option.debug_flag)?;
    let mut ptr = header.header_size;
    let mut comment = "".to_string();
    let mut is_transpearent = false;
    let mut transperarent_color = 0x00;
    let mut delay_time = 0 ;
    let warnings:Option<ImgWarnings> = None;

    if option.debug_flag >0 {
        option.drawer.verbose(&format!("{:?}",&header),None)?;
    }


    option.drawer.init(header.width,header.height,None)?;

    loop {
        let c = reader.read_byte()?;

        match c {   // BLOCK LOOP
            END => {

            },
            EXTEND_BLOCK => {
                let ext = reader.read_byte()?;
                match ext {
                    END => { },
                    COMMENT_LABEL => {
                        let mut s = "Comment: ".to_string();
                        loop {
                            let len = reader.read_byte()? as usize;
                            if len == 0 {break;}
                            if ext == COMMENT_LABEL {
                                s = s.to_owned() + &reader.read_ascii_string(len)?;
                            }                            
                            ptr += len;
                        }
                        if option.debug_flag > 0 {
                            option.drawer.verbose(&s,None)?;
                        }
                        comment += &s;
                    },
                    GRAPHIC_CONTROLE => {
                        let _len = reader.read_byte()? as usize; //5
                        let flag = reader.read_byte()?;                            
                        delay_time = reader.read_u16_le()?;

                        if is_transpearent {
                            header.color_table[transperarent_color].alpha = 0xff;
                        }

                        if flag & 0x1 == 1 {
                            is_transpearent = true
                        } else {
                            is_transpearent = false
                        }

                        transperarent_color = reader.read_byte()? as usize;

                        if option.debug_flag > 0 {
                            let s = format!("Grahic Controle {} delay {}ms  transpearent {:?}",flag,delay_time,is_transpearent);
                            option.drawer.verbose(&s,None)?;
                        }
                        reader.read_byte()?;// is 00
                    },
                    0xff => {   // Netscape 2.0 (Animation Flag)
                        loop {
                            let len = reader.read_byte()? as usize;
                            if len == 0 {break;}
                            let s = "Animation tag: ".to_owned() + &reader.read_ascii_string(len)?;
                            if option.debug_flag > 0 {
                                option.drawer.verbose(&s,None)?;
                            }
                        }
                    },
                    _ => {
                        loop {
                            let len = reader.read_byte()? as usize;
                            if len == 0 {break;}
                            ptr += len;
                        }
                    }
                }
            },

            SEPARATER => {
                let lscd = GifLscd::new(reader)?;
                ptr = ptr + 9;
                let has_local_pallet;
                let mut local_color_table = Vec::new();
                if lscd.field & 0x80 == 0x80 {
                    has_local_pallet = true;
                    let color_table_size = (1 << ((lscd.field & 0x07) + 1)) as usize;
                    for _ in 0..color_table_size {
                        let color = RGBA{
                            red: reader.read_byte()?,
                            green: reader.read_byte()?,
                            blue: reader.read_byte()?,
                            alpha: 0xff,
                        };
                        local_color_table.push(color);
                    }
                    if is_transpearent {
                        local_color_table[transperarent_color].alpha = 0x00;
                    }
                } else {
                    has_local_pallet = false;
                    if is_transpearent {
                        header.color_table[transperarent_color].alpha = 0x00;
                    }
                    header.color_table[transperarent_color].alpha = 0xff;
                }
                // LZW block 
                let lzw_min_bits = reader.read_byte()? as usize;
                let mut buf :Vec<u8> = Vec::new();
                'lzw_read: loop {
                    let len = reader.read_byte()? as usize;
                    if len == 0 {
                        break 'lzw_read;
                    }
                    buf.append(&mut reader.read_bytes_as_vec(len)?);
                }
                let mut decoder = Lzwdecode::gif(lzw_min_bits);
                let data = decoder.decode(&buf)?;
//                use weezl::{BitOrder, decode::Decoder};
//                let data = Decoder::new(BitOrder::Lsb,lzw_min_bits as u8).decode(&buf);
//                let data = data.unwrap();
                let color_table = if has_local_pallet {&local_color_table} else {&header.color_table};

                let width = if (lscd.xsize as usize) < header.width {lscd.xsize as usize} else {header.width};
                let height = if (lscd.ysize as usize) < header.height {lscd.ysize as usize} else {header.height};
                if option.debug_flag > 0 {
                    option.drawer.verbose(&format!("{:?}",lscd),None)?;
                }

                let interlace_start_y = [0,4,2,1];
                let interlace_delta_y = [8,8,4,2];
                let mut interlace_mode = 0;
                let mut interlace_y = interlace_start_y[interlace_mode];
                let is_interlace = if (lscd.field & 0x40) == 0x40 {true} else {false};

                for y in lscd.ystart as usize..height {
                    let mut line :Vec<u8> = vec![0;width*4];
                    let offset = y * width;
                    for x in 0..width as usize {
                        let color = data[offset + x] as usize;
                        line[x*4  ] = color_table[color].red;
                        line[x*4+1] = color_table[color].green;
                        line[x*4+2] = color_table[color].blue;
                        line[x*4+3] = color_table[color].alpha;

                    }
                    if is_interlace {
                        option.drawer.draw(lscd.xstart as usize,interlace_y,width,1,&line,None)?;
                        if interlace_y == 16 {
                            interlace_y += 8;
                        } else {
                            interlace_y += interlace_delta_y[interlace_mode];
                        }
                        if interlace_y >= height {
                            interlace_mode += 1;
                            if interlace_mode >= interlace_start_y.len() {break}
                            interlace_y = interlace_start_y[interlace_mode];
                        }
                    } else {
                        option.drawer.draw(lscd.xstart as usize,y,width,1,&line,None)?;
                    }
                }


                let result = option.drawer.next(Some(NextOptions::wait((delay_time * 10) as u64)))?;
                if let Some(response) = result {
                    if response.response == ResposeCommand::Abort {
                        return Ok(None);
                    }
                }
                
            },
            END_MARKER => {
                break;
            },
            _ => {
                return Err(Box::new(ImgError::new_const(ImgErrorKind::IllegalData,"read error in gif decode".to_string())))
            },
        };
    }

    Ok(warnings)
}