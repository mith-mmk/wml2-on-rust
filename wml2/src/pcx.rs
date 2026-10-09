//! ZSoft PCX decoding and RGB/indexed encoding.
//!
//! Legacy palette convention: version 0/3 low-depth color images use the
//! standard EGA palette, except packed 2-bit, one-plane version 0/2/3 images,
//! which use ZSoft's CGA header controls. Version 4/5 uses RGB header palettes.
//! CGA C=1 requests a display-dependent monochrome interpretation and is
//! explicitly unsupported. Single-plane 1-bit output retains the established
//! white-zero/black-one convention regardless of header colors or version.
//! This layout/version convention does not identify the original display.
//! Header versions and C/P/I fields follow the ZSoft Technical Reference Manual
//! (1988): <https://msxcomputermagazine.nl/mccm/millennium/milc/formats/topic_0.htm>.

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

// RGBI's conventional 16-color palette, including brown at index 6.
const EGA_PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],
    [0, 0, 170],
    [0, 170, 0],
    [0, 170, 170],
    [170, 0, 0],
    [170, 0, 170],
    [170, 85, 0],
    [170, 170, 170],
    [85, 85, 85],
    [85, 85, 255],
    [85, 255, 85],
    [85, 255, 255],
    [255, 85, 85],
    [255, 85, 255],
    [255, 255, 85],
    [255, 255, 255],
];

fn legacy_palette(data: &[u8], bits: u8, planes: usize) -> Result<[[u8; 3]; 16], Error> {
    let mut palette = [[0; 3]; 16];
    for (index, rgb) in palette.iter_mut().enumerate() {
        rgb.copy_from_slice(&data[16 + index * 3..19 + index * 3]);
    }
    if bits == 2 && planes == 1 && matches!(data[1], 0 | 2 | 3) {
        // ZSoft defines C/P/I in bits 7/6/5; lower bits are ignored.
        let control = data[19];
        if control & 0x80 != 0 {
            return Err(err(
                ImgErrorKind::NoSupportFormat,
                "PCX CGA monochrome display interpretation is unsupported",
            ));
        }
        let indices = if control & 0x40 == 0 {
            [2, 4, 6]
        } else {
            [3, 5, 7]
        };
        let intensity = if control & 0x20 != 0 { 8 } else { 0 };
        palette[0] = EGA_PALETTE[usize::from(data[16] >> 4)];
        for (slot, index) in indices.into_iter().enumerate() {
            palette[slot + 1] = EGA_PALETTE[index + intensity];
        }
    } else if bits < 8 && matches!(data[1], 0 | 3) {
        palette = EGA_PALETTE;
    }
    Ok(palette)
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let data = read_all(reader)?;
    if data.len() < 128 || data[0] != 0x0a || data[2] != 1 {
        return Err(err(ImgErrorKind::IllegalData, "Not a PCX image"));
    }
    if !matches!(data[1], 0 | 2 | 3 | 4 | 5) {
        return Err(err(
            ImgErrorKind::NoSupportFormat,
            "Unsupported PCX version",
        ));
    }
    let bits_per_plane = data[3];
    // The header palette has only 16 entries. Reject wider packed indices.
    if !matches!(bits_per_plane, 1 | 2 | 4 | 8)
        || data[65] == 0
        || data[65] > 4
        || (bits_per_plane < 8 && u16::from(bits_per_plane) * u16::from(data[65]) > 4)
    {
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
    let palette16 = legacy_palette(&data, bits_per_plane, planes)?;
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
        if bits_per_plane == 8 && planes >= 3 {
            let output_row = &mut output[y * width * 4..(y + 1) * width * 4];
            let red = &row[..width];
            let green = &row[bytes_per_line..bytes_per_line + width];
            let blue = &row[bytes_per_line * 2..bytes_per_line * 2 + width];
            if planes == 4 {
                let alpha = &row[bytes_per_line * 3..bytes_per_line * 3 + width];
                for ((((pixel, r), g), b), a) in output_row
                    .chunks_exact_mut(4)
                    .zip(red)
                    .zip(green)
                    .zip(blue)
                    .zip(alpha)
                {
                    pixel.copy_from_slice(&[*r, *g, *b, *a]);
                }
            } else {
                for (((pixel, r), g), b) in
                    output_row.chunks_exact_mut(4).zip(red).zip(green).zip(blue)
                {
                    pixel.copy_from_slice(&[*r, *g, *b, 255]);
                }
            }
        } else {
            for x in 0..width {
                let rgba = {
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
                    if bits_per_plane == 1 && planes == 1 {
                        let value = if index == 0 { 255 } else { 0 };
                        [value, value, value, 255]
                    } else if bits_per_plane == 8 {
                        palette256_values.as_ref().map_or_else(
                            || palette_color(&palette16, index),
                            |palette| palette_color(palette, index),
                        )
                    } else {
                        palette_color(&palette16, index)
                    }
                };
                let dst = (y * width + x) * 4;
                output[dst..dst + 4].copy_from_slice(&rgba);
            }
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

#[inline(always)]
fn reserve_encoded(output: &mut Vec<u8>, additional: usize, limit: usize) -> Result<(), Error> {
    let next_len = output
        .len()
        .checked_add(additional)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PCX encoded size overflow"))?;
    crate::limits::check(next_len, limit, "PCX encoded bytes")?;
    if next_len > output.capacity() {
        // Grow geometrically without reserving beyond the caller's byte budget.
        let capacity = output
            .capacity()
            .checked_mul(2)
            .unwrap_or(next_len)
            .max(next_len)
            .min(limit);
        output.try_reserve_exact(capacity - output.len())?;
    }
    Ok(())
}

fn encode_rle_row<const CHECKED: bool>(
    row: &[u8],
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), Error> {
    let mut offset = 0;
    while offset < row.len() {
        let value = row[offset];
        let mut count = 1usize;
        while count < 63 && offset + count < row.len() && row[offset + count] == value {
            count += 1;
        }
        let encoded_count = if count > 1 || value & 0xc0 == 0xc0 {
            2
        } else {
            1
        };
        if CHECKED {
            reserve_encoded(output, encoded_count, limit)?;
        }
        if encoded_count == 2 {
            output.push(0xc0 | count as u8);
        }
        output.push(value);
        offset += count;
    }
    Ok(())
}

/// Encodes version-5 PCX with row-local RLE.
///
/// `color_type` accepts `Ascii("rgb")` (default) or `Ascii("indexed")`.
/// Indexed output preserves up to 256 exact RGB colors without quantization;
/// palette entries follow first pixel occurrence in row-major order.
///
/// PCX RGB has no alpha channel. Nonopaque input is rejected unless the
/// `alpha` option is explicitly `DataMap::Ascii("discard")`. The default
/// `"reject"` preserves callers from accidental transparency loss.
pub fn encode(option: &mut crate::draw::EncodeOptions<'_>) -> Result<Vec<u8>, Error> {
    let indexed = match option.options.as_ref().and_then(|m| m.get("color_type")) {
        None => false,
        Some(DataMap::Ascii(value)) if value == "rgb" => false,
        Some(DataMap::Ascii(value)) if value == "indexed" => true,
        _ => {
            return Err(err(
                ImgErrorKind::InvalidParameter,
                "PCX color_type must be rgb or indexed",
            ));
        }
    };
    let discard_alpha = match option.options.as_ref().and_then(|m| m.get("alpha")) {
        None => false,
        Some(DataMap::Ascii(value)) if value == "reject" => false,
        Some(DataMap::Ascii(value)) if value == "discard" => true,
        _ => {
            return Err(err(
                ImgErrorKind::InvalidParameter,
                "PCX alpha must be reject or discard",
            ));
        }
    };
    let profile = option
        .drawer
        .encode_start(None)?
        .ok_or_else(|| err(ImgErrorKind::EncodeError, "PCX image profile is missing"))?;
    let width = profile.width;
    let height = profile.height;
    // BytesPerLine is even and stored in u16; 65535-wide images cannot fit.
    if width == 0 || width > 65534 || height == 0 || height > 65536 {
        return Err(err(
            ImgErrorKind::InvalidParameter,
            "PCX dimensions exceed header limits",
        ));
    }
    let stride = (width + 1) & !1;
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PCX image size overflow"))?;
    let rgba_len = pixels
        .checked_mul(4)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "PCX image size overflow"))?;
    let limits = crate::limits::current();
    crate::limits::check(pixels, limits.pixels, "pixels")?;
    crate::limits::check(rgba_len, limits.expanded_bytes, "RGBA image")?;
    crate::limits::check(128, limits.expanded_bytes, "PCX encoded bytes")?;
    let mut output = vec![0u8; 128];
    output[0] = 10;
    output[1] = 5;
    output[2] = 1;
    output[3] = 8;
    output[8..10].copy_from_slice(&((width - 1) as u16).to_le_bytes());
    output[10..12].copy_from_slice(&((height - 1) as u16).to_le_bytes());
    output[65] = if indexed { 1 } else { 3 };
    output[66..68].copy_from_slice(&(stride as u16).to_le_bytes());
    output[68..70].copy_from_slice(&1u16.to_le_bytes());
    let mut row = vec![0u8; stride * if indexed { 1 } else { 3 }];
    let mut colors = std::collections::HashMap::<[u8; 3], u8>::new();
    let mut palette = Vec::<[u8; 3]>::new();
    for y in 0..height {
        let source = option
            .drawer
            .encode_pick(0, y, width, 1, None)?
            .ok_or_else(|| err(ImgErrorKind::EncodeError, "PCX source row is missing"))?;
        if source.len() != width * 4 {
            return Err(err(
                ImgErrorKind::EncodeError,
                "PCX source row must contain exactly RGBA8 pixels",
            ));
        }
        for (x, pixel) in source.chunks_exact(4).enumerate() {
            if !discard_alpha && pixel[3] != 255 {
                return Err(err(
                    ImgErrorKind::EncodeError,
                    "PCX cannot preserve alpha; use alpha=discard explicitly",
                ));
            }
            if indexed {
                let rgb = [pixel[0], pixel[1], pixel[2]];
                let index = if let Some(index) = colors.get(&rgb) {
                    *index
                } else {
                    if palette.len() == 256 {
                        return Err(err(
                            ImgErrorKind::EncodeError,
                            "PCX indexed output requires at most 256 exact RGB colors",
                        ));
                    }
                    let index = palette.len() as u8;
                    palette.push(rgb);
                    colors.insert(rgb, index);
                    index
                };
                row[x] = index;
            } else {
                for channel in 0..3 {
                    row[channel * stride + x] = pixel[channel];
                }
            }
        }
        // If a whole row's worst-case encoding fits, one reservation proves
        // every following packet safe. Near the limit, check actual packets
        // instead; compressible rows are never rejected by an upper bound.
        let row_bound = row.len().checked_mul(2).ok_or_else(|| {
            err(
                ImgErrorKind::InvalidParameter,
                "PCX encoded row size overflow",
            )
        })?;
        let whole_row_fits = output
            .len()
            .checked_add(row_bound)
            .is_some_and(|length| length <= limits.expanded_bytes);
        if whole_row_fits {
            reserve_encoded(&mut output, row_bound, limits.expanded_bytes)?;
        }
        if whole_row_fits {
            encode_rle_row::<false>(&row, &mut output, limits.expanded_bytes)?;
        } else {
            encode_rle_row::<true>(&row, &mut output, limits.expanded_bytes)?;
        }
    }
    if indexed {
        reserve_encoded(&mut output, 769, limits.expanded_bytes)?;
        output.push(12);
        for rgb in &palette {
            output.extend_from_slice(rgb);
        }
        output.resize(output.len() + (256 - palette.len()) * 3, 0);
    }
    option.drawer.encode_end(None)?;
    Ok(output)
}
