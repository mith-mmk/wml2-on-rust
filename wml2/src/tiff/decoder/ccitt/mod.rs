mod white;
mod black;

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
    bptr: usize,
    bits: u32,
}


impl BitReader {
    pub fn new(data:&[u8]) -> Self {
        Self {
            buffer: data.to_vec(),
            ptr: 0,
            bptr: 0,
            bits: 0,
        }
    }

    fn fill_bits(&mut self) -> Result<(),ImgError>{
        if self.bptr < 8 && self.ptr < self.buffer.len() {
            return Err(ImgError::new_const(ImgErrorKind::IOError,"buffer overrun".to_string()))
        }

        if self.bptr < 24 {
            if self.ptr < self.buffer.len() {
                self.bits = self.bits << 8 | self.buffer[self.ptr] as u32;
                self.ptr += 1;
                self.bptr += 8;
            }
        }

        Ok(())
    }

    fn get_bit(&self) -> bool {
        if (self.bits >> self.bptr) & 0x01 == 0x01 {
            true
        } else {
            false
        }
    }

    fn get_bits(&mut self,size:usize) -> Result<usize,ImgError> {
        self.fill_bits()?;
        let mut val = 0;
        for _ in 0..size {
            val = val << 1;
            if self.get_bit() {
                val += 1;
            }
            if self.bptr == 0 {break;}
            self.bptr -= 1;
        }
        Ok(val)
    }

    fn value(&mut self, tree:&Value) -> Result<isize,ImgError> {
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
                    return Err(ImgError::new_const(ImgErrorKind::DecodeError, 
                            "A value is none in a table".to_owned()))
                },
            }
        }
        Ok(val)
    }
}

pub fn decode(buf:&[u8],width:usize,height:usize,photometric_interpretation: u16) -> Result<Vec<u8>,ImgError> {
    let mut data = vec![];
    let white = white_tree();
    let black = black_tree();

    let mut reader = BitReader::new(buf);
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