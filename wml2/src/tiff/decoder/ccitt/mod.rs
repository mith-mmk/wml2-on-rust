mod white;
mod black;
type Error = Box<dyn std::error::Error>;
use crate::tiff::decoder::Tiff;
use crate::tiff::header::Compression;
use crate::tiff::header::DataPack;
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

/*
#[derive(Debug)]
#[derive(PartialEq)]
pub enum Mode {
    Pass,
    Horiz,
    V,
    Vr(usize),
    Vl(usize),
    D2Ext(usize),
    D1Ext(usize),
    EOL,
    None
}
*/

/*
impl Mode {
    fn get(reader:&mut BitReader) -> Result<Mode,Error> {
        if reader.look_bits(12)? == 1 {
            reader.skip_bits(12);
            return Ok(Mode::EOL)
        }
        if reader.get_bits(1)? == 1 {
            return Ok(Mode::V(0))        // 1
        }
        let mode = reader.get_bits(2)?;

        match mode {
            0b01 => {   // 001
                return Ok(Mode::Horiz)
            },
            0b10 => {   // 010
                return Ok(Mode::V(-1))
            },
            0b11 => {   // 011
                return Ok(Mode::V(1))
            }
            _ => {}
        }

        if reader.get_bits(1)? == 1 {
            return Ok(Mode::Pass)   // 0001
        }
        if reader.get_bits(1)? == 1 {
            if reader.get_bits(1)? == 0 {
                return Ok(Mode::V(-2))  // 000010
            } else {
                return Ok(Mode::V(2))   // 000011
            }
        }
        if reader.get_bits(1)? == 1 {
            if reader.get_bits(1)? == 0 {
                return Ok(Mode::V(-3))  // 0000010
            } else {
                return Ok(Mode::V(3))   // 0000011
            }
        }

        // 0000001
        if reader.get_bits(1)? == 1 {
            let val = reader.get_bits(3)?;
            return Ok(Mode::D2Ext(val))  // 0000001xxx
        }
        
        if reader.get_bits(2)? == 1 {
            let val = reader.get_bits(3)?;
            return Ok(Mode::D1Ext(val))  // 00000001xxx
        }
        Ok(Mode::None)
    }
}
*/



pub struct BitReader {
    pub buffer: Vec<u8>,
    ptr: usize,
    left_bits: usize,
    last_byte: u32,
    is_lsb:bool,
    warning:bool,
}


impl BitReader {
    pub fn new(data:&[u8],is_lsb:bool) -> Self {
        let this = Self {
            buffer: data.to_vec(),
            last_byte: 0,
            ptr: 0,
            left_bits: 0,
            is_lsb: is_lsb,
            warning: false,
        };
//        this.fill_bits();

        this
    }


    fn look_bits(&mut self,size:usize) -> Result<usize,Error> {
        if self.is_lsb {
            return self.look_bits_lsb(size)
        } else {
            return self.look_bits_msb(size)
        }
    }

    /*
    fn get_bits(&mut self,size:usize) -> Result<usize,Error> {
        if self.is_lsb {
            return self.get_bits_lsb(size)
        } else {
            return self.get_bits_msb(size)
        }
    }
    */

    fn look_bits_msb(&mut self,size:usize) -> Result<usize,Error> {
        while self.left_bits <= 24 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
                    self.warning = true;
                    if size >=  12 {
                        return Ok(0x1) // send EOL
                    } else {
                        return Ok(0x0)
                    }
                }
            }

            self.last_byte = (self.last_byte << 8) | (self.buffer[self.ptr] as u32);
            self.ptr +=1;
            self.left_bits += 8;
        }

        let bits = (self.last_byte >> (self.left_bits - size)) & ((1 << size) - 1);

        Ok(bits as usize)
    }



    fn skip_bits(&mut self,size:usize) {
        self.left_bits -= size;
    }

    fn look_bits_lsb(&mut self,size:usize) -> Result<usize,Error> {
        while self.left_bits <= 24 {
            if self.ptr >= self.buffer.len() { 
                if self.left_bits <= 8 && self.left_bits < size {
                    self.warning = true;
                    if size >=  12 {
                        return Ok(0x1) // send EOL
                    } else {
                        return Ok(0x0)
                    }
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

    /*
    fn get_bits_msb(&mut self,size:usize) -> Result<usize,Error> {
        let bits = self.look_bits_msb(size);
        self.skip_bits(size);
        bits
    }

    fn get_bits_lsb(&mut self,size:usize) -> Result<usize,Error> {
        let bits = self.look_bits_lsb(size);
        self.skip_bits(size);
        bits
    }
    */

    fn value(&mut self,tree:&HuffmanTree) -> Result<i32,Error> {
        let pos = self.look_bits(tree.working_bits)?;
        let (mut bits, mut val) = tree.matrix[pos];
        if bits == -1 {
            let pos = self.look_bits(tree.max_bits)?;
            (bits,val) = tree.append[pos];
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
//    fn flush(&mut self) {
//        self.left_bits -= self.left_bits % 8;
//    }
}
  

pub fn decode(buf:&[u8],header: &Tiff) -> Result<(Vec<u8>,bool),Error> {
    let mut t4_options = 0;
    for header in &header.tiff_headers.headers {
        if header.tagid == 0x0124 {
            if let DataPack::Long(option) = &header.data {
                t4_options = option[0];
                break;
            }
        }
    }
    let mut t6_options = 0;
    for header in &header.tiff_headers.headers {
        if header.tagid == 0x0124 {
            if let DataPack::Long(option) = &header.data {
                t6_options = option[0];
                break;
            }
        }
    }

    /*
    let mut encoding = 1;   //1D

    if header.compression == Compression::CCITTGroup4Fax {
        encoding = 2;
    }*/

    if t4_options & 0x01 > 0 {
//        encoding = 2;   //2D
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT uncompress is no support".to_string())))
    }
    if t4_options & 0x2 > 0 || t6_options & 0x2 > 0 {
//        encoding = 0;   // UNCOMPRESSED
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT uncompress is no support".to_string())))

    }



    let width = header.width.clone() as usize;
    let height = header.height.clone() as usize;
    let photometric_interpretation = header.photometric_interpretation.clone();
    let is_lsb = if header.fill_order == 2 {true} else {false};
    let mut data = vec![];
    let white = if photometric_interpretation == 0 {white_tree()} else {black_tree()};
    let black = if photometric_interpretation == 1 {white_tree()} else {black_tree()};


    let mut reader = BitReader::new(buf,is_lsb);


    let mut code1;

    // seek first EOL
    if header.compression == Compression::CCITTGroup3Fax {
        loop {
            code1 = reader.look_bits(12)?;
            if code1 != 0 { break };
            reader.skip_bits(1);
        }
        if code1 == 1 {
            reader.skip_bits(12);
        }    
    }

//    let mut next_line_2d = if header.compression == Compression::CCITTGroup4Fax {true} else {false};

    let mut x = 0;
    let mut y = 0;
    let mut eol = true;
    /*
    if encoding == 2 && header.compression == Compression::CCITTGroup3Fax {
        if reader.get_bits(1)? == 0 {
            next_line_2d = true
        }
    }
    */

//    let mut mode = if next_line_2d { Mode::get(&mut reader)? } else { Mode::Horiz }; 
//    let mut mode = Mode::Horiz; 
//    let mut codes = vec![];
//    let mut code_num = 0;

    while y < height {
        while x < width && !eol {
            let run_len = reader.run_len(&white)?;
            if run_len == -2 {  // EOL
                eol = true
            }
            let run_len = run_len.min((width - x) as i32);

            for _ in 0..run_len {
                data.push(0x00);
                x += 1;
            }
            let run_len = reader.run_len(&black)?;
            
            let run_len = run_len.min((width - x) as i32);
            if run_len == -2 {  // EOL
                eol = true;
            }

            for _ in 0..run_len {
                data.push(0xff);
                x += 1;
            }
        }

        for _ in x..width {
            data.push(0x00);
        }

//        code_num = 0;
        x = 0;
        y += 1;
        /*
        if encoding == 2 {
            if reader.look_bits(12)? == 1 {  // EOL?
                reader.skip_bits(12);
            }
            if header.compression == Compression::CCITTGroup3Fax {
                let v = reader.get_bits(1)?;
                if v == 0 {
                    next_line_2d = true
                } else {
                    next_line_2d = false
                }    
            }
        }*/
        
            /*
            mode = if next_line_2d { Mode::get(&mut reader)? } else { Mode::Horiz }; 

            if !next_line_2d {
                codes.clear();
            }
            */
            eol = false;
            /*
            if header.compression == Compression::CCITTHuffmanRLE {
                reader.flush();
            }

        } else {
           if next_line_2d { mode =  Mode::get(&mut reader)? }; 
        }
        */

    }

    Ok((data,reader.warning))    
}
