/// not impl

pub trait ByteReader {
    fn read_f64(&mut self, flag: bool) -> Result<f64,ImgError>;
    fn read_f32(&mut self, flag: bool) -> Result<f32,ImgError>;

    fn read_u128(&mut self, flag: bool) -> Result<u128,ImgError>;
    fn read_i128(&mut self, flag: bool) -> Result<i128,ImgError>;

    fn read_u64(&mut self, flag: bool) -> Result<u64,ImgError>;
    fn read_i64(&mut self, flag: bool) -> Result<i64,ImgError>;
    fn read_u32(&mut self, flag: bool) -> Result<u32,ImgError>;
    fn read_i32(&mut self, flag: bool) -> Result<i32,ImgError>;
    fn read_u16(&mut self, flag: bool) -> Result<u16,ImgError>;
    fn read_i16(&mut self, flag: bool) -> Result<i16,ImgError>;

    fn read_byte(&mut self) -> Result<u8,ImgError>;
    fn read_i8(&mut self) -> Result<i8,ImgError>;
    fn read_bytes(&mut self,bytes:usize) -> Result<i8,ImgError>;

    fn read_u128be(&mut self) -> Result<u128,ImgError>;
    fn read_i128be(&mut self) -> Result<i128,ImgError>;
    fn read_f64be(&mut self) -> Result<f64,ImgError>;
    fn read_f32be(&mut self) -> Result<f32,ImgError>;
    fn read_u64be(&mut self) -> Result<u64,ImgError>;
    fn read_i64be(&mut self) -> Result<i64,ImgError>;
    fn read_i32be(&mut self) -> Result<i32,ImgError>;
    fn read_u32be(&mut self) -> Result<u32,ImgError>;
    fn read_u16be(&mut self) -> Result<u16,ImgError>;
    fn read_i16be(&mut self) -> Result<i16,ImgError>;

    fn read_u128le(&mut self) -> Result<u128,ImgError>;
    fn read_i128le(&mut self) -> Result<i128,ImgError>;
    fn read_f64le(&mut self) -> Result<f64,ImgError>;
    fn read_f32le(&mut self) -> Result<f32,ImgError>;
    fn read_u64le(&mut self) -> Result<u64,ImgError>;
    fn read_u32le(&mut self) -> Result<u32,ImgError>;
    fn read_u16le(&mut self) -> Result<u16,ImgError>;
    fn read_i64le(&mut self) -> Result<i64,ImgError>;
    fn read_i32le(&mut self) -> Result<i32,ImgError>;
    fn read_i16le(&mut self) -> Result<i16,ImgError>;
}

pub struct PicReader {
    buffer: Vec<u8>,
    ptr: usize,
}

impl PicReader {
    pub fn new (buffer: &[u8]) -> Self {
        Self {
            buffer: buffer.to_vec(),
            ptr: 0,
        }
    }

    pub fn seek(&self,ptr: usize) -> Result<bool,ImgError>{
        if ptr >= self.buffer.len() {
            return Err(ImgError::new_const(ImgErrorKind::OutboundIndex, &"Seek error"))
        } else {
            self.ptr = ptr;
            Ok(true)
        }
    }

    #[inline]
    fn bound_check(&self, ptr:usize) -> bool {
        if ptr >= self.ptr {
            return false
        }
        true
    }

impl ByteReader for PicReader {
    
    fn read_f64(&mut self, flag: bool) -> Result<f64,ImgError> {
        if bound_check(self.ptr + 8) {
            return Ok(read_f64(self.buffer, self.ptr,flag))
        }
        return Err(ImgError::new_const(ImgErrorKind::OutboundIndex, &"f64 read")
    }
    fn read_f32(&mut self, flag: bool) -> Result<f32,ImgError> {
         <body> }

    fn read_u128(&mut self, flag: bool) -> Result<u128,ImgError>{ <body> }
    fn read_i128(&mut self, flag: bool) -> Result<i128,ImgError>{ <body> }

    fn read_u64(&mut self, flag: bool) -> Result<u64,ImgError>{ <body> }
    fn read_i64(&mut self, flag: bool) -> Result<i64,ImgError>{ <body> }
    fn read_u32(&mut self, flag: bool) -> Result<u32,ImgError>{ <body> }
    fn read_i32(&mut self, flag: bool) -> Result<i32,ImgError>{ <body> }
    fn read_u16(&mut self, flag: bool) -> Result<u16,ImgError>{ <body> }
    fn read_i16(&mut self, flag: bool) -> Result<i16,ImgError>{ <body> }

    fn read_byte(&mut self) -> Result<u8,ImgError>{ <body> }
    fn read_i8(&mut self) -> Result<i8,ImgError>{ <body> }
    fn read_bytes(&mut self,bytes:usize) -> Result<i8,ImgError>{ <body> }

    fn read_u128be(&mut self) -> Result<u128,ImgError>{ <body> }
    fn read_i128be(&mut self) -> Result<i128,ImgError>{ <body> }
    fn read_f64be(&mut self) -> Result<f64,ImgError>{ <body> }
    fn read_f32be(&mut self) -> Result<f32,ImgError>{ <body> }
    fn read_u64be(&mut self) -> Result<u64,ImgError>{ <body> }
    fn read_i64be(&mut self) -> Result<i64,ImgError>{ <body> }
    fn read_i32be(&mut self) -> Result<i32,ImgError>{ <body> }
    fn read_u32be(&mut self) -> Result<u32,ImgError>{ <body> }
    fn read_u16be(&mut self) -> Result<u16,ImgError>{ <body> }
    fn read_i16be(&mut self) -> Result<i16,ImgError>{ <body> }

    fn read_u128le(&mut self) -> Result<u128,ImgError>;
    fn read_i128le(&mut self) -> Result<i128,ImgError>;
    fn read_f64le(&mut self) -> Result<f64,ImgError>;
    fn read_f32le(&mut self) -> Result<f32,ImgError>;
    fn read_u64le(&mut self) -> Result<u64,ImgError>;
    fn read_u32le(&mut self) -> Result<u32,ImgError>;
    fn read_u16le(&mut self) -> Result<u16,ImgError>;
    fn read_i64le(&mut self) -> Result<i64,ImgError>;
    fn read_i32le(&mut self) -> Result<i32,ImgError>;
    fn read_i16le(&mut self) -> Result<i16,ImgError>;
}