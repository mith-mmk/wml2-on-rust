//! DirectDraw Surface decoder for common legacy and DX10 texture layouts.

use bin_rs::reader::BinaryReader;

use crate::draw::DecodeOptions;
use crate::error::ImgErrorKind;
use crate::metadata::DataMap;
use crate::retro::{draw_rgba, err, read_all};
use crate::warning::ImgWarnings;

type Error = Box<dyn std::error::Error>;

#[derive(Clone, Copy)]
enum UncompressedFormat {
    Legacy,
    Rgba8,
    Bgra8,
    Bgrx8,
}

#[derive(Clone, Copy)]
enum Compression {
    Uncompressed(UncompressedFormat),
    Bc1,
    Bc2 { premultiplied: bool },
    Bc3 { premultiplied: bool },
    Bc4 { signed: bool },
    Bc5 { signed: bool },
}

fn le32(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "DDS header is truncated"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn expand5(value: u32) -> u8 {
    ((value << 3) | (value >> 2)) as u8
}

fn color565(value: u16) -> [u8; 3] {
    [
        expand5(u32::from((value >> 11) & 0x1f)),
        (((u32::from((value >> 5) & 0x3f) << 2) | (u32::from((value >> 5) & 0x3f) >> 4)) & 0xff)
            as u8,
        expand5(u32::from(value & 0x1f)),
    ]
}

fn decode_bc4(block: &[u8], signed: bool) -> [u8; 16] {
    if signed {
        let a0 = i16::from(i8::from_ne_bytes([block[0]]));
        let a1 = i16::from(i8::from_ne_bytes([block[1]]));
        let mut table = [0i16; 8];
        table[0] = a0;
        table[1] = a1;
        if a0 > a1 {
            for i in 1..7 {
                table[i + 1] = ((7 - i) as i16 * a0 + i as i16 * a1) / 7;
            }
        } else {
            for i in 1..5 {
                table[i + 1] = ((5 - i) as i16 * a0 + i as i16 * a1) / 5;
            }
            table[6] = -128;
            table[7] = 127;
        }
        let mut bits = 0u64;
        for (i, &byte) in block[2..8].iter().enumerate() {
            bits |= u64::from(byte) << (8 * i);
        }
        let mut result = [0u8; 16];
        for (i, value) in result.iter_mut().enumerate() {
            *value = (table[((bits >> (3 * i)) & 7) as usize] + 128) as u8;
        }
        return result;
    }

    let a0 = block[0];
    let a1 = block[1];
    let mut table = [0u8; 8];
    table[0] = a0;
    table[1] = a1;
    if a0 > a1 {
        for i in 1..7 {
            table[i + 1] = (((7 - i) * usize::from(a0) + i * usize::from(a1)) / 7) as u8;
        }
    } else {
        for i in 1..5 {
            table[i + 1] = (((5 - i) * usize::from(a0) + i * usize::from(a1)) / 5) as u8;
        }
        table[6] = 0;
        table[7] = 255;
    }
    let mut bits = 0u64;
    for (i, &byte) in block[2..8].iter().enumerate() {
        bits |= u64::from(byte) << (8 * i);
    }
    let mut result = [0u8; 16];
    for (i, value) in result.iter_mut().enumerate() {
        *value = table[((bits >> (3 * i)) & 7) as usize];
    }
    result
}

fn decode_bc1(block: &[u8], allow_transparent: bool) -> [[u8; 4]; 16] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let rgb0 = color565(c0);
    let rgb1 = color565(c1);
    let mut colors = [[0u8; 4]; 4];
    colors[0] = [rgb0[0], rgb0[1], rgb0[2], 255];
    colors[1] = [rgb1[0], rgb1[1], rgb1[2], 255];
    if c0 > c1 || !allow_transparent {
        for channel in 0..3 {
            colors[2][channel] =
                ((2 * u16::from(rgb0[channel]) + u16::from(rgb1[channel])) / 3) as u8;
            colors[3][channel] =
                ((u16::from(rgb0[channel]) + 2 * u16::from(rgb1[channel])) / 3) as u8;
        }
        colors[2][3] = 255;
        colors[3][3] = 255;
    } else {
        for channel in 0..3 {
            colors[2][channel] = ((u16::from(rgb0[channel]) + u16::from(rgb1[channel])) / 2) as u8;
        }
        colors[2][3] = 255;
        colors[3] = [0, 0, 0, 0];
    }
    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    let mut result = [[0u8; 4]; 16];
    for (i, pixel) in result.iter_mut().enumerate() {
        *pixel = colors[((indices >> (2 * i)) & 3) as usize];
    }
    result
}

fn unpremultiply(value: u8, alpha: u8) -> u8 {
    if alpha == 0 {
        0
    } else {
        ((u16::from(value) * 255 + u16::from(alpha) / 2) / u16::from(alpha)).min(255) as u8
    }
}

fn unpremultiply_rgb(pixel: &mut [u8; 4]) {
    let alpha = pixel[3];
    pixel[0] = unpremultiply(pixel[0], alpha);
    pixel[1] = unpremultiply(pixel[1], alpha);
    pixel[2] = unpremultiply(pixel[2], alpha);
}

fn decode_block(block: &[u8], compression: Compression) -> Result<[[u8; 4]; 16], Error> {
    match compression {
        Compression::Bc1 => Ok(decode_bc1(block, true)),
        Compression::Bc2 { premultiplied } => {
            let colors = decode_bc1(&block[8..], false);
            let mut result = colors;
            for i in 0..16 {
                let alpha = if i & 1 == 0 {
                    block[i / 2] & 0x0f
                } else {
                    block[i / 2] >> 4
                };
                result[i][3] = alpha * 17;
                if premultiplied {
                    unpremultiply_rgb(&mut result[i]);
                }
            }
            Ok(result)
        }
        Compression::Bc3 { premultiplied } => {
            let alpha = decode_bc4(block, false);
            let mut result = decode_bc1(&block[8..], false);
            for i in 0..16 {
                result[i][3] = alpha[i];
                if premultiplied {
                    unpremultiply_rgb(&mut result[i]);
                }
            }
            Ok(result)
        }
        Compression::Bc4 { signed } => {
            let red = decode_bc4(block, signed);
            let mut result = [[0u8; 4]; 16];
            for i in 0..16 {
                result[i] = [red[i], red[i], red[i], 255];
            }
            Ok(result)
        }
        Compression::Bc5 { signed } => {
            let red = decode_bc4(&block[..8], signed);
            let green = decode_bc4(&block[8..16], signed);
            let mut result = [[0u8; 4]; 16];
            for i in 0..16 {
                result[i] = [red[i], green[i], 0, 255];
            }
            Ok(result)
        }
        Compression::Uncompressed(_) => Err(err(
            ImgErrorKind::IllegalData,
            "DDS block compression is missing",
        )),
    }
}

fn compression_from_fourcc(fourcc: &[u8]) -> Option<Compression> {
    match fourcc {
        b"DXT1" => Some(Compression::Bc1),
        b"DXT2" => Some(Compression::Bc2 {
            premultiplied: true,
        }),
        b"DXT3" => Some(Compression::Bc2 {
            premultiplied: false,
        }),
        b"DXT4" => Some(Compression::Bc3 {
            premultiplied: true,
        }),
        b"DXT5" => Some(Compression::Bc3 {
            premultiplied: false,
        }),
        b"ATI1" | b"BC4U" => Some(Compression::Bc4 { signed: false }),
        b"BC4S" => Some(Compression::Bc4 { signed: true }),
        b"ATI2" | b"BC5U" => Some(Compression::Bc5 { signed: false }),
        b"BC5S" => Some(Compression::Bc5 { signed: true }),
        _ => None,
    }
}

fn compression_from_dxgi(value: u32) -> Option<Compression> {
    match value {
        71 | 72 => Some(Compression::Bc1),
        74 | 75 => Some(Compression::Bc2 {
            premultiplied: false,
        }),
        77 | 78 => Some(Compression::Bc3 {
            premultiplied: false,
        }),
        27..=29 => Some(Compression::Uncompressed(UncompressedFormat::Rgba8)),
        80 => Some(Compression::Bc4 { signed: false }),
        81 => Some(Compression::Bc4 { signed: true }),
        83 => Some(Compression::Bc5 { signed: false }),
        84 => Some(Compression::Bc5 { signed: true }),
        87 | 90 | 91 => Some(Compression::Uncompressed(UncompressedFormat::Bgra8)),
        88 | 92 | 93 => Some(Compression::Uncompressed(UncompressedFormat::Bgrx8)),
        _ => None,
    }
}

fn extract_mask(value: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let normalized = (value & mask) >> shift;
    let maximum = mask >> shift;
    ((u64::from(normalized) * 255 + u64::from(maximum) / 2) / u64::from(maximum)) as u8
}

fn checked_output_len(width: usize, height: usize) -> Result<usize, Error> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "DDS image size overflow"))?;
    let output_len = pixels
        .checked_mul(4)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "DDS output size overflow"))?;
    let limits = crate::limits::current();
    crate::limits::check(pixels, limits.pixels, "pixels")?;
    crate::limits::check(output_len, limits.expanded_bytes, "RGBA image")?;
    Ok(output_len)
}

fn decode_uncompressed(
    data: &[u8],
    offset: usize,
    width: usize,
    height: usize,
    pitch: usize,
    flags: u32,
    bit_count: usize,
    masks: [u32; 4],
) -> Result<Vec<u8>, Error> {
    let output_len = checked_output_len(width, height)?;
    let bytes_per_pixel = bit_count.div_ceil(8);
    let minimum_pitch = width
        .checked_mul(bytes_per_pixel)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "DDS scanline size overflow"))?;
    if !matches!(bytes_per_pixel, 1 | 2 | 3 | 4) || pitch < minimum_pitch {
        return Err(err(
            ImgErrorKind::IllegalData,
            "Unsupported DDS pixel layout",
        ));
    }
    let end = offset
        .checked_add(
            pitch
                .checked_mul(height)
                .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "DDS image size overflow"))?,
        )
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "DDS image offset overflow"))?;
    let source = data
        .get(offset..end)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "DDS pixel data is truncated"))?;
    let mut output = vec![0u8; output_len];
    let luminance = flags & 0x0002_0000 != 0;
    let alpha_only = flags & 0x0000_0002 != 0 && flags & 0x0000_0040 == 0;
    for y in 0..height {
        for x in 0..width {
            let src = y * pitch + x * bytes_per_pixel;
            let mut value = 0u32;
            for byte in 0..bytes_per_pixel {
                value |= u32::from(source[src + byte]) << (8 * byte);
            }
            let alpha = if masks[3] != 0 {
                extract_mask(value, masks[3])
            } else {
                255
            };
            let (r, g, b) = if luminance {
                let l = extract_mask(value, masks[0]);
                (l, l, l)
            } else if alpha_only {
                (0, 0, 0)
            } else {
                (
                    extract_mask(value, masks[0]),
                    extract_mask(value, masks[1]),
                    extract_mask(value, masks[2]),
                )
            };
            let dst = (y * width + x) * 4;
            output[dst..dst + 4].copy_from_slice(&[
                r,
                g,
                b,
                if alpha_only {
                    extract_mask(value, masks[0])
                } else {
                    alpha
                },
            ]);
        }
    }
    Ok(output)
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let data = read_all(reader)?;
    if data.len() < 128 || &data[0..4] != b"DDS " || le32(&data, 4)? != 124 {
        return Err(err(ImgErrorKind::IllegalData, "Not a DDS image"));
    }
    let height = usize::try_from(le32(&data, 12)?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "DDS height is invalid"))?;
    let width = usize::try_from(le32(&data, 16)?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "DDS width is invalid"))?;
    if width == 0 || height == 0 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "DDS dimensions must be non-zero",
        ));
    }
    let pitch_or_linear = usize::try_from(le32(&data, 20)?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "DDS pitch is invalid"))?;
    let mipmaps = le32(&data, 28)?;
    let pf_flags = le32(&data, 80)?;
    let fourcc = &data[84..88];
    let mut data_offset = 128usize;
    let compression = if pf_flags & 0x4 != 0 {
        if fourcc == b"DX10" {
            let dxgi = le32(&data, 128)?;
            data_offset = 148;
            compression_from_dxgi(dxgi)
                .ok_or_else(|| err(ImgErrorKind::NoSupportFormat, "Unsupported DDS DX10 format"))?
        } else {
            compression_from_fourcc(fourcc)
                .ok_or_else(|| err(ImgErrorKind::NoSupportFormat, "Unsupported DDS compression"))?
        }
    } else {
        Compression::Uncompressed(UncompressedFormat::Legacy)
    };
    let output = if matches!(compression, Compression::Uncompressed(_)) {
        let legacy_bit_count = usize::try_from(le32(&data, 88)?)?;
        let legacy_masks = [
            le32(&data, 92)?,
            le32(&data, 96)?,
            le32(&data, 100)?,
            le32(&data, 104)?,
        ];
        let (bit_count, masks) = match compression {
            Compression::Uncompressed(UncompressedFormat::Legacy) => {
                (legacy_bit_count, legacy_masks)
            }
            Compression::Uncompressed(UncompressedFormat::Rgba8) => {
                (32, [0x0000_00ff, 0x0000_ff00, 0x00ff_0000, 0xff00_0000])
            }
            Compression::Uncompressed(UncompressedFormat::Bgra8) => {
                (32, [0x00ff_0000, 0x0000_ff00, 0x0000_00ff, 0xff00_0000])
            }
            Compression::Uncompressed(UncompressedFormat::Bgrx8) => {
                (32, [0x00ff_0000, 0x0000_ff00, 0x0000_00ff, 0])
            }
            _ => unreachable!("compressed DDS reached uncompressed path"),
        };
        let bytes_per_pixel = bit_count.div_ceil(8);
        decode_uncompressed(
            &data,
            data_offset,
            width,
            height,
            if pf_flags & 0x8 != 0 {
                pitch_or_linear
            } else {
                width * bytes_per_pixel
            },
            pf_flags,
            bit_count,
            masks,
        )?
    } else {
        let output_len = checked_output_len(width, height)?;
        let block_bytes = if matches!(compression, Compression::Bc1 | Compression::Bc4 { .. }) {
            8
        } else {
            16
        };
        let blocks_w = width.div_ceil(4);
        let blocks_h = height.div_ceil(4);
        let image_bytes = blocks_w
            .checked_mul(blocks_h)
            .and_then(|value| value.checked_mul(block_bytes))
            .ok_or_else(|| {
                err(
                    ImgErrorKind::InvalidParameter,
                    "DDS block image size overflow",
                )
            })?;
        let compressed = data
            .get(data_offset..data_offset + image_bytes)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "DDS block data is truncated"))?;
        let mut output = vec![0u8; output_len];
        for by in 0..blocks_h {
            for bx in 0..blocks_w {
                let block_offset = (by * blocks_w + bx) * block_bytes;
                let block = &compressed[block_offset..block_offset + block_bytes];
                let pixels = decode_block(block, compression)?;
                for py in 0..4 {
                    for px in 0..4 {
                        let x = bx * 4 + px;
                        let y = by * 4 + py;
                        if x < width && y < height {
                            let dst = (y * width + x) * 4;
                            output[dst..dst + 4].copy_from_slice(&pixels[py * 4 + px]);
                        }
                    }
                }
            }
        }
        output
    };
    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("DDS".to_string()))?;
    option
        .drawer
        .set_metadata("width", DataMap::UInt(width as u64))?;
    option
        .drawer
        .set_metadata("height", DataMap::UInt(height as u64))?;
    option
        .drawer
        .set_metadata("mipmap count", DataMap::UInt(u64::from(mipmaps)))?;
    draw_rgba(option, width, height, &output)?;
    Ok(None)
}
