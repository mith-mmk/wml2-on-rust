/*
 * io/mod.rs  Mith@mmk (C) 2022
 * 
 */

#[inline]
pub fn read_f32 (buf: &[u8],ptr: usize,flag: bool) -> f32 {
    f32::from_bits(read_u32(buf,ptr,flag))
}

#[inline]
pub fn read_f64 (buf: &[u8],ptr: usize,flag: bool) -> f64 {
    f64::from_bits(read_u64(buf,ptr,flag))
}

#[inline]
pub fn read_byte(buf: &[u8],ptr: usize ) -> u8 {
    buf[ptr]
}

#[inline]
pub fn read_i8(buf: &[u8],ptr: usize ) -> i8 {
    ((
            &buf[ptr]
        ) as *const u8) as i8
}

#[inline]
pub fn read_u16be (buf: &[u8],ptr: usize ) -> u16 {
    (buf[ptr] as u16) << 8 | (buf[ptr+1] as u16)
}

#[inline]
pub fn read_i16be (buf: &[u8],ptr: usize ) -> i16 {
        ((
            &(buf[ptr] as u16) << 8 | (buf[ptr+1] as u16)
        ) as u16) as i16
}

#[inline]
pub fn read_u32be (buf: &[u8],ptr: usize ) -> u32 {
    (buf[ptr  ] as u32) << 24 | (buf[ptr+1] as u32) << 16 |
    (buf[ptr+2] as u32) << 8  | (buf[ptr+3] as u32)
}

#[inline]
pub fn read_i32be (buf: &[u8],ptr: usize ) -> i32 {
        ((
            (buf[ptr  ] as u32) << 24 | (buf[ptr+1] as u32) << 16 |
            (buf[ptr+2] as u32) << 8  | (buf[ptr+3] as u32)
        ) as u32) as i32
}

#[inline]
pub fn read_u64be (buf: &[u8],ptr: usize) -> u64 {
    (buf[ptr  ] as u64) << 56 | (buf[ptr+1] as u64) << 48 |
    (buf[ptr+2] as u64) << 40 | (buf[ptr+3] as u64) << 32 |
    (buf[ptr+4] as u64) << 24 | (buf[ptr+5] as u64) << 16 |
    (buf[ptr+6] as u64) << 8  | (buf[ptr+7] as u64)  
}

#[inline]
pub fn read_u128be (buf: &[u8],ptr: usize) -> u128 {
    let b0 = read_u64be(buf,ptr);
    let b1 = read_u64be(buf,ptr);
    ((b0 as u128) << 64) | b1 as u128
}


#[allow(unused)]
#[inline]
pub fn read_i64be (buf: &[u8],ptr: usize ) -> i64 {
        ((
            (buf[ptr  ] as u64) << 56 | (buf[ptr+1] as u64) << 48 |
            (buf[ptr+2] as u64) << 40 | (buf[ptr+3] as u64) << 32 |
            (buf[ptr+4] as u64) << 24 | (buf[ptr+5] as u64) << 16 |
            (buf[ptr+6] as u64) << 8  | (buf[ptr+7] as u64) 
        ) as  u64) as i64
}

#[inline]
pub fn read_i16le (buf: &[u8],ptr: usize ) -> i16 {
    unsafe {
        *((
            &(buf[ptr] as u16) << 8 | (buf[ptr+1] as u16)
        ) as *const u16) as i16
    }
}

#[inline]
pub fn read_u16le (buf: &[u8],ptr: usize ) -> u16 {
    (buf[ptr+1] as u16) << 8 | buf[ptr] as u16
}

#[inline]
pub fn read_u32le (buf: &[u8],ptr: usize ) -> u32 {
    (buf[ptr+3] as u32) << 24 | (buf[ptr+2] as u32) << 16 |
    (buf[ptr+1] as u32) << 8  | (buf[ptr  ]) as u32        
}

#[inline]
pub fn read_i32le (buf: &[u8],ptr: usize ) -> i32 {
      ((buf[ptr+3] as u32) << 24 | (buf[ptr+2] as u32) << 16 |
            (buf[ptr+1] as u32) << 8  | (buf[ptr  ]) as u32) as i32
}

#[allow(unused)]
#[inline]
pub fn read_u64le (buf: &[u8],ptr: usize ) -> u64 {
    (buf[ptr+7] as u64) << 56 | (buf[ptr+6] as u64) << 48 |
    (buf[ptr+5] as u64) << 40 | (buf[ptr+4] as u64) << 32 |
    (buf[ptr+3] as u64) << 24 | (buf[ptr+2] as u64) << 16 |
    (buf[ptr+1] as u64) << 8  | buf[ptr] as u64 
}

#[allow(unused)]
#[inline]
pub fn read_i64le (buf: &[u8],ptr: usize ) -> i64 {
(
        ((buf[ptr+7] as u64) << 56 | (buf[ptr+6] as u64) << 48 |
            (buf[ptr+5] as u64) << 40 | (buf[ptr+4] as u64) << 32 |
            (buf[ptr+3] as u64) << 24 | (buf[ptr+2] as u64) << 16 |
            (buf[ptr+1] as u64) << 8  | buf[ptr] as u64 
        ) as u64) as i64
}

#[inline]
pub fn read_u16 (buf: &[u8],ptr: usize ,flag: bool) -> u16 {
    if flag {
        read_u16le(buf,ptr)
    } else {
        read_u16be(buf,ptr)
    }
} 


#[inline]
pub fn read_u32 (buf: &[u8],ptr: usize ,flag: bool) -> u32 {
    if flag {
        read_u32le(buf,ptr)
    } else {
        read_u32be(buf,ptr)
    }
} 

#[allow(unused)]
#[inline]
pub fn read_u64 (buf: &[u8],ptr: usize ,flag: bool) -> u64 {
    if flag {
        read_u64le(buf,ptr)
    } else {
        read_u64be(buf,ptr)
    }
} 

#[inline]
pub fn read_i16 (buf: &[u8],ptr: usize ,flag: bool) -> i16 {
    if flag {
        read_i16le(buf,ptr)
    } else {
        read_i16be(buf,ptr)
    }
} 

#[inline]
pub fn read_i32 (buf: &[u8],ptr: usize ,flag: bool) -> i32 {
    if flag {
        read_i32le(buf,ptr)
    } else {
        read_i32be(buf,ptr)
    }
} 

#[allow(unused)]
#[inline]
pub fn read_i64 (buf: &[u8],ptr: usize ,flag: bool) -> i64 {
    if flag {
        read_i64le(buf,ptr)
    } else {
        read_i64be(buf,ptr)
    }
} 

#[inline]
pub fn read_string (buf: &[u8],ptr: usize ,num: usize) -> String {
    let mut s = Vec::new();
    for i in 0..num {
//        if buf.len() >= ptr + i {break;}
        if buf[ptr + i] == 0 {break;}
        s.push(buf[ptr + i]);
    }
    let res = String::from_utf8(s);
    match res {
        Ok(strings) => {
            return strings;
        },
        _ => {
            return "".to_string();
        }
    }
}

#[inline]
pub fn read_bytes (buf: &[u8],ptr: usize ,length: usize) -> Vec<u8> {
    let mut c = Vec::new();
    for i in 0..length {
        c.push(buf[ptr + i]);
    }
    c
}

