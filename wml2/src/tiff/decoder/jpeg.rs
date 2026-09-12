//! JPEG-compressed TIFF decoding helpers.

type Error = Box<dyn std::error::Error>;
use crate::draw::DecodeOptions;
use crate::draw::ImageBuffer;
use crate::draw::InitOptions;
use crate::error::{ImgError, ImgErrorKind};
use crate::tiff::block::blocks;
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
