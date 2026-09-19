//! ZSoft PCX decoder.

use bin_rs::reader::BinaryReader;

use crate::draw::DecodeOptions;
use crate::error::ImgErrorKind;
use crate::metadata::DataMap;
use crate::retro::{draw_rgba, err, read_all};
use crate::warning::ImgWarnings;

type Error = Box<dyn std::error::Error>;

fn le16(data: &[u8], offset: usize) -> Result<u16, Error> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PCX header is truncated"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_row(data: &[u8], cursor: &mut usize, length: usize) -> Result<Vec<u8>, Error> {
    let mut row = Vec::with_capacity(length);
    while row.len() < length {
        let value = *data
            .get(*cursor)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "PCX scanline is truncated"))?;
        *cursor += 1;
        if value & 0xc0 == 0xc0 {
            let count = usize::from(value & 0x3f);
            if count == 0 || row.len() + count > length {
                return Err(err(
                    ImgErrorKind::IllegalData,
                    "PCX RLE run exceeds scanline",
                ));
            }
            let repeated = *data
                .get(*cursor)
                .ok_or_else(|| err(ImgErrorKind::IllegalData, "PCX RLE value is truncated"))?;
            *cursor += 1;
            row.resize(row.len() + count, repeated);
        } else {
            row.push(value);
        }
    }
    Ok(row)
}

fn sample_plane(row: &[u8], x: usize, bits: u8) -> u8 {
    let per_byte = 8 / usize::from(bits);
    let byte = row[x / per_byte];
    let shift = (per_byte - 1 - (x % per_byte)) * usize::from(bits);
    (byte >> shift) & ((1u8 << bits) - 1)
}

fn palette_color(palette: &[[u8; 3]], index: usize) -> [u8; 4] {
    let rgb = palette.get(index).copied().unwrap_or([0, 0, 0]);
    [rgb[0], rgb[1], rgb[2], 255]
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let data = read_all(reader)?;
    if data.len() < 128 || data[0] != 0x0a || data[2] != 1 {
        return Err(err(ImgErrorKind::IllegalData, "Not a PCX image"));
    }
    let bits_per_plane = data[3];
    if !matches!(bits_per_plane, 1 | 2 | 4 | 8) || data[65] == 0 || data[65] > 4 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "Unsupported PCX pixel layout",
        ));
    }
    let xmin = usize::from(le16(&data, 4)?);
    let ymin = usize::from(le16(&data, 6)?);
    let xmax = usize::from(le16(&data, 8)?);
    let ymax = usize::from(le16(&data, 10)?);
    let width = xmax
        .checked_sub(xmin)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PCX width is invalid"))?;
    let height = ymax
        .checked_sub(ymin)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PCX height is invalid"))?;
    if width == 0 || height == 0 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PCX dimensions must be non-zero",
        ));
    }
    let planes = usize::from(data[65]);
    if bits_per_plane == 8 && !matches!(planes, 1 | 3 | 4) {
        return Err(err(
            ImgErrorKind::IllegalData,
            "Unsupported PCX pixel layout",
        ));
    }
    let bytes_per_line = usize::from(le16(&data, 66)?);
    let minimum_line = width
        .checked_mul(usize::from(bits_per_plane))
        .map(|value| value.div_ceil(8))
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PCX scanline size overflow"))?;
    if bytes_per_line < minimum_line {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PCX bytes-per-line is too small",
        ));
    }
    let row_len = bytes_per_line
        .checked_mul(planes)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PCX scanline size overflow"))?;
    let palette256 = if bits_per_plane == 8
        && planes == 1
        && data.len() >= 769
        && data[data.len() - 769] == 12
    {
        Some(&data[data.len() - 768..])
    } else {
        None
    };
    if bits_per_plane == 8 && planes == 1 && palette256.is_none() {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PCX 256-color palette is missing",
        ));
    }
    let image_end = palette256.map_or(data.len(), |_| data.len() - 769);
    let mut palette16 = [[0u8; 3]; 16];
    for (index, rgb) in palette16.iter_mut().enumerate() {
        let offset = 16 + index * 3;
        rgb.copy_from_slice(&data[offset..offset + 3]);
    }
    let palette256_values = palette256.map(|bytes| {
        let mut palette = [[0u8; 3]; 256];
        for (index, rgb) in palette.iter_mut().enumerate() {
            rgb.copy_from_slice(&bytes[index * 3..index * 3 + 3]);
        }
        palette
    });
    let pixels_count = width
        .checked_mul(height)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PCX image size overflow"))?;
    let output_len = pixels_count
        .checked_mul(4)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PCX output size overflow"))?;
    let limits = crate::limits::current();
    crate::limits::check(pixels_count, limits.pixels, "pixels")?;
    crate::limits::check(output_len, limits.expanded_bytes, "RGBA image")?;
    let mut output = vec![0u8; output_len];
    let mut cursor = 128usize;
    for y in 0..height {
        if cursor >= image_end {
            return Err(err(
                ImgErrorKind::IllegalData,
                "PCX image data is truncated",
            ));
        }
        let row = read_row(&data[..image_end], &mut cursor, row_len)?;
        for x in 0..width {
            let rgba = if bits_per_plane == 8 && planes >= 3 {
                let r = row[x];
                let g = row[bytes_per_line + x];
                let b = row[bytes_per_line * 2 + x];
                let a = if planes >= 4 {
                    row[bytes_per_line * 3 + x]
                } else {
                    255
                };
                [r, g, b, a]
            } else {
                let index = if bits_per_plane == 8 {
                    usize::from(row[x])
                } else {
                    let mut index = 0usize;
                    for plane in 0..planes {
                        index |= usize::from(sample_plane(
                            &row[plane * bytes_per_line..],
                            x,
                            bits_per_plane,
                        )) << (plane * usize::from(bits_per_plane));
                    }
                    index
                };
                if bits_per_plane == 8 {
                    palette256_values.map_or_else(
                        || palette_color(&palette16, index),
                        |palette| palette_color(&palette, index),
                    )
                } else {
                    palette_color(&palette16, index)
                }
            };
            let dst = (y * width + x) * 4;
            output[dst..dst + 4].copy_from_slice(&rgba);
        }
    }
    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("PCX".to_string()))?;
    option
        .drawer
        .set_metadata("width", DataMap::UInt(width as u64))?;
    option
        .drawer
        .set_metadata("height", DataMap::UInt(height as u64))?;
    draw_rgba(option, width, height, &output)?;
    Ok(None)
}
