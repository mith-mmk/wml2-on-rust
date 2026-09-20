//! XLD4 (Q4) image decoder.
//!
//! Q4 stores a fixed 640x400 image with a 16-colour palette.  Each palette
//! and image block is compressed by the same two-stage stream: an adaptive
//! MSB-first bit code followed by a small LZW-style dictionary.  The decoded
//! bytes are then expanded from the format's nibble RLE.

use bin_rs::reader::BinaryReader;

use crate::draw::DecodeOptions;
use crate::error::{ImgError, ImgErrorKind};
use crate::metadata::DataMap;
use crate::retro::{draw_rgba, err, read_all};
use crate::warning::ImgWarnings;

type Error = Box<dyn std::error::Error>;

const WIDTH: usize = 640;
const HEIGHT: usize = 400;
const PALETTE_BYTES: usize = 16 * 6;
const HEADER_SIZE: usize = 22;
const BLOCK_HEADER_SIZE: usize = 6;
const MAX_WORK_BUFFER: usize = 0xfdf0;
const INITIAL_DICTIONARY: usize = 0x12;

const PALETTE_ORDER: [u8; 16] = [
    0x00, 0x02, 0x04, 0x06, 0x01, 0x03, 0x05, 0x07, 0x08, 0x0a, 0x0c, 0x0e, 0x09, 0x0b, 0x0d, 0x0f,
];

fn output_layout() -> Result<(usize, usize), Error> {
    let expected_pixels = WIDTH
        .checked_mul(HEIGHT)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "Q4 image size overflow"))?;
    crate::limits::check(expected_pixels, crate::limits::current().pixels, "pixels")?;
    let output_len = expected_pixels.checked_mul(4).ok_or_else(|| {
        err(
            ImgErrorKind::InvalidParameter,
            "Q4 RGBA image size overflow",
        )
    })?;
    crate::limits::check(
        output_len,
        crate::limits::current().expanded_bytes,
        "RGBA image",
    )?;
    Ok((expected_pixels, output_len))
}

fn le16(data: &[u8], offset: usize) -> Result<u16, Error> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 header is truncated"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn le24(data: &[u8], offset: usize) -> Result<usize, Error> {
    let bytes = data
        .get(offset..offset + 3)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 header is truncated"))?;
    Ok(usize::from(bytes[0]) | (usize::from(bytes[1]) << 8) | (usize::from(bytes[2]) << 16))
}

struct BitReader<'a> {
    data: &'a [u8],
    bit_offset: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            bit_offset: 0,
        }
    }

    fn read_bits(&mut self, count: usize) -> Option<u32> {
        let end = self.bit_offset.checked_add(count)?;
        if end > self.data.len().checked_mul(8)? {
            return None;
        }
        let mut value = 0u32;
        for _ in 0..count {
            let byte = self.data[self.bit_offset / 8];
            let bit = (byte >> (7 - self.bit_offset % 8)) & 1;
            value = (value << 1) | u32::from(bit);
            self.bit_offset += 1;
        }
        Some(value)
    }
}

fn decode_stage1(data: &[u8]) -> Result<Vec<u16>, Error> {
    if data.is_empty() {
        return Err(err(
            ImgErrorKind::IllegalData,
            "Q4 compressed block is empty",
        ));
    }
    let mut padded = Vec::with_capacity(data.len() + 3);
    padded.extend_from_slice(data);
    padded.extend_from_slice(&[0, 0, 0]);
    let mut reader = BitReader::new(&padded);
    let mut bit_width = 3usize;
    let mut codes = Vec::new();
    loop {
        if (reader.bit_offset % 8) + bit_width >= 24 {
            return Ok(codes);
        }
        let value = reader.read_bits(bit_width).ok_or_else(|| {
            Box::new(ImgError::new_const(
                ImgErrorKind::IllegalData,
                format!(
                    "Q4 bit stream is truncated: {} bytes at bit {} width {}",
                    data.len(),
                    reader.bit_offset,
                    bit_width
                ),
            )) as Error
        })?;
        match value {
            0 => return Ok(codes),
            1 => {
                bit_width = bit_width
                    .checked_add(1)
                    .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 code width overflow"))?;
                if bit_width > 16 {
                    return Err(err(
                        ImgErrorKind::IllegalData,
                        "Q4 code width is unsupported",
                    ));
                }
            }
            value => {
                let code = value - 2;
                codes.push(
                    u16::try_from(code).map_err(|_| {
                        err(ImgErrorKind::IllegalData, "Q4 dictionary code overflow")
                    })?,
                );
                if codes.len() > MAX_WORK_BUFFER / 2 {
                    return Err(err(
                        ImgErrorKind::InvalidParameter,
                        "Q4 dictionary stream is too large",
                    ));
                }
            }
        }
    }
}

fn decode_stage2(codes: &[u16]) -> Result<Vec<u8>, Error> {
    let first = codes
        .first()
        .copied()
        .map(usize::from)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 dictionary stream is empty"))?;
    if first >= INITIAL_DICTIONARY {
        return Err(err(
            ImgErrorKind::IllegalData,
            "Q4 dictionary stream starts with an invalid code",
        ));
    }

    let mut storage = vec![0u8; MAX_WORK_BUFFER];
    let mut entries = vec![(0usize, 0usize); INITIAL_DICTIONARY];
    let initial_offset = MAX_WORK_BUFFER - INITIAL_DICTIONARY;
    for (index, entry) in entries.iter_mut().enumerate() {
        storage[initial_offset + index] = index as u8;
        *entry = (initial_offset + index, 1);
    }

    let mut output = Vec::new();
    let mut write_offset = 0usize;
    let mut previous = first;
    let mut current = INITIAL_DICTIONARY - 1;
    let mut code_offset = 1usize;

    loop {
        let next = codes
            .get(code_offset)
            .copied()
            .map(usize::from)
            .unwrap_or(0);
        if next > current {
            break;
        }

        let (source_offset, source_len) = entries.get(previous).copied().ok_or_else(|| {
            err(
                ImgErrorKind::IllegalData,
                "Q4 dictionary references an invalid previous code",
            )
        })?;
        let first_byte = if next == current {
            storage[source_offset]
        } else {
            let (next_offset, next_len) = entries.get(next).copied().ok_or_else(|| {
                err(
                    ImgErrorKind::IllegalData,
                    "Q4 dictionary references an invalid next code",
                )
            })?;
            if next_len == 0 {
                return Err(err(
                    ImgErrorKind::IllegalData,
                    "Q4 dictionary entry is empty",
                ));
            }
            storage[next_offset]
        };
        let new_len = source_len.checked_add(1).ok_or_else(|| {
            err(
                ImgErrorKind::InvalidParameter,
                "Q4 dictionary entry overflow",
            )
        })?;
        let new_end = write_offset.checked_add(new_len).ok_or_else(|| {
            err(
                ImgErrorKind::InvalidParameter,
                "Q4 dictionary storage overflow",
            )
        })?;
        if new_end > initial_offset {
            return Err(err(
                ImgErrorKind::InvalidParameter,
                "Q4 dictionary storage is exhausted",
            ));
        }
        storage.copy_within(source_offset..source_offset + source_len, write_offset);
        storage[write_offset + source_len] = first_byte;

        let output_end = output
            .len()
            .checked_add(source_len)
            .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "Q4 output size overflow"))?;
        if output_end > MAX_WORK_BUFFER {
            return Err(err(
                ImgErrorKind::InvalidParameter,
                "Q4 output block is too large",
            ));
        }
        output.extend_from_slice(&storage[source_offset..source_offset + source_len]);

        if current == entries.len() {
            entries.push((write_offset, new_len));
        } else if current < entries.len() {
            entries[current] = (write_offset, new_len);
        } else {
            return Err(err(
                ImgErrorKind::IllegalData,
                "Q4 dictionary index is not sequential",
            ));
        }
        write_offset = new_end;

        if code_offset >= codes.len() {
            break;
        }
        current = current.checked_add(1).ok_or_else(|| {
            err(
                ImgErrorKind::InvalidParameter,
                "Q4 dictionary index overflow",
            )
        })?;
        previous = next;
        code_offset += 1;
    }
    Ok(output)
}

fn decode_block(data: &[u8]) -> Result<Vec<u8>, Error> {
    let stage1 = decode_stage1(data)?;
    decode_stage2(&stage1)
}

fn scale_nibble(value: u8) -> Result<u8, Error> {
    if value > 0x0f {
        return Err(err(
            ImgErrorKind::IllegalData,
            "Q4 palette component is out of range",
        ));
    }
    Ok(value * 17)
}

fn decode_palette(data: &[u8]) -> Result<[[u8; 3]; 16], Error> {
    if data.len() < PALETTE_BYTES {
        return Err(err(ImgErrorKind::IllegalData, "Q4 palette is truncated"));
    }
    let mut palette = [[0u8; 3]; 16];
    for (index, color) in palette.iter_mut().enumerate() {
        let offset = index * 6;
        color[0] = scale_nibble(data[offset + 1])?;
        color[1] = scale_nibble(data[offset + 3])?;
        color[2] = scale_nibble(data[offset + 5])?;
    }
    Ok(palette)
}

fn decode_pixels(data: &[u8], expected: usize, output: &mut Vec<u8>) -> Result<(), Error> {
    let mut offset = 0usize;
    let block_start = output.len();
    while output.len() - block_start < expected {
        let token = *data
            .get(offset)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 pixel RLE is truncated"))?;
        offset += 1;
        let (color, count) = if token != 0x10 {
            if token >= 16 {
                return Err(err(
                    ImgErrorKind::IllegalData,
                    "Q4 palette index is invalid",
                ));
            }
            (PALETTE_ORDER[usize::from(token)], 1usize)
        } else {
            let repeat = *data
                .get(offset)
                .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 pixel RLE is truncated"))?;
            offset += 1;
            if repeat == 0 {
                let palette_index = *data
                    .get(offset)
                    .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 pixel RLE is truncated"))?;
                offset += 1;
                if palette_index >= 16 {
                    return Err(err(
                        ImgErrorKind::IllegalData,
                        "Q4 palette index is invalid",
                    ));
                }
                let high = *data
                    .get(offset)
                    .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 pixel RLE is truncated"))?;
                offset += 1;
                let low = *data
                    .get(offset)
                    .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 pixel RLE is truncated"))?;
                offset += 1;
                (
                    PALETTE_ORDER[usize::from(palette_index)],
                    usize::from(high) * 17 + usize::from(low),
                )
            } else {
                let low = *data
                    .get(offset)
                    .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 pixel RLE is truncated"))?;
                offset += 1;
                (0, usize::from(repeat) * 17 + usize::from(low))
            }
        };
        let remaining = expected - (output.len() - block_start);
        output.extend(std::iter::repeat_n(color, count.min(remaining)));
    }
    Ok(())
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let data = read_all(reader)?;
    if data.len() < HEADER_SIZE || &data[11..16] != b"MAJYO" {
        return Err(err(ImgErrorKind::IllegalData, "Not a Q4 image"));
    }
    if data[2] != 2 && (data[1] > 1 || data[3] > 1) {
        return Err(err(ImgErrorKind::IllegalData, "Invalid Q4 header"));
    }
    let declared_size = le24(&data, 8)?;
    if declared_size != 0 && declared_size > data.len() {
        return Err(err(ImgErrorKind::IllegalData, "Q4 file is truncated"));
    }

    let palette_compressed_len = usize::from(le16(&data, 16)?);
    let palette_end = HEADER_SIZE
        .checked_add(palette_compressed_len)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "Q4 palette offset overflow"))?;
    let palette_compressed = data
        .get(HEADER_SIZE..palette_end)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 palette block is truncated"))?;
    let palette = decode_palette(&decode_block(palette_compressed)?)?;

    let mut cursor = palette_end;
    let skip_count =
        usize::from((data[4] & 0x02 != 0) as u8) + usize::from((data[4] & 0x08 != 0) as u8);
    for _ in 0..skip_count {
        let header = data
            .get(cursor..cursor + BLOCK_HEADER_SIZE)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 auxiliary block is truncated"))?;
        let block_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
        cursor = cursor
            .checked_add(BLOCK_HEADER_SIZE)
            .and_then(|value| value.checked_add(block_len))
            .ok_or_else(|| {
                err(
                    ImgErrorKind::InvalidParameter,
                    "Q4 auxiliary block overflow",
                )
            })?;
        if cursor > data.len() {
            return Err(err(
                ImgErrorKind::IllegalData,
                "Q4 auxiliary block is truncated",
            ));
        }
    }

    let (expected_pixels, output_len) = output_layout()?;
    let mut indices = Vec::new();
    indices.try_reserve_exact(expected_pixels)?;
    while cursor < data.len() {
        let header = data
            .get(cursor..cursor + BLOCK_HEADER_SIZE)
            .ok_or_else(|| {
                err(
                    ImgErrorKind::IllegalData,
                    "Q4 image block header is truncated",
                )
            })?;
        let compressed_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
        let original_len = usize::from(u16::from_le_bytes([header[4], header[5]]));
        let payload_start = cursor + BLOCK_HEADER_SIZE;
        let payload_end = payload_start
            .checked_add(compressed_len)
            .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "Q4 image block overflow"))?;
        let payload = data
            .get(payload_start..payload_end)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "Q4 image block is truncated"))?;
        let expected_block_pixels = original_len.checked_mul(2).ok_or_else(|| {
            err(
                ImgErrorKind::InvalidParameter,
                "Q4 image block size overflow",
            )
        })?;
        if expected_block_pixels == 0 {
            return Err(err(ImgErrorKind::IllegalData, "Q4 image block is empty"));
        }
        let total_after_block = indices
            .len()
            .checked_add(expected_block_pixels)
            .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "Q4 image size overflow"))?;
        if total_after_block > expected_pixels {
            return Err(err(
                ImgErrorKind::IllegalData,
                "Q4 image blocks exceed the canvas",
            ));
        }
        decode_pixels(&decode_block(payload)?, expected_block_pixels, &mut indices)?;
        cursor = payload_end;
    }
    if indices.len() != expected_pixels {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::IllegalData,
            format!(
                "Q4 image blocks do not fill the canvas: {} of {} pixels at offset {}",
                indices.len(),
                expected_pixels,
                cursor
            ),
        )));
    }

    let mut output = Vec::new();
    output.try_reserve_exact(output_len)?;
    for index in indices {
        let color = palette[usize::from(index)];
        output.extend_from_slice(&[color[0], color[1], color[2], 255]);
    }
    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("Q4".to_string()))?;
    option
        .drawer
        .set_metadata("width", DataMap::UInt(WIDTH as u64))?;
    option
        .drawer
        .set_metadata("height", DataMap::UInt(HEIGHT as u64))?;
    draw_rgba(option, WIDTH, HEIGHT, &output)?;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q4_output_layout_respects_expanded_byte_limit() {
        let result = crate::limits::scope(
            crate::limits::DecodeLimits {
                expanded_bytes: WIDTH * HEIGHT * 4 - 1,
                ..crate::limits::DecodeLimits::unlimited()
            },
            output_layout,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("RGBA image"));
    }
}
