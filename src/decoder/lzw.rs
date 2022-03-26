type Error = Box<dyn std::error::Error>;
use bin_rs::io::read_byte;
use crate::error::ImgError;
use crate::error::ImgErrorKind;


const MAX_TABLE:usize = 4096;

pub struct Lzwdecode {
    buffer: Vec<u8>,
    cbl: usize,
    recovery_cbl: usize,
    last_byte: u64,
    left_bits: usize,
    bit_mask: u64,
    ptr: usize,
    clear: usize,
    end: usize,
    max_table: usize,
    dic: Vec<Vec<u8>>,
    prev_code: usize,
    is_init: bool,
    is_lsb: bool,
    is_tiff: usize,

}

impl Lzwdecode {
    pub fn gif(lzw_min_bits: usize) -> Self {
        Self::new(lzw_min_bits,true,false)
    }

    // no debug
    pub fn tiff(lzw_min_bits: usize,is_lsb: bool) -> Self {
        Self::new(lzw_min_bits,is_lsb,true)
    }

    pub fn new(lzw_min_bits: usize,is_lsb: bool,is_tiff: bool) -> Self{
        let cbl = lzw_min_bits +1;
        let clear_code = 1 << lzw_min_bits;
        let max_table = MAX_TABLE;
        let is_tiff = if is_tiff {1} else {0};
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
            max_table: max_table,         
            dic: Vec::with_capacity(max_table),
            prev_code: clear_code,
            is_init: false,
            is_lsb: is_lsb,   // GIF Must True
            is_tiff: is_tiff, // if tiff set 1
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

    fn get_bits(&mut self) -> Result<usize,Error> {
        if self.is_lsb {
            return self.get_bits_lsb()
        } else {
            return self.get_bits_msb()
        }
    }

    fn get_bits_msb(&mut self) -> Result<usize,Error> {
        let size = self.cbl;
        while self.left_bits <= 16 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IOError, "data shortage".to_string())))
                }
                break;
            }
            self.last_byte = (self.last_byte << 8) | (self.buffer[self.ptr] as u64);
            self.ptr +=1;
            self.left_bits += 8;
        }
        let bits = (self.last_byte >>  (24 - self.left_bits)) & self.bit_mask;

        self.left_bits -= size;
        Ok(bits as usize)
    }

    fn get_bits_lsb(&mut self) -> Result<usize,Error> {
        let size = self.cbl;
        while self.left_bits <= 16 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IOError, "data shortage".to_string())))
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

    // Multi chuck image data decoding is not debug.
    pub fn decode(&mut self,buf: &[u8]) -> Result<Vec<u8>,Error> {
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
            let code = self.get_bits()?;    // GIF Lsb only Tiff use Lsb or Msb

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
                return Err(Box::new(ImgError::new_const(ImgErrorKind::IlligalData, "Over table in LZW".to_string())));
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
                    if self.dic.len() + self.is_tiff == self.bit_mask as usize + 1 && self.dic.len() < self.max_table{ 
                            // tiff use self.dic.len() + 1 
                        self.cbl += 1;
                        self.bit_mask = (self.bit_mask << 1) | 1;
                    }
                }
            }
            self.prev_code = code;
        }
    } 
}