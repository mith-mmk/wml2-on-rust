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
    pub working_bits: usize,
    pub max_bits: usize,
    pub append_bits: usize,
    pub matrix: Vec<(i32,i32)>,
    pub append: Vec<(i32,i32)>,
}

#[derive(Debug)]
pub enum Mode {
    Pass,
    Horiz,
    V0,
    Vr1,
    Vr2,
    Vr3,
    Vl1,
    Vl2,
    Vl3,
    D2Ext(usize),
    D1Ext(usize),
    None
}


impl Mode {
    fn get(reader:&mut BitReader) -> Result<Mode,Error> {
        if reader.get_bits(1)? == 1 {
            return Ok(Mode::V0)
        }
        let mode = reader.get_bits(2)?;

        match mode {
            0b01 => {
                return Ok(Mode::Horiz)
            },
            0b10 => {
                return Ok(Mode::Vl1)
            },
            0b11 => {
                return Ok(Mode::Vr1)
            }
            _ => {}
        }
        // 0001
        if reader.get_bits(1)? == 1 {
            return Ok(Mode::Pass)
        }
        // 00001
        if reader.get_bits(1)? == 1 {
            if reader.get_bits(1)? == 0 {
                return Ok(Mode::Vl2)
            } else {
                return Ok(Mode::Vr2)
            }
        }
        // 000001
        if reader.get_bits(1)? == 1 {
            if reader.get_bits(1)? == 0 {
                return Ok(Mode::Vl3)
            } else {
                return Ok(Mode::Vr3)
            }
        }

        // 0000001
        if reader.get_bits(1)? == 1 {
            let val = reader.get_bits(3)?;
            return Ok(Mode::D2Ext(val))
        }
        
        if reader.get_bits(2)? == 1 {
            let val = reader.get_bits(3)?;
            return Ok(Mode::D1Ext(val))
        }

        Ok(Mode::None)
    }
}




pub struct BitReader {
    pub buffer: Vec<u8>,
    ptr: usize,
    left_bits: usize,
    last_byte: u32,
    is_lsb:bool,
    flag:bool,
}


impl BitReader {
    pub fn new(data:&[u8],is_lsb:bool) -> Self {
        let this = Self {
            buffer: data.to_vec(),
            last_byte: 0,
            ptr: 0,
            left_bits: 0,
            is_lsb: is_lsb,
            flag: false
        };
//        this.fill_bits();

        this
    }

    // use 32bit
    fn fill_bits(&mut self) {
        self.last_byte = 0;
        let ptr = self.ptr;
        if self.is_lsb {
            self.last_byte = (read_byte(&self.buffer,  ptr).reverse_bits() as u32) << 24;
            self.last_byte |= (read_byte(&self.buffer,ptr + 1).reverse_bits() as u32) << 16;
            self.last_byte |= (read_byte(&self.buffer,ptr + 2).reverse_bits() as u32) << 8;
            self.last_byte |= read_byte(&self.buffer,ptr + 3).reverse_bits() as u32;
        } else {
            self.last_byte = (read_byte(&self.buffer,  ptr) as u32) << 24;
            self.last_byte |= (read_byte(&self.buffer,ptr + 1) as u32) << 16;
            self.last_byte |= (read_byte(&self.buffer,ptr + 2) as u32) << 8;
            self.last_byte |= read_byte(&self.buffer,ptr + 3) as u32;
        }
        self.left_bits = 32;
        self.ptr = 4;

    }

    fn look_bits(&mut self,size:usize) -> Result<usize,Error> {
        if self.is_lsb {
            return self.look_bits_lsb(size)
        } else {
            return self.look_bits_msb(size)
        }
    }

    fn get_bits(&mut self,size:usize) -> Result<usize,Error> {
        if self.is_lsb {
            return self.get_bits_lsb(size)
        } else {
            return self.get_bits_msb(size)
        }
    }

    fn look_bits_msb(&mut self,size:usize) -> Result<usize,Error> {
        while self.left_bits <= 24 {
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

        let bits = (self.last_byte >> (self.left_bits - size)) & ((1 << size) - 1);

        Ok(bits as usize)
    }

    fn get_bits_msb(&mut self,size:usize) -> Result<usize,Error> {
        let bits = self.look_bits_msb(size);
        self.left_bits -= size;
        bits
    }

    fn skip_bits(&mut self,size:usize) {
        self.left_bits -= size;
    }

    fn look_bits_lsb(&mut self,size:usize) -> Result<usize,Error> {
        while self.left_bits <= 24 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
 //                   return Err(Box::new(ImgError::new_const(ImgErrorKind::IOError, "data shortage".to_string())))
                }
                break;
            }
            self.last_byte = (self.last_byte << 8) | (self.buffer[self.ptr].reverse_bits() as u32);
            self.ptr +=1;
            self.left_bits += 8;
        }

        let bits = (self.last_byte >> (self.left_bits - size)) & ((1 << size) - 1);

        Ok(bits as usize)

    }


    fn get_bits_lsb(&mut self,size:usize) -> Result<usize,Error> {
        let bits = self.look_bits_lsb(size);
        self.left_bits -= size;
        bits
    }

    fn value(&mut self,tree:&HuffmanTree) -> Result<i32,Error> {
        let pos = self.look_bits(tree.working_bits)?;
        let (mut bits, mut val) = tree.matrix[pos];
        if self.flag {
            if tree.working_bits == 9 {
                println!("{} {} {:09b} {}",bits,val,pos,tree.working_bits);
            } else {
                println!("{} {} {:06b} {}",bits,val,pos,tree.working_bits);
            }

        }
        if bits == -1 {
            let pos = self.look_bits(tree.max_bits)?;
            (bits,val) = tree.append[pos];
            if self.flag {
                if tree.working_bits == 9 {
                    println!("{} {} {:012b} {}",bits,val,pos,tree.working_bits);
                } else {
                    println!("{} {} {:013b} {}",bits,val,pos,tree.working_bits);
                }
        
            }
            if bits == -1 { //fill
                self.skip_bits(1);
                return self.value(tree);
            }
        }
        self.skip_bits(bits as usize);
        Ok(val)
    }

    fn run_len(&mut self,tree: &HuffmanTree) -> Result<i32,Error> {
        let mut run_len = self.value(&tree)?;
        if run_len == -2 {
            return Ok(-2);
        }
        let mut tolal_run = run_len;
        while run_len >= 64 {
            run_len = self.value(&tree)?;
            tolal_run += run_len;
        }
        Ok(tolal_run)
    }
 
    // skip next byte
    fn flush(&mut self) {
        self.left_bits -= self.left_bits % 8;
    }
}
  

pub fn decode(buf:&[u8],header: &Tiff) -> Result<Vec<u8>,Error> {

    let width = header.width.clone() as usize;
    let height = header.height.clone() as usize;
    let photometric_interpretation = header.photometric_interpretation.clone();
    let is_lsb = if header.fill_order == 2 {true} else {false};
    let mut data = vec![];
    let white = if photometric_interpretation == 0 {white_tree()} else {black_tree()};
    let black = if photometric_interpretation == 1 {white_tree()} else {black_tree()};


    let mut reader = BitReader::new(buf,is_lsb);
    let mut x = 0;
    let mut y = 0;

    let mut code1;

    
    let mut white_run = [0_usize;64];
    let mut black_run = [0_usize;64];
    // seek first EOL
    loop {
        code1 = reader.look_bits(12)?;
        if code1 != 0 { break };
        reader.skip_bits(1);
    }
    if code1 == 1 {
        reader.skip_bits(12);
    }

    let mut next_line_2d = false;

    /*
    if header.compression != Compression::CCITTHuffmanRLE {
        let bit = reader.get_bits(1)?;
        if bit == 0 {
            next_line_2d = true;
        }
    }*/

    let mode = if next_line_2d { Mode::get(&mut reader)? } else { Mode::Horiz }; 


    loop {


        if y >= height {
            break;
        }

        if y >= 487 && y <= 490 {
            println!("X{} y{}",x,y);
        }

        if y >= 487 {reader.flag = true}
        if y >= 491 {reader.flag = false}

      
        let run_len = reader.run_len(&white)?;
        white_run[run_len as usize % 64] = white_run[run_len as usize % 64] + 1;

        if run_len == -2 {  // EOL

            for _ in x..width {
                data.push(0x00);
            }

            x = 0;
            y += 1;
//            continue;
        } 
//        let ex = width.min(x + run_len as usize);

        for _ in 0..run_len {
            data.push(0x00);
            x += 1;
        }

        let run_len = reader.run_len(&black)?;
        black_run[run_len as usize % 64] = black_run[run_len as usize % 64] + 1;

        if run_len == -2 {  // EOL
            for _ in x..width {
                data.push(0x00);
            }
            /*
            let bit = reader.get_bits(1)?;
            if bit == 0 {
                next_line_2d = true;
            }*/
            x = 0;
            y += 1;
//            continue;
        } 
//        let ex = width.min(x + run_len as usize + 1);
//        let ex = x + run_len as usize + 1;
        for _ in 0..run_len {
            data.push(0xff);
            x += 1;
        }
        if x >= width {
            x = 0;
            y += 1;
        }
    }

    Ok(data)    
}
