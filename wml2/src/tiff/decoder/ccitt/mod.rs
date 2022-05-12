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
        encoding = 2;
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
    let mut data = Vec::with_capacity(width * height);
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
    if encoding == 2 && header.compression == Compression::CCITTGroup3Fax {
        if reader.get_bits(1)? == 0 {
            is2d = true
        }
    }

    let mut codes = vec![];

    let mut y = 0;

    while y < height {
        print!("\ny {:04} {:4} ",y,codes.len());

        let mut a0 = 0;
        let mut code_num = 0;
        let mut eol = false;
        let pre_codes = codes.clone();
        codes = vec![];

        while a0 < width && !eol {
            let mode = if is2d { Mode::get(&mut reader)? } else { Mode::Horiz }; 
            match mode {
                Mode::Horiz => {
                    let run_len = reader.run_len(&white)?;
                    if run_len == -2 {  // EOL
                        eol = true;
                    } else {
                        let run_len = run_len.min((width - a0) as i32);
                        a0 += run_len as usize; // a1

                        for _ in 0..run_len {
                            data.push(0x00);
                        }
                        codes.push(a0);
//                        code_num += 1;
                    }

                    let run_len = reader.run_len(&black)?;
                    if run_len == -2 {  // EOL
                        eol = true;
                    } else {
                        let run_len = run_len.min((width - a0) as i32);
                        a0 += run_len as usize; // a2

                        for _ in 0..run_len {
                            data.push(0xff);
                        }
                        print!("{} ",a0);
                        codes.push(a0);
//                       code_num += 1;
                    }
                },

                Mode::V(n) => { 
                    print!("V{} ",n);

                    let b1 = if pre_codes.len() > code_num {pre_codes[code_num]} else {width}; 
                    let run_len = b1 + n - a0 ;  //a1 - a0
                    let run_len = run_len.min(width - a0);
                    a0 += run_len as usize;
                    codes.push(a0);
                    //let mut ptr = data.len() - width + n;   // b0
                    let color = if code_num % 2 == 0 {0} else {0xff};
                    code_num += 1;
                    for _ in 0..run_len {
                    //    let color = data[ptr];
                    //    ptr += 1;
                        data.push(color);
                    } 

                    print!("{} ",a0);
                },
                Mode::Vl(n) => { 
                    print!("Vl{} ",n);
                    if pre_codes.len() > code_num {
                        let b1 = if pre_codes.len() > code_num {pre_codes[code_num]} else {width}; 
                        let run_len = b1 - n - a0 ;  //a1 - a0
                        let run_len = run_len.min(width - a0);
                        a0 += run_len as usize;
                        codes.push(a0);
                   //     let mut ptr = data.len() - width - n;   // b0
                        let color = if code_num % 2 == 0 {0} else {0xff};
                        code_num += 1;
//                        let color = data[ptr];
                        for _ in 0..run_len {
//                            let color = data[ptr];
//                            ptr += 1;
                            data.push(color);
                        } 
                    }
                    print!("{} ",a0);
                },

                Mode::Pass => {
                    let b2 = if pre_codes.len() > code_num + 1{
                        pre_codes[code_num + 1]
                    } else {
                         width
                    };
                    let run_len = b2 - a0;
                    code_num += 2;
                    let run_len = run_len.min(width - a0);
                    a0 += run_len as usize;
                    codes.push(a0);

                    for _ in 0..run_len {
                        data.push(0x00);
                    }
                    print!("Ps {} ",a0);
                },
                Mode::D2Ext(val) => {
                    print!("2D Ext ({}) ",val );   
                    if val != 0x7 { // no compression
                        break;
//                        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT 3G 2D Ext is not support".to_string())))
                    }
                    let mut buf = uncompress(&mut reader)?;
                    a0 += buf.len();
                    codes.push(a0);

                    data.append(&mut buf);
                    print!("{} ",a0);   

                }
                Mode::D1Ext(val) => {
                    print!("1D Ext ({}) ",val );
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT 3G 1D Ext is not support".to_string())))
                }
                Mode::None => { // fill ?
                    print!("None ");
                    break;
//                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT 3G 2D decodes error".to_string())))
                },
                Mode::EOL => {
                    eol = true;
                }
            }
        }

        y += 1;
        for _ in a0..width {
            data.push(0x00);
        }

        if header.compression == Compression::CCITTGroup3Fax && !eol {
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

        if encoding == 2 && header.compression == Compression::CCITTGroup3Fax {
            is2d = if reader.get_bits(1)? == 0 { true } else { false };
            if is2d == false {
                codes.clear();
            }
        }
        print!(" is2d {} ",is2d);
 
        if header.compression == Compression::CCITTHuffmanRLE {
            reader.flush();
        }


    }

    Ok((data,reader.warning))    
}
