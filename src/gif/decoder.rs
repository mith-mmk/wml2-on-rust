use crate::draw::NextOptions;
use crate::color::RGBA;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use crate::warning::ImgWarning;
use crate::draw::DecodeOptions;
use super::header::*;
use crate::io::*;

const SEPARATER: u8 = b',';
const EXTEND_BLOCK:u8 = b'!';
const COMMENT_LABEL:u8 = 0xfe;
const GRAPHIC_CONTROLE:u8 = 0xf9;
const END_MARKER:u8 = b';';
const END:u8 = 0x00;

// under construction...
/*
struct LzwInfrate {
    buffer: Vec<u8>,
    cbl: usize,
    rcbl: usize,
    last_byte: u64,
    left_bits: usize,
    ptr: usize,
    clear: usize,
    end: usize,
    max_bits: usize,
    max_table:usize,
    dic: Vec<Vec<u8>>,
    dic_size: usize,
    is_init: bool,
}

impl LzwInfrate {
    const CLEAR: usize = 0x100;
    const END: usize = 0x101;
    const DATA: usize = 0x102;

    pub fn new(cbl: usize) -> Self{
        Self {
            buffer: Vec::new(),
            cbl: cbl +1,
            rcbl: cbl +1,
            last_byte: 0,
            left_bits: 0,
            ptr: 0,
            clear:  1 << cbl,
            end:  1 << cbl +1,            
            max_bits: MAX_BITS,
            max_table: MAX_TABLE,
            dic: Vec::with_capacity(MAX_TABLE),
            dic_size: 0,
            is_init: false,
        }
    }

    fn get_bits(&mut self) -> Result<usize,ImgError> {
        let size = self.cbl;
        while self.left_bits <= 8 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
                    return Err(ImgError::new_const(ImgErrorKind::IOError, &"data shortage"))
                }
                break;
            }
            self.last_byte = (self.last_byte >> 8) & 0xff | ((self.buffer[self.ptr] as u64) << 8) as u64;
            self.ptr +=1;
            self.left_bits += 8;
        }
        let mask = ((1 << size) - 1) << (16 - self.left_bits);
        let bits = (self.last_byte & mask) >>  (16 - self.left_bits);
        self.left_bits -= size;
        Ok(bits as usize)
    }

    fn init_table(&mut self) {
        self.dic = (0..LzwInfrate::END+1).map(|i| [i as u8].to_vec()).collect();
        self.cbl = self.rcbl;
        self.dic_size = Self::END+1;
    }

    pub fn infrate(&mut self,buf: &[u8]) -> Result<Vec<u8>,ImgError> {
        if self.is_init == false {
            self.init_table();
            self.last_byte = 0;
            self.buffer = buf.to_vec();
            self.last_byte = read_byte(buf, 0) as u64;
            self.last_byte |= (read_byte(buf, 1) as u64) << 8 ;
            self.left_bits = 16;
            self.ptr = 2;
            self.is_init = true;
        } else {
            self.ptr = 0;
        }

        let mut data :Vec<u8> = Vec::new();
        let mut table :Vec<u8> = Vec::new();
        let mut prev_d = 0;

        loop {
            let d = self.get_bits()?;
            println!("data {} {:08x}",d,self.ptr);

            if d == self.clear {
                self.init_table();
                prev_d = d;
                let d = self.get_bits()?;
                self.dic.push(vec![prev_d as u8,d as u8]);
                println!("*dic[]{} d {:?}",self.dic.len(),self.dic[self.dic.len() - 1]);
//                table = vec![d as u8];
            } else if d == self.end {
                return Ok(data);
            } else if d == self.dic.len() {
//                self.dic.push(table);
//                table = vec![d as u8];
            } else if d < self.max_table {
                //
                
            } else {
                return Err(ImgError::new_const(ImgErrorKind::IlligalData, &"Over number in LZW"));
            }
        }
    } 
}
*/

pub fn decode<'decode>(buffer: &[u8],option:&mut DecodeOptions) 
                    -> Result<Option<ImgWarning>,ImgError> {
    let mut header = GifHeader::new(buffer,option.debug_flag)?;
    let mut ptr = header.header_size;
    let mut comment = "".to_string();
    let mut is_transpearent = false;
    let mut transperarent_color = 0x00;
    let mut delay_time = 0 ;

    println!("{:?}",&header);


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
                            let len = read_byte(buffer, ptr) as usize; ptr += len;
                            if len == 0 {break;}
                            if ext == COMMENT_LABEL {
                                s = s.to_owned() + &read_string(buffer, ptr, len);
                            }                            
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

                        ptr += len;

                    },
                    _ => {
                        let len = read_byte(buffer, ptr) as usize;
                        ptr += len;
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
                println! ("LZW Minimum Code Side {} ",lzw_min_bits);
                let mut buf :Vec<u8> = Vec::new();
                'lzw_read: loop {
                    let len = read_byte(buffer,ptr) as usize; ptr += 1;
                    if len == 0 {
                        break 'lzw_read;
                    }
                    buf.append(&mut read_bytes(buffer,ptr,len)); ptr += len;
                }
//                let mut infrater = LzwInfrate::new(lzw_min_bits);
//                let data = infrater.infrate(&buf)?;
                use weezl::{BitOrder, decode::Decoder};
                let data = Decoder::new(BitOrder::Lsb,lzw_min_bits as u8).decode(&buf);
                let data = data.unwrap();
                let color_table = if has_local_pallet {&local_color_table} else {&header.color_table};

                let width = if (lscd.xsize as usize) < header.width {lscd.xsize as usize} else {header.width};
                let height = if (lscd.ysize as usize) < header.height {lscd.ysize as usize} else {header.height};
                println!("{:?}",lscd);
                println!("{} {} {} = {}",width,height,width*height,data.len());


                for y in 0..height {
                    let mut line :Vec<u8> = vec![0;width*4];
                    let offset = y * width;
                    for x in 0..width as usize {
                        let color = data[offset + x] as usize;
                        line[x*4  ] = color_table[color].red;
                        line[x*4+1] = color_table[color].green;
                        line[x*4+2] = color_table[color].blue;
                        line[x*4+3] = color_table[color].alpha;

                    }
                    option.drawer.draw(0,y,width,1,&line,None)?;
                }


                option.drawer.next(Some(NextOptions::wait((delay_time * 10) as u64)))?;
                
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