//! Truevision TGA decoder.

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
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA header is truncated"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn scale5(value: u8) -> u8 {
    (u16::from(value) * 255 / 31) as u8
}

fn decode_color(data: &[u8], depth: u8, alpha_bits: u8) -> Result<[u8; 4], Error> {
    match depth {
        15 | 16 => {
            let value = u16::from_le_bytes([data[0], data[1]]);
            let alpha = if depth == 16 && alpha_bits > 0 {
                if value & 0x8000 != 0 { 255 } else { 0 }
            } else {
                255
            };
            let red = scale5(((value >> 10) & 0x1f) as u8);
            let green = scale5(((value >> 5) & 0x1f) as u8);
            let blue = scale5((value & 0x1f) as u8);
            if alpha == 0 {
                Ok([0, 0, 0, 0])
            } else {
                Ok([red, green, blue, alpha])
            }
        }
        24 => Ok([data[2], data[1], data[0], 255]),
        32 => {
            if data[3] == 0 {
                Ok([0, 0, 0, 0])
            } else {
                Ok([data[2], data[1], data[0], data[3]])
            }
        }
        _ => Err(err(
            ImgErrorKind::IllegalData,
            "Unsupported TGA color depth",
        )),
    }
}

fn palette_color(
    palette: &[u8],
    start: u16,
    depth: u8,
    index: u16,
    alpha_bits: u8,
) -> Result<[u8; 4], Error> {
    let relative = index.checked_sub(start).ok_or_else(|| {
        err(
            ImgErrorKind::IllegalData,
            "TGA palette index is below the palette",
        )
    })?;
    let bytes_per_entry = usize::from(depth.div_ceil(8));
    let offset = usize::from(relative)
        .checked_mul(bytes_per_entry)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA palette offset overflow"))?;
    let entry = palette
        .get(offset..offset + bytes_per_entry)
        .ok_or_else(|| {
            err(
                ImgErrorKind::IllegalData,
                "TGA palette index is out of range",
            )
        })?;
    match depth {
        8 => Ok([entry[0], entry[0], entry[0], 255]),
        15 | 16 | 24 | 32 => decode_color(entry, depth, alpha_bits),
        _ => Err(err(
            ImgErrorKind::IllegalData,
            "Unsupported TGA palette depth",
        )),
    }
}

fn decode_pixel(
    data: &[u8],
    base_type: u8,
    pixel_depth: u8,
    palette: &[u8],
    palette_start: u16,
    palette_depth: u8,
    alpha_bits: u8,
) -> Result<[u8; 4], Error> {
    match base_type {
        1 => {
            let index = match pixel_depth {
                8 => u16::from(data[0]),
                16 => u16::from_le_bytes([data[0], data[1]]),
                _ => {
                    return Err(err(
                        ImgErrorKind::IllegalData,
                        "Unsupported TGA palette index depth",
                    ));
                }
            };
            palette_color(palette, palette_start, palette_depth, index, alpha_bits)
        }
        2 => decode_color(data, pixel_depth, alpha_bits),
        3 => match pixel_depth {
            8 => Ok([data[0], data[0], data[0], 255]),
            16 => {
                if data[1] == 0 {
                    Ok([0, 0, 0, 0])
                } else {
                    Ok([data[0], data[0], data[0], data[1]])
                }
            }
            _ => Err(err(
                ImgErrorKind::IllegalData,
                "Unsupported TGA grayscale depth",
            )),
        },
        _ => Err(err(ImgErrorKind::IllegalData, "Unsupported TGA image type")),
    }
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let data = read_all(reader)?;
    if data.len() < 18 {
        return Err(err(ImgErrorKind::IllegalData, "TGA header is truncated"));
    }
    let color_map_type = data[1];
    let image_type = data[2];
    if color_map_type > 1 || !matches!(image_type, 1 | 2 | 3 | 9 | 10 | 11) {
        return Err(err(ImgErrorKind::IllegalData, "Invalid TGA image type"));
    }
    let base_type = image_type & 0x07;
    let width = usize::from(le16(&data, 12)?);
    let height = usize::from(le16(&data, 14)?);
    if width == 0 || height == 0 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "TGA dimensions must be non-zero",
        ));
    }
    let pixel_depth = data[16];
    let alpha_bits = data[17] & 0x0f;
    if data[17] & 0xc0 != 0 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "Interleaved TGA images are unsupported",
        ));
    }
    let image_id_end = 18usize
        .checked_add(usize::from(data[0]))
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA image ID offset overflow"))?;
    if image_id_end > data.len() {
        return Err(err(ImgErrorKind::IllegalData, "TGA image ID is truncated"));
    }
    let palette_start = le16(&data, 3)?;
    let palette_length = le16(&data, 5)?;
    let palette_depth = data[7];
    let palette_bytes = usize::from(palette_length)
        .checked_mul(usize::from(palette_depth.div_ceil(8)))
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA color map size overflow"))?;
    let image_offset = image_id_end
        .checked_add(palette_bytes)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA image offset overflow"))?;
    let palette = data
        .get(image_id_end..image_offset)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA color map is truncated"))?;
    if base_type == 1 && (color_map_type != 1 || palette_length == 0) {
        return Err(err(
            ImgErrorKind::IllegalData,
            "TGA indexed image has no color map",
        ));
    }
    let bytes_per_pixel = usize::from(pixel_depth.div_ceil(8));
    if bytes_per_pixel == 0 {
        return Err(err(ImgErrorKind::IllegalData, "TGA pixel depth is invalid"));
    }
    let pixels_count = width
        .checked_mul(height)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "TGA image size overflow"))?;
    let output_len = pixels_count
        .checked_mul(4)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "TGA output size overflow"))?;
    let limits = crate::limits::current();
    crate::limits::check(pixels_count, limits.pixels, "pixels")?;
    crate::limits::check(output_len, limits.expanded_bytes, "RGBA image")?;
    let rle = image_type >= 9;
    // ImageMagick treats a declared 1-bit alpha plane with no set bits as
    // opaque, so retain that compatibility for uncompressed 16-bit true-color
    // samples while honoring the alpha plane when it contains an opaque bit.
    let alpha_bits = if pixel_depth == 16 && alpha_bits > 0 && base_type == 2 && !rle {
        let image_bytes = pixels_count
            .checked_mul(bytes_per_pixel)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA image size overflow"))?;
        let image_end = image_offset
            .checked_add(image_bytes)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA image offset overflow"))?;
        let image_data = data
            .get(image_offset..image_end)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA pixel data is truncated"))?;
        if image_data.chunks_exact(2).any(|pixel| pixel[1] & 0x80 != 0) {
            alpha_bits
        } else {
            0
        }
    } else {
        alpha_bits
    };
    let mut decoded = Vec::with_capacity(output_len);
    let mut cursor = image_offset;
    while decoded.len() / 4 < pixels_count {
        let (count, run) = if rle {
            let packet = *data
                .get(cursor)
                .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA RLE packet is truncated"))?;
            cursor += 1;
            (usize::from(packet & 0x7f) + 1, packet & 0x80 != 0)
        } else {
            (1, false)
        };
        if count > pixels_count - decoded.len() / 4 {
            return Err(err(
                ImgErrorKind::IllegalData,
                "TGA RLE packet exceeds image size",
            ));
        }
        if run {
            let pixel = data
                .get(cursor..cursor + bytes_per_pixel)
                .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA RLE pixel is truncated"))?;
            let rgba = decode_pixel(
                pixel,
                base_type,
                pixel_depth,
                palette,
                palette_start,
                palette_depth,
                alpha_bits,
            )?;
            cursor += bytes_per_pixel;
            for _ in 0..count {
                decoded.extend_from_slice(&rgba);
            }
        } else {
            for _ in 0..count {
                let pixel = data
                    .get(cursor..cursor + bytes_per_pixel)
                    .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA pixel data is truncated"))?;
                let rgba = decode_pixel(
                    pixel,
                    base_type,
                    pixel_depth,
                    palette,
                    palette_start,
                    palette_depth,
                    alpha_bits,
                )?;
                cursor += bytes_per_pixel;
                decoded.extend_from_slice(&rgba);
            }
        }
    }
    let mut output = vec![0u8; output_len];
    let top_origin = data[17] & 0x20 != 0;
    let right_origin = data[17] & 0x10 != 0;
    for stored_y in 0..height {
        for stored_x in 0..width {
            let x = if right_origin {
                width - 1 - stored_x
            } else {
                stored_x
            };
            let y = if top_origin {
                stored_y
            } else {
                height - 1 - stored_y
            };
            let src = (stored_y * width + stored_x) * 4;
            let dst = (y * width + x) * 4;
            output[dst..dst + 4].copy_from_slice(&decoded[src..src + 4]);
        }
    }
    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("TGA".to_string()))?;
    option
        .drawer
        .set_metadata("width", DataMap::UInt(width as u64))?;
    option
        .drawer
        .set_metadata("height", DataMap::UInt(height as u64))?;
    draw_rgba(option, width, height, &output)?;
    Ok(None)
}
