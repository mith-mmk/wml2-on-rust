//! JPEG-compressed TIFF decoding helpers.

type Error = Box<dyn std::error::Error>;
use crate::draw::DecodeOptions;
use crate::draw::ImageBuffer;
use crate::draw::InitOptions;
use crate::error::{ImgError, ImgErrorKind};
use crate::tiff::block::{TiffBlock, TiffBlockKind, blocks};
use crate::tiff::header::*;
use crate::warning::ImgWarnings;
use bin_rs::reader::BinaryReader;

fn draw_jpeg(
    data: Vec<u8>,
    x: usize,
    y: usize,
    draw_width: usize,
    draw_height: usize,
    color_space: Option<&str>,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let mut image = ImageBuffer::new();
    let mut part_option = DecodeOptions {
        debug_flag: option.debug_flag,
        drawer: &mut image,
    };
    let mut reader = bin_rs::reader::BytesReader::from(data);
    let ws = crate::decode_guard::run(
        &mut reader,
        &mut part_option,
        crate::limits::current(),
        |reader, options| {
            crate::jpeg::decoder::decode_inner_with_color_space(reader, options, color_space)
        },
    )?;
    let width = image.width;
    let height = image.height;

    if let Some(buffer) = image.buffer.as_ref() {
        let width = draw_width.min(width);
        let height = draw_height.min(height);
        if image.width < draw_width || image.height < draw_height {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::DecodeError,
                "JPEG tile output is smaller than its TIFF block".to_string(),
            )));
        }
        let row_bytes = width.checked_mul(4).ok_or_else(|| {
            ImgError::new_const(
                ImgErrorKind::DecodeError,
                "JPEG tile row size overflows".to_string(),
            )
        })?;
        let source_row_bytes = image.width.checked_mul(4).ok_or_else(|| {
            ImgError::new_const(
                ImgErrorKind::DecodeError,
                "JPEG tile source row size overflows".to_string(),
            )
        })?;
        let required = image.height.checked_mul(source_row_bytes).ok_or_else(|| {
            ImgError::new_const(
                ImgErrorKind::DecodeError,
                "JPEG tile size overflows".to_string(),
            )
        })?;
        if buffer.len() < required {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::DecodeError,
                "JPEG tile output is truncated".to_string(),
            )));
        }
        if width == image.width && height == image.height {
            option.drawer.draw(x, y, width, height, buffer, None)?;
        } else {
            let mut clipped = Vec::with_capacity(height * row_bytes);
            for row in 0..height {
                let start = row * source_row_bytes;
                clipped.extend_from_slice(&buffer[start..start + row_bytes]);
            }
            option.drawer.draw(x, y, width, height, &clipped, None)?;
        }
    }

    Ok(ws)
}

fn old_jpeg_error(message: impl Into<String>) -> Error {
    std::io::Error::other(message.into()).into()
}

fn append_old_jpeg_payload(output: &mut Vec<u8>, payload: &[u8]) -> Result<(), Error> {
    let new_len = output
        .len()
        .checked_add(payload.len())
        .ok_or_else(|| old_jpeg_error("old-style JPEG assembly size overflows"))?;
    crate::limits::check(
        new_len,
        crate::limits::current().expanded_bytes,
        "old-style JPEG assembly",
    )?;
    output.try_reserve(payload.len())?;
    output.extend_from_slice(payload);
    Ok(())
}

fn read_at<B: BinaryReader>(
    reader: &mut B,
    offset: u64,
    length: usize,
    input_len: u64,
) -> Result<Vec<u8>, Error> {
    let length_u64 = u64::try_from(length)?;
    let end = offset
        .checked_add(length_u64)
        .ok_or_else(|| old_jpeg_error("old-style JPEG table range overflows"))?;
    if end > input_len {
        return Err(old_jpeg_error(format!(
            "old-style JPEG table range is outside the TIFF: offset {offset}, length {length}, input {input_len}"
        )));
    }
    reader.seek(std::io::SeekFrom::Start(offset))?;
    let data = reader.read_bytes_as_vec(length)?;
    if data.len() != length {
        return Err(old_jpeg_error("old-style JPEG table is truncated"));
    }
    Ok(data)
}

fn append_marker(output: &mut Vec<u8>, code: u8, payload: &[u8]) -> Result<(), Error> {
    let length = payload
        .len()
        .checked_add(2)
        .ok_or_else(|| old_jpeg_error("old-style JPEG marker length overflows"))?;
    let length = u16::try_from(length)
        .map_err(|_| old_jpeg_error("old-style JPEG marker payload is too large"))?;
    append_old_jpeg_payload(output, &[0xff, code])?;
    append_old_jpeg_payload(output, &length.to_be_bytes())?;
    append_old_jpeg_payload(output, payload)?;
    Ok(())
}

fn read_huffman_table<B: BinaryReader>(
    reader: &mut B,
    offset: u64,
    input_len: u64,
) -> Result<Vec<u8>, Error> {
    let counts = read_at(reader, offset, 16, input_len)?;
    let symbols = counts
        .iter()
        .try_fold(0usize, |sum, value| sum.checked_add(usize::from(*value)))
        .ok_or_else(|| old_jpeg_error("old-style JPEG Huffman table size overflows"))?;
    let mut table = counts;
    table.extend_from_slice(&read_at(
        reader,
        offset
            .checked_add(16)
            .ok_or_else(|| old_jpeg_error("old-style JPEG table offset overflows"))?,
        symbols,
        input_len,
    )?);
    Ok(table)
}

fn old_jpeg_header<B: BinaryReader>(
    reader: &mut B,
    header: &Tiff,
    width: usize,
    height: usize,
    input_len: u64,
) -> Result<Vec<u8>, Error> {
    let width = u16::try_from(width)
        .map_err(|_| old_jpeg_error("old-style JPEG block width exceeds baseline JPEG range"))?;
    let height = u16::try_from(height)
        .map_err(|_| old_jpeg_error("old-style JPEG block height exceeds baseline JPEG range"))?;
    if header.jpeg_q_tables.len() != 3
        || header.jpeg_dc_tables.len() != 3
        || header.jpeg_ac_tables.len() != 3
    {
        return Err(old_jpeg_error(
            "old-style JPEG is missing one or more Q/DC/AC table offsets",
        ));
    }
    let [horizontal, vertical] = header
        .ycbcr_sub_sampling
        .as_slice()
        .try_into()
        .map_err(|_| old_jpeg_error("old-style JPEG has invalid YCbCrSubSampling"))?;
    if !matches!(horizontal, 1 | 2 | 4) || !matches!(vertical, 1 | 2 | 4) || vertical > horizontal {
        return Err(old_jpeg_error(
            "old-style JPEG has unsupported YCbCrSubSampling",
        ));
    }
    let mut output = Vec::new();
    append_old_jpeg_payload(&mut output, &[0xff, 0xd8])?;
    for (id, offset) in header.jpeg_q_tables.iter().enumerate() {
        let table = read_at(reader, *offset, 64, input_len)?;
        let mut payload = Vec::with_capacity(65);
        payload.push(u8::try_from(id)?);
        payload.extend_from_slice(&table);
        append_marker(&mut output, 0xdb, &payload)?;
    }
    for (id, offset) in header.jpeg_dc_tables.iter().enumerate() {
        let table = read_huffman_table(reader, *offset, input_len)?;
        let mut payload = Vec::with_capacity(table.len() + 1);
        payload.push(u8::try_from(id)?);
        payload.extend_from_slice(&table);
        append_marker(&mut output, 0xc4, &payload)?;
    }
    for (id, offset) in header.jpeg_ac_tables.iter().enumerate() {
        let table = read_huffman_table(reader, *offset, input_len)?;
        let mut payload = Vec::with_capacity(table.len() + 1);
        payload.push(0x10 | u8::try_from(id)?);
        payload.extend_from_slice(&table);
        append_marker(&mut output, 0xc4, &payload)?;
    }
    let frame = [
        8,
        (height >> 8) as u8,
        height as u8,
        (width >> 8) as u8,
        width as u8,
        3,
        1,
        u8::try_from((horizontal << 4) | vertical)?,
        0,
        2,
        0x11,
        1,
        3,
        0x11,
        2,
    ];
    append_marker(&mut output, 0xc0, &frame)?;
    if header.jpeg_restart_interval != 0 {
        append_marker(
            &mut output,
            0xdd,
            &header.jpeg_restart_interval.to_be_bytes(),
        )?;
    }
    let scan = [3, 1, 0x00, 2, 0x11, 3, 0x22, 0, 63, 0];
    append_marker(&mut output, 0xda, &scan)?;
    Ok(output)
}

fn read_block_payload<B: BinaryReader>(
    reader: &mut B,
    block: &TiffBlock,
    input_len: u64,
) -> Result<Vec<u8>, Error> {
    block.validate_range(input_len)?;
    read_at(
        reader,
        block.offset,
        usize::try_from(block.compressed_len)?,
        input_len,
    )
}

fn assemble_interchange<B: BinaryReader>(
    reader: &mut B,
    header: &Tiff,
    blocks: &[TiffBlock],
    input_len: u64,
) -> Result<Vec<u8>, Error> {
    let offset = header
        .jpeg_interchange_format
        .ok_or_else(|| old_jpeg_error("old-style JPEG interchange offset is missing"))?;
    let length = header
        .jpeg_interchange_format_length
        .ok_or_else(|| old_jpeg_error("old-style JPEG interchange length is missing"))?;
    let length = usize::try_from(length)
        .map_err(|_| old_jpeg_error("old-style JPEG interchange length is too large"))?;
    crate::limits::check(
        length,
        crate::limits::current().expanded_bytes,
        "old-style JPEG assembly",
    )?;
    let mut output = read_at(reader, offset, length, input_len)?;
    if !output.starts_with(&[0xff, 0xd8]) {
        return Err(old_jpeg_error(
            "old-style JPEG interchange data must begin with SOI",
        ));
    }
    if output.ends_with(&[0xff, 0xd9]) {
        output.truncate(output.len() - 2);
    }
    for (index, block) in blocks.iter().enumerate() {
        let payload = read_block_payload(reader, block, input_len)?;
        if index == 0 {
            if !payload.starts_with(&[0xff, 0xda]) {
                return Err(old_jpeg_error(
                    "old-style JPEG first strip must begin with a scan marker",
                ));
            }
        } else if payload.starts_with(&[0xff, 0xda]) {
            return Err(old_jpeg_error(
                "old-style JPEG continuation strip unexpectedly contains a scan marker",
            ));
        }
        append_old_jpeg_payload(&mut output, &payload)?;
    }
    if !output.ends_with(&[0xff, 0xd9]) {
        append_old_jpeg_payload(&mut output, &[0xff, 0xd9])?;
    }
    Ok(output)
}

fn assemble_tables<B: BinaryReader>(
    reader: &mut B,
    header: &Tiff,
    blocks: &[TiffBlock],
    width: usize,
    height: usize,
    input_len: u64,
) -> Result<Vec<u8>, Error> {
    let mut output = old_jpeg_header(reader, header, width, height, input_len)?;
    for block in blocks {
        let payload = read_block_payload(reader, block, input_len)?;
        if payload.starts_with(&[0xff, 0xd8]) || payload.starts_with(&[0xff, 0xda]) {
            return Err(old_jpeg_error(
                "old-style JPEG table form payload must contain entropy data only",
            ));
        }
        append_old_jpeg_payload(&mut output, &payload)?;
    }
    append_old_jpeg_payload(&mut output, &[0xff, 0xd9])?;
    Ok(output)
}

fn validate_old_jpeg_header(header: &Tiff) -> Result<(), Error> {
    if header.jpeg_proc != 1 {
        return Err(old_jpeg_error(format!(
            "unsupported old-style JPEGProc {}",
            header.jpeg_proc
        )));
    }
    if header.photometric_interpretation != 6
        || header.samples_per_pixel != 3
        || header.bitspersamples.as_slice() != [8, 8, 8]
        || header.planar_config != 1
    {
        return Err(old_jpeg_error(
            "old-style JPEG requires contiguous 8-bit three-component YCbCr",
        ));
    }
    if header.jpeg_interchange_format.is_some() && header.jpeg_interchange_format_length.is_none() {
        return Err(old_jpeg_error(
            "old-style JPEG interchange length is missing",
        ));
    }
    if header.jpeg_interchange_format.is_none() && header.jpeg_interchange_format_length.is_some() {
        return Err(old_jpeg_error(
            "old-style JPEG interchange offset is missing",
        ));
    }
    Ok(())
}

pub fn decode_old_jpeg_compresson<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    validate_old_jpeg_header(header)?;
    if initialize {
        let init = if animation {
            Some(InitOptions {
                loop_count: 1,
                background: None,
                animation: true,
            })
        } else {
            None
        };
        option
            .drawer
            .init(header.width as usize, header.height as usize, init)?;
    }
    let input_len = reader.seek(std::io::SeekFrom::End(0))?;
    let block_list = blocks(header)?;
    if header.jpeg_interchange_format.is_some() {
        let data = assemble_interchange(reader, header, &block_list, input_len)?;
        let warning = draw_jpeg(
            data,
            0,
            0,
            header.width as usize,
            header.height as usize,
            Some("YUV"),
            option,
        )?;
        return Ok(warning);
    }
    if block_list
        .iter()
        .any(|block| block.kind == TiffBlockKind::Tile)
    {
        let mut warnings = None;
        for block in block_list {
            let data = assemble_tables(
                reader,
                header,
                std::slice::from_ref(&block),
                block.stored_width,
                block.stored_height,
                input_len,
            )?;
            let warning = draw_jpeg(
                data,
                block.x,
                block.y,
                block.draw_width,
                block.draw_height,
                Some("YUV"),
                option,
            )?;
            warnings = ImgWarnings::append(warnings, warning);
        }
        return Ok(warnings);
    }
    let data = assemble_tables(
        reader,
        header,
        &block_list,
        header.width as usize,
        header.height as usize,
        input_len,
    )?;
    let warning = draw_jpeg(
        data,
        0,
        0,
        header.width as usize,
        header.height as usize,
        Some("YUV"),
        option,
    )?;
    Ok(warning)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_jpeg_assembly_append_respects_expanded_byte_limit() {
        let result = crate::limits::scope(
            crate::limits::DecodeLimits {
                expanded_bytes: 4,
                ..crate::limits::DecodeLimits::unlimited()
            },
            || {
                let mut output = vec![0u8; 4];
                append_old_jpeg_payload(&mut output, &[0])
            },
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("old-style JPEG assembly")
        );
    }
}

// Tiff in JPEG is a multi parts image.
pub fn decode_jpeg_compresson<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    let jpeg_tables = &header.jpeg_tables;
    let metadata;
    if jpeg_tables.is_empty() {
        metadata = vec![0xff, 0xd8]; // SOI
    } else if !jpeg_tables.starts_with(&[0xff, 0xd8]) || !jpeg_tables.ends_with(&[0xff, 0xd9]) {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::DecodeError,
            "JPEG tables must begin with SOI and end with EOI".to_string(),
        )));
    } else {
        let len = jpeg_tables.len() - 2;
        metadata = jpeg_tables[..len].to_vec(); // remove EOI
    }
    let mut warnings: Option<ImgWarnings> = None;
    if initialize {
        let init = if animation {
            Some(InitOptions {
                loop_count: 1,
                background: None,
                animation: true,
            })
        } else {
            None
        };
        option
            .drawer
            .init(header.width as usize, header.height as usize, init)?;
    }

    let input_len = reader.seek(std::io::SeekFrom::End(0))?;
    let block_list = blocks(header)?;
    // TIFF Technical Note 2 permits arbitrary JPEG component IDs. The TIFF
    // photometric tag, rather than JFIF/Adobe markers or IDs, defines the
    // stored three-component color space.
    let color_space = match header.photometric_interpretation {
        2 => Some("RGB"),
        6 => Some("YUV"),
        _ => None,
    };
    for block in block_list {
        block.validate_range(input_len)?;
        reader.seek(std::io::SeekFrom::Start(block.offset))?;
        let mut data = vec![];
        data.append(&mut metadata.to_vec());
        let count = usize::try_from(block.compressed_len)?;
        let buf = reader.read_bytes_as_vec(count)?;
        if !buf.starts_with(&[0xff, 0xd8]) || !buf.ends_with(&[0xff, 0xd9]) {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::DecodeError,
                "JPEG tile payload must begin with SOI and end with EOI".to_string(),
            )));
        }
        data.append(&mut buf[2..].to_vec()); // remove SOI

        let ws = draw_jpeg(
            data,
            block.x,
            block.y,
            block.draw_width,
            block.draw_height,
            color_space,
            option,
        )?;
        warnings = ImgWarnings::append(warnings, ws);
    }
    Ok(warnings)
}
