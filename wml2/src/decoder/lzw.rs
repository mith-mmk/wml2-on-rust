//! LZW decoder used by GIF and TIFF.

type Error = Box<dyn std::error::Error>;
use crate::error::ImgError;
use crate::error::ImgErrorKind;

const MAX_TABLE: usize = 4096;
const MAX_CBL: usize = 12;

pub struct Lzwdecode {
    buffer: Vec<u8>,
    cbl: usize,
    recovery_cbl: usize,
    last_byte: u32,
    left_bits: usize,
    bit_mask: u32,
    ptr: usize,
    clear: usize,
    end: usize,
    max_table: usize,
    dic: Vec<Vec<u8>>,
    prev_code: usize,
    is_init: bool,
    is_lsb: bool,
    is_tiff: usize,
}

impl Lzwdecode {
    fn insufficient_bits_error() -> Error {
        Box::new(ImgError::new_const(
            ImgErrorKind::IOError,
            "data shortage".to_string(),
        ))
    }

    /// for GIF LZW
    pub fn gif(lzw_min_bits: usize) -> Self {
        Self::new(lzw_min_bits, true, false)
    }

    /// TIFF LZW with the legacy boolean code-order selector.
    ///
    /// New code should prefer [`Self::tiff_standard`],
    /// [`Self::tiff_wml2_lsb`], or [`Self::tiff_libtiff_compat`] so that
    /// TIFF FillOrder and LZW code packing are not confused.
    pub fn tiff(is_lsb: bool) -> Self {
        if is_lsb {
            Self::tiff_wml2_lsb()
        } else {
            Self::tiff_standard()
        }
    }

    /// Standard TIFF LZW: MSB-first code packing and early code-width change.
    pub fn tiff_standard() -> Self {
        Self::new(8, false, true)
    }

    /// WML2's historical non-standard LSB-first TIFF extension.
    pub fn tiff_wml2_lsb() -> Self {
        Self::new(8, true, true)
    }

    /// LibTIFF's old TIFF LZW stream: LSB-first packing and late code-width
    /// change. LibTIFF detects this format from the stream signature.
    pub fn tiff_libtiff_compat() -> Self {
        Self::new(8, true, false)
    }

    pub fn new(lzw_min_bits: usize, is_lsb: bool, is_tiff: bool) -> Self {
        let cbl = lzw_min_bits + 1;
        let clear_code = 1 << lzw_min_bits;
        let max_table = MAX_TABLE;
        let is_tiff = if is_tiff { 1 } else { 0 };
        Self {
            buffer: Vec::new(),
            cbl,
            recovery_cbl: cbl,
            bit_mask: (1 << cbl) - 1,
            last_byte: 0,
            left_bits: 0,
            ptr: 0,
            clear: clear_code,
            end: clear_code + 1,
            max_table,
            dic: Vec::with_capacity(max_table),
            prev_code: clear_code,
            is_init: false,
            is_lsb,  // GIF Must True
            is_tiff, // if tiff set 1
        }
    }

    // use 32bit
    fn fill_bits(&mut self) {
        self.clear_dic();
        self.last_byte = 0;
        let count = self.buffer.len().saturating_sub(self.ptr).min(3);
        let ptr = self.ptr;
        if self.is_lsb {
            for i in 0..count {
                self.last_byte |= u32::from(self.buffer[ptr + i]) << ((3 - count + i) * 8);
            }
        } else {
            for i in 0..count {
                self.last_byte = (self.last_byte << 8) | u32::from(self.buffer[ptr + i]);
            }
        }
        self.left_bits = count * 8;
        self.ptr += count;
    }

    fn get_bits(&mut self) -> Result<usize, Error> {
        if self.is_lsb {
            return self.get_bits_lsb();
        } else {
            return self.get_bits_msb();
        }
    }

    fn get_bits_msb(&mut self) -> Result<usize, Error> {
        let size = self.cbl;
        while self.left_bits <= 16 {
            if self.ptr >= self.buffer.len() {
                if self.left_bits < size {
                    return Err(Self::insufficient_bits_error());
                }
                break;
            }

            self.last_byte = (self.last_byte << 8) | (self.buffer[self.ptr] as u32);
            self.ptr += 1;
            self.left_bits += 8;
        }
        if self.left_bits < size {
            return Err(Self::insufficient_bits_error());
        }
        let bits = (self.last_byte >> (self.left_bits - size)) & self.bit_mask;

        self.left_bits -= size;
        Ok(bits as usize)
    }

    fn get_bits_lsb(&mut self) -> Result<usize, Error> {
        let size = self.cbl;
        while self.left_bits <= 16 {
            if self.ptr >= self.buffer.len() {
                if self.left_bits < size {
                    return Err(Self::insufficient_bits_error());
                }
                break;
            }
            self.last_byte =
                (self.last_byte >> 8) & 0xffff | ((self.buffer[self.ptr] as u32) << 16);

            self.ptr += 1;
            self.left_bits += 8;
        }
        if self.left_bits < size {
            return Err(Self::insufficient_bits_error());
        }
        let bits = (self.last_byte >> (24 - self.left_bits)) & self.bit_mask;

        self.left_bits -= size;
        Ok(bits as usize)
    }

    fn clear_dic(&mut self) {
        self.dic = (0..self.end + 1)
            .map(|i| {
                if i < self.clear {
                    vec![i as u8]
                } else {
                    vec![]
                }
            })
            .collect();
        self.cbl = self.recovery_cbl;
        self.bit_mask = ((1_u64 << self.cbl) - 1) as u32;
    }

    // Multi chuck image data decoding is not debug.
    pub fn decode(&mut self, buf: &[u8]) -> Result<Vec<u8>, Error> {
        self.decode_inner(buf, None)
    }

    /// Decode one complete compressed stream with a strict output budget.
    /// Unlike the streaming API, incomplete input without EOI is an error.
    pub fn decode_with_limit(&mut self, buf: &[u8], max_output: usize) -> Result<Vec<u8>, Error> {
        self.decode_inner(buf, Some(max_output))
    }

    fn append_output(data: &mut Vec<u8>, bytes: &[u8], limit: Option<usize>) -> Result<(), Error> {
        let length = data.len().checked_add(bytes.len()).ok_or_else(|| {
            Box::new(ImgError::new_const(
                ImgErrorKind::OutOfMemory,
                "LZW output size overflow".into(),
            )) as Error
        })?;
        if limit.is_some_and(|maximum| length > maximum) {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::OutOfMemory,
                "LZW output exceeds decode limit".into(),
            )));
        }
        data.try_reserve(bytes.len())?;
        data.extend_from_slice(bytes);
        Ok(())
    }

    fn decode_inner(&mut self, buf: &[u8], limit: Option<usize>) -> Result<Vec<u8>, Error> {
        self.buffer = buf.to_vec();
        if !self.is_init {
            self.fill_bits();
            self.is_init = true;
        } else {
            self.ptr = 0;
        }

        let mut data: Vec<u8> = Vec::new();
        self.prev_code = self.clear; // NULL

        loop {
            let res = self.get_bits(); // GIF Lsb only Tiff use Lsb or Msb
            // If data is shotage,it returns values and waits a next buffer.
            let code = match res {
                Ok(code) => code,
                Err(error) if limit.is_some() => return Err(error),
                Err(_) => return Ok(data),
            };

            if code == self.clear {
                let prev = self
                    .dic
                    .get(self.prev_code)
                    .ok_or_else(Self::insufficient_bits_error)?;
                Self::append_output(&mut data, prev, limit)?;
                self.clear_dic();
            } else if code == self.end {
                let prev = self.dic.get(self.prev_code).ok_or_else(|| {
                    Box::new(ImgError::new_const(
                        ImgErrorKind::IllegalData,
                        "invalid previous LZW code".to_string(),
                    )) as Error
                })?;
                Self::append_output(&mut data, prev, limit)?;
                return Ok(data);
            } else if code > self.dic.len() {
                let message = format!(
                    "Over table in LZW.Table size is {},but code is {}",
                    self.dic.len(),
                    code
                );
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::IllegalData,
                    message,
                )));
            } else {
                let append_code;
                if code == self.dic.len() {
                    append_code = *self
                        .dic
                        .get(self.prev_code)
                        .and_then(|value| value.first())
                        .ok_or_else(|| {
                            Box::new(ImgError::new_const(
                                ImgErrorKind::IllegalData,
                                "invalid previous LZW code".to_string(),
                            )) as Error
                        })?;
                } else {
                    append_code = *self
                        .dic
                        .get(code)
                        .and_then(|value| value.first())
                        .ok_or_else(|| {
                            Box::new(ImgError::new_const(
                                ImgErrorKind::IllegalData,
                                "invalid LZW code".to_string(),
                            )) as Error
                        })?;
                }
                if self.prev_code != self.end && self.prev_code != self.clear {
                    let prev = self.dic.get(self.prev_code).ok_or_else(|| {
                        Box::new(ImgError::new_const(
                            ImgErrorKind::IllegalData,
                            "invalid previous LZW code".to_string(),
                        )) as Error
                    })?;
                    Self::append_output(&mut data, prev, limit)?;
                    if self.dic.len() < self.max_table {
                        let mut table = prev.clone();
                        table.push(append_code);
                        self.dic.push(table);
                    }
                    // Tiff LZW is increment entry value before next loop.
                    let next = self.dic.len() + self.is_tiff;
                    if next == self.bit_mask as usize + 1
                        && next < self.max_table
                        && self.cbl < MAX_CBL
                    {
                        self.cbl += 1;
                        self.bit_mask = (self.bit_mask << 1) | 1;
                    }
                }
            }
            self.prev_code = code;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_tiff_lzw_checks_expansion_and_stream_end() {
        for lsb in [false, true] {
            let pixels = vec![7; 4096];
            let encoded = crate::encoder::lzw::encode_tiff(&pixels, lsb).unwrap();
            assert_eq!(
                Lzwdecode::tiff(lsb)
                    .decode_with_limit(&encoded, pixels.len())
                    .unwrap(),
                pixels
            );
            assert!(
                Lzwdecode::tiff(lsb)
                    .decode_with_limit(&encoded, 32)
                    .is_err()
            );
            assert!(
                Lzwdecode::tiff(lsb)
                    .decode_with_limit(&encoded[..encoded.len() - 2], pixels.len())
                    .is_err()
            );
            for len in 0..3 {
                assert!(
                    Lzwdecode::tiff(lsb)
                        .decode_with_limit(&[0, 0][..len], 16)
                        .is_err()
                );
            }
        }
    }
}
