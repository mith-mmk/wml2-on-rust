//! TIFF decoder implementation.

type Error = Box<dyn std::error::Error>;
#[cfg(feature = "tiff-jpeg")]
use self::jpeg::{decode_jpeg_compresson, decode_old_jpeg_compresson};
use crate::color::RGBA;
use crate::draw::*;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use crate::metadata::DataMap;
use crate::tiff::header::*;
use crate::tiff::warning::TiffWarning;
use crate::warning::ImgWarnings;
use bin_rs::io::read_u16;
use bin_rs::io::read_u32;
use bin_rs::reader::BinaryReader;
mod ccitt;
#[cfg(feature = "tiff-jpeg")]
mod jpeg;
mod packbits;
mod ycbcr;

use self::compression::decompress_block;
use crate::tiff::block::{TiffBlock, blocks};
use crate::tiff::page::is_display_page;
mod compression;

fn create_pallet(bits: usize, is_black_zero: bool) -> Vec<RGBA> {
    let color_max = 1 << bits;
    let mut pallet = Vec::with_capacity(color_max);

    if is_black_zero {
        for i in 0..color_max {
            let gray = (i * 255 / (color_max - 1)) as u8;
            pallet.push(RGBA {
                red: gray,
                green: gray,
                blue: gray,
                alpha: 0xff,
            });
        }
    } else {
        for i in 0..color_max {
            let gray = 255 - ((i * 255 / (color_max - 1)) as u8);
            pallet.push(RGBA {
                red: gray,
                green: gray,
                blue: gray,
                alpha: 0xff,
            });
        }
    }
    pallet
}

fn planar_to_chuncky(data: &[u8], header: &Tiff) -> Result<Vec<u8>, Error> {
    let channels = usize::from(header.samples_per_pixel);
    if channels <= 1 || header.bitspersamples.len() != channels {
        return Ok(data.to_vec());
    }
    let bits = *header
        .bitspersamples
        .first()
        .ok_or_else(|| std::io::Error::other("TIFF has no BitsPerSample"))?;
    if bits < 8
        || header
            .bitspersamples
            .iter()
            .any(|value| *value != bits || *value % 8 != 0)
    {
        return Err(
            std::io::Error::other("planar TIFF requires uniform byte-aligned samples").into(),
        );
    }
    let sample_bytes = usize::from(bits / 8);
    let pixels = (header.width as usize)
        .checked_mul(header.height as usize)
        .ok_or_else(|| std::io::Error::other("TIFF planar pixel count overflows"))?;
    let plane_len = pixels
        .checked_mul(sample_bytes)
        .ok_or_else(|| std::io::Error::other("TIFF planar size overflows"))?;
    let total = plane_len
        .checked_mul(channels)
        .ok_or_else(|| std::io::Error::other("TIFF planar size overflows"))?;
    if data.len() < total {
        return Err(std::io::Error::other("TIFF planar data is truncated").into());
    }
    let mut output = vec![0u8; total];
    for pixel in 0..pixels {
        for plane in 0..channels {
            let src = plane * plane_len + pixel * sample_bytes;
            let dst = (pixel * channels + plane) * sample_bytes;
            output[dst..dst + sample_bytes].copy_from_slice(&data[src..src + sample_bytes]);
        }
    }
    Ok(output)
}

fn has_bytes(data: &[u8], offset: usize, count: usize) -> bool {
    offset
        .checked_add(count)
        .map(|end| end <= data.len())
        .unwrap_or(false)
}

fn block_row_bytes(block: &TiffBlock, header: &Tiff) -> Result<usize, Error> {
    let bits: usize = if header.planar_config == 2 {
        usize::from(
            *header
                .bitspersamples
                .get(block.plane)
                .ok_or_else(|| std::io::Error::other("TIFF plane has no BitsPerSample"))?,
        )
    } else {
        header
            .bitspersamples
            .iter()
            .try_fold(0usize, |sum, bits| sum.checked_add(usize::from(*bits)))
            .ok_or_else(|| std::io::Error::other("TIFF row bit count overflows"))?
    };
    block
        .stored_width
        .checked_mul(bits)
        .and_then(|bits| bits.checked_add(7))
        .map(|bits| bits / 8)
        .ok_or_else(|| std::io::Error::other("TIFF block row size overflows").into())
}

fn block_expected_bytes(block: &TiffBlock, header: &Tiff) -> Result<usize, Error> {
    block_row_bytes(block, header)?
        .checked_mul(block.stored_height)
        .ok_or_else(|| std::io::Error::other("TIFF block size overflows").into())
}

fn read_block<B: BinaryReader + ?Sized>(
    reader: &mut B,
    block: &TiffBlock,
) -> Result<Vec<u8>, Error> {
    reader.seek(std::io::SeekFrom::Start(block.offset))?;
    let len = usize::try_from(block.compressed_len)
        .map_err(|_| std::io::Error::other("TIFF block length does not fit usize"))?;
    if len == 0 {
        return Err(std::io::Error::other("TIFF block has zero compressed length").into());
    }
    let data = reader.read_bytes_as_vec(len)?;
    if data.len() != len {
        return Err(std::io::Error::other("TIFF block is truncated").into());
    }
    Ok(data)
}

/// Read the first TIFF page into interleaved native unsigned 16-bit samples.
/// This is used by the optional high-resolution adapter and deliberately
/// shares the block, compression, and Predictor implementations with RGBA8.
#[cfg(feature = "high-bit-depth")]
pub(crate) fn decode_samples_u16(
    reader: &mut dyn BinaryReader,
    page: &Tiff,
) -> Result<Vec<u16>, Error> {
    let channels = usize::from(page.samples_per_pixel);
    if channels == 0 || page.bitspersamples.len() != channels {
        return Err(std::io::Error::other("TIFF BitsPerSample/SamplesPerPixel mismatch").into());
    }
    if page.bitspersamples.iter().any(|bits| *bits != 16) {
        return Err(std::io::Error::other("native TIFF output requires 16-bit samples").into());
    }
    let width = usize::try_from(page.width)?;
    let height = usize::try_from(page.height)?;
    let sample_bytes = crate::tiff::sample::sample_bytes(16)?;
    let output_len = width
        .checked_mul(height)
        .and_then(|v| v.checked_mul(channels))
        .and_then(|v| v.checked_mul(sample_bytes))
        .ok_or_else(|| std::io::Error::other("TIFF native sample size overflows"))?;
    crate::limits::check(
        output_len,
        crate::limits::current().expanded_bytes,
        "expanded image",
    )?;
    let mut output = vec![0u8; output_len];
    let block_list = blocks(page)?;
    let input_len = reader.seek(std::io::SeekFrom::End(0))?;
    for block in &block_list {
        block.validate_range(input_len)?;
        let expected = block_expected_bytes(block, page)?;
        crate::limits::check(
            expected,
            crate::limits::current().expanded_bytes,
            "expanded TIFF block",
        )?;
        let compressed = read_block(reader, block)?;
        let mut data = decompress_block(&page.compression, &compressed, Some(expected))?;
        if page.predictor == 2 {
            crate::tiff::predictor::apply_predictor(
                &mut data,
                block_row_bytes(block, page)?,
                block.stored_height,
                16,
                if page.planar_config == 2 { 1 } else { channels },
                page.tiff_headers.endian,
            )?;
        }
        let source_row = if page.planar_config == 2 {
            block.stored_width * 2
        } else {
            block.stored_width * channels * 2
        };
        for row in 0..block.draw_height {
            for col in 0..block.draw_width {
                let src = row * source_row
                    + col
                        * if page.planar_config == 2 {
                            2
                        } else {
                            channels * 2
                        };
                let dst = ((block.y + row) * width + block.x + col) * channels * 2
                    + if page.planar_config == 2 {
                        block.plane * 2
                    } else {
                        0
                    };
                let src_end = src
                    .checked_add(2)
                    .ok_or_else(|| std::io::Error::other("TIFF native source offset overflows"))?;
                let dst_end = dst.checked_add(2).ok_or_else(|| {
                    std::io::Error::other("TIFF native destination offset overflows")
                })?;
                if src_end > data.len() || dst_end > output.len() {
                    return Err(std::io::Error::other("TIFF native block is truncated").into());
                }
                if page.planar_config == 2 {
                    output[dst..dst_end].copy_from_slice(&data[src..src_end]);
                } else {
                    let full_dst = ((block.y + row) * width + block.x + col) * channels * 2;
                    let full_end = full_dst + channels * 2;
                    if full_end > output.len() {
                        return Err(
                            std::io::Error::other("TIFF native destination overflows").into()
                        );
                    }
                    output[full_dst..full_end].copy_from_slice(&data[src..src + channels * 2]);
                }
            }
        }
    }
    crate::tiff::sample::decode_samples_u16(
        &output,
        width,
        height,
        &page.bitspersamples,
        channels,
        1,
        page.tiff_headers.endian,
        1,
    )
}

fn decode_blocked<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    if initialize {
        init_canvas(option, header, animation)?;
    }
    if header.photometric_interpretation == 6 {
        ycbcr::decode(reader, option, header, false, animation)?;
        return Ok(None);
    }
    let block_list = blocks(header)?;
    let input_len = reader.seek(std::io::SeekFrom::End(0))?;
    for block in &block_list {
        block.validate_range(input_len)?;
    }

    // A planar page is assembled by sample, so a high-bit-depth plane cannot
    // accidentally be interleaved by byte offset. This path is intentionally
    // byte-aligned; packed planar samples remain rejected as malformed by the
    // common block contract rather than being silently corrupted.
    if header.planar_config == 2 && header.samples_per_pixel > 1 {
        let bits = *header
            .bitspersamples
            .first()
            .ok_or_else(|| std::io::Error::other("TIFF has no BitsPerSample"))?;
        if bits < 8 || header.bitspersamples.iter().any(|value| *value != bits) {
            return Err(
                std::io::Error::other("planar TIFF requires uniform byte-aligned samples").into(),
            );
        }
        let sample_bytes = usize::from(bits / 8);
        let channels = usize::from(header.samples_per_pixel);
        if block_list.len() % channels != 0 {
            return Err(std::io::Error::other(
                "TIFF planar block count is not divisible by plane count",
            )
            .into());
        }
        let blocks_per_plane = block_list.len() / channels;
        for local in 0..blocks_per_plane {
            let first = block_list[local];
            let tile_len = first
                .stored_width
                .checked_mul(first.stored_height)
                .and_then(|v| v.checked_mul(channels))
                .and_then(|v| v.checked_mul(sample_bytes))
                .ok_or_else(|| std::io::Error::other("TIFF planar tile size overflows"))?;
            crate::limits::check(
                tile_len,
                crate::limits::current().expanded_bytes,
                "expanded TIFF block",
            )?;
            let mut tile = vec![0u8; tile_len];
            for plane in 0..channels {
                let block = block_list[plane * blocks_per_plane + local];
                if block.x != first.x
                    || block.y != first.y
                    || block.stored_width != first.stored_width
                    || block.stored_height != first.stored_height
                {
                    return Err(std::io::Error::other(
                        "TIFF planar block geometry differs between planes",
                    )
                    .into());
                }
                let expected = block_expected_bytes(&block, header)?;
                crate::limits::check(
                    expected,
                    crate::limits::current().expanded_bytes,
                    "expanded TIFF block",
                )?;
                let compressed = read_block(reader, &block)?;
                let mut data = decompress_block(&header.compression, &compressed, Some(expected))?;
                if header.predictor == 2 {
                    crate::tiff::predictor::apply_predictor(
                        &mut data,
                        block_row_bytes(&block, header)?,
                        block.stored_height,
                        bits,
                        1,
                        header.tiff_headers.endian,
                    )?;
                }
                let source_row_bytes = block.stored_width * sample_bytes;
                for row in 0..block.draw_height {
                    for col in 0..block.draw_width {
                        let src = row * source_row_bytes + col * sample_bytes;
                        let dst = (row * first.stored_width * channels + col * channels + plane)
                            * sample_bytes;
                        if src + sample_bytes > data.len() || dst + sample_bytes > tile.len() {
                            return Err(
                                std::io::Error::other("TIFF planar block is truncated").into()
                            );
                        }
                        tile[dst..dst + sample_bytes]
                            .copy_from_slice(&data[src..src + sample_bytes]);
                    }
                }
            }
            draw_tile_internal(
                &tile,
                first.y,
                first.draw_height,
                first.x,
                first.draw_width,
                option,
                header,
                true,
                Some(first.stored_width),
                Some(first.stored_height),
            )?;
        }
        return Ok(None);
    }

    for block in &block_list {
        let expected = block_expected_bytes(block, header)?;
        crate::limits::check(
            expected,
            crate::limits::current().expanded_bytes,
            "expanded TIFF block",
        )?;
        let compressed = read_block(reader, block)?;
        let mut data = decompress_block(&header.compression, &compressed, Some(expected))?;
        if header.predictor == 2 {
            crate::tiff::predictor::apply_predictor(
                &mut data,
                block_row_bytes(block, header)?,
                block.stored_height,
                *header
                    .bitspersamples
                    .first()
                    .ok_or_else(|| std::io::Error::other("TIFF has no BitsPerSample"))?,
                usize::from(header.samples_per_pixel),
                header.tiff_headers.endian,
            )?;
        }
        draw_tile_internal(
            &data,
            block.y,
            block.draw_height,
            block.x,
            block.draw_width,
            option,
            header,
            true,
            Some(block.stored_width),
            Some(block.stored_height),
        )?;
    }
    Ok(None)
}

pub fn draw_strip(
    data: &[u8],
    y: usize,
    strip: usize,
    option: &mut DecodeOptions,
    header: &Tiff,
) -> Result<Option<ImgWarnings>, Error> {
    draw_tile(data, y, strip, 0, header.width as usize, option, header)
}

pub fn draw_tile(
    data: &[u8],
    y: usize,
    strip: usize,
    x: usize,
    width: usize,
    option: &mut DecodeOptions,
    header: &Tiff,
) -> Result<Option<ImgWarnings>, Error> {
    draw_tile_internal(data, y, strip, x, width, option, header, false, None, None)
}

fn draw_tile_internal(
    data: &[u8],
    y: usize,
    strip: usize,
    x: usize,
    width: usize,
    option: &mut DecodeOptions,
    header: &Tiff,
    prepared: bool,
    stored_width: Option<usize>,
    stored_height: Option<usize>,
) -> Result<Option<ImgWarnings>, Error> {
    crate::tiff::page::validate_alpha_support(header)?;
    if data.is_empty() {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::DecodeError,
            "Data empty.".to_string(),
        )));
    }

    // Prepared blocks are already decoded and interleaved. Borrow their bytes;
    // only the public raw drawing path needs ownership for predictor/planar work.
    let mut data = std::borrow::Cow::Borrowed(data);
    let mut predictor = if prepared { 1 } else { header.predictor };
    if predictor == 2 {
        let channels = if header.planar_config == 2 {
            1
        } else {
            usize::from(header.samples_per_pixel)
        };
        let bits = *header
            .bitspersamples
            .first()
            .ok_or_else(|| std::io::Error::other("TIFF has no BitsPerSample"))?;
        let row_bits = if header.planar_config == 2 {
            usize::try_from(header.width)?.checked_mul(usize::from(bits))
        } else {
            usize::try_from(header.width)?.checked_mul(
                header
                    .bitspersamples
                    .iter()
                    .map(|value| usize::from(*value))
                    .sum(),
            )
        };
        let row_bytes = row_bits
            .and_then(|value| value.checked_add(7))
            .map(|value| value / 8)
            .ok_or_else(|| std::io::Error::other("TIFF predictor row size overflows"))?;
        let predictor_rows = usize::try_from(header.height)?
            .checked_mul(if header.planar_config == 2 {
                usize::from(header.samples_per_pixel)
            } else {
                1
            })
            .ok_or_else(|| std::io::Error::other("TIFF predictor plane rows overflow"))?;
        crate::tiff::predictor::apply_predictor(
            data.to_mut(),
            row_bytes,
            predictor_rows,
            bits,
            channels,
            header.tiff_headers.endian,
        )?;
        predictor = 1;
    }
    let stored_width = stored_width.unwrap_or(usize::try_from(header.width)?);
    let stored_height = stored_height.unwrap_or(usize::try_from(header.height)?);

    // no debug
    if !prepared && header.planar_config == 2 && header.samples_per_pixel > 1 {
        data = std::borrow::Cow::Owned(planar_to_chuncky(&data, header)?);
    }

    let color_table = if let Some(color_table) = header.color_table.as_ref() {
        Some(std::borrow::Cow::Borrowed(color_table.as_slice()))
    } else {
        let bitspersample = if header.bitspersample >= 8 {
            8
        } else {
            header.bitspersample
        };
        match header.photometric_interpretation {
            0 if header.bitspersample <= 8 && header.samples_per_pixel == 1 => {
                // WhiteIsZero
                Some(std::borrow::Cow::Owned(create_pallet(
                    bitspersample as usize,
                    false,
                )))
            }
            1 if header.bitspersample <= 8 && header.samples_per_pixel == 1 => {
                // BlackIsZero
                Some(std::borrow::Cow::Owned(create_pallet(
                    bitspersample as usize,
                    true,
                )))
            }
            0 | 1 => {
                // High-bit-depth grayscale is normalized from its sample
                // value below; it is not an indexed color table.
                None
            }
            2 => {
                if header.samples_per_pixel < 3 {
                    return Err(Box::new(ImgError::new_const(
                        ImgErrorKind::DecodeError,
                        "RGB image needs Sample per pixel >=3.".to_string(),
                    )));
                } else {
                    None
                }
            }
            3 => {
                // RGB Palette
                Some(std::borrow::Cow::Owned(create_pallet(
                    bitspersample as usize,
                    true,
                )))
            }
            5 => {
                if header.samples_per_pixel < 4 {
                    return Err(Box::new(ImgError::new_const(
                        ImgErrorKind::DecodeError,
                        "YMCK image needs Sample per pixel >=4.".to_string(),
                    )));
                } else {
                    None
                }
            }
            6 => {
                if header.samples_per_pixel < 3 {
                    return Err(Box::new(ImgError::new_const(
                        ImgErrorKind::DecodeError,
                        "YCbCr image needs Sample per pixel >=3.".to_string(),
                    )));
                } else {
                    None
                }
            }
            _ => {
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::DecodeError,
                    "Not Supprt Color model.".to_string(),
                )));
            }
        }
    };
    if header.bitspersample <= 8 && color_table.is_none() {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::DecodeError,
            "This is an index color image,but A color table is empty.".to_string(),
        )));
    }
    let palette = match header.photometric_interpretation {
        0 | 1 if header.bitspersample <= 8 && header.samples_per_pixel == 1 => {
            let palette = color_table.as_deref().ok_or_else(|| {
                Box::new(ImgError::new_const(
                    ImgErrorKind::DecodeError,
                    "This is an index color image,but A color table is empty.".to_string(),
                )) as Error
            })?;
            let required_len = 1usize << header.bitspersample as usize;
            if palette.len() < required_len {
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::DecodeError,
                    format!(
                        "Color table is too short. expected at least {} entries, got {}",
                        required_len,
                        palette.len()
                    ),
                )));
            }
            palette
        }
        3 => {
            let palette = color_table.as_deref().ok_or_else(|| {
                Box::new(ImgError::new_const(
                    ImgErrorKind::DecodeError,
                    "This is an index color image,but A color table is empty.".to_string(),
                )) as Error
            })?;
            let index_bits = header
                .bitspersamples
                .first()
                .ok_or_else(|| std::io::Error::other("TIFF palette has no index BitsPerSample"))?;
            let required_len = 1usize
                .checked_shl(u32::from(*index_bits))
                .ok_or_else(|| std::io::Error::other("TIFF color table size overflows"))?;
            if palette.len() < required_len {
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::DecodeError,
                    format!(
                        "Color table is too short. expected at least {} entries, got {}",
                        required_len,
                        palette.len()
                    ),
                )));
            }
            palette
        }
        _ => &[],
    };

    let row_bits = stored_width
        .checked_mul(usize::from(header.bitspersample))
        .ok_or_else(|| std::io::Error::other("TIFF row size overflows"))?;
    let row_len = row_bits
        .checked_add(7)
        .map(|bits| bits / 8)
        .ok_or_else(|| std::io::Error::other("TIFF row size overflows"))?;

    let draw_rows = strip.min(stored_height);
    let end_y = y
        .checked_add(draw_rows)
        .ok_or_else(|| std::io::Error::other("TIFF row position overflows"))?;
    for (l, y) in (y..end_y).enumerate() {
        let mut buf = vec![];
        let mut prevs = vec![0_u8; header.samples_per_pixel as usize];
        // Packed grayscale branches below use `i` as a sample index (the
        // byte offset is derived from it); byte-aligned branches use it as a
        // byte offset. Reset both forms at each row so row padding is skipped.
        let mut i = if header.bitspersample < 8 {
            let samples_per_row = row_len
                .checked_mul(8)
                .and_then(|value| value.checked_div(usize::from(header.bitspersample)))
                .ok_or_else(|| std::io::Error::other("TIFF packed row index overflows"))?;
            l.checked_mul(samples_per_row)
                .ok_or_else(|| std::io::Error::other("TIFF packed row index overflows"))?
        } else {
            l.checked_mul(row_len)
                .ok_or_else(|| std::io::Error::other("TIFF row offset overflows"))?
        };

        for pixel in 0..stored_width {
            match header.photometric_interpretation {
                3 => {
                    let row_start = l
                        .checked_mul(row_len)
                        .ok_or_else(|| std::io::Error::other("TIFF palette row overflows"))?;
                    let row_end = row_start
                        .checked_add(row_len)
                        .ok_or_else(|| std::io::Error::other("TIFF palette row overflows"))?;
                    let row = data
                        .get(row_start..row_end)
                        .ok_or_else(|| std::io::Error::other("TIFF palette row is truncated"))?;
                    let index = pixel
                        .checked_mul(usize::from(header.samples_per_pixel))
                        .ok_or_else(|| std::io::Error::other("TIFF palette index overflows"))?;
                    let value = crate::tiff::sample::read_sample_with_fill_order(
                        row,
                        header.bitspersamples[0],
                        header.tiff_headers.endian,
                        index,
                        header.fill_order,
                    )?;
                    let color = palette.get(usize::try_from(value)?).ok_or_else(|| {
                        std::io::Error::other("TIFF palette index is out of range")
                    })?;
                    buf.extend_from_slice(&[color.red, color.green, color.blue, color.alpha]);
                }
                0 | 1 => {
                    match header.bitspersamples[0] {
                        16 => {
                            if header.photometric_interpretation != 3
                                && !(header.max_sample_values.len() == 1
                                    && header.max_sample_values[0] == 32767
                                    && header.tiff_headers.endian == bin_rs::Endian::LittleEndian)
                            {
                                let bytes = usize::from(header.samples_per_pixel)
                                    .checked_mul(2)
                                    .ok_or_else(|| {
                                        std::io::Error::other(
                                            "TIFF grayscale sample size overflows",
                                        )
                                    })?;
                                if !has_bytes(&data, i, bytes) {
                                    return Ok(None);
                                }
                                let sample =
                                    u32::from(read_u16(&data, i, header.tiff_headers.endian));
                                let alpha = if matches!(header.extra_samples.first(), Some(&1 | &2))
                                    && header.samples_per_pixel > 1
                                {
                                    u32::from(read_u16(&data, i + 2, header.tiff_headers.endian))
                                } else {
                                    u32::from(u16::MAX)
                                };
                                let associated = header.extra_samples.first() == Some(&1);
                                let rgba = if associated {
                                    crate::tiff::color::gray_to_rgba8(
                                        sample,
                                        u32::from(u16::MAX),
                                        header.photometric_interpretation == 0,
                                        alpha,
                                        true,
                                    )
                                } else {
                                    crate::tiff::color::gray_to_rgba8(
                                        sample >> 8,
                                        255,
                                        header.photometric_interpretation == 0,
                                        alpha >> 8,
                                        false,
                                    )
                                };
                                buf.push(rgba.red);
                                buf.push(rgba.green);
                                buf.push(rgba.blue);
                                buf.push(rgba.alpha);
                                i += bytes;
                                continue;
                            }
                            // Illegal tiff(TOWNS TIFF)
                            if header.max_sample_values.len() == 1
                                && header.max_sample_values[0] == 32767
                                && header.tiff_headers.endian == bin_rs::Endian::LittleEndian
                            {
                                if !has_bytes(&data, i, 2) {
                                    return Ok(None);
                                }
                                let color = read_u16(&data, i, header.tiff_headers.endian) >> 8;
                                let temp_r = (color >> 5 & 0x1f) as u8;
                                let r = temp_r << 3 | temp_r >> 2;
                                let temp_g = (color >> 10 & 0x1f) as u8;
                                let g = temp_g << 3 | temp_g >> 2;
                                let temp_b = (color & 0x1f) as u8;
                                let b = temp_b << 3 | temp_b >> 2;
                                buf.push(r);
                                buf.push(g);
                                buf.push(b);
                                buf.push(0xff);
                                i += 2;
                            } else {
                                // 16 bit glayscale
                                if !has_bytes(&data, i, 1) {
                                    return Ok(None);
                                }
                                let mut color = data[i];
                                if predictor == 2 {
                                    color += prevs[0];
                                    prevs[0] = color;
                                }
                                buf.push(color);
                                buf.push(color);
                                buf.push(color);
                                buf.push(0xff);
                                i += header.bitspersamples[0] as usize / 8;
                            }
                        }
                        32 => {
                            let bytes = usize::from(header.samples_per_pixel)
                                .checked_mul(4)
                                .ok_or_else(|| {
                                    std::io::Error::other("TIFF grayscale sample size overflows")
                                })?;
                            if !has_bytes(&data, i, bytes) {
                                return Ok(None);
                            }
                            let sample = read_u32(&data, i, header.tiff_headers.endian);
                            let alpha = if matches!(header.extra_samples.first(), Some(&1 | &2))
                                && header.samples_per_pixel > 1
                            {
                                read_u32(&data, i + 4, header.tiff_headers.endian)
                            } else {
                                u32::MAX
                            };
                            let associated = header.extra_samples.first() == Some(&1);
                            let rgba = if associated {
                                crate::tiff::color::gray_to_rgba8(
                                    sample,
                                    u32::MAX,
                                    header.photometric_interpretation == 0,
                                    alpha,
                                    true,
                                )
                            } else {
                                crate::tiff::color::gray_to_rgba8(
                                    sample >> 24,
                                    255,
                                    header.photometric_interpretation == 0,
                                    alpha >> 24,
                                    false,
                                )
                            };
                            buf.push(rgba.red);
                            buf.push(rgba.green);
                            buf.push(rgba.blue);
                            buf.push(rgba.alpha);
                            i += bytes;
                        }
                        8 => {
                            if header.samples_per_pixel > 1 {
                                if !has_bytes(&data, i, usize::from(header.samples_per_pixel)) {
                                    return Ok(None);
                                }
                                let associated = header.extra_samples.first() == Some(&1);
                                let alpha = if matches!(header.extra_samples.first(), Some(&1 | &2))
                                    && header.samples_per_pixel > 1
                                {
                                    u32::from(data[i + 1])
                                } else {
                                    255
                                };
                                let rgba = crate::tiff::color::gray_to_rgba8(
                                    u32::from(data[i]),
                                    255,
                                    header.photometric_interpretation == 0,
                                    alpha,
                                    associated,
                                );
                                buf.push(rgba.red);
                                buf.push(rgba.green);
                                buf.push(rgba.blue);
                                buf.push(rgba.alpha);
                                i += usize::from(header.samples_per_pixel);
                                continue;
                            }
                            if i >= data.len() {
                                //return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Buffer shotage".to_string())));
                                return Ok(None);
                            }
                            let mut color = data[i];
                            if predictor == 2 {
                                color += prevs[0];
                                prevs[0] = color;
                            }

                            let rgba = &palette[color as usize];

                            buf.push(rgba.red);
                            buf.push(rgba.green);
                            buf.push(rgba.blue);
                            buf.push(rgba.alpha);
                            i += header.samples_per_pixel as usize;
                        }
                        4 => {
                            if i / 2 >= data.len() {
                                return Ok(None);
                            }
                            let c;
                            let color = data[i / 2];
                            if i % 2 == 0 {
                                if header.fill_order == 1 {
                                    c = (color >> 4) as usize;
                                } else {
                                    c = (color.reverse_bits() & 0xf) as usize;
                                }
                            } else if header.fill_order == 1 {
                                c = (color & 0xf) as usize;
                            } else {
                                c = (color.reverse_bits() >> 4) as usize;
                            }

                            let rgba = &palette[c];

                            buf.push(rgba.red);
                            buf.push(rgba.green);
                            buf.push(rgba.blue);
                            buf.push(rgba.alpha);
                            i += 1; //  i += header.samples_per_pixel as usize; ?
                        }
                        2 => {
                            // usually illegal
                            if i / 4 >= data.len() {
                                return Ok(None);
                            }
                            let c;
                            let color = data[i / 4];
                            let shift = (i % 4) * 2;
                            if header.fill_order == 1 {
                                c = ((color >> (6 - shift)) & 0x3) as usize;
                            } else {
                                c = ((color.reverse_bits() >> (6 - shift)) & 0x3) as usize;
                            }

                            let rgba = &palette[c];

                            buf.push(rgba.red);
                            buf.push(rgba.green);
                            buf.push(rgba.blue);
                            buf.push(rgba.alpha);
                            i += 1;
                        }
                        1 => {
                            if i / 8 >= data.len() {
                                return Ok(None);
                            }
                            let c;
                            let color = data[i / 8];
                            let shift = i % 8;
                            if header.fill_order == 1 {
                                c = ((color >> (7 - shift)) & 0x1) as usize;
                            } else {
                                c = ((color.reverse_bits() >> (7 - shift)) & 0x1) as usize;
                            }

                            let rgba = &palette[c];

                            buf.push(rgba.red);
                            buf.push(rgba.green);
                            buf.push(rgba.blue);
                            buf.push(rgba.alpha);
                            i += 1;
                        }

                        _ => {
                            return Err(Box::new(ImgError::new_const(
                                ImgErrorKind::DecodeError,
                                "This bit per sample is not support.".to_string(),
                            )));
                        }
                    }
                }
                2 => {
                    //RGB
                    let (mut r, mut g, mut b, mut a) = (0, 0, 0, 0xff);
                    match header.bitspersamples[0] {
                        //bit per samples same (8,8,8), but also (8,16,8) pattern
                        8 => {
                            if i + 2 >= data.len() {
                                //                                return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Buffer shotage".to_string())));
                                return Ok(None);
                            }
                            r = data[i];
                            g = data[i + 1];
                            b = data[i + 2];
                            a = if !header.extra_samples.is_empty()
                                && (header.extra_samples[0] == 1 || header.extra_samples[0] == 2)
                                && header.samples_per_pixel > 3
                            {
                                data[i + 3]
                            } else {
                                0xff
                            };
                            i += header.samples_per_pixel as usize;
                        }
                        16 => {
                            if header.samples_per_pixel >= 3 {
                                let bytes = header.samples_per_pixel as usize * 2;
                                if !has_bytes(&data, i, bytes) {
                                    return Ok(None);
                                }
                                let max = u32::from(u16::MAX);
                                let r16 = u32::from(read_u16(&data, i, header.tiff_headers.endian));
                                let g16 =
                                    u32::from(read_u16(&data, i + 2, header.tiff_headers.endian));
                                let b16 =
                                    u32::from(read_u16(&data, i + 4, header.tiff_headers.endian));
                                let a16 = if !header.extra_samples.is_empty()
                                    && (header.extra_samples[0] == 1
                                        || header.extra_samples[0] == 2)
                                    && header.samples_per_pixel > 3
                                {
                                    u32::from(read_u16(&data, i + 6, header.tiff_headers.endian))
                                } else {
                                    max
                                };
                                if header.extra_samples.first() == Some(&1) {
                                    (r, g, b) = crate::tiff::color::unassociate_alpha_precision(
                                        r16, g16, b16, a16, max,
                                    );
                                } else {
                                    r = (r16 >> 8) as u8;
                                    g = (g16 >> 8) as u8;
                                    b = (b16 >> 8) as u8;
                                }
                                a = if header.extra_samples.first() == Some(&1) {
                                    crate::tiff::color::quantize(a16, max)
                                } else {
                                    (a16 >> 8) as u8
                                };
                                i += header.samples_per_pixel as usize * 2;
                            }
                        }
                        32 => {
                            let bytes = header.samples_per_pixel as usize * 4;
                            if !has_bytes(&data, i, bytes) {
                                return Ok(None);
                            }
                            let max = u32::MAX;
                            let r32 = read_u32(&data, i, header.tiff_headers.endian);
                            let g32 = read_u32(&data, i + 4, header.tiff_headers.endian);
                            let b32 = read_u32(&data, i + 8, header.tiff_headers.endian);
                            let a32 = if !header.extra_samples.is_empty()
                                && (header.extra_samples[0] == 1 || header.extra_samples[0] == 2)
                                && header.samples_per_pixel > 3
                            {
                                read_u32(&data, i + 12, header.tiff_headers.endian)
                            } else {
                                max
                            };
                            if header.extra_samples.first() == Some(&1) {
                                (r, g, b) = crate::tiff::color::unassociate_alpha_precision(
                                    r32, g32, b32, a32, max,
                                );
                            } else {
                                r = (r32 >> 24) as u8;
                                g = (g32 >> 24) as u8;
                                b = (b32 >> 24) as u8;
                            }
                            a = if header.extra_samples.first() == Some(&1) {
                                crate::tiff::color::quantize(a32, max)
                            } else {
                                (a32 >> 24) as u8
                            };
                            i += header.samples_per_pixel as usize * 4;
                        }
                        _ => {
                            return Err(Box::new(ImgError::new_const(
                                ImgErrorKind::DecodeError,
                                "This bit per sample is not support.".to_string(),
                            )));
                        }
                    }

                    if predictor == 2 {
                        r += prevs[0];
                        prevs[0] = r;
                        g += prevs[1];
                        prevs[1] = g;
                        b += prevs[2];
                        prevs[2] = b;
                        if !header.extra_samples.is_empty() && header.extra_samples[0] == 2 {
                            a += prevs[3];
                            prevs[3] = a;
                        }
                    }
                    if header.extra_samples.first() == Some(&1)
                        && header.samples_per_pixel > 3
                        && header.bitspersamples[0] == 8
                    {
                        (r, g, b) = crate::tiff::color::unassociate_alpha(r, g, b, a);
                    }
                    buf.push(r);
                    buf.push(g);
                    buf.push(b);
                    buf.push(a);
                }
                // 4 : Transparentary musk is not support
                5 => {
                    //CMYK
                    let bits = header.bitspersamples[0];
                    let spp = usize::from(header.samples_per_pixel);
                    let (c, m, y, k, a, maximum) = if bits == 8 {
                        let bytes = spp;
                        if !has_bytes(&data, i, bytes) {
                            return Ok(None);
                        }
                        let alpha = if header.extra_samples.first() == Some(&2) && spp > 4 {
                            u32::from(data[i + 4])
                        } else {
                            255
                        };
                        (
                            u32::from(data[i]),
                            u32::from(data[i + 1]),
                            u32::from(data[i + 2]),
                            u32::from(data[i + 3]),
                            alpha,
                            255,
                        )
                    } else if bits == 16 {
                        let bytes = spp
                            .checked_mul(2)
                            .ok_or_else(|| std::io::Error::other("CMYK sample size overflows"))?;
                        if !has_bytes(&data, i, bytes) {
                            return Ok(None);
                        }
                        let alpha = if header.extra_samples.first() == Some(&2) && spp > 4 {
                            u32::from(read_u16(&data, i + 8, header.tiff_headers.endian))
                        } else {
                            u32::from(u16::MAX)
                        };
                        (
                            u32::from(read_u16(&data, i, header.tiff_headers.endian)),
                            u32::from(read_u16(&data, i + 2, header.tiff_headers.endian)),
                            u32::from(read_u16(&data, i + 4, header.tiff_headers.endian)),
                            u32::from(read_u16(&data, i + 6, header.tiff_headers.endian)),
                            alpha,
                            u32::from(u16::MAX),
                        )
                    } else if bits == 32 {
                        let bytes = spp
                            .checked_mul(4)
                            .ok_or_else(|| std::io::Error::other("CMYK sample size overflows"))?;
                        if !has_bytes(&data, i, bytes) {
                            return Ok(None);
                        }
                        let alpha = if header.extra_samples.first() == Some(&2) && spp > 4 {
                            read_u32(&data, i + 16, header.tiff_headers.endian)
                        } else {
                            u32::MAX
                        };
                        (
                            read_u32(&data, i, header.tiff_headers.endian),
                            read_u32(&data, i + 4, header.tiff_headers.endian),
                            read_u32(&data, i + 8, header.tiff_headers.endian),
                            read_u32(&data, i + 12, header.tiff_headers.endian),
                            alpha,
                            u32::MAX,
                        )
                    } else if bits < 8 {
                        // Packed CMYK (for example, 6-bit samples) is read per
                        // row so padding bits at the end of each row are ignored.
                        let row_start = l
                            .checked_mul(row_len)
                            .ok_or_else(|| std::io::Error::other("CMYK row offset overflows"))?;
                        let row_end = row_start
                            .checked_add(row_len)
                            .ok_or_else(|| std::io::Error::other("CMYK row end overflows"))?;
                        if row_end > data.len() {
                            return Ok(None);
                        }
                        let sample_index = pixel
                            .checked_mul(spp)
                            .ok_or_else(|| std::io::Error::other("CMYK sample index overflows"))?;
                        let row = &data[row_start..row_end];
                        let c = crate::tiff::sample::read_sample_with_fill_order(
                            row,
                            bits,
                            header.tiff_headers.endian,
                            sample_index,
                            header.fill_order,
                        )?;
                        let m = crate::tiff::sample::read_sample_with_fill_order(
                            row,
                            bits,
                            header.tiff_headers.endian,
                            sample_index.checked_add(1).ok_or_else(|| {
                                std::io::Error::other("CMYK sample index overflows")
                            })?,
                            header.fill_order,
                        )?;
                        let y = crate::tiff::sample::read_sample_with_fill_order(
                            row,
                            bits,
                            header.tiff_headers.endian,
                            sample_index.checked_add(2).ok_or_else(|| {
                                std::io::Error::other("CMYK sample index overflows")
                            })?,
                            header.fill_order,
                        )?;
                        let k = crate::tiff::sample::read_sample_with_fill_order(
                            row,
                            bits,
                            header.tiff_headers.endian,
                            sample_index.checked_add(3).ok_or_else(|| {
                                std::io::Error::other("CMYK sample index overflows")
                            })?,
                            header.fill_order,
                        )?;
                        let alpha = if header.extra_samples.first() == Some(&2) && spp > 4 {
                            crate::tiff::sample::read_sample_with_fill_order(
                                row,
                                bits,
                                header.tiff_headers.endian,
                                sample_index.checked_add(4).ok_or_else(|| {
                                    std::io::Error::other("CMYK sample index overflows")
                                })?,
                                header.fill_order,
                            )?
                        } else {
                            (1u32 << bits) - 1
                        };
                        (c, m, y, k, alpha, (1u32 << bits) - 1)
                    } else {
                        return Err(Box::new(ImgError::new_const(
                            ImgErrorKind::DecodeError,
                            "This bit per sample is not support.".to_string(),
                        )));
                    };
                    if bits == 8 {
                        i += spp;
                    } else if bits == 16 {
                        i += spp
                            .checked_mul(2)
                            .ok_or_else(|| std::io::Error::other("CMYK sample size overflows"))?;
                    } else if bits == 32 {
                        i += spp
                            .checked_mul(4)
                            .ok_or_else(|| std::io::Error::other("CMYK sample size overflows"))?;
                    } else {
                        // Keep the packed row cursor at the end of the row;
                        // packed samples use sample_index above instead.
                        i = l
                            .checked_add(1)
                            .and_then(|value| value.checked_mul(row_len))
                            .ok_or_else(|| std::io::Error::other("CMYK row end overflows"))?;
                    }
                    let rgba = crate::tiff::color::cmyk_to_rgba8(c, m, y, k, maximum, a);
                    buf.push(rgba.red);
                    buf.push(rgba.green);
                    buf.push(rgba.blue);
                    buf.push(rgba.alpha);
                }
                // not support
                /*
                6 => {  // YCbCr ... this function is not suport YCbCrCoficients,positioning...,yet.  ....4:1:1 sampling
                    let (mut y, mut cb, mut cr, mut a);
                    match header.bitspersamples[0] {  //bit per samples same (8,8,8), but also (8,16,8) pattern
                        8 => {
                            y = data[i];
                            cb = data[i+1];
                            cr = data[i+2];
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 3 {
                                data[i+3] } else { 0xff };
                            i += header.samples_per_pixel as usize;
                        },
                        16 => {
                            y = (read_u16(&data,i,header.tiff_headers.endian) >> 8) as u8;
                            cb = (read_u16(&data,i+2,header.tiff_headers.endian) >> 8) as u8;
                            cr = (read_u16(&data,i+4,header.tiff_headers.endian) >> 8) as u8;
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 3 {
                                    (read_u16(&data,i+6,header.tiff_headers.endian) >> 8) as u8 } else { 0xff };
                            i += header.samples_per_pixel as usize * 2;
                        },
                        32 => {
                            y = (read_u32(&data,i,header.tiff_headers.endian) >> 24) as u8;
                            cb = (read_u32(&data,i+4,header.tiff_headers.endian) >> 24) as u8;
                            cr = (read_u32(&data,i+8,header.tiff_headers.endian) >> 24) as u8;
                            a = if header.extra_samples.len() > 0 && header.extra_samples[0] == 2
                                        && header.samples_per_pixel > 3 {
                                    (read_u32(&data,i+12,header.tiff_headers.endian) >> 24) as u8 } else { 0xff };
                            i += header.samples_per_pixel as usize * 4;
                        },
                        _ => {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"This bit per sample is not support.".to_string())));
                        }
                    }

                    if predictor == 2 {
                        y += prevs[0];
                        prevs[0] = y;
                        cb += prevs[1];
                        prevs[1] = cb;
                        cr += prevs[2];
                        prevs[2] = cr;
                        if header.extra_samples.len() > 0 && header.extra_samples[0]  == 2 {
                            a += prevs[3];
                            prevs[3] = a;
                        }
                    }
                    // Bt601.1
                    let lr = 0.299;
                    let lg = 0.587;
                    let lb = 0.114;

                    let r = ((y as f32 + (2.0 - 2.0 * lr ) * cr as f32) as i16).clamp(0,255) as u8;
                    let b = ((y as f32 + (2.0 - 2.0 * lb ) * cr as f32) as i16).clamp(0,255) as u8;
                    let g = (((y as f32 - lb * b as f32 - lr * r as f32) / lg) as i16).clamp(0,255) as u8;

                    buf.push(r);
                    buf.push(g);
                    buf.push(b);
                    buf.push(a);

                }
                */
                _ => {
                    //                    return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Not support color space.".to_string())));
                }
            }
        }

        option.drawer.draw(x, y, width, 1, &buf, None)?;
    }
    Ok(None)
}

pub fn draw(
    data: &[u8],
    option: &mut DecodeOptions,
    header: &Tiff,
) -> Result<Option<ImgWarnings>, Error> {
    if data.is_empty() {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::DecodeError,
            "Data empty.".to_string(),
        )));
    }
    draw_strip(data, 0, header.height as usize, option, header)
}

fn init_canvas(option: &mut DecodeOptions, header: &Tiff, animation: bool) -> Result<(), Error> {
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
    Ok(())
}

pub fn decode_lzw_compresson<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    decode_blocked(reader, option, header, initialize, animation)
}

pub fn decode_packbits_compresson<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    decode_blocked(reader, option, header, initialize, animation)
}

pub fn decode_deflate_compresson<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    decode_blocked(reader, option, header, initialize, animation)
}

pub fn decode_none_compresson<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    decode_blocked(reader, option, header, initialize, animation)
}

pub fn decode_ccitt_compresson<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    if initialize {
        init_canvas(option, header, animation)?;
    }
    let block_list = blocks(header)?;
    let input_len = reader.seek(std::io::SeekFrom::End(0))?;
    for block in block_list {
        block.validate_range(input_len)?;
        let pixels = block
            .stored_width
            .checked_mul(block.stored_height)
            .ok_or_else(|| std::io::Error::other("CCITT block pixel count overflows"))?;
        crate::limits::check(
            pixels,
            crate::limits::current().pixels,
            "CCITT block pixels",
        )?;
        crate::limits::check(
            pixels,
            crate::limits::current().expanded_bytes,
            "CCITT block expansion",
        )?;
        let compressed = read_block(reader, &block)?;
        let mut ccitt_header = Tiff::empty();
        ccitt_header.width = u32::try_from(block.stored_width)?;
        ccitt_header.height = u32::try_from(block.stored_height)?;
        ccitt_header.rows_per_strip = u32::try_from(block.stored_height)?;
        ccitt_header.bitspersample = 8;
        ccitt_header.bitspersamples = vec![8];
        ccitt_header.planar_config = 1;
        ccitt_header.photometric_interpretation = header.photometric_interpretation;
        ccitt_header.fill_order = header.fill_order;
        ccitt_header.compression = header.compression.clone();
        ccitt_header.t4_options = header.t4_options;
        ccitt_header.t6_options = header.t6_options;
        ccitt_header.tiff_headers.endian = header.tiff_headers.endian;
        let (data, _warning) = ccitt::decode(&compressed, &ccitt_header)?;
        draw_tile_internal(
            &data,
            block.y,
            block.draw_height,
            block.x,
            block.draw_width,
            option,
            &ccitt_header,
            true,
            Some(block.stored_width),
            Some(block.stored_height),
        )?;
    }
    Ok(None)
}

fn compression_decode<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<Option<ImgWarnings>, Error> {
    match header.compression {
        Compression::NoneCompression => {
            return decode_none_compresson(reader, option, header, initialize, animation);
        }
        Compression::LZW => {
            return decode_lzw_compresson(reader, option, header, initialize, animation);
        }
        Compression::Jpeg => {
            #[cfg(feature = "tiff-jpeg")]
            return decode_jpeg_compresson(reader, option, header, initialize, animation);
            #[cfg(not(feature = "tiff-jpeg"))]
            {
                let _ = (reader, option, header, initialize, animation);
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::NoSupportFormat,
                    "TIFF JPEG compression support is disabled by feature flags".to_string(),
                )));
            }
        }
        Compression::OldJpeg => {
            #[cfg(feature = "tiff-jpeg")]
            return decode_old_jpeg_compresson(reader, option, header, initialize, animation);
            #[cfg(not(feature = "tiff-jpeg"))]
            {
                let _ = (reader, option, header, initialize, animation);
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::NoSupportFormat,
                    "TIFF old-style JPEG support is disabled by feature flags".to_string(),
                )));
            }
        }
        Compression::Packbits => {
            return decode_packbits_compresson(reader, option, header, initialize, animation);
        }
        Compression::AdobeDeflate | Compression::DEFLATE => {
            return decode_deflate_compresson(reader, option, header, initialize, animation);
        }
        Compression::CCITTHuffmanRLE
        | Compression::CCITTGroup3Fax
        | Compression::CCITTGroup4Fax => {
            return decode_ccitt_compresson(reader, option, header, initialize, animation);
        }
        _ => Err(Box::new(ImgError::new_const(
            ImgErrorKind::NoSupportFormat,
            "TIFF compression is not supported".to_string(),
        ))),
    }
}

pub fn decode<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let mut document = Tiff::new(reader)?;
    let appended = std::mem::take(&mut *document.multi_page);
    let mut pages = Vec::with_capacity(appended.len().saturating_add(1));
    if is_display_page(&document) {
        pages.push(document);
        pages.extend(appended.into_iter().filter(is_display_page));
    } else {
        let mut iter = appended.into_iter();
        let first = iter
            .find(is_display_page)
            .ok_or_else(|| std::io::Error::other("TIFF contains no display page"))?;
        pages.push(first);
        pages.extend(iter.filter(is_display_page));
    }
    let count = pages.len();
    let header = pages
        .first()
        .ok_or_else(|| std::io::Error::other("TIFF contains no display page"))?;
    option
        .drawer
        .set_metadata("image pages", DataMap::UInt(u64::try_from(count)?))?;

    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("Tiff".to_owned()))?;
    option
        .drawer
        .set_metadata("width", DataMap::UInt(header.width as u64))?;
    option
        .drawer
        .set_metadata("height", DataMap::UInt(header.height as u64))?;
    option
        .drawer
        .set_metadata("bits per pixel", DataMap::UInt(header.bitspersample as u64))?;
    option
        .drawer
        .set_metadata("Tiff headers", DataMap::Exif(header.tiff_headers.clone()))?;
    option.drawer.set_metadata(
        "compression",
        DataMap::Ascii(header.compression.to_string()),
    )?;
    if let Some(ref icc_profile) = header.icc_profile {
        option.drawer.set_metadata(
            "Source ICC Profile",
            DataMap::ICCProfile(icc_profile.to_vec()),
        )?;
    }
    if let Some(icc_profile) = crate::tiff::page::rgba8_icc_profile(&header) {
        option
            .drawer
            .set_metadata("ICC Profile", DataMap::ICCProfile(icc_profile.to_vec()))?;
    }
    let mut warnings = None;

    let warn = compression_decode(reader, option, &pages[0], true, count > 1)?;

    warnings = ImgWarnings::append(warnings, warn);

    if count > 1 {
        for append in pages.iter().skip(1) {
            let rect = ImageRect {
                width: append.width as usize,
                height: append.height as usize,
                start_x: append.startx as i32,
                start_y: append.starty as i32,
            };
            let opt = NextOptions {
                flag: NextOption::Next,
                await_time: 0,
                image_rect: Some(rect),
                dispose_option: None,
                blend: None,
            };

            let result = option.drawer.next(Some(opt))?;
            if let Some(response) = result {
                if response.response == ResponseCommand::Abort {
                    return Ok(warnings);
                }
            }
            let result = compression_decode(reader, option, append, false, false);
            match result {
                Ok(warn) => {
                    warnings = ImgWarnings::append(warnings, warn);
                }
                Err(error) => {
                    let warning = TiffWarning::new(error.to_string());
                    warnings = ImgWarnings::add(warnings, Box::new(warning));
                }
            }
        }
    }

    option.drawer.terminate(None)?;
    Ok(warnings)
}
