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
        if count > remaining {
            return Err(err(
                ImgErrorKind::IllegalData,
                "Q4 pixel run exceeds the declared block size",
            ));
        }
        output.extend(std::iter::repeat_n(color, count));
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
    // The fixed canvas cannot fit smaller budgets; reject before allocating
    // palette/dictionary scratch for an image that cannot be returned.
    let (expected_pixels, output_len) = output_layout()?;

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

    fn decode_error(data: &[u8]) -> String {
        let mut reader = bin_rs::reader::BytesReader::new(data);
        let mut image = crate::draw::ImageBuffer::new();
        let mut options = DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        };
        let error = decode(&mut reader, &mut options).unwrap_err();
        assert!(image.buffer.is_none());
        error.to_string()
    }

    #[test]
    fn rejects_truncated_headers_and_declared_file_lengths() {
        let mut header = [0u8; HEADER_SIZE];
        header[2] = 2;
        header[11..16].copy_from_slice(b"MAJYO");
        for length in 0..HEADER_SIZE {
            assert!(decode_error(&header[..length]).contains("Not a Q4"));
        }
        header[8..11].copy_from_slice(&[0xff; 3]);
        assert!(decode_error(&header).contains("file is truncated"));
    }

    #[test]
    fn rejects_missing_auxiliary_payload_and_empty_image_blocks() {
        // Ninety-six literal zero codes produce a valid all-zero palette in
        // the existing stream model; this is a parser-boundary vector only.
        let palette: Vec<_> = [0x49, 0x24, 0x92].into_iter().cycle().take(36).collect();
        assert_eq!(decode_block(&palette).unwrap(), vec![0; PALETTE_BYTES]);
        let mut file = vec![0u8; HEADER_SIZE];
        file[2] = 2;
        file[11..16].copy_from_slice(b"MAJYO");
        file[16..18].copy_from_slice(&(palette.len() as u16).to_le_bytes());
        file.extend_from_slice(&palette);
        file[4] = 2;
        assert!(decode_error(&file).contains("auxiliary block is truncated"));
        file.extend_from_slice(&[0xff, 0xff, 0, 0, 0, 0]);
        assert!(decode_error(&file).contains("auxiliary block is truncated"));
        file[4] = 0;
        assert!(decode_error(&file).contains("image block is truncated"));
        file.truncate(HEADER_SIZE + palette.len());
        file.extend_from_slice(&[0; BLOCK_HEADER_SIZE]);
        assert!(decode_error(&file).contains("image block is empty"));
    }

    #[test]
    fn exact_runs_preserve_previous_blocks_and_allow_unused_tail() {
        let mut output = vec![9, 8];
        decode_pixels(&[0x10, 1, 0, 0xff], 17, &mut output).unwrap();
        assert_eq!(&output[..2], &[9, 8]);
        assert_eq!(&output[2..], &[0; 17]);
        let mut colored = Vec::new();
        decode_pixels(&[0x10, 0, 3, 1, 2], 19, &mut colored).unwrap();
        assert_eq!(colored, vec![PALETTE_ORDER[3]; 19]);
    }

    #[test]
    fn pixel_runs_accept_maximum_counts_and_zero_length_boundaries() {
        // Parser arithmetic vectors, not independently verified Q4 files:
        // both count encodings support the full high/low-byte range.
        let maximum = 255 * 17 + 255;
        for tokens in [&[0x10, 255, 255][..], &[0x10, 0, 3, 255, 255][..]] {
            let mut output = vec![9, 8];
            decode_pixels(tokens, maximum, &mut output).unwrap();
            assert_eq!(&output[..2], &[9, 8]);
            assert_eq!(output.len(), maximum + 2);
            assert!(decode_pixels(tokens, maximum - 1, &mut Vec::new()).is_err());
        }
        // Preserve existing helper behavior; the file parser still rejects
        // image blocks with zero declared size. Zero-run semantics are unverified.
        let mut output = vec![9];
        decode_pixels(&[0x10, 0, 3, 0, 0, 2], 1, &mut output).unwrap();
        assert_eq!(output, [9, PALETTE_ORDER[2]]);
        decode_pixels(&[], 0, &mut output).unwrap();
        assert_eq!(output, [9, PALETTE_ORDER[2]]);
    }

    #[test]
    fn rejects_truncated_and_invalid_pixel_tokens() {
        for bytes in [
            &[][..],
            &[0x10],
            &[0x10, 1],
            &[0x10, 0],
            &[0x10, 0, 3],
            &[0x10, 0, 3, 1],
            &[0x11],
            &[0x10, 0, 16, 1, 2],
        ] {
            assert!(
                decode_pixels(bytes, 19, &mut Vec::new()).is_err(),
                "{bytes:?}"
            );
        }
    }

    #[test]
    fn palette_components_and_size_are_bounded() {
        assert!(decode_palette(&[0; PALETTE_BYTES - 1]).is_err());
        let mut palette = [0; PALETTE_BYTES];
        palette[1] = 15;
        assert_eq!(decode_palette(&palette).unwrap()[0], [255, 0, 0]);
        palette[1] = 16;
        assert!(decode_palette(&palette).is_err());
    }

    #[test]
    fn stage1_width_controls_cross_bytes_without_emitting_codes() {
        // Hand-packed MSB stream: 010 | 001 | 1111 | 0001 | 10001 | 00000.
        // Width controls 001 and 0001 grow 3 -> 4 -> 5; values map to value - 2.
        assert_eq!(decode_stage1(&[0x47, 0xc6, 0x20]).unwrap(), [0, 13, 15]);
        // Explicit stop ignores subsequent bytes; synthetic zeros also stop a
        // short stream. This preserves the decoder's padding contract, whose
        // general format semantics remain independently unverified.
        assert_eq!(decode_stage1(&[0x40]).unwrap(), [0]);
        assert_eq!(decode_stage1(&[0x40, 0xff, 0xff]).unwrap(), [0]);
    }

    #[test]
    fn bit_reader_crosses_bytes_and_failed_reads_preserve_position() {
        let mut reader = BitReader::new(&[0xb3, 0x69]);
        assert_eq!(reader.read_bits(3), Some(0b101));
        assert_eq!(reader.read_bits(7), Some(0b1001101));
        assert_eq!(reader.read_bits(7), None);
        assert_eq!(reader.bit_offset, 10);
        assert_eq!(reader.read_bits(6), Some(0b101001));
        assert_eq!(reader.read_bits(1), None);
    }

    #[test]
    fn dictionary_equality_and_reuse_have_hand_derived_output() {
        // current starts at 17. Equality synthesizes previous + its first
        // byte: [2], then [2,2], then [2,2,2] are emitted in turn.
        assert_eq!(decode_stage2(&[2, 17, 18]).unwrap(), [2; 6]);
        // Literal 2 followed by 3 installs entry 17 = [2,3]. Reusing it
        // must retain both bytes even after subsequent dictionary writes.
        assert_eq!(decode_stage2(&[2, 3, 17]).unwrap(), [2, 3, 2, 3]);
        assert_eq!(decode_stage2(&[2, 3, 17, 18]).unwrap(), [2, 3, 2, 3, 3, 2]);
        // Missing next code currently flushes the final previous entry.
        assert_eq!(decode_stage2(&[2]).unwrap(), [2]);
        // An out-of-range next code currently terminates before emission.
        // These assertions describe existing behavior, not independent proof
        // that the general file format requires this termination convention.
        assert!(decode_stage2(&[2, 18]).unwrap().is_empty());
        assert_eq!(decode_stage2(&[2, 3, 19]).unwrap(), [2]);
    }

    #[test]
    fn dictionary_first_code_and_storage_are_bounded() {
        assert!(decode_stage2(&[]).is_err());
        assert!(decode_stage2(&[INITIAL_DICTIONARY as u16]).is_err());
        let result = decode_stage2(&vec![0; MAX_WORK_BUFFER]);
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("dictionary storage")
        );
    }

    #[test]
    fn code_count_and_bit_width_are_bounded() {
        // MSB-first 010 repeated: code zero, without increasing bit width.
        let bytes: Vec<_> = [0x49, 0x24, 0x92]
            .into_iter()
            .cycle()
            .take((MAX_WORK_BUFFER / 2 + 1).div_ceil(8) * 3)
            .collect();
        assert!(
            decode_stage1(&bytes)
                .unwrap_err()
                .to_string()
                .contains("stream is too large")
        );
        let mut bits = Vec::new();
        for width in 3..=16 {
            bits.extend(std::iter::repeat_n(0u8, width - 1));
            bits.push(1);
        }
        let mut bytes = vec![0u8; bits.len().div_ceil(8)];
        for (i, bit) in bits.into_iter().enumerate() {
            bytes[i / 8] |= bit << (7 - i % 8);
        }
        assert!(
            decode_stage1(&bytes)
                .unwrap_err()
                .to_string()
                .contains("width is unsupported")
        );
    }

    #[test]
    fn output_layout_checks_pixel_and_byte_thresholds() {
        for limits in [
            crate::limits::DecodeLimits {
                pixels: WIDTH * HEIGHT - 1,
                ..crate::limits::DecodeLimits::unlimited()
            },
            crate::limits::DecodeLimits {
                expanded_bytes: WIDTH * HEIGHT * 4 - 1,
                ..crate::limits::DecodeLimits::unlimited()
            },
        ] {
            assert!(crate::limits::scope(limits, output_layout).is_err());
        }
        assert_eq!(
            crate::limits::scope(
                crate::limits::DecodeLimits {
                    pixels: WIDTH * HEIGHT,
                    expanded_bytes: WIDTH * HEIGHT * 4,
                    ..crate::limits::DecodeLimits::unlimited()
                },
                output_layout
            )
            .unwrap(),
            (WIDTH * HEIGHT, WIDTH * HEIGHT * 4)
        );
    }

    #[test]
    fn rejects_run_longer_than_the_declared_block() {
        let mut output = Vec::new();
        let result = decode_pixels(&[0x10, 1, 0], 16, &mut output);
        assert!(
            result.is_err(),
            "a 17-pixel run must not be silently clipped"
        );
        assert!(output.is_empty());
    }

    #[test]
    fn rejects_impossible_output_before_palette_decompression() {
        use bin_rs::reader::BytesReader;
        let mut header = [0u8; HEADER_SIZE];
        header[2] = 2;
        header[11..16].copy_from_slice(b"MAJYO");
        let mut reader = BytesReader::new(&header);
        let mut image = crate::draw::ImageBuffer::new();
        let mut options = DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        };
        let result = crate::limits::scope(
            crate::limits::DecodeLimits {
                expanded_bytes: 0,
                ..crate::limits::DecodeLimits::unlimited()
            },
            || decode(&mut reader, &mut options),
        );
        assert!(result.unwrap_err().to_string().contains("RGBA image"));
        assert!(image.buffer.is_none());
    }

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
