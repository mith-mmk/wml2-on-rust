//! Truevision TGA decoder and encoder.
//!
//! Encoding accepts RGBA8 via `PickCallback`. `color_type` selects truecolor
//! (default), grayscale or indexed. Grayscale requires equal RGB channels;
//! indexed uses exact colors without quantization. Gray8/palette24 reject
//! nonopaque alpha. Grayscale depth 8/16 defaults to 16; indexed uses depth 8
//! only, palette_depth 24/32 defaults to 32 (up to 256 exact colors).
//! 16-bit index encoding is deferred because external decoders disagree.
//! Truecolor `depth = UInt(24|32)` defaults
//! to 32 (straight alpha retained); 24 explicitly discards alpha without
//! compositing. `compression = Ascii("none"|"rle")` defaults to RLE.
//! Output is top-left BGR/BGRA, with packets confined to individual rows.
//! Extension metadata and interleaved encoding are not emitted. Transparent RGB
//! is preserved in the file; the decoder normalizes zero-alpha pixels to black.

//! TGA2 extension attributes 0/1 ignore stored alpha, 3 uses straight alpha,
//! and 4 is converted from premultiplied to straight RGBA8 (nearest integer,
//! saturation, alpha-zero RGB black). Undefined-retained type2 and unknown
//! types fail explicitly because unused attribute bits cannot be represented
//! faithfully by RGBA8. Explicit useful-alpha storage must agree with the
//! descriptor's alpha bit count. Without an extension, legacy interpretation
//! and the all-zero 16-bit alpha compatibility fallback are unchanged.
//! Referenced post-image areas are bounds-checked and excluded from pixels;
//! ancillary metadata content and overlapping metadata semantics are not decoded.
//! Alpha codes follow the Attributes Type table in Truevision TGA 2.0: it lists
//! 4 as premultiplied, although the following example refers to 3. The table
//! determines this decoder's interpretation.
//! Specification: <https://raw.githubusercontent.com/DEAKSoftware/Truevision-TGA/master/truevision-tga.pdf>

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

fn le32(data: &[u8], offset: usize) -> Result<usize, Error> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or_else(|| {
            err(
                ImgErrorKind::IllegalData,
                "TGA metadata offset is truncated",
            )
        })?
        .try_into()?;
    usize::try_from(u32::from_le_bytes(bytes))
        .map_err(|_| err(ImgErrorKind::IllegalData, "TGA metadata offset overflow"))
}

fn metadata_range(data: &[u8], footer: usize, start: usize, size: usize) -> Result<&[u8], Error> {
    let end = start
        .checked_add(size)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA metadata size overflow"))?;
    if start < 18 || end > footer {
        return Err(err(
            ImgErrorKind::IllegalData,
            "TGA metadata lies outside the post-image area",
        ));
    }
    data.get(start..end)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "TGA metadata is truncated"))
}

// Return the earliest referenced post-image area, so malformed pixel payloads
// cannot borrow bytes from a valid v2 footer or ancillary block.
fn post_image(data: &[u8], height: usize, pixel_depth: u8) -> Result<(usize, Option<u8>), Error> {
    if data.len() < 26 || !data.ends_with(b"TRUEVISION-XFILE.\0") {
        return Ok((data.len(), None));
    }
    let footer = data.len() - 26;
    if footer < 18 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "TGA footer overlaps the image header",
        ));
    }
    let mut end = footer;
    let extension = le32(data, footer)?;
    let developer = le32(data, footer + 4)?;
    let mut attributes = None;
    if extension != 0 {
        let header = metadata_range(data, footer, extension, 2)?;
        let size = usize::from(u16::from_le_bytes([header[0], header[1]]));
        if size < 495 {
            return Err(err(
                ImgErrorKind::IllegalData,
                "TGA extension must contain at least 495 bytes",
            ));
        }
        let block = metadata_range(data, footer, extension, size)?;
        let value = block[494];
        if value == 2 || value > 4 {
            return Err(err(
                ImgErrorKind::NoSupportFormat,
                "TGA undefined or unsupported attributes cannot be represented as RGBA8",
            ));
        }
        attributes = Some(value);
        end = end.min(extension);
        for (field, size) in [(482, 2048), (490, height * 4)] {
            let offset = le32(block, field)?;
            if offset != 0 {
                metadata_range(data, footer, offset, size)?;
                end = end.min(offset);
            }
        }
        let stamp = le32(block, 486)?;
        if stamp != 0 {
            let header = metadata_range(data, footer, stamp, 2)?;
            let size = 2 + usize::from(header[0])
                * usize::from(header[1])
                * usize::from(pixel_depth.div_ceil(8));
            metadata_range(data, footer, stamp, size)?;
            end = end.min(stamp);
        }
    }
    if developer != 0 {
        let header = metadata_range(data, footer, developer, 2)?;
        let count = usize::from(u16::from_le_bytes([header[0], header[1]]));
        let block = metadata_range(data, footer, developer, 2 + count * 10)?;
        end = end.min(developer);
        for entry in block[2..].chunks_exact(10) {
            let offset = le32(entry, 2)?;
            let size = le32(entry, 6)?;
            if size != 0 {
                metadata_range(data, footer, offset, size)?;
                end = end.min(offset);
            }
        }
    }
    Ok((end, attributes))
}

fn scale5(value: u8) -> u8 {
    (u16::from(value) * 255 / 31) as u8
}

fn decode_color(data: &[u8], depth: u8, alpha_bits: u8) -> Result<[u8; 4], Error> {
    match depth {
        15 | 16 => {
            let value = u16::from_le_bytes([data[0], data[1]]);
            let alpha = if depth == 16 && alpha_bits > 0 && alpha_bits != 255 {
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
            if alpha_bits == 255 {
                Ok([data[2], data[1], data[0], 255])
            } else if data[3] == 0 {
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
                if alpha_bits == 255 {
                    Ok([data[0], data[0], data[0], 255])
                } else if data[1] == 0 {
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
    let (image_end, attributes) = post_image(&data, height, pixel_depth)?;
    let data = &data[..image_end];
    // Sentinel 255 means explicitly ignore stored attributes, before legacy
    // transparent-RGB normalization. Type2 cannot be carried as unused bits
    // through RGBA8, so it is explicitly rejected above.
    if matches!(attributes, Some(3 | 4)) {
        let expected = match (base_type, pixel_depth, data[7]) {
            (1, _, 16) | (2, 16, _) => 1,
            (1, _, 32) | (2, 32, _) | (3, 16, _) => 8,
            _ => {
                return Err(err(
                    ImgErrorKind::IllegalData,
                    "TGA useful alpha requires alpha-bearing storage",
                ));
            }
        };
        if alpha_bits != expected {
            return Err(err(
                ImgErrorKind::IllegalData,
                "TGA alpha bits conflict with useful-alpha extension",
            ));
        }
    }
    let alpha_bits = match attributes {
        Some(0 | 1) => 255,
        _ => alpha_bits,
    };
    if data[17] & 0xc0 == 0xc0 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "Reserved TGA interleaving mode",
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
    let alpha_bits =
        if attributes.is_none() && pixel_depth == 16 && alpha_bits > 0 && base_type == 2 && !rle {
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
    if attributes == Some(4) {
        for pixel in decoded.chunks_exact_mut(4) {
            let alpha = u32::from(pixel[3]);
            if alpha != 0 {
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
    }
    let mut output = vec![0u8; output_len];
    let top_origin = data[17] & 0x20 != 0;
    let right_origin = data[17] & 0x10 != 0;
    // Legacy TGA interleave stores all rows of one phase before the next.
    let interleave = match (data[17] >> 6) & 3 {
        1 => 2,
        2 => 4,
        _ => 1,
    };
    for stored_y in 0..height {
        let logical_y = if interleave == 1 {
            stored_y
        } else {
            let mut row = stored_y;
            let mut phase = 0;
            loop {
                let count = if phase < height {
                    (height - phase).div_ceil(interleave)
                } else {
                    0
                };
                if row < count {
                    break phase + row * interleave;
                }
                row -= count;
                phase += 1;
            }
        };
        for stored_x in 0..width {
            let x = if right_origin {
                width - 1 - stored_x
            } else {
                stored_x
            };
            let y = if top_origin {
                logical_y
            } else {
                height - 1 - logical_y
            };
            let src = (stored_y * width + stored_x) * 4;
            let dst = (y * width + x) * 4;
            output[dst..dst + 4].copy_from_slice(&decoded[src..src + 4]);
        }
    }
    if let Some(value) = attributes {
        option
            .drawer
            .set_metadata("TGA Attributes Type", DataMap::UInt(u64::from(value)))?;
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

/// Encodes RGBA8 as true-color, exact indexed color, or grayscale TGA.
pub fn encode(option: &mut crate::draw::EncodeOptions<'_>) -> Result<Vec<u8>, Error> {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        TrueColor,
        Grayscale,
        Indexed,
    }
    let options = option.options.as_ref();
    let get = |key: &str| options.and_then(|o| o.get(key));
    let kind = match get("color_type") {
        None => Kind::TrueColor,
        Some(DataMap::Ascii(v)) if v.eq_ignore_ascii_case("truecolor") => Kind::TrueColor,
        Some(DataMap::Ascii(v)) if v.eq_ignore_ascii_case("grayscale") => Kind::Grayscale,
        Some(DataMap::Ascii(v)) if v.eq_ignore_ascii_case("indexed") => Kind::Indexed,
        _ => {
            return Err(err(
                ImgErrorKind::InvalidParameter,
                "TGA color_type must be truecolor, grayscale or indexed",
            ));
        }
    };
    let depth = match get("depth") {
        None => match kind {
            Kind::TrueColor => 32,
            Kind::Grayscale => 16,
            Kind::Indexed => 8,
        },
        Some(DataMap::UInt(v))
            if (kind == Kind::TrueColor && matches!(v, 24 | 32))
                || (kind == Kind::Grayscale && matches!(v, 8 | 16))
                || (kind == Kind::Indexed && *v == 8) =>
        {
            *v as usize
        }
        _ => {
            return Err(err(
                ImgErrorKind::InvalidParameter,
                "TGA depth must be 24/32 for truecolor, 8/16 for grayscale, or 8 for indexed",
            ));
        }
    };
    let palette_depth = match (kind, get("palette_depth")) {
        (_, None) => 32,
        (Kind::Indexed, Some(DataMap::UInt(v))) if matches!(v, 24 | 32) => *v as usize,
        _ => {
            return Err(err(
                ImgErrorKind::InvalidParameter,
                "TGA palette_depth is indexed-only UInt(24) or UInt(32)",
            ));
        }
    };
    let rle = match get("compression") {
        None => true,
        Some(DataMap::Ascii(v)) if v.eq_ignore_ascii_case("none") => false,
        Some(DataMap::Ascii(v)) if v.eq_ignore_ascii_case("rle") => true,
        _ => {
            return Err(err(
                ImgErrorKind::InvalidParameter,
                "TGA compression must be none or rle",
            ));
        }
    };
    let profile = option
        .drawer
        .encode_start(None)?
        .ok_or_else(|| err(ImgErrorKind::EncodeError, "TGA source profile is missing"))?;
    let (width, height) = (profile.width, profile.height);
    if width == 0 || height == 0 || width > u16::MAX as usize || height > u16::MAX as usize {
        return Err(err(
            ImgErrorKind::InvalidParameter,
            "TGA dimensions must be 1..=65535",
        ));
    }
    let row_len = width
        .checked_mul(4)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "TGA row size overflow"))?;
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "TGA image size overflow"))?;
    let bytes_per_pixel = depth / 8;
    let palette_capacity = if depth == 8 { 256 } else { u16::MAX as usize };
    let palette_bound = if kind == Kind::Indexed {
        pixels.min(palette_capacity) * (palette_depth / 8)
    } else {
        0
    };
    let output_bound = pixels
        .checked_mul(bytes_per_pixel + usize::from(rle))
        .and_then(|n| n.checked_add(44 + palette_bound))
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "TGA output size overflow"))?;
    let limits = crate::limits::current();
    crate::limits::check(pixels, limits.pixels, "TGA pixels")?;
    crate::limits::check(
        pixels
            .checked_mul(4)
            .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "TGA source size overflow"))?,
        limits.expanded_bytes,
        "TGA source",
    )?;
    crate::limits::check(output_bound, limits.expanded_bytes, "TGA encoded bytes")?;
    let mut output = vec![0; 18];
    let base_type = match kind {
        Kind::TrueColor => 2,
        Kind::Grayscale => 3,
        Kind::Indexed => 1,
    };
    output[2] = base_type + if rle { 8 } else { 0 };
    if kind == Kind::Indexed {
        output[1] = 1;
        output[7] = palette_depth as u8;
    }
    output[12..14].copy_from_slice(&(width as u16).to_le_bytes());
    output[14..16].copy_from_slice(&(height as u16).to_le_bytes());
    output[16] = depth as u8;
    let alpha = match kind {
        Kind::TrueColor => depth == 32,
        Kind::Grayscale => depth == 16,
        Kind::Indexed => palette_depth == 32,
    };
    output[17] = 0x20 | if alpha { 8 } else { 0 };
    let mut palette = Vec::<u8>::new();
    let mut indices = std::collections::HashMap::<[u8; 4], u16>::new();
    for y in 0..height {
        let rgba = option
            .drawer
            .encode_pick(0, y, width, 1, None)?
            .ok_or_else(|| err(ImgErrorKind::EncodeError, "TGA source row is missing"))?;
        if rgba.len() != row_len {
            return Err(err(
                ImgErrorKind::EncodeError,
                "TGA source row must contain exactly width * 4 RGBA8 bytes",
            ));
        }
        let mut row = Vec::with_capacity(width * bytes_per_pixel);
        match kind {
            Kind::TrueColor => {
                if depth == 32 {
                    pack_truecolor::<4>(&rgba, &mut row);
                } else {
                    pack_truecolor::<3>(&rgba, &mut row);
                }
            }
            Kind::Grayscale => {
                for pixel in rgba.chunks_exact(4) {
                    if pixel[0] != pixel[1] || pixel[1] != pixel[2] {
                        return Err(err(
                            ImgErrorKind::InvalidParameter,
                            "TGA grayscale requires equal RGB channels; no implicit luminance conversion",
                        ));
                    }
                    if depth == 8 && pixel[3] != 255 {
                        return Err(err(
                            ImgErrorKind::InvalidParameter,
                            "TGA grayscale8 cannot preserve alpha; use depth 16",
                        ));
                    }
                    row.push(pixel[0]);
                    if depth == 16 {
                        row.push(pixel[3]);
                    }
                }
            }
            Kind::Indexed => {
                for pixel in rgba.chunks_exact(4) {
                    if palette_depth == 24 && pixel[3] != 255 {
                        return Err(err(
                            ImgErrorKind::InvalidParameter,
                            "TGA 24-bit palette cannot preserve alpha; use palette_depth 32",
                        ));
                    }
                    let color = [pixel[0], pixel[1], pixel[2], pixel[3]];
                    let index = if let Some(index) = indices.get(&color) {
                        *index
                    } else {
                        if indices.len() >= palette_capacity {
                            return Err(err(
                                ImgErrorKind::InvalidParameter,
                                "TGA indexed color capacity exceeded; no implicit quantization",
                            ));
                        }
                        let index = indices.len() as u16;
                        indices.insert(color, index);
                        palette.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
                        if palette_depth == 32 {
                            palette.push(pixel[3]);
                        }
                        index
                    };
                    row.push(index as u8);
                }
            }
        }
        if rle {
            encode_rle_row(&row, bytes_per_pixel, &mut output);
        } else {
            output.extend_from_slice(&row);
        }
    }
    if kind == Kind::Indexed {
        output[5..7].copy_from_slice(&(indices.len() as u16).to_le_bytes());
        // Payload was gathered in one callback pass; insert the completed map
        // before it without a second, potentially changing source read.
        output.splice(18..18, palette);
    }
    // A valid v2 footer with no extension/developer areas.
    output.extend_from_slice(&[0; 8]);
    output.extend_from_slice(b"TRUEVISION-XFILE.\0");
    option.drawer.encode_end(None)?;
    Ok(output)
}

fn pack_truecolor<const BPP: usize>(rgba: &[u8], row: &mut Vec<u8>) {
    for pixel in rgba.chunks_exact(4) {
        row.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        if BPP == 4 {
            row.push(pixel[3]);
        }
    }
}

fn encode_rle_row(row: &[u8], bpp: usize, output: &mut Vec<u8>) {
    let pixels = row.len() / bpp;
    let pixel = |index: usize| &row[index * bpp..(index + 1) * bpp];
    let run_length = |index: usize| {
        let mut count = 1;
        while count < 128 && index + count < pixels && pixel(index + count) == pixel(index) {
            count += 1;
        }
        count
    };
    let mut index = 0;
    while index < pixels {
        let run = run_length(index);
        if run >= 2 {
            output.push(0x80 | (run - 1) as u8);
            output.extend_from_slice(pixel(index));
            index += run;
        } else {
            let start = index;
            index += 1;
            while index < pixels && index - start < 128 && run_length(index) < 2 {
                index += 1;
            }
            output.push((index - start - 1) as u8);
            output.extend_from_slice(&row[start * bpp..index * bpp]);
        }
    }
}
