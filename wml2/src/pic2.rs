//! PIC2 image decoder.
//!
//! PIC2 is a big-endian, block-oriented format.  The P2SS block used by the
//! collected files combines a small arithmetic coder with colour prediction;
//! this module ports that read-only path from the public xvpic2 reference.

use bin_rs::reader::BinaryReader;

use crate::draw::DecodeOptions;
use crate::error::ImgErrorKind;
use crate::metadata::DataMap;
use crate::retro::{draw_rgba, err, read_all};
use crate::warning::ImgWarnings;

type Error = Box<dyn std::error::Error>;

const HEADER_SIZE: usize = 124;
const BLOCK_HEADER_SIZE: usize = 26;
const CACHE_SIZE: usize = 32;
const CONTEXT_SIZE: usize = 128;
const CACHE_GROUPS: usize = 8 * 8 * 8;

fn be16(data: &[u8], offset: usize) -> Result<u16, Error> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PIC2 header is truncated"))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn be_i16(data: &[u8], offset: usize) -> Result<i16, Error> {
    Ok(be16(data, offset)? as i16)
}

fn be32(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PIC2 header is truncated"))?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn expand_bits(value: u32, bits: u8) -> u8 {
    if bits >= 8 {
        value as u8
    } else {
        ((value * 255 + ((1u32 << bits) - 1) / 2) / ((1u32 << bits) - 1)) as u8
    }
}

fn exchange_rg(value: u32, bits: u8) -> u32 {
    let mask = (1u32 << bits) - 1;
    let r_mask = mask << (bits * 2);
    let g_mask = mask << bits;
    ((value << bits) & r_mask) | ((value >> bits) & g_mask) | (value & mask)
}

fn packed_to_rgba(value: u32, depth: u16) -> [u8; 4] {
    let bits = (depth / 3) as u8;
    let mask = (1u32 << bits) - 1;
    [
        expand_bits((value >> (bits * 2)) & mask, bits),
        expand_bits((value >> bits) & mask, bits),
        expand_bits(value & mask, bits),
        255,
    ]
}

struct BitReader<'a> {
    data: &'a [u8],
    offset: usize,
    current: u8,
    remaining: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8], offset: usize) -> Self {
        Self {
            data,
            offset,
            current: 0,
            remaining: 0,
        }
    }

    fn read_bit(&mut self) -> Result<u8, Error> {
        if self.remaining == 0 {
            self.current = *self
                .data
                .get(self.offset)
                .ok_or_else(|| err(ImgErrorKind::IllegalData, "PIC2 bitstream is truncated"))?;
            self.offset += 1;
            self.remaining = 8;
        }
        self.remaining -= 1;
        Ok((self.current >> self.remaining) & 1)
    }

    fn read_bits(&mut self, count: usize) -> Result<u32, Error> {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | u32::from(self.read_bit()?);
        }
        Ok(value)
    }
}

struct Arithmetic<'a> {
    bits: BitReader<'a>,
    aa: u64,
    dd: u64,
    mulu: Vec<u16>,
    colbits: u8,
    width: usize,
    prev: Vec<u32>,
    now: Vec<u32>,
    next: Vec<u32>,
    flag_now: Vec<i16>,
    flag_next: Vec<i16>,
    flag2_now: Vec<i16>,
    flag2_next: Vec<i16>,
    flag2_next2: Vec<i16>,
    cache: Vec<[u32; CACHE_SIZE]>,
    cache_pos: Vec<u16>,
    cache_hit_c: usize,
}

impl<'a> Arithmetic<'a> {
    fn new(payload: &'a [u8], width: usize, depth: u16) -> Result<Self, Error> {
        let colbits = u8::try_from(depth / 3)
            .map_err(|_| err(ImgErrorKind::IllegalData, "PIC2 color depth is invalid"))?;
        if depth == 0 || depth > 24 || depth % 3 != 0 || colbits < 3 {
            return Err(err(
                ImgErrorKind::NoSupportFormat,
                "Unsupported PIC2 P2SS depth",
            ));
        }
        let context_bytes = CONTEXT_SIZE * 2;
        if payload.len() < context_bytes {
            return Err(err(
                ImgErrorKind::IllegalData,
                "PIC2 P2SS context is truncated",
            ));
        }
        let mut mulu = vec![0u16; 16_384];
        let mut probabilities = [0u16; CONTEXT_SIZE];
        for (index, probability) in probabilities.iter_mut().enumerate() {
            *probability = u16::from_be_bytes([payload[index * 2], payload[index * 2 + 1]]);
        }
        for (index, value) in mulu.iter_mut().enumerate() {
            let product = ((index / 128 + 128) * usize::from(probabilities[index & 127])) / 256;
            *value = u16::try_from(product.max(1)).unwrap_or(u16::MAX);
        }
        let mut bits = BitReader::new(payload, context_bytes);
        let dd = u64::from(bits.read_bits(16)?);
        let storage = width
            .checked_add(8)
            .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PIC2 row size overflow"))?;
        Ok(Self {
            bits,
            aa: 0xffff,
            dd,
            mulu,
            colbits,
            width,
            prev: vec![0; storage],
            now: vec![0; storage],
            next: vec![0; storage],
            flag_now: vec![0; storage],
            flag_next: vec![0; storage],
            flag2_now: vec![0; storage],
            flag2_next: vec![0; storage],
            flag2_next2: vec![0; storage],
            cache: vec![[0; CACHE_SIZE]; CACHE_GROUPS],
            cache_pos: vec![0; CACHE_GROUPS],
            cache_hit_c: 16,
        })
    }

    fn decode_bit(&mut self, context: usize) -> Result<bool, Error> {
        let index = ((self.aa & 0x7f00) as usize / 2)
            .checked_add(context)
            .ok_or_else(|| {
                err(
                    ImgErrorKind::IllegalData,
                    "PIC2 arithmetic context overflow",
                )
            })?;
        let probability = u64::from(*self.mulu.get(index).ok_or_else(|| {
            err(
                ImgErrorKind::IllegalData,
                "PIC2 arithmetic context is invalid",
            )
        })?);
        let result = if self.dd >= probability {
            self.dd -= probability;
            self.aa -= probability;
            true
        } else {
            self.aa = probability;
            false
        };
        while self.aa & 0x8000 == 0 {
            self.dd = (self.dd << 1) | u64::from(self.bits.read_bit()?);
            self.aa <<= 1;
        }
        Ok(result)
    }

    fn decode_nn(&mut self, context: usize) -> Result<i32, Error> {
        let mut width = 0usize;
        while width <= 7 && !self.decode_bit(context + width)? {
            width += 1;
        }
        if width == 8 {
            return Ok(255);
        }
        let mut value = (1i32 << width.min(7)) - 1;
        for bit in 0..width {
            if self.decode_bit(context + 8 + bit)? {
                value |= 1 << bit;
            }
        }
        Ok(value)
    }

    fn expand_chain(&mut self, x: usize, value: u32) -> Result<(), Error> {
        let base = x as isize + 4;
        let flag = self.flag_now[base as usize];
        let context = match flag {
            -5 => 80 + 30,
            -4 => 80 + 24,
            -3 => 80 + 18,
            -2 => 80 + 12,
            -1 => 80 + 6,
            0 => 80,
            _ => 80,
        };
        if self.decode_bit(context)? {
            return Ok(());
        }
        let target = if self.decode_bit(context + 1)? {
            (x as isize, -1)
        } else if self.decode_bit(context + 2)? {
            (x as isize - 1, -2)
        } else if self.decode_bit(context + 3)? {
            (x as isize + 1, -3)
        } else if self.decode_bit(context + 4)? {
            (x as isize - 2, -4)
        } else {
            (x as isize + 2, -5)
        };
        if target.0 < 0 || target.0 >= self.width as isize {
            // The reference decoder has four pixels of guard space around
            // every row and simply discards an edge chain that lands there.
            return Ok(());
        }
        let index = (target.0 as usize) + 4;
        self.next[index] = value;
        self.flag_next[index] = target.1;
        Ok(())
    }

    fn get_number(&mut self, context: usize, previous: i32) -> Result<i32, Error> {
        let max = (1i32 << self.colbits) - 1;
        let mut number = self.decode_nn(context)?;
        if previous > max / 2 {
            if number > (max - previous) * 2 {
                number = max - number;
            } else if number & 1 != 0 {
                number = number / 2 + previous + 1;
            } else {
                number = previous - number / 2;
            }
        } else if number > previous * 2 {
            // The original decoder leaves this branch as-is for a large
            // positive delta.  Clamp only malformed values at the boundary.
        } else if number & 1 != 0 {
            number = number / 2 + previous + 1;
        } else {
            number = previous - number / 2;
        }
        Ok(number.clamp(0, max))
    }

    fn read_color(&mut self, x: usize) -> Result<u32, Error> {
        let base = x + 4;
        let previous = self.prev[base];
        let key = ((previous >> ((self.colbits - 3) * 3)) & 0x1c0)
            | ((previous >> ((self.colbits - 3) * 2)) & 0x038)
            | ((previous >> (self.colbits - 3)) & 0x007);
        let key = if self.colbits == 5 {
            exchange_rg(key, 3) as usize
        } else {
            key as usize
        };
        if self.decode_bit(self.cache_hit_c)? {
            self.cache_hit_c = 16;
            let left = self.now[base - 1];
            let r_mask = ((1u32 << self.colbits) - 1) << (self.colbits * 2);
            let g_mask = ((1u32 << self.colbits) - 1) << self.colbits;
            let b_mask = (1u32 << self.colbits) - 1;
            let r = ((previous & r_mask) + (left & r_mask)) >> (self.colbits * 2 + 1);
            let g = ((previous & g_mask) + (left & g_mask)) >> (self.colbits + 1);
            let b = ((previous & b_mask) + (left & b_mask)) >> 1;
            let g0 = self.get_number(32, g as i32)?;
            let r0 = (r as i32 + g0 - g as i32).clamp(0, (1 << self.colbits) - 1);
            let b0 = (b as i32 + g0 - g as i32).clamp(0, (1 << self.colbits) - 1);
            let r0 = self.get_number(48, r0)?;
            let b0 = self.get_number(64, b0)?;
            let position = (usize::from(self.cache_pos[key]) + CACHE_SIZE - 1) & (CACHE_SIZE - 1);
            self.cache_pos[key] = position as u16;
            let value = (r0 as u32) << (self.colbits * 2) | (g0 as u32) << self.colbits | b0 as u32;
            self.cache[key][position] = value;
            Ok(value)
        } else {
            self.cache_hit_c = 15;
            let selected = self.decode_nn(17)? as usize;
            let middle = usize::from(self.cache_pos[key]);
            let first = (middle + selected / 2) & (CACHE_SIZE - 1);
            let second = (middle + selected) & (CACHE_SIZE - 1);
            let value = self.cache[key][second];
            self.cache[key][second] = self.cache[key][first];
            self.cache[key][first] = self.cache[key][middle];
            self.cache[key][middle] = value;
            Ok(value)
        }
    }

    fn decode(mut self, height: usize) -> Result<Vec<u32>, Error> {
        let pixels = self
            .width
            .checked_mul(height)
            .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PIC2 image size overflow"))?;
        let mut output = vec![0u32; pixels];
        let base = 4;
        for y in 0..height {
            let left_edge = if y == 0 {
                0
            } else {
                self.prev[base + self.width - 1]
            };
            self.now[base - 1] = left_edge;
            self.flag_next.fill(0);
            self.flag2_next2.fill(0);
            for x in 0..self.width {
                let index = base + x;
                let value = if self.flag_now[index] < 0 {
                    let value = self.now[index];
                    if y + 1 < height {
                        self.expand_chain(x, value)?;
                    }
                    value
                } else if self.decode_bit(self.flag2_now[index] as usize)? {
                    self.flag2_now[index + 1] += 1;
                    self.flag2_now[index + 2] += 1;
                    self.flag2_next[index - 1] += 1;
                    self.flag2_next[index] += 1;
                    self.flag2_next[index + 1] += 1;
                    self.flag2_next2[index - 1] += 1;
                    self.flag2_next2[index] += 1;
                    self.flag2_next2[index + 1] += 1;
                    let value = self.read_color(x)?;
                    if y + 1 < height {
                        self.expand_chain(x, value)?;
                    }
                    value
                } else if x == 0 {
                    self.now[index - 1]
                } else {
                    self.now[index - 1]
                };
                self.now[index] = value;
                output[y * self.width + x] = value;
            }
            std::mem::swap(&mut self.prev, &mut self.now);
            std::mem::swap(&mut self.now, &mut self.next);
            std::mem::swap(&mut self.flag_now, &mut self.flag_next);
            let old_flag2_now = std::mem::replace(&mut self.flag2_now, self.flag2_next);
            self.flag2_next = self.flag2_next2;
            self.flag2_next2 = old_flag2_now;
        }
        Ok(output)
    }
}

fn decode_raw(
    payload: &[u8],
    width: usize,
    height: usize,
    depth: u16,
    id: &[u8; 4],
) -> Result<Vec<u32>, Error> {
    let bytes_per_pixel = usize::from(depth.div_ceil(8));
    if !matches!(depth, 15 | 24) {
        return Err(err(
            ImgErrorKind::NoSupportFormat,
            "Unsupported PIC2 raw depth",
        ));
    }
    let size = width
        .checked_mul(height)
        .and_then(|value| value.checked_mul(bytes_per_pixel))
        .ok_or_else(|| {
            err(
                ImgErrorKind::InvalidParameter,
                "PIC2 raw image size overflow",
            )
        })?;
    if payload.len() < size {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PIC2 raw block is truncated",
        ));
    }
    let mut output = Vec::with_capacity(width * height);
    let mut offset = 0;
    for _ in 0..width * height {
        let value = if depth == 24 {
            let value = (u32::from(payload[offset]) << 16)
                | (u32::from(payload[offset + 1]) << 8)
                | u32::from(payload[offset + 2]);
            offset += 3;
            value
        } else {
            let a = u16::from(payload[offset]);
            let b = u16::from(payload[offset + 1]);
            offset += 2;
            let value = if id == b"P2BM" {
                (a << 8) | b
            } else {
                (b << 8) | a
            };
            exchange_rg(u32::from(value >> 1), 5)
        };
        output.push(value);
    }
    Ok(output)
}

struct Header {
    flag: u16,
    size: usize,
    depth: u16,
    x_aspect: u16,
    y_aspect: u16,
    width: usize,
    height: usize,
}

struct Block {
    offset: usize,
    id: [u8; 4],
    size: usize,
    flag: u16,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    opaque: u32,
}

fn parse_header(data: &[u8]) -> Result<(usize, Header), Error> {
    let base = if data.starts_with(b"P2DT") {
        0
    } else if data.len() >= 132 && &data[128..132] == b"P2DT" {
        128
    } else {
        return Err(err(ImgErrorKind::IllegalData, "Not a PIC2 image"));
    };
    if data.len() < base + HEADER_SIZE {
        return Err(err(ImgErrorKind::IllegalData, "PIC2 header is truncated"));
    }
    let relative_size = usize::try_from(be32(data, base + 106)?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "PIC2 header size is invalid"))?;
    if relative_size < HEADER_SIZE
        || base.checked_add(relative_size).is_none()
        || base + relative_size > data.len()
    {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PIC2 block offset is invalid",
        ));
    }
    let width = usize::from(be16(data, base + 116)?);
    let height = usize::from(be16(data, base + 118)?);
    let depth = be16(data, base + 110)?;
    if width == 0 || height == 0 || depth == 0 || depth > 24 || depth % 3 != 0 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PIC2 image dimensions or depth are invalid",
        ));
    }
    let mut palette_end = base + HEADER_SIZE;
    if be16(data, base + 98)? & 1 != 0 {
        let _pal_bits = *data
            .get(palette_end)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "PIC2 palette is truncated"))?;
        let palette_count = usize::from(be16(data, palette_end + 1)?);
        palette_end = palette_end
            .checked_add(
                3 + palette_count.checked_mul(3).ok_or_else(|| {
                    err(ImgErrorKind::InvalidParameter, "PIC2 palette size overflow")
                })?,
            )
            .ok_or_else(|| {
                err(
                    ImgErrorKind::InvalidParameter,
                    "PIC2 palette offset overflow",
                )
            })?;
    }
    if palette_end > base + relative_size {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PIC2 comment area is invalid",
        ));
    }
    Ok((
        base,
        Header {
            flag: be16(data, base + 98)?,
            size: relative_size,
            depth,
            x_aspect: be16(data, base + 112)?,
            y_aspect: be16(data, base + 114)?,
            width,
            height,
        },
    ))
}

fn parse_block(data: &[u8], offset: usize) -> Result<Option<Block>, Error> {
    if offset + 4 > data.len() {
        return Ok(None);
    }
    if data[offset..offset + 4].iter().all(|&byte| byte == 0) {
        return Ok(None);
    }
    let size = usize::try_from(be32(data, offset + 4)?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "PIC2 block size is invalid"))?;
    let end = offset
        .checked_add(size)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PIC2 block size overflow"))?;
    if size < 8 || end > data.len() {
        return Err(err(ImgErrorKind::IllegalData, "PIC2 block is truncated"));
    }
    let id = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ];
    if !matches!(&id, b"P2SS" | b"P2BM" | b"P2BI") {
        return Ok(Some(Block {
            offset,
            id,
            size,
            flag: 0,
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            opaque: 0,
        }));
    }
    if size < BLOCK_HEADER_SIZE {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PIC2 image block header is truncated",
        ));
    }
    let x = be_i16(data, offset + 14)?;
    let y = be_i16(data, offset + 16)?;
    let width = usize::from(be16(data, offset + 10)?);
    let height = usize::from(be16(data, offset + 12)?);
    if x < 0 || y < 0 || width == 0 || height == 0 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PIC2 block bounds are invalid",
        ));
    }
    Ok(Some(Block {
        offset,
        id,
        size,
        flag: be16(data, offset + 8)?,
        width,
        height,
        x: x as usize,
        y: y as usize,
        opaque: be32(data, offset + 18)?,
    }))
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let data = read_all(reader)?;
    let (base, header) = parse_header(&data)?;
    let canvas_width = header.width;
    let canvas_height = header.height;
    let mut position = base + header.size;
    let mut blocks = Vec::new();
    while let Some(block) = parse_block(&data, position)? {
        let right = block
            .x
            .checked_add(block.width)
            .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PIC2 block x overflow"))?;
        let bottom = block
            .y
            .checked_add(block.height)
            .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PIC2 block y overflow"))?;
        if right > canvas_width || bottom > canvas_height {
            return Err(err(
                ImgErrorKind::IllegalData,
                "PIC2 block is outside canvas",
            ));
        }
        position += block.size;
        blocks.push(block);
    }
    let canvas_pixels = canvas_width
        .checked_mul(canvas_height)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PIC2 canvas size overflow"))?;
    let mut output = vec![0u8; canvas_pixels * 4];
    let mut decoded_block = false;
    for block in blocks {
        if block.width == 0 || block.height == 0 {
            continue;
        }
        let block_start = block.offset;
        let payload = &data[block_start + BLOCK_HEADER_SIZE..block_start + block.size];
        let pixels = match &block.id {
            b"P2SS" => {
                decoded_block = true;
                Arithmetic::new(payload, block.width, header.depth)?.decode(block.height)?
            }
            b"P2BM" | b"P2BI" => {
                decoded_block = true;
                decode_raw(payload, block.width, block.height, header.depth, &block.id)?
            }
            _ => continue,
        };
        let opaque = (block.flag & 1 != 0).then_some(block.opaque);
        for y in 0..block.height {
            for x in 0..block.width {
                let value = pixels[y * block.width + x];
                if opaque == Some(value) {
                    continue;
                }
                let rgba = packed_to_rgba(value, header.depth);
                let dst = ((block.y + y) * canvas_width + block.x + x) * 4;
                output[dst..dst + 4].copy_from_slice(&rgba);
            }
        }
    }
    if !decoded_block {
        return Err(err(
            ImgErrorKind::NoSupportFormat,
            "PIC2 has no supported image block",
        ));
    }
    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("PIC2".to_string()))?;
    option
        .drawer
        .set_metadata("width", DataMap::UInt(canvas_width as u64))?;
    option
        .drawer
        .set_metadata("height", DataMap::UInt(canvas_height as u64))?;
    option
        .drawer
        .set_metadata("depth", DataMap::UInt(u64::from(header.depth)))?;
    option
        .drawer
        .set_metadata("header size", DataMap::UInt(header.size as u64))?;
    option
        .drawer
        .set_metadata("aspect x", DataMap::UInt(u64::from(header.x_aspect)))?;
    option
        .drawer
        .set_metadata("aspect y", DataMap::UInt(u64::from(header.y_aspect)))?;
    option
        .drawer
        .set_metadata("header flags", DataMap::UInt(u64::from(header.flag)))?;
    draw_rgba(option, canvas_width, canvas_height, &output)?;
    Ok(None)
}
