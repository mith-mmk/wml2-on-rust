//! Non-JPEG TIFF YCbCr block decoding.

use super::{decompress_block, read_block};
use crate::draw::DecodeOptions;
use crate::tiff::block::{TiffBlock, TiffBlockKind, blocks};
use crate::tiff::header::Tiff;
use bin_rs::reader::BinaryReader;
use std::io;

type Error = Box<dyn std::error::Error>;

fn checked_ratio(numerator: u32, denominator: u32, name: &str) -> Result<f64, Error> {
    if denominator == 0 {
        return Err(io::Error::other(format!("TIFF YCbCr {name} has a zero denominator")).into());
    }
    Ok(f64::from(numerator) / f64::from(denominator))
}

fn validate_header(header: &Tiff) -> Result<(usize, usize), Error> {
    if header.samples_per_pixel != 3
        || header.bitspersamples.as_slice() != [8, 8, 8]
        || header.planar_config != 1
    {
        return Err(io::Error::other(
            "TIFF non-JPEG YCbCr requires contiguous 8-bit three-component samples",
        )
        .into());
    }
    if !header.extra_samples.is_empty() {
        return Err(io::Error::other("TIFF non-JPEG YCbCr does not support extra samples").into());
    }
    if header.ycbcr_coefficients.len() != 3 {
        return Err(io::Error::other("TIFF YCbCrCoefficients must contain three values").into());
    }
    if header.reference_black_white.len() != 6 {
        return Err(io::Error::other("TIFF ReferenceBlackWhite must contain six values").into());
    }
    for value in &header.ycbcr_coefficients {
        checked_ratio(value.n, value.d, "coefficient")?;
    }
    for value in &header.reference_black_white {
        checked_ratio(value.n, value.d, "ReferenceBlackWhite value")?;
    }
    for pair in header.reference_black_white.chunks_exact(2) {
        let black = checked_ratio(pair[0].n, pair[0].d, "ReferenceBlackWhite value")?;
        let white = checked_ratio(pair[1].n, pair[1].d, "ReferenceBlackWhite value")?;
        if white <= black {
            return Err(io::Error::other(
                "TIFF YCbCr ReferenceBlackWhite white value must exceed black value",
            )
            .into());
        }
    }
    let [horizontal, vertical] = header
        .ycbcr_sub_sampling
        .as_slice()
        .try_into()
        .map_err(|_| io::Error::other("TIFF YCbCrSubSampling must contain two values"))?;
    if !matches!(horizontal, 1 | 2 | 4) || !matches!(vertical, 1 | 2 | 4) || vertical > horizontal {
        return Err(io::Error::other(
            "TIFF YCbCrSubSampling must be 1, 2, or 4 with vertical <= horizontal",
        )
        .into());
    }
    if !matches!(header.ycbcr_positioning, 1 | 2) {
        return Err(io::Error::other(format!(
            "unsupported TIFF YCbCrPositioning {}",
            header.ycbcr_positioning
        ))
        .into());
    }
    Ok((usize::from(horizontal), usize::from(vertical)))
}

fn expected_bytes(
    block: &TiffBlock,
    storage_height: usize,
    horizontal: usize,
    vertical: usize,
) -> Result<usize, Error> {
    let units_x = block
        .stored_width
        .checked_add(horizontal - 1)
        .ok_or_else(|| io::Error::other("TIFF YCbCr unit count overflows"))?
        / horizontal;
    let units_y = storage_height
        .checked_add(vertical - 1)
        .ok_or_else(|| io::Error::other("TIFF YCbCr unit count overflows"))?
        / vertical;
    let samples_per_unit = horizontal
        .checked_mul(vertical)
        .and_then(|value| value.checked_add(2))
        .ok_or_else(|| io::Error::other("TIFF YCbCr unit size overflows"))?;
    units_x
        .checked_mul(units_y)
        .and_then(|value| value.checked_mul(samples_per_unit))
        .ok_or_else(|| io::Error::other("TIFF YCbCr block size overflows").into())
}

fn expand(code: u8, black: f64, white: f64, coding_range: f64) -> f64 {
    (f64::from(code) - black) * coding_range / (white - black)
}

fn convert(y: u8, cb: u8, cr: u8, header: &Tiff) -> Result<[u8; 4], Error> {
    let reference = &header.reference_black_white;
    let y = expand(
        y,
        checked_ratio(reference[0].n, reference[0].d, "ReferenceBlackWhite value")?,
        checked_ratio(reference[1].n, reference[1].d, "ReferenceBlackWhite value")?,
        255.0,
    );
    let cb = expand(
        cb,
        checked_ratio(reference[2].n, reference[2].d, "ReferenceBlackWhite value")?,
        checked_ratio(reference[3].n, reference[3].d, "ReferenceBlackWhite value")?,
        127.0,
    );
    let cr = expand(
        cr,
        checked_ratio(reference[4].n, reference[4].d, "ReferenceBlackWhite value")?,
        checked_ratio(reference[5].n, reference[5].d, "ReferenceBlackWhite value")?,
        127.0,
    );
    let kr = checked_ratio(
        header.ycbcr_coefficients[0].n,
        header.ycbcr_coefficients[0].d,
        "coefficient",
    )?;
    let kg = checked_ratio(
        header.ycbcr_coefficients[1].n,
        header.ycbcr_coefficients[1].d,
        "coefficient",
    )?;
    let kb = checked_ratio(
        header.ycbcr_coefficients[2].n,
        header.ycbcr_coefficients[2].d,
        "coefficient",
    )?;
    if kg == 0.0 {
        return Err(io::Error::other("TIFF YCbCr green coefficient is zero").into());
    }
    let red = y + cr * (2.0 - 2.0 * kr);
    let blue = y + cb * (2.0 - 2.0 * kb);
    let green = (y - kb * blue - kr * red) / kg;
    Ok([
        red.round().clamp(0.0, 255.0) as u8,
        green.round().clamp(0.0, 255.0) as u8,
        blue.round().clamp(0.0, 255.0) as u8,
        255,
    ])
}

fn chroma_index(coordinate: usize, factor: usize, positioning: u16) -> usize {
    if positioning == 1 {
        // Centered samples are half a unit inside the first luminance group;
        // nearest-neighbour expansion preserves each TIFF data unit exactly.
        let center = (factor - 1) as f64 / 2.0;
        ((coordinate as f64 - center) / factor as f64)
            .round()
            .max(0.0) as usize
    } else {
        coordinate / factor
    }
}

fn decode_block(
    data: &[u8],
    block: &TiffBlock,
    header: &Tiff,
    horizontal: usize,
    vertical: usize,
    storage_height: usize,
    option: &mut DecodeOptions,
) -> Result<(), Error> {
    let units_x = block.stored_width.div_ceil(horizontal);
    let samples_per_unit = horizontal * vertical + 2;
    let mut output = Vec::with_capacity(block.draw_width * block.draw_height * 4);
    for row in 0..block.draw_height {
        for col in 0..block.draw_width {
            let unit_x = chroma_index(col, horizontal, header.ycbcr_positioning).min(units_x - 1);
            let unit_y = chroma_index(row, vertical, header.ycbcr_positioning)
                .min(storage_height.div_ceil(vertical) - 1);
            let unit = (unit_y * units_x + unit_x) * samples_per_unit;
            let y_index = unit + (row % vertical) * horizontal + (col % horizontal);
            let cb_index = unit + horizontal * vertical;
            let cr_index = cb_index + 1;
            let y = *data
                .get(y_index)
                .ok_or_else(|| io::Error::other("TIFF YCbCr Y sample is truncated"))?;
            let cb = *data
                .get(cb_index)
                .ok_or_else(|| io::Error::other("TIFF YCbCr Cb sample is truncated"))?;
            let cr = *data
                .get(cr_index)
                .ok_or_else(|| io::Error::other("TIFF YCbCr Cr sample is truncated"))?;
            output.extend_from_slice(&convert(y, cb, cr, header)?);
        }
    }
    option.drawer.draw(
        block.x,
        block.y,
        block.draw_width,
        block.draw_height,
        &output,
        None,
    )?;
    Ok(())
}

pub(crate) fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    header: &Tiff,
    initialize: bool,
    animation: bool,
) -> Result<(), Error> {
    if initialize {
        super::init_canvas(option, header, animation)?;
    }
    let (horizontal, vertical) = validate_header(header)?;
    let block_list = blocks(header)?;
    let input_len = reader.seek(std::io::SeekFrom::End(0))?;
    for block in &block_list {
        block.validate_range(input_len)?;
        let storage_height = match block.kind {
            TiffBlockKind::Strip if header.rows_per_strip != 0 => {
                usize::try_from(header.rows_per_strip)?
            }
            _ => block.stored_height,
        };
        let expected = expected_bytes(block, storage_height, horizontal, vertical)?;
        crate::limits::check(
            expected,
            crate::limits::current().expanded_bytes,
            "expanded TIFF YCbCr block",
        )?;
        let compressed = read_block(reader, block)?;
        let mut data = decompress_block(&header.compression, &compressed, Some(expected))?;
        if header.predictor == 2 {
            let row_bytes = expected
                .checked_div(storage_height)
                .filter(|_| expected % storage_height == 0)
                .ok_or_else(|| io::Error::other("TIFF YCbCr predictor row size is invalid"))?;
            crate::tiff::predictor::apply_predictor(
                &mut data,
                row_bytes,
                storage_height,
                8,
                3,
                header.tiff_headers.endian,
            )?;
        }
        decode_block(
            &data,
            block,
            header,
            horizontal,
            vertical,
            storage_height,
            option,
        )?;
    }
    Ok(())
}
