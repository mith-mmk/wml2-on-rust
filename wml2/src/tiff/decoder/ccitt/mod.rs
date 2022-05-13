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

#[derive(Debug)]
#[derive(PartialEq)]
pub enum Mode {
    Pass,
    Horiz,
    V(usize),
    Vl(usize),
    D2Ext(usize),
    D1Ext(usize),
    EOL,
    None
}


impl Mode {
    fn get(reader:&mut BitReader) -> Result<Mode,Error> {
        if reader.look_bits(12)? == 1 {
            reader.skip_bits(12);
            return Ok(Mode::EOL)
        }

        if reader.get_bits(1)? == 1 {
            return Ok(Mode::V(0))        // 1
        }
        // 0xx
        let mode = reader.get_bits(2)?;

        match mode {
            0b01 => {   // 001
                return Ok(Mode::Horiz)
            },
            0b10 => {   // 010
                return Ok(Mode::Vl(1))
            },
            0b11 => {   // 011
                return Ok(Mode::V(1))
            }
            _ => {}
        }

        // 000x
        if reader.get_bits(1)? == 1 {
            return Ok(Mode::Pass)   // 0001
        }

        // 0000x
        if reader.get_bits(1)? == 1 {
            if reader.get_bits(1)? == 0 {
                return Ok(Mode::Vl(2))  // 000010
            } else {
                return Ok(Mode::V(2))   // 000011
            }
        }

        // 000000x
        if reader.get_bits(1)? == 1 {
            if reader.get_bits(1)? == 0 {
                return Ok(Mode::Vl(3))  // 0000010
            } else {
                return Ok(Mode::V(3))   // 0000011
            }
        }

        // 000000x
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

    fn get_bits_msb(&mut self,size:usize) -> Result<usize,Error> {
        let bits = self.look_bits_msb(size);
        self.skip_bits(size);
        bits
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


    fn get_bits_lsb(&mut self,size:usize) -> Result<usize,Error> {
        let bits = self.look_bits_lsb(size);
        self.skip_bits(size);
        bits
    }

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
    fn flush(&mut self) {
        self.left_bits -= self.left_bits % 8;
    }

}

fn uncompress(reader:&mut BitReader) -> Result<Vec<u8>,Error> {
    let mut buf:Vec<u8> = Vec::new();
    loop {
        let mut val = reader.look_bits(12)?;
        if val == 1 { // EOL
            reader.skip_bits(12);
            return Ok(buf)
        } 
        val = val >> 1;
        if val < 32 {
            match val {
                1 => {
                    buf.append(&mut vec![0,0,0,0]);
                    reader.skip_bits(11);                    
                }
                2..=3 => {
                    buf.append(&mut vec![0,0,0]);
                    reader.skip_bits(10);   
                },
                4..=7 => {
                    buf.append(&mut vec![0,0]);
                    reader.skip_bits(9);   
                },
                8..=15 => {
                    buf.push(0);
                    reader.skip_bits(8);   
                },
                16..=31 => {
                    reader.skip_bits(7);   
                },
                _ => {

                }
            }
            break;
        }
        val = val >> 5;
        match val {
            0 => {
                buf.append(&mut vec![0,0,0,0,0]);
                reader.skip_bits(6);                    
            }
            1 => {
                buf.append(&mut vec![0,0,0,0,0xff]);
                reader.skip_bits(5);                    
            },
            2..=3=> {
                buf.append(&mut vec![0,0,0,0xff]);
                reader.skip_bits(4);                    
            }
            4..=7 => {
                buf.append(&mut vec![0,0,0xff]);
                reader.skip_bits(3);   
            },
            8..=15 => {
                buf.append(&mut vec![0,0xff]);
                reader.skip_bits(2);   
            },
            _ => {
                buf.push(0xff);
                reader.skip_bits(1);  
            },
        }
    }
    Ok(buf)
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


    let mut encoding = 1;   //1D

    if header.compression == Compression::CCITTGroup4Fax {
        encoding = 3;
    }

    if t4_options & 0x01 > 0 {
        encoding = 2;   //2D
//        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT 3G 2D is no support".to_string())))
    }
    if t4_options & 0x2 > 0 || t6_options & 0x2 > 0 {
//        encoding = 0;   // UNCOMPRESSED
        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT uncompress is no support".to_string())))

    }



    let width = header.width.clone() as usize;
    let height = header.height.clone() as usize;
    let photometric_interpretation = header.photometric_interpretation.clone();
    let is_lsb = if header.fill_order == 2 {true} else {false};
    let mut data =vec![0_u8;width * height];
    let white = if photometric_interpretation == 0 {white_tree()} else {black_tree()};
    let black = if photometric_interpretation == 1 {white_tree()} else {black_tree()};

    let mut reader = BitReader::new(buf,is_lsb);

    let mut code1;

    // seek first EOL
    if header.compression == Compression::CCITTGroup3Fax {
        loop {
            code1 = reader.look_bits(12)?;
            if code1 > 0 { break };
            reader.skip_bits(1);
        }
        if code1 == 1 {
            reader.skip_bits(12);
        }    
    }

    let mut is2d = if header.compression == Compression::CCITTGroup4Fax {true} else {false};
    if encoding == 2 {
        if reader.get_bits(1)? == 0 {
            is2d = true
        }
    }

    let mut codes = vec![];

    let mut y = 0;

    while y < height {
        if encoding == 2 {
            print!("\ny {:04} ",y);
        }

        let mut a0 = 0;
        let mut code_num = 1;
        let mut b1 = 0;
        let mut eol = false;
        let pre_codes = codes;
        codes = vec![0];

        while a0 < width && !eol {
            let mode = if is2d { Mode::get(&mut reader)? } else { Mode::Horiz }; 
            match mode {
                Mode::Horiz => {
                    let mut len1 = reader.run_len(&white)?;
                    if len1 == -2 {  // EOL
                        eol = true;
                        len1 = 0;
                    }
                    if codes.len() & 0x1 > 0 {
                        codes.push(len1 as usize);
                        if encoding == 2 {
                            print!("Hw {} ",len1);
                        }
                    } else {
                        len1 = 0;
                    }

                    let mut len2 = reader.run_len(&black)?;
                    if len2 == -2 {  // EOL
                        eol = true;
                        len2 = 0
                    }
                    codes.push(len2 as usize);
                    if encoding == 2 {
                        print!("Hb {} ",len2);
                    }
                    a0 += len1 as usize + len2 as usize;
                },

                Mode::V(n) => { 
                    while b1 <= a0  && b1 < width && pre_codes.len() > code_num {
                        b1 += pre_codes[code_num];
                        code_num += 1;
                    }
                    print!("V{} ",n);
                    let len = (b1 + n).checked_sub(a0).unwrap_or(0);
                    codes.push(len);
                    a0 = b1 + n;
                    print!("{} ",a0);
                },
                Mode::Vl(n) => { 
                    print!("Vl{} ",n);
                    while b1 <= a0  && b1 < width && pre_codes.len() > code_num {
                        b1 += pre_codes[code_num];
                        code_num += 1;
                    }
                    if b1 > a0 + n {
                        codes.push(b1 - a0 - n);
                    } else if b1 >  n {
                        codes.push(n);
                    }
                    a0 = b1 - n;
                    print!("{} ",a0);
                },

                Mode::Pass => {
                    b1 += if pre_codes.len() > code_num + 1 {
                        pre_codes[code_num] + pre_codes[code_num + 1]
                    } else if pre_codes.len() > code_num {
                        pre_codes[code_num]
                    } else {
                        0
                    };
                    code_num += 2;
                    a0 = b1;
                    print!("Ps {} ",a0);
                },
                Mode::D2Ext(val) => {
                    print!("2D Ext ({}) ",val );   
//                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT 3G 2D Ext is not support".to_string())))
                }
                Mode::D1Ext(val) => {
                    print!("1D Ext ({}) ",val );
//                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT 3G 1D Ext is not support".to_string())))
                }
                Mode::None => { // fill ?
                    print!("None ");
                    break;
//                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT 3G 2D decodes error".to_string())))
                },
                Mode::EOL => {
                    print!("EOL ");
                    eol = true;
                    break;
                }
            }
        }

        if a0 < width  {
//            codes.push(width - a0);
        }
        codes.push(0);
        codes.push(0);


        let mut sum = 0;
        print!(" {:?} ",codes);
        let mut ptr = y * width; 
        for (i,code) in codes.iter().enumerate() {
            let code = if sum + code >= width {width - sum} else {*code};
            sum += code;
            if i & 0x1 == 1 {
                for _ in 0..code {
                    data[ptr] = 0;
                    ptr += 1;
                }
            } else {
                for _ in 0..code {
                    data[ptr] = 0xff;
                    ptr += 1;
                }
            }
        }    
        

        y += 1;
        if y > height {break;}

        if encoding <= 2 && !eol {
            let mut code1;

            loop {
                code1 = reader.look_bits(12)?;
                if code1 == 1 {     // skip left 
                    reader.skip_bits(12);
                    break 
                };
                reader.skip_bits(1);
            }
        }

        if encoding == 2 {
            is2d = if reader.get_bits(1)? == 0 { true } else { false };
            print!(" is2d {} ",is2d);
        }
 
        if header.compression == Compression::CCITTHuffmanRLE {
            reader.flush();
        }
    }

    Ok((data,reader.warning))    
}
