mod white;
mod black;
type Error = Box<dyn std::error::Error>;
use crate::tiff::decoder::Tiff;
use bin_rs::io::read_byte;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
pub use white::white_tree;
pub use black::black_tree;

#[derive(Debug)]
pub struct HuffmanTree{
    pub tree: Value
}

#[derive(Debug)]
pub enum Value {
    Tree2(Box<[Value;2]>),
    Tree4(Box<[Value;4]>),
    Tree8(Box<[Value;8]>),
    Tree(Box<[Value;16]>),
    Value(u32),
    EOL,
    None,
}

pub struct BitReader {
    pub buffer: Vec<u8>,
    ptr: usize,
    left_bits: usize,
    last_byte: u32,
    bit_mask: u32,
    is_lsb:bool,
}


impl BitReader {
    pub fn new(data:&[u8],is_lsb:bool) -> Self {
        let mut this = Self {
            buffer: data.to_vec(),
            last_byte: 0,
            ptr: 0,
            left_bits: 0,
            bit_mask: 0,
            is_lsb: is_lsb,
        };
        this.fill_bits();

        this
    }

    // use 32bit
    fn fill_bits(&mut self) {
        self.last_byte = 0;
        let ptr = self.ptr;
        if self.is_lsb {
            self.last_byte = read_byte(&self.buffer,  ptr) as u32;
            self.last_byte |= (read_byte(&self.buffer,ptr + 1) as u32) << 8;
            self.last_byte |= (read_byte(&self.buffer,ptr + 2) as u32) << 16;
        } else {
            self.last_byte = (read_byte(&self.buffer,  ptr) as u32) << 16;
            self.last_byte |= (read_byte(&self.buffer,ptr + 1) as u32) << 8;
            self.last_byte |= read_byte(&self.buffer,ptr + 2) as u32;
        }
        self.left_bits = 24;
        self.ptr = 3;

    }

    fn get_bits(&mut self,size:usize) -> Result<usize,Error> {
        if self.is_lsb {
            return self.get_bits_lsb(size)
        } else {
            return self.get_bits_msb(size)
        }
    }

    fn get_bits_msb(&mut self,size:usize) -> Result<usize,Error> {
        while self.left_bits <= 16 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IOError, "data shortage".to_string())))
                }
                break;
            }

            self.last_byte = (self.last_byte << 8) | (self.buffer[self.ptr] as u32);
            self.ptr +=1;
            self.left_bits += 8;
        }
        let bits = (self.last_byte >> (self.left_bits - size)) & self.bit_mask;

        self.left_bits -= size;
        Ok(bits as usize)
    }

    fn get_bits_lsb(&mut self,size:usize) -> Result<usize,Error> {
        while self.left_bits <= 16 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::IOError, "data shortage".to_string())))
                }
                break;
            }
            self.last_byte = (self.last_byte >> 8) & 0xffff | ((self.buffer[self.ptr] as u32) << 16);

            self.ptr +=1;
            self.left_bits += 8;
        }
        let bits = (self.last_byte >>  (24 - self.left_bits)) & self.bit_mask;

        self.left_bits -= size;
        Ok(bits as usize)
    }

    fn value(&mut self, tree:&Value) -> Result<isize,Error> {
        let val;
        let mut tree = tree;
    
        loop {
            match tree {
                Value::Value(v) => {
                    val = *v as isize;
                    break;
                },
                Value::Tree(next_tree) => {
                    let i = self.get_bits(4)?;
                    tree = &next_tree[i];
                },
                Value::Tree2(next_tree) => {
                    let i = self.get_bits(1)?;
                    tree = &next_tree[i];
                },
                Value::Tree4(next_tree) => {
                    let i = self.get_bits(2)?;
                    tree = &next_tree[i];
                },
                Value::Tree8(next_tree) => {
                    let i = self.get_bits(3)?;
                    tree = &next_tree[i];
                },
                Value::EOL => {
                    val = -1;
                    break;
                },
                Value::None => {
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, 
                            "A value is none in a table".to_owned())))
                },
            }
        }
        Ok(val)
    }
}

pub fn decode(buf:&[u8],header: &Tiff) -> Result<Vec<u8>,Error> {
    let width = header.width.clone() as usize;
    let height = header.height.clone() as usize;
    let photometric_interpretation = header.photometric_interpretation.clone();
    let is_lsb = if header.fill_order==2 {true} else {false};

    let mut data = vec![];
    let white = white_tree();
    let black = black_tree();

    let mut reader = BitReader::new(buf,is_lsb);
    let mut x = 0;
    let mut y = 0;

    loop {
        if y >= height {
            break;
        }
        let mut run_len = reader.value(&white.tree)?;
        if run_len == -1 {
            x = 0;
            y += 1;
            continue;
        } else if run_len >= 64 {
            run_len += reader.value(&white.tree)?;
        }
        for _ in 0..run_len {
            if photometric_interpretation == 0 {
                data.push(0xff);
                data.push(0xff);
                data.push(0xff);
                data.push(0xff);
            } else {
                data.push(0x00);
                data.push(0x00);
                data.push(0x00);
                data.push(0x00);
            }
            x += 1;
        }
        let mut run_len = reader.value(&black.tree)?;
        if run_len == -1 {
            x = 0;
            y += 1;
            continue;
        } else if run_len >= 64 {
            run_len += reader.value(&black.tree)?;
        }

        for _ in 0..run_len {
            if photometric_interpretation == 0 {
                data.push(0x00);
                data.push(0x00);
                data.push(0x00);
                data.push(0x00);
            } else {
                data.push(0xff);
                data.push(0xff);
                data.push(0xff);
                data.push(0xff);
            }

            x += 1;
        }
        if x >= width {
            x = 0;
            y += 1;
        }
    }

    Ok(data)    
}