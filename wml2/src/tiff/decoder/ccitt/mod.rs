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
#[derive(Clone)]
pub enum Mode {
    Pass,
    Horiz,
    V,
    Vr(usize),
    Vl(usize),
    Ext2D(usize),
    Ext1D(usize),
    EOL,
    None
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

    fn get_bits(&mut self,size:usize) -> Result<usize,Error> {
        let bits = self.look_bits(size);
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

    fn mode(&mut self) -> Result<Mode,Error> {
        let array:[(usize,Mode);128] = [
            // 0000000
            (12,Mode::None), 
            // 0000001
            (10,Mode::Ext2D(0)),
            // 0000010
            (7,Mode::Vl(3)),
            // 0000011
            (7,Mode::Vr(3)),
            // 000010x
            (6,Mode::Vl(2)),(6,Mode::Vl(2)),
            // 000011x
            (6,Mode::Vr(2)),(6,Mode::Vr(2)),
            // 0001xxx
            (4,Mode::Pass),(4,Mode::Pass),(4,Mode::Pass),(4,Mode::Pass),
            (4,Mode::Pass),(4,Mode::Pass),(4,Mode::Pass),(4,Mode::Pass),

            // 001xxxx
            (3,Mode::Horiz), (3,Mode::Horiz), (3,Mode::Horiz), (3,Mode::Horiz), 
            (3,Mode::Horiz), (3,Mode::Horiz), (3,Mode::Horiz), (3,Mode::Horiz), 
            (3,Mode::Horiz), (3,Mode::Horiz), (3,Mode::Horiz), (3,Mode::Horiz), 
            (3,Mode::Horiz), (3,Mode::Horiz), (3,Mode::Horiz), (3,Mode::Horiz), 
            // 010xxxx
            (3,Mode::Vl(1)), (3,Mode::Vl(1)), (3,Mode::Vl(1)), (3,Mode::Vl(1)), 
            (3,Mode::Vl(1)), (3,Mode::Vl(1)), (3,Mode::Vl(1)), (3,Mode::Vl(1)), 
            (3,Mode::Vl(1)), (3,Mode::Vl(1)), (3,Mode::Vl(1)), (3,Mode::Vl(1)), 
            (3,Mode::Vl(1)), (3,Mode::Vl(1)), (3,Mode::Vl(1)), (3,Mode::Vl(1)), 
            // 011xxxx 16
            (3,Mode::Vr(1)), (3,Mode::Vr(1)), (3,Mode::Vr(1)), (3,Mode::Vr(1)), 
            (3,Mode::Vr(1)), (3,Mode::Vr(1)), (3,Mode::Vr(1)), (3,Mode::Vr(1)), 
            (3,Mode::Vr(1)), (3,Mode::Vr(1)), (3,Mode::Vr(1)), (3,Mode::Vr(1)), 
            (3,Mode::Vr(1)), (3,Mode::Vr(1)), (3,Mode::Vr(1)), (3,Mode::Vr(1)), 
            // 1xxxxxx 64
            (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),
            (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),
            (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),
            (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),
            (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),
            (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),
            (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),
            (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),  (1,Mode::V), (1,Mode::V),
        ];
        if self.look_bits(12)? == 1 {
            self.skip_bits(12);
            return Ok(Mode::EOL)
        }
        let (bits,mode) = array[self.look_bits(7)?].clone();

        if bits <= 7 {
            self.skip_bits(bits);
            return Ok(mode)
        }

        if bits == 10 {
            self.skip_bits(10);
            let n = self.look_bits(3)?;
            self.skip_bits(3);
            return Ok(Mode::Ext2D(n))
        }
        self.skip_bits(bits);

        if bits == 12 {
            if self.look_bits(9)? == 1 {
                self.skip_bits(9);
                let n = self.look_bits(3)?;
                self.skip_bits(3);
                return Ok(Mode::Ext1D(n))
            }
        }
        Ok(Mode::None)        
    }
 
// skip next byte
//    fn flush(&mut self) {
//        self.left_bits -= self.left_bits % 8;
//    }
}

fn search_b1(codes:&[u8],a0:usize,color:u8) -> usize {
    let mut i = a0 ;
    for _ in i..codes.len() {
        if codes[i] == color {
            break;
        }
        i += 1;
    }
    for _ in i..codes.len() {
        if codes[i] != color {
            break;
        }
        i += 1;
    }
    print!("b1:{} ",i);
    return i;
}

fn search_b2(codes:&[u8],a0:usize,color:u8) -> usize {
    let b1 = search_b1(codes, a0, color);
    let color = color ^ 0xff;
    search_b1(codes, b1, color)
}

const WHITE:u8 = 0;
const UNDEF:u8 = 1;
const TERMINATE:u8 = 2;
const BLACK:u8 = 0xff;


  

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
//        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, "CCITT uncompress is no support".to_string())))
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
    if encoding <= 2 {
        loop {
            code1 = reader.look_bits(12)?;
            if code1 != 0 { break };
            reader.skip_bits(1);
        }
        if code1 == 1 {
            reader.skip_bits(12);
        }    
    }

    let mut is2d = if encoding == 3 {true} else {false};

    let mut y = 0;
    
    if encoding == 2 {
        if reader.get_bits(1)? == 0 {
            is2d = true
        }
    }

    /* const
    let white = 0;
    let unknown = 1;
    let terminate =2;
    let black = 0xff;
    */

    //    let mut mode = Mode::Horiz; 
    let mut ref_codes = vec![WHITE;width];
    ref_codes.push(TERMINATE);
    let mut cur_codes = Vec::with_capacity(width+1);
    for _ in 0..width {cur_codes.push(UNDEF);}
    cur_codes.push(TERMINATE);


    loop {
        let mut a0 = 0;
        let mut eol = false;

        if encoding >= 2{
            print!("y {} {} ",y,is2d);
        }
        while a0 < width && !eol {
            let mode = if is2d { reader.mode()? } else { Mode::Horiz }; 
            match mode {
                Mode::Horiz => {
                    let mut color = if is2d {cur_codes[a0]} else {WHITE};
                    if color == UNDEF {
                        color = ref_codes[a0];
                    }

                    let (mut len1, mut len2);
                    if color == WHITE {
                        if encoding >= 2 {
                            print!("Hw ");
                        }

                        len1 = reader.run_len(&white)?;
                        if len1 == -2 {  // EOL
                            eol = true;
                            len1 = 0;
                        }
                        len2 = reader.run_len(&black)?;                        
                        if len2 == -2 {  // EOL
                            eol = true;
                            len2 = 0;
                        }    
                    } else {
                        if encoding >= 2 {
                            print!("Hb ");
                        }
                        len1 = reader.run_len(&black)?;
                        if len1 == -2 {  // EOL
                            eol = true;
                            len1 = 0;
                        }
            
                        len2 = reader.run_len(&white)?;                        
                        if len2 == -2 {  // EOL
                            eol = true;
                            len2 = 0;
                        }    
                    }

//                    let mut color = color ^ 0xff;
                    for i in a0..(a0 + len1 as usize).min(width) {
                        cur_codes[i] = color;
                    }

                    a0 += len1 as usize;

                    color ^= 0xff;

                    for i in a0..(a0 + len2 as usize).min(width) {
                        cur_codes[i] = color;
                    }
                    a0 += len2 as usize;

                    if a0 < width && cur_codes[a0] == UNDEF {
                        cur_codes[a0] = color^BLACK;
                    }

                    if encoding >= 2{
                        print!("{} {} {} ",len1,len2,a0);
                    }
                },
                Mode::Pass => {
                    print!("Ps ");
                    let color = if cur_codes[a0] == UNDEF {ref_codes[a0]} else {cur_codes[a0]};
                    let b1 = search_b1(&ref_codes, a0, color);
                    let b2 = search_b1(&ref_codes, b1, color^BLACK);
                    for i in a0..=b2 {
                        cur_codes[i] = color;
                    }
                    a0 = b2;
                    print!("{} ",a0);
                },
                Mode::V => {
                    print!("V ");
                    let color = if a0 == 0 {WHITE} else {cur_codes[a0]};
                    let a1 = search_b1(&ref_codes, a0, color);
                    for i in a0..a1 {
                        cur_codes[i] = color;
                    }
                    if a1 < cur_codes.len() && cur_codes[a1] == UNDEF {
                        cur_codes[a1] = color^BLACK;
                    }
                    a0 = a1;
                    print!("{} ",a0);
                },
                Mode::Vr(n) => {
                    print!("Vr({}) ",n);
                    let color = if a0 == 0 {WHITE} else {cur_codes[a0]};
                    let b1 = search_b1(&ref_codes, a0, color);
                    let a1 = (b1 + n).min(width);
                    for i in a0..a1 {
                        cur_codes[i] = color;
                    }
                    if a1 < cur_codes.len() && cur_codes[a1] == UNDEF {
                        cur_codes[a1] = color^BLACK;
                    }
                    a0 = a1;
                    print!("{} ",a0);
                },
                Mode::Vl(n) => {
                    print!("Vl({}) ",n);
                    let color = if a0 == 0 {WHITE} else {cur_codes[a0]};
                    let b1 = search_b1(&ref_codes, a0, color);
                    let a1 = b1.checked_sub(n).unwrap_or(0);
                    for i in a0..a1 {
                        cur_codes[i] = color;
                    }
                    if a1 < cur_codes.len() && cur_codes[a1] == UNDEF {
                        cur_codes[a1] = color^BLACK;
                    }
                    a0 = a1;
                    print!("{} ",a0);
                },
                Mode::Ext1D(n) => {
                    let ptr = reader.ptr - 32;
                    println!("");
                    for i in 0..64 {
                        print!("{:08b} ",reader.buffer[ptr + i].reverse_bits());
                        if i % 8 == 7 {
                            println!("");
                        }
                    }
                    let message = format!("not support 1D Ext({}) for CCITT decoder",n);
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, message)));                    
                },
                Mode::Ext2D(n) => {
                    let ptr = reader.ptr - 32;
                    println!("");
                    for i in 0..64 {
                        print!("{:08b} ",reader.buffer[ptr + i].reverse_bits());
                        if i % 8 == 7 {
                            println!("");
                        }
                    }
                    let message = format!("not support 2D Ext({}) for CCITT decoder",n);
                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError, message)));
                },
                Mode::None =>  {    // fill?
                    print!("None ");
                    reader.get_bits(1)?;
                    // error
                },
                Mode::EOL =>  {
                    print!("EOL ");
                    eol = true;
                    break;
                }
            }
        }
        if encoding >= 2 {
            println!("\n");
            if y < 122 && y >= 115 {
                println!("{:?}",cur_codes);
            }
        }

        for i in 0..width {
            if cur_codes[i] == BLACK {
                data.push(0xff);
            } else {
                data.push(0);
            }
        }


        y += 1;
        ref_codes = cur_codes.clone();
        cur_codes = vec![UNDEF;width];
        cur_codes.push(TERMINATE);
    
        if y >= height { break;}

        if encoding <=2 && !eol {
            loop {
                if reader.look_bits(12)? == 1 {  // EOL?
                    reader.skip_bits(12);
                    break;
                }
                reader.skip_bits(1);    // fill
            }
        }

        if encoding == 2{
            let v = reader.get_bits(1)?;
            is2d = if v == 0 { true } else { false };
        }
    }

    Ok((data,reader.warning))    
}
