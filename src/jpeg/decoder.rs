/*
 * jpeg/decoder.rs  Mith@mmk (C) 2022
 * use MIT License
 */

use crate::jpeg::header::Component;
use crate::jpeg::header::HuffmanTable;
use crate::jpeg::header::JpegHaeder;
use crate::jpeg::util::print_header;
use crate::jpeg::worning::JPEGWorning::SimpleAddMessage as WorningAddMessage;
use crate::jpeg::worning::JPEGWorning;
use crate::jpeg::worning::WorningKind;
use crate::error::ImgError::SimpleAddMessage;
use crate::error::{ImgError,ErrorKind};
use crate::error::ImgError::{Simple};
use crate::DecodeOptions;


struct BitReader {
    buffer: Vec<u8>,
    ptr : usize,
    bptr : usize,
    b: u8,
    rst: bool,
    rst_ptr : usize,
    prev_rst: usize,
    eof_flag: bool,
}

#[allow(unused)]
impl BitReader {
    pub fn new(buffer:&[u8]) -> Self{
        let ptr:usize = 0;
        let bptr:usize = 0;
        let b:u8 = 0;
        Self{
            buffer: buffer.to_vec(),
            ptr: ptr,
            bptr: bptr,
            b: b,
            rst: false,
            rst_ptr: 0,
            prev_rst: 7,
            eof_flag: false,
        }
    }

    fn get_byte(self: &mut Self) -> Result<u8,ImgError> {
        if self.ptr >= self.buffer.len() {
            return Err(Simple(ErrorKind::OutboundIndex));
        }
        self.b = self.buffer[self.ptr];
        self.ptr = self.ptr + 1;
        Ok(self.b)
    }

    fn rst(self: &mut Self) -> Result<bool,ImgError> {
        if self.buffer[self.ptr] == 0xff {
            match self.buffer[self.ptr+1] {
                0xd0..=0xd7 =>  {    // RST    
                    self.ptr = self.ptr + 2;
                    self.bptr = 0;
                    return Ok(true);
                },
                _ => {
                    return Ok(false);
                },
            }
        }
        Ok(false)
    }

    fn next_marker(self: &mut Self) -> Result<u8,ImgError> {
        if self.get_byte()? != 0xff {
            return Err(SimpleAddMessage(ErrorKind::DecodeError,"Nothing marker".to_string()));
        }
        loop {
            let b = self.get_byte()?; 
            if b != 0xff {
                return Ok(b);
            }
        }
    }

    #[inline]
    fn next_byte(self: &mut Self) -> Result<u8,ImgError> {
        if self.get_byte()? == 0xff {
            match self.get_byte()? {
                0x00 => {
                    self.b = 0xff;
                },
                0xd0..=0xd7 =>  {    // RST    
                    let rst_no = (self.b & 0x7) as usize;
                    if rst_no != (self.prev_rst + 1) % 8 {
                        return Err(SimpleAddMessage(ErrorKind::DecodeError,format!("No Interval RST {} -> {}",self.prev_rst,rst_no)))
                    }

                    self.prev_rst = rst_no;
                    self.rst = true;
                    self.rst_ptr = self.ptr;
                },
                0xd9=> { // EOI
                    self.b = 0xff;
                },
                _ =>{
                    self.b = 0xff;
                    return Err(SimpleAddMessage(ErrorKind::DecodeError,"FF after  00 or RST".to_string()))
                },                    
            }
        }
        Ok(self.b)
    }

    pub fn get_bit(self: &mut Self) -> Result<usize,ImgError> {
        if self.bptr == 0 {
            self.bptr = 8;
            self.next_byte()?;
        }
        self.bptr -= 1;
        let r = (self.b  >> self.bptr) as usize & 0x1;
        Ok(r)
    }

    #[inline]
    pub fn get_bits(self: &mut Self,mut bits:usize) -> Result<i32,ImgError> {
        if self.bptr == 0 {
            self.next_byte()?;
            self.bptr = 8;
        }
        let mut v = 0_i32;

        loop {
            if bits > self.bptr {
                v = (v << self.bptr) | (self.b as i32 & ((1 << self.bptr) -1));
                bits -= self.bptr;
                self.next_byte();
                self.bptr = 8;
            } else {
                self.bptr -= bits;
                v = (v << bits) | (self.b as i32 >> self.bptr) & ((1 << bits) -1);
                break;
            }
        }
        Ok(v)
    }

    pub fn eof(self: &mut Self) -> bool {
        if self.buffer.len() - 2 < self.ptr || self.eof_flag
         {self.eof_flag=true}
        self.eof_flag
    }

    pub fn reset(self: &mut Self) {
        self.ptr = 0;
        self.eof_flag = false;
    }

    pub fn set_offset(self: &mut Self ,offset: usize) -> Result<usize,ImgError> {
        if offset < self.buffer.len() {
            self.ptr = offset;
            self.eof_flag = false;
            Ok(self.ptr)
        } else {
            Err(Simple(ErrorKind::OutboundIndex))
        }
    }

    pub fn offset(self: &mut Self) -> usize {
        self.ptr
    }

}

#[inline]
fn huffman_read (bit_reader:&mut BitReader,table: &HuffmanDecodeTable)  -> Result<u32,ImgError>{
    let mut d = 0;
    for l in 0..16 {
        d = (d << 1) | bit_reader.get_bit()?;
        if table.max[l] >= d as i32 {
            let p = d  - table.min[l] as usize + table.pos[l];
            return Ok(table.val[p] as u32)                      
        }
    }
    Err(SimpleAddMessage(ErrorKind::OutboundIndex,"Huffman read Overflow".to_string()))  
}


#[derive(std::cmp::PartialEq)]
pub(crate) struct HuffmanDecodeTable {
    pos: Vec::<usize>,
    val: Vec::<usize>,
    min: Vec::<i32>,
    max: Vec::<i32>,     
}

#[inline]
fn dc_read(bitread: &mut BitReader,dc_decode:&HuffmanDecodeTable,pred:i32) -> Result<i32,ImgError> {
    let ssss = huffman_read(bitread,&dc_decode)?;
    let v = bitread.get_bits(ssss as usize)?;
    let diff = extend(v,ssss as usize);
    let dc = diff + pred;
    Ok(dc)
}

#[inline] // for base line huffman
fn ac_read(bitread: &mut BitReader,ac_decode:&HuffmanDecodeTable) -> Result<Vec<i32>,ImgError> {
    let mut zigzag : usize= 1;
    let mut zz  = [0_i32;64];
    loop {  // F2.2.2
        let ac = huffman_read(bitread,&ac_decode)?;
        
        let ssss = ac & 0xf;
        let rrrr = ac >> 4;
        if ssss == 0 {
            if ac == 0x00 { //EOB
                return Ok(zz.to_vec())
            }
            if rrrr == 15 { //ZRL
                zigzag = zigzag + 16;
                continue
            }
            return Ok(zz.to_vec())   // N/A
        } else {
            zigzag = zigzag + rrrr as usize;
            let v = bitread.get_bits(ssss as usize)?;
            zz[zigzag] = extend(v,ssss as usize);
        }
        if zigzag >= 63 {
            return Ok(zz.to_vec())
        }
        zigzag = zigzag + 1;
    }
}

#[inline]
fn baseline_read(bitread: &mut BitReader,dc_decode:&HuffmanDecodeTable,ac_decode:&HuffmanDecodeTable,pred: i32)-> Result<(Vec<i32>,i32),ImgError> {
    let dc = dc_read(bitread, dc_decode, pred)?;
    let mut zz = ac_read(bitread, ac_decode)?;
    zz[0] = dc;
    Ok((zz,dc))
}


#[inline]
fn extend(mut v:i32,t: usize) -> i32 {
    if t == 0 {
        return v;
    }
    let mut vt = 1 << (t-1);

    if v < vt {
        vt = (-1 << t) + 1;
        v = v + vt;
    }
    v
}

/* fast idct is change fast alogrythm from orthodox idct
fn idct(f :&[i32]) -> Vec<u8> {
    let vals :Vec<u8> = (0..64).map(|i| {
        let (x,y) = ((i%8) as f32,(i/8) as f32);
        // IDCT from CCITT Rec. T.81 (1992 E) p.27 A3.3
        let mut val: f32=0.0;
        for u in 0..8 {
            let cu = if u == 0 {1.0 / 2.0_f32.sqrt()} else {1.0};
            for v in 0..8 {
                let cv = if v == 0 {1.0_f32 / 2.0_f32.sqrt()} else {1.0};
                val += cu * cv * (f[v*8 + u] as f32)
                    * ((2.0 * x + 1.0) * u as f32 * PI / 16.0_f32).cos()
                    * ((2.0 * y + 1.0) * v as f32 * PI / 16.0_f32).cos();
            }
        }
        val = val / 4.0;

        // level shift from CCITT Rec. T.81 (1992 E) p.26 A3.1
        let v = val.round() as i32 + 128;
        v.clamp(0,255) as u8
    }).collect();
    vals
}
*/

#[inline]
// AAN algorythm
fn fast_idct(f: &[i32]) -> Vec<u8> {
    let mut _f  = [0_f32;64];
    let mut vals = [0_u8;64];
    let m0 = 1.847759;
    let m1 = 1.4142135;
    let m3 = 1.4142135;
    let m5 = 0.76536685;
    let m2 = m0 - m5;
    let m4 = m0 + m5;

    let s0 = 0.35355338;
    let s1 = 0.49039263;
    let s2 = 0.46193975;
    let s3 = 0.4157348;
    let s4 = 0.35355338;
    let s5 = 0.2777851;
    let s6 = 0.19134171;
    let s7 = 0.09754512;
    
    for i in 0..8 {
        let g0 = f[0*8 + i] as f32 * s0;
        let g1 = f[4*8 + i] as f32 * s4;
        let g2 = f[2*8 + i] as f32 * s2;
        let g3 = f[6*8 + i] as f32 * s6;
        let g4 = f[5*8 + i] as f32 * s5;
        let g5 = f[1*8 + i] as f32 * s1;
        let g6 = f[7*8 + i] as f32 * s7;
        let g7 = f[3*8 + i] as f32 * s3;
    
        let f0 = g0;
        let f1 = g1;
        let f2 = g2;
        let f3 = g3;
        let f4 = g4 - g7;
        let f5 = g5 + g6;
        let f6 = g5 - g6;
        let f7 = g4 + g7;
    
        let e0 = f0;
        let e1 = f1;
        let e2 = f2 - f3;
        let e3 = f2 + f3;
        let e4 = f4;
        let e5 = f5 - f7;
        let e6 = f6;
        let e7 = f5 + f7;
        let e8 = f4 + f6;
    
        let d0 = e0;
        let d1 = e1;
        let d2 = e2 * m1;
        let d3 = e3;
        let d4 = e4 * m2;
        let d5 = e5 * m3;
        let d6 = e6 * m4;
        let d7 = e7;
        let d8 = e8 * m5;
    
        let c0 = d0 + d1;
        let c1 = d0 - d1;
        let c2 = d2 - d3;
        let c3 = d3;
        let c4 = d4 + d8;
        let c5 = d5 + d7;
        let c6 = d6 - d8;
        let c7 = d7;
        let c8 = c5 - c6;
    
        let b0 = c0 + c3;
        let b1 = c1 + c2;
        let b2 = c1 - c2;
        let b3 = c0 - c3;
        let b4 = c4 - c8;
        let b5 = c8;
        let b6 = c6 - c7;
        let b7 = c7;
        
        _f[0 * 8 + i] = b0 + b7;
        _f[1 * 8 + i] = b1 + b6;
        _f[2 * 8 + i] = b2 + b5;
        _f[3 * 8 + i] = b3 + b4;
        _f[4 * 8 + i] = b3 - b4;
        _f[5 * 8 + i] = b2 - b5;
        _f[6 * 8 + i] = b1 - b6;
        _f[7 * 8 + i] = b0 - b7; 
    }
    
    for i in 0..8 {
        let g0 = _f[i*8 + 0] as f32 * s0;
        let g1 = _f[i*8 + 4] as f32 * s4;
        let g2 = _f[i*8 + 2] as f32 * s2;
        let g3 = _f[i*8 + 6] as f32 * s6;
        let g4 = _f[i*8 + 5] as f32 * s5;
        let g5 = _f[i*8 + 1] as f32 * s1;
        let g6 = _f[i*8 + 7] as f32 * s7;
        let g7 = _f[i*8 + 3] as f32 * s3;
    
        let f0 = g0;
        let f1 = g1;
        let f2 = g2;
        let f3 = g3;
        let f4 = g4 - g7;
        let f5 = g5 + g6;
        let f6 = g5 - g6;
        let f7 = g4 + g7;
    
        let e0 = f0;
        let e1 = f1;
        let e2 = f2 - f3;
        let e3 = f2 + f3;
        let e4 = f4;
        let e5 = f5 - f7;
        let e6 = f6;
        let e7 = f5 + f7;
        let e8 = f4 + f6;
    
        let d0 = e0;
        let d1 = e1;
        let d2 = e2 * m1;
        let d3 = e3;
        let d4 = e4 * m2;
        let d5 = e5 * m3;
        let d6 = e6 * m4;
        let d7 = e7;
        let d8 = e8 * m5;
    
        let c0 = d0 + d1;
        let c1 = d0 - d1;
        let c2 = d2 - d3;
        let c3 = d3;
        let c4 = d4 + d8;
        let c5 = d5 + d7;
        let c6 = d6 - d8;
        let c7 = d7;
        let c8 = c5 - c6;
    
        let b0 = c0 + c3;
        let b1 = c1 + c2;
        let b2 = c1 - c2;
        let b3 = c0 - c3;
        let b4 = c4 - c8;
        let b5 = c8;
        let b6 = c6 - c7;
        let b7 = c7;
        
        vals[i * 8 + 0] = ((b0 + b7) as i32 + 128).clamp(0,255) as u8;
        vals[i * 8 + 1] = ((b1 + b6) as i32 + 128).clamp(0,255) as u8;
        vals[i * 8 + 2] = ((b2 + b5) as i32 + 128).clamp(0,255) as u8;
        vals[i * 8 + 3] = ((b3 + b4) as i32 + 128).clamp(0,255) as u8;
        vals[i * 8 + 4] = ((b3 - b4) as i32 + 128).clamp(0,255) as u8;
        vals[i * 8 + 5] = ((b2 - b5) as i32 + 128).clamp(0,255) as u8;
        vals[i * 8 + 6] = ((b1 - b6) as i32 + 128).clamp(0,255) as u8;
        vals[i * 8 + 7] = ((b0 - b7) as i32 + 128).clamp(0,255) as u8;
    }
    vals.to_vec()
}


// Glayscale
fn y_to_rgb  (yuv: &Vec<Vec<u8>>,hv_maps:&Vec<Component>) -> Vec<u8> {
    let mut buffer:Vec<u8> = (0 .. hv_maps[0].h * hv_maps[0].v * 64 * 4).map(|_| 0).collect();
    for v in 0..hv_maps[0].v {
        for h in 0..hv_maps[0].h {
            let gray = &yuv[v*hv_maps[0].h + h];
            for y in 0..8 {
                let offset = (y + v *8) * hv_maps[0].h * 8 * 4;
                for x in 0..8 {
                    let xx = (x + h * 8) * 4;
                    let cy = gray[y * 8 + x];
                    buffer[xx + offset    ] = cy;   // R
                    buffer[xx + offset + 1] = cy;   // G
                    buffer[xx + offset + 2] = cy;   // B
                    buffer[xx + offset + 3] = 0xff; // A
                }
            }
        }
    }
    buffer
}

fn yuv_to_rgb (yuv: &Vec<Vec<u8>>,hv_maps:&Vec<Component>,(h_max,v_max):(usize,usize)) -> Vec<u8> {
    let mut buffer:Vec<u8> = (0..h_max * v_max * 64 * 4).map(|_| 0).collect();
    let y_map = 0;
    let u_map = y_map + hv_maps[0].h * hv_maps[0].v;
    let v_map = u_map + hv_maps[1].h * hv_maps[1].v;

    let uy = v_max / hv_maps[1].v as usize;
    let vy = v_max / hv_maps[2].v as usize;
    let ux = h_max / hv_maps[1].h as usize;
    let vx = h_max / hv_maps[2].h as usize;

    for v in 0..v_max {
        let mut u_map_cur = u_map + v / v_max;
        let mut v_map_cur = v_map + v / v_max;

        for h in 0..h_max {
            let gray = &yuv[v*h_max + h];
            u_map_cur = u_map_cur + h / h_max;
            v_map_cur = v_map_cur + h / h_max;

            for y in 0..8 {
                let offset = ((y + v * 8) * (8 * h_max)) * 4;
                for x in 0..8 {
                    let xx = (x + h * 8) * 4;
                    let shift = 4090;
                    let cy = gray[y * 8 + x] as i32;
                    let cb = yuv[u_map_cur][(((y + v * 8) / uy % 8) * 8)  + ((x + h * 8) / ux) % 8] as i32;
                    let cr = yuv[v_map_cur][(((y + v * 8) / vy % 8) * 8)  + ((x + h * 8) / vx) % 8] as i32;

                    let crr = (1.402 * shift as f32) as i32;
                    let cbg = (- 0.34414 * shift as f32) as i32;
                    let crg = (- 0.71414 * shift as f32) as i32;
                    let cbb = (1.772 * shift as f32) as i32;


                    let red  = cy + (crr * (cr - 128))/shift;
                    let green= cy + (cbg * (cb - 128) + crg * (cr - 128))/shift;
                    let blue = cy + (cbb * (cb - 128))/shift;

                    let red = red.clamp(0,255) as u8;
                    let green = green.clamp(0,255) as u8;
                    let blue = blue.clamp(0,255) as u8;

                    buffer[xx + offset    ] = red; //R
                    buffer[xx + offset + 1] = green; //G
                    buffer[xx + offset + 2] = blue; //B
                    buffer[xx + offset + 3] = 0xff; //A
                }
            }
        }
    }

    buffer
}

/* spec known */
fn ycck_to_rgb (yuv: &Vec<Vec<u8>>,hv_maps:&Vec<Component>,(h_max,v_max):(usize,usize)) -> Vec<u8> {
    let mut buffer:Vec<u8> = (0..h_max * v_max * 64 * 4).map(|_| 0).collect();
    let y_map = 0;
    let c1_map = y_map + hv_maps[0].h * hv_maps[0].v;
    let c2_map = c1_map + hv_maps[1].h * hv_maps[1].v;
    let k_map = c2_map + hv_maps[2].h * hv_maps[2].v;

    let yy = v_max / hv_maps[0].v as usize;
    let c1y = v_max / hv_maps[1].v as usize;
    let c2y = v_max / hv_maps[2].v as usize;
    let ky =  v_max / hv_maps[3].v as usize;

    let yx = h_max / hv_maps[0].h as usize;
    let c1x = h_max / hv_maps[1].h as usize;
    let c2x = h_max / hv_maps[2].h as usize;
    let kx = h_max / hv_maps[3].h as usize;

    for v in 0..v_max {
        let mut y_map_cur = y_map + v / v_max;
        let mut c1_map_cur = c1_map + v / v_max;
        let mut c2_map_cur = c2_map + v / v_max;
        let mut k_map_cur = k_map + v / v_max;

        for h in 0..h_max {
            y_map_cur = y_map_cur + h / h_max;
            c1_map_cur = c1_map_cur + h / h_max;
            c1_map_cur = c2_map_cur + h / h_max;
            k_map_cur = k_map_cur + h / h_max;

            for y in 0..8 {
                let offset = ((y + v * 8) * (8 * h_max)) * 4;
                for x in 0..8 {
                    let xx = (x + h * 8) * 4;
                    let yin = yuv[y_map_cur][(((y + v * 8)  % 8) * 8)  + ((x + h * 8)) % 8] as i32;
                    let c1  = yuv[c1_map_cur][(((y + v * 8) / c1y % 8) * 8)  + ((x + h * 8) / c1x) % 8] as i32;
                    let c2  = yuv[c2_map_cur][(((y + v * 8) / c2y % 8) * 8)  + ((x + h * 8) / c2x) % 8] as i32;
                    let key = yuv[k_map_cur][(((y + v * 8) % 8) * 8)  + (x + h * 8) % 8] as i32;

                    let cy = yin;
                    let cb = 255 - c1;
                    let cr = 255 - c2;

                    let shift = 4096;

                    let crr = (1.402 * shift as f32) as i32;
                    let cbg = (- 0.34414 * shift as f32) as i32;
                    let crg = (- 0.71414 * shift as f32) as i32;
                    let cbb = (1.772 * shift as f32) as i32;


                    let red  = cy + (crr * (cr - 128))/shift;
                    let green= cy + (cbg * (cb - 128) + crg * (cr - 128))/shift;
                    let blue = cy + (cbb * (cb - 128))/shift;

                    let red = red.clamp(0,255) as u8;
                    let green = green.clamp(0,255) as u8;
                    let blue = blue.clamp(0,255) as u8;

                    buffer[xx + offset    ] = red; //R
                    buffer[xx + offset + 1] = green; //G
                    buffer[xx + offset + 2] = blue; //B
                    buffer[xx + offset + 3] = 0xff; //A

                    buffer[xx + offset    ] = red; //R
                    buffer[xx + offset + 1] = green; //G
                    buffer[xx + offset + 2] = blue; //B
                    buffer[xx + offset + 3] = 0xff; //A
                }
            }
        }
    }

    buffer
}

/* spec known */
fn cmyk_to_rgb (yuv: &Vec<Vec<u8>>,hv_maps:&Vec<Component>,(h_max,v_max):(usize,usize)) -> Vec<u8> {
    let mut buffer:Vec<u8> = (0..h_max * v_max * 64 * 4).map(|_| 0).collect();
    let k_map = 0;
    let m_map = k_map + hv_maps[0].h * hv_maps[0].v;
    let c_map = m_map + hv_maps[1].h * hv_maps[1].v;
    let y_map = c_map + hv_maps[2].h * hv_maps[2].v;

    let ky = v_max / hv_maps[0].v as usize;
    let cy = v_max / hv_maps[1].v as usize;
    let my = v_max / hv_maps[2].v as usize;
    let yy = v_max / hv_maps[3].v as usize;

    let kx = h_max / hv_maps[0].h as usize;
    let cx = h_max / hv_maps[1].h as usize;
    let mx = h_max / hv_maps[2].h as usize;
    let yx = h_max / hv_maps[3].h as usize;

    for v in 0..v_max {
        let mut c_map_cur = c_map + v / cy;
        let mut m_map_cur = m_map + v / my;
        let mut y_map_cur = y_map + v / yy;
        let mut k_map_cur = k_map + v / ky;

        for h in 0..h_max {
            c_map_cur = c_map_cur + h / cx;
            m_map_cur = m_map_cur + h / mx;
            y_map_cur = y_map_cur + h / yx;
            k_map_cur = k_map_cur + h / kx;

            for y in 0..8 {
                let offset = ((y + v * 8) * (8 * h_max)) * 4;
                for x in 0..8 {
                    let xx = (x + h * 8) * 4;
                    let cc = yuv[c_map_cur][(((y + v * 8) / cy % 8) * 8)  + ((x + h * 8) / cx) % 8] as i32;
                    let cm = yuv[m_map_cur][(((y + v * 8) / my % 8) * 8)  + ((x + h * 8) / mx) % 8] as i32;
                    let cy = yuv[y_map_cur][(((y + v * 8) / yy % 8) * 8)  + ((x + h * 8) / yx) % 8] as i32;
                    let ck = yuv[k_map_cur][(((y + v * 8) / ky % 8) * 8)  + ((x + h * 8) / kx) % 8] as i32;

                    // from Japn Color 2011 Coated
                    // R  69 K  + (255 - 69) Y +        0 C  
                    // G          (255 -204) Y + (255-160)C
                    // B  92 K            32 Y +            + 131M

                    let red   = (cy as i32).clamp(0,255) as u8;
                    let green = (cm as i32).clamp(0,255) as u8;
                    let blue  = (cc as i32).clamp(0,255) as u8;

                    buffer[xx + offset    ] = red; //R
                    buffer[xx + offset + 1] = green; //G
                    buffer[xx + offset + 2] = blue; //B
                    buffer[xx + offset + 3] = 0xff; //A

                }
            }
        }
    }

    buffer
}

pub(crate) fn huffman_extend(huffman_tables:&Vec<HuffmanTable>) -> (Vec<HuffmanDecodeTable>,Vec<HuffmanDecodeTable>) {

    let mut ac_decode : Vec<HuffmanDecodeTable> = Vec::new();
    let mut dc_decode : Vec<HuffmanDecodeTable> = Vec::new();

    for huffman_table in huffman_tables.iter() {

        let mut current_max: Vec<i32> = Vec::new();
        let mut current_min: Vec<i32> = Vec::new();

        let mut code :i32 = 0;
        let mut pos :usize = 0;
        for l in 0..16 {
            if huffman_table.len[l] != 0 {
                current_min.push(code); 
                for _ in 0..huffman_table.len[l] {
                    if pos >= huffman_table.val.len() { break;}
                    pos = pos + 1;
                    code = code + 1;
                }
                current_max.push(code - 1); 
            } else {
                current_min.push(-1);
                current_max.push(-1);
            }
            code = code << 1;
        }
        
        if huffman_table.ac {
            let val : Vec<usize> = huffman_table.val.iter().map(|i| *i).collect();
            let pos : Vec<usize> = huffman_table.pos.iter().map(|i| *i).collect();
            ac_decode.push(HuffmanDecodeTable{
                val: val,
                pos: pos,
                max: current_max,
                min: current_min,
            });
        } else {
            let val : Vec<usize> = huffman_table.val.iter().map(|i| *i).collect();
            let pos : Vec<usize> = huffman_table.pos.iter().map(|i| *i).collect();
            dc_decode.push(HuffmanDecodeTable{
                val: val,
                pos: pos,
                max: current_max,
                min: current_min,
            });
        }
    }

    (ac_decode,dc_decode)
}

pub fn decode<'decode>(buffer: &[u8],option:&mut DecodeOptions) 
    -> Result<Option<JPEGWorning>,ImgError> {

    let mut worning: Option<JPEGWorning> = None;
        // Make Huffman Table
    // Scan Header
    let header = JpegHaeder::new(buffer,0)?;

    if option.debug_flag > 0 {
        let boxstr = print_header(&header,option.debug_flag);
        option.drawer.verbose(&boxstr)?;
    }
    
    if header.is_hierachical {
        return Err(SimpleAddMessage(ErrorKind::DecodeError,"Hierachical is not support".to_string()));
    }

    let huffman_scan_header  = header.huffman_scan_header.as_ref().unwrap();
    match header.huffman_tables {
        None => {
            return Err(SimpleAddMessage(ErrorKind::DecodeError,"Not undefined Huffman Tables".to_string()));
        },
        _ => {

        }
    }

    match header.frame_header {
        None => {
            return Err(SimpleAddMessage(ErrorKind::DecodeError,"Not undefined Frame Header".to_string()));
        },
        _ => {

        }
    }

    let fh = header.frame_header.as_ref().unwrap();
    let width = fh.width;
    let height = fh.height;
    let plane = fh.plane;
    if plane == 0 || plane > 4 {
        return Err(SimpleAddMessage(ErrorKind::DecodeError,"Not support planes".to_string()));
    }
    match fh.component {
        None => {
            return Err(SimpleAddMessage(ErrorKind::DecodeError,"Not undefined Frame Header Component".to_string()));
        },
        _ => {

        }
    }

    let component = fh.component.as_ref().unwrap();

    match header.quantization_tables {
        None => {
            return Err(SimpleAddMessage(ErrorKind::DecodeError,"Not undefined Quantization Tables".to_string()));
        },
        _ => {

        }
    }

    if fh.is_huffman == false {
        return Err(SimpleAddMessage(ErrorKind::DecodeError,"This decoder suport huffman only".to_string()));
    }

    if fh.is_baseline == false {
        return Err(SimpleAddMessage(ErrorKind::DecodeError,"This Decoder support Baseline Only".to_string()));
    }

    if fh.is_differential == true {
        return Err(SimpleAddMessage(ErrorKind::DecodeError,"This Decoder not support differential".to_string()));
    }

    if plane == 4 {
        worning = Some(JPEGWorning::SimpleAddMessage(WorningKind::UnknowFormat,"Plane 4 color translation rule is known".to_string()));
    }

    // decode
    option.drawer.init(width,height)?;
    // take buffer for progressive 
    // progressive has 2mode
    //  - Spectral selection control
    //  - Successive approximation control
    /*  huffman for progressive
        EOBn -> 1 << n + get.bits(n)
        todo()
    */

    // loop start    

    let quantization_tables = header.quantization_tables.as_ref().unwrap();
    let (ac_decode,dc_decode) = huffman_extend(&header.huffman_tables.as_ref().unwrap());


    let slice = &buffer[header.imageoffset..];
    let bitread :&mut BitReader = &mut BitReader::new(&slice);
    let mut h_max = 1;
    let mut v_max = 1;
    let mut dy = 8;
    let mut dx = 8;
    let mut scan : Vec<(usize,usize,usize,usize)> = Vec::new();
    let mcu_size = {
        let mut size = 0;
        for i in 0..component.len() {
            size = size + component[i].h * component[i].v;
            let tq = component[i].tq;
            for _ in 0..component[i].h * component[i].v {
                scan.push((huffman_scan_header.tdcn[i],
                            huffman_scan_header.tacn[i],
                            i,tq));
            }
            dx = usize::max(component[i].h * 8 ,dx);
            dy = usize::max(component[i].v * 8 ,dy);
            h_max = usize::max(component[i].h ,h_max);
            v_max = usize::max(component[i].v ,v_max);
        }
        size
    };

    let mut preds: Vec::<i32> = (0..component.len()).map(|_| 0).collect();

    let mcu_y_max =(height+dy-1)/dy;
    let mcu_x_max =(width+dx-1)/dx;

    let mut mcu_interval = if header.interval > 0 { header.interval as isize} else {-1};


    for mcu_y in 0..mcu_y_max {
        for mcu_x in 0..mcu_x_max {
            let mut mcu_units :Vec<Vec<u8>> = Vec::new();
            for scannumber in 0..mcu_size {
                let (dc_current,ac_current,i,tq) = scan[scannumber];
                let ret = baseline_read(bitread
                            ,&dc_decode[dc_current]
                            ,&ac_decode[ac_current]
                            ,preds[i]);
                let (zz,pred);
                match ret {
                    Ok((_zz,_pred)) => {
                        zz = _zz;
                        pred = _pred; 
                    }
                    Err(r) => {
                        return Ok(Some(WorningAddMessage(WorningKind::DataCorruption,r.fmt())));
                    }
                }
                preds[i] = pred;

                let sq = &super::util::ZIG_ZAG_SEQUENCE;
                let zz :Vec<i32> = (0..64).map(|i| zz[sq[i]] * quantization_tables[tq].q[sq[i]] as i32).collect();
                let ff = fast_idct(&zz);
                mcu_units.push(ff);
            }

            // Only implement RGB

            let data = if plane == 3 {yuv_to_rgb(&mcu_units,&component,(h_max,v_max))}  // RGB
                         else if plane == 4 { // hasBug
                            if header.adobe_color_transform == 2 {ycck_to_rgb(&mcu_units,&component,(h_max,v_max))}  // YCcK Spec Unknown
                            else if header.adobe_color_transform == 1 {yuv_to_rgb(&mcu_units,&component,(h_max,v_max))} // RGBA
                            else {cmyk_to_rgb(&mcu_units,&component,(h_max,v_max))} // CMYK Spec Unknown
                         }
                         else {y_to_rgb(&mcu_units,&component)}; // g / ga

            option.drawer.draw(mcu_x*dx,mcu_y*dy,dx,dy,&data)?;

            if header.interval > 0 {
                mcu_interval = mcu_interval - 1;
                if mcu_interval == 0 { 
                    if  bitread.rst()? == true {
                        mcu_interval = header.interval as isize;
                        for i in 0..preds.len() {
                            preds[i] = 0;
                        }
                    } else {
                        worning = Some(WorningAddMessage(WorningKind::IlligalRSTMaker,"no mcu interval".to_string()));
                        return Ok(worning)
                    }
                } else if bitread.rst()? == true {
                    worning = Some(WorningAddMessage(WorningKind::IlligalRSTMaker,"mismatch mcu interval".to_string()));
                    mcu_interval = header.interval as isize;
                    for i in 0..preds.len() {
                        preds[i] = 0;
                    }
   //                 return Ok(worning);
                }
            }
        }
    }

    match bitread.next_marker() {
        Ok(marker) => {
            match marker {
                0xd9 => {   // EOI
                },
                0xdd => {
                    option.drawer.terminate()?;
                    return Ok(Some(WorningAddMessage(WorningKind::UnexpectMaker,"DNL,No Support Multi scan/frame".to_string())))
                },
               _ => {
                    option.drawer.terminate()?;
                    return Ok(Some(WorningAddMessage(WorningKind::UnexpectMaker,"No Support Multi scan/frame".to_string())))
                // offset = bitread.offset() -2
                // new_jpeg_header = read_makers(buffer[offset:],opt,false,true);
                // jpeg_header <= new Huffman Table if exit
                // jpeg_header <= new Quantize Table if exit
                // jpeg_header <= new Restart Interval if exit
                // jpeg_header <= new Add Comment Table if exit
                // jpeg_header <= new Add Appn if exit
                // goto loop
               },
            }
        },
        Err(..) => {
            worning = Some(WorningAddMessage(WorningKind::UnexpectMaker,"Not found EOI".to_string()));
        }
    }
    option.drawer.terminate()?;
    Ok(worning)

}
