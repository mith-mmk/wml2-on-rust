use super::header::*;
use crate::io::*;
use crate::draw::*;
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
const MAX_TABLE:usize = 4096;


struct Lzwdecode {
    buffer: Vec<u8>,
    cbl: usize,
    recovery_cbl: usize,
    last_byte: u64,
    left_bits: usize,
    bit_mask: u64,
    ptr: usize,
    clear: usize,
    end: usize,
    dic: Vec<Vec<u8>>,
    prev_code: usize,
    is_init: bool,
}

impl Lzwdecode {

    pub fn new(lzw_min_bits: usize) -> Self{
        let cbl = lzw_min_bits +1;
        let clear_code = 1 << lzw_min_bits;
        Self {
            buffer: Vec::new(),
            cbl: cbl,
            recovery_cbl: cbl,
            bit_mask: (1 << cbl) -1,
            last_byte: 0,
            left_bits: 0,
            ptr: 0,
            clear: clear_code,
            end:   clear_code+ 1,            
            dic: Vec::with_capacity(MAX_TABLE),
            prev_code: clear_code,
            is_init: false,
        }
    }
    
    fn fill_bits(&mut self) {
        self.clear_dic();
        self.last_byte = 0;
        let ptr = self.ptr;
        self.last_byte = read_byte(&self.buffer,  ptr) as u64;
        self.last_byte |= (read_byte(&self.buffer,ptr + 1) as u64) << 8;
        self.last_byte |= (read_byte(&self.buffer,ptr + 2) as u64) << 16;
        self.left_bits = 24;
        self.ptr = 3;

    }

    fn get_bits(&mut self) -> Result<usize,ImgError> {
        let size = self.cbl;
        while self.left_bits <= 16 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
                    return Err(ImgError::new_const(ImgErrorKind::IOError, &"data shortage"))
                }
                break;
            }
            self.last_byte = (self.last_byte >> 8) & 0xffff | ((self.buffer[self.ptr] as u64) << 16);
            self.ptr +=1;
            self.left_bits += 8;
        }
        let bits = (self.last_byte >>  (24 - self.left_bits)) & self.bit_mask;

        self.left_bits -= size;
        Ok(bits as usize)
    }

    fn clear_dic(&mut self) {
        self.dic = (0..self.end+1).map(|i| if i < self.clear { vec![i as u8] } else {vec![]}).collect();
        self.cbl = self.recovery_cbl;
        self.bit_mask = (1 << self.cbl) - 1;

    }

    pub fn decode(&mut self,buf: &[u8]) -> Result<Vec<u8>,ImgError> {
        self.buffer = buf.to_vec();
        if self.is_init == false {
            self.fill_bits();
            self.is_init = true;
        } else {
            self.ptr = 0;
        }

        let mut data :Vec<u8> = Vec::new();
        self.prev_code = self.clear;  // NULL

        loop {
            let code = self.get_bits()?;



            if code == self.clear {
                for p in &self.dic[self.prev_code] {
                    data.push(*p);
                }

                self.clear_dic();
            } else if code == self.end {
                let mut table :Vec<u8> = Vec::new();
                for p in &self.dic[self.prev_code] {
                    data.push(*p);
                    table.push(*p);
                }
                let append_code = self.dic[self.prev_code][0];
                table.push(append_code);
                return Ok(data)
            } else if code > self.dic.len() {
                return Err(ImgError::new_const(ImgErrorKind::IlligalData, &"Over table in LZW"));
            } else {
                let append_code;
                if code == self.dic.len() {
                    append_code = self.dic[self.prev_code][0];
                } else {
                    append_code = self.dic[code][0];
                }
                if self.prev_code != self.end && self.prev_code != self.clear {
                    let mut table :Vec<u8> = Vec::new();
                    for p in &self.dic[self.prev_code] {
                        data.push(*p);
                        table.push(*p);
                    }
                    table.push(append_code);
                    self.dic.push(table);
                    if self.dic.len() == self.bit_mask as usize + 1 && self.dic.len() < MAX_TABLE{
                        self.cbl += 1;
                        self.bit_mask = (self.bit_mask << 1) | 1;
                    }
                }
            }
            self.prev_code = code;
        }
    } 
}


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
                let mut decoder = Lzwdecode::new(lzw_min_bits);
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

                let mut interlace_start_y = [0,4,2,1];
                let mut interlace_delta_y = [8,8,4,2];
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