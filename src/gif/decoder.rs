use super::header::*;
use crate::io::*;
use crate::draw::*;
use crate::decoder::lzw::Lzwdecode;
use crate::color::RGBA;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use crate::warning::ImgWarning;

const SEPARATER: u8 = b',';     // 0x2c
const EXTEND_BLOCK:u8 = b'!';   // 0x21
const COMMENT_LABEL:u8 = 0xfe;
const GRAPHIC_CONTROLE:u8 = 0xf9;
const END_MARKER:u8 = b';';     // 0x3c
const END:u8 = 0x00;



pub fn decode<'decode>(buffer: &[u8],option:&mut DecodeOptions) 
                    -> Result<Option<ImgWarning>,ImgError> {
    let mut header = GifHeader::new(buffer,option.debug_flag)?;
    let mut ptr = header.header_size;
    let mut comment = "".to_string();
    let mut is_transpearent = false;
    let mut transperarent_color = 0x00;
    let mut delay_time = 0 ;

    if option.debug_flag >0 {
        option.drawer.verbose(&format!("{:?}",&header),None)?;
    }


    option.drawer.init(header.width,header.height,None)?;

    loop {
        let c = read_byte(buffer,ptr); ptr += 1;

        match c {   // BLOCK LOOP
            END => {

            },
            EXTEND_BLOCK => {
                let ext = read_byte(buffer, ptr); ptr +=1;
                match ext {
                    END => { },
                    COMMENT_LABEL => {
                        let mut s = "Comment: ".to_string();
                        loop {
                            let len = read_byte(buffer, ptr) as usize; ptr += 1;
                            if len == 0 {break;}
                            if ext == COMMENT_LABEL {
                                s = s.to_owned() + &read_string(buffer, ptr, len);
                            }                            
                            ptr += len;
                        }
                        if option.debug_flag > 0 {
                            option.drawer.verbose(&s,None)?;
                        }
                        comment += &s;
                    },
                    GRAPHIC_CONTROLE => {
                        let len = read_byte(buffer, ptr) as usize;

                        let flag = read_byte(buffer, ptr + 1);
                            
                        delay_time = read_u16le(buffer, ptr + 2);

                        if is_transpearent {
                            header.color_table[transperarent_color].alpha = 0xff;
                        }

                        if flag & 0x1 == 1 {
                            is_transpearent = true
                        } else {
                            is_transpearent = false
                        }

                        transperarent_color = read_byte(buffer, ptr + 4) as usize;

                        if option.debug_flag > 0 {
                            let s = format!("Grahic Controle {} delay {}ms  transpearent {:?}",flag,delay_time,is_transpearent);
                            option.drawer.verbose(&s,None)?;
                        }

                        ptr += len + 1;
                    },
                    0xff => {   // Netscape 2.0 (Animation Flag)
                        loop {
                            let len = read_byte(buffer, ptr) as usize; ptr += 1;
                            if len == 0 {break;}
                            let s = "Animation tag: ".to_owned() + &read_string(buffer,ptr,len); ptr += len;
                            if option.debug_flag > 0 {
                                option.drawer.verbose(&s,None)?;
                            }
                        }
                    },
                    _ => {
                        loop {
                            let len = read_byte(buffer, ptr) as usize; ptr += 1;
                            if len == 0 {break;}
                            ptr += len;
                        }
                    }
                }
            },

            SEPARATER => {
                let lscd = GifLscd::new(buffer,ptr);
                ptr = ptr + 9;
                let has_local_pallet;
                let mut local_color_table = Vec::new();
                if lscd.field & 0x80 == 0x80 {
                    has_local_pallet = true;
                    let color_table_size = (1 << ((lscd.field & 0x07) + 1)) as usize;
                    for _ in 0..color_table_size {
                        let color = RGBA{
                            red: read_byte(buffer, ptr),
                            green: read_byte(buffer, ptr+1),
                            blue: read_byte(buffer, ptr+2),
                            alpha: 0xff,
                        };
                        ptr += 3;
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
                let lzw_min_bits = read_byte(buffer, ptr) as usize; ptr += 1;
                let mut buf :Vec<u8> = Vec::new();
                'lzw_read: loop {
                    let len = read_byte(buffer,ptr) as usize; ptr += 1;
                    if len == 0 {
                        break 'lzw_read;
                    }
                    buf.append(&mut read_bytes(buffer,ptr,len)); ptr += len;
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
                        println!("{}",interlace_y);
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
                return Err(ImgError::new_const(ImgErrorKind::IlligalData,&"read error in gif decode"))
            },
        };
        if ptr >= buffer.len() {
            return Err(ImgError::new_const(ImgErrorKind::OutboundIndex,&"data shotage in gif decode"))
        }
    }

    Ok(None)
}