//! DirectDraw Surface decoder for common legacy and DX10 texture layouts,
//! and an uncompressed RGBA8 encoder for a single 2D surface.

use bin_rs::reader::BinaryReader;

use crate::draw::{DecodeOptions, EncodeOptions};
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
    Bc7,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanline_size_multiplication_is_checked() {
        crate::limits::scope(crate::limits::DecodeLimits::unlimited(), || {
            let error = decode_uncompressed(&[], 0, usize::MAX / 4, 1, 0, 0, 64, [0; 4])
                .expect_err("scanline size overflow must be reported");
            assert!(error.to_string().contains("DDS scanline size overflow"));
        });
    }

    #[test]
    fn bc7_partition_tables_have_valid_subsets_and_fixup_anchors() {
        for partition in 0..64 {
            for subsets in [2usize, 3] {
                let (pattern, anchors) = if subsets == 2 {
                    (
                        BC7_PARTITION_TWO[partition],
                        [0, BC7_ANCHOR_TWO[partition], 0],
                    )
                } else {
                    (
                        BC7_PARTITION_THREE[partition],
                        [
                            0,
                            BC7_ANCHOR_THREE_SECOND[partition],
                            BC7_ANCHOR_THREE_THIRD[partition],
                        ],
                    )
                };
                assert_eq!(pattern & 3, 0);
                let mut seen = [false; 3];
                for pixel in 0..16 {
                    let subset = ((pattern >> (pixel * 2)) & 3) as usize;
                    assert!(subset < subsets);
                    seen[subset] = true;
                }
                for (subset, anchor) in anchors[..subsets].iter().enumerate() {
                    assert!(*anchor < 16);
                    assert_eq!((pattern >> (u32::from(*anchor) * 2)) & 3, subset as u32);
                    assert!(seen[subset]);
                }
            }
        }
    }

    #[test]
    fn encoder_checks_pixels_and_encoded_size_before_reading_rows() {
        for (pixels, expanded_bytes, expected) in [
            (0, 132, "pixels"),
            (1, 3, "RGBA image"),
            (1, 131, "encoded DDS"),
        ] {
            let limits = crate::limits::DecodeLimits {
                pixels,
                expanded_bytes,
                ..crate::limits::DecodeLimits::unlimited()
            };
            crate::limits::scope(limits, || {
                let mut source = crate::draw::ImageBuffer::from_buffer(1, 1, vec![0; 4]);
                let error = encode(&mut EncodeOptions {
                    debug_flag: 0,
                    drawer: &mut source,
                    options: None,
                })
                .unwrap_err();
                assert!(error.to_string().contains(expected), "{error}");
            });
        }
    }
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

// This small kernel also serves BC2/BC3. Keep it in the block dispatcher so
// header-handling changes do not add an out-of-line call to every BC1 block.
#[inline(always)]
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

// BC7 partition and anchor data adapted from Microsoft DirectXTex
// BC6HBC7.cpp, commit 1acf4ebe6330e06b7eb52e8db63d4f824ad77427:
// https://github.com/microsoft/DirectXTex/blob/1acf4ebe6330e06b7eb52e8db63d4f824ad77427/DirectXTex/BC6HBC7.cpp
// Values checked against Khronos ARB_texture_compression_bptc revision9,
// Tables P2/P3/A2/A3a/A3b (specification data, not decoder implementation).
// The following license applies to the adapted table data:
// MIT License
//
// Copyright (c) Microsoft Corporation.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE

const BC7_PARTITION_TWO: [u32; 64] = [
    0x50505050, 0x40404040, 0x54545454, 0x54505040, 0x50404000, 0x55545450, 0x55545040, 0x54504000,
    0x50400000, 0x55555450, 0x55544000, 0x54400000, 0x55555440, 0x55550000, 0x55555500, 0x55000000,
    0x55150100, 0x00004054, 0x15010000, 0x00405054, 0x00004050, 0x15050100, 0x05010000, 0x40505054,
    0x00404050, 0x05010100, 0x14141414, 0x05141450, 0x01155440, 0x00555500, 0x15014054, 0x05414150,
    0x44444444, 0x55005500, 0x11441144, 0x05055050, 0x05500550, 0x11114444, 0x41144114, 0x44111144,
    0x15055054, 0x01055040, 0x05041050, 0x05455150, 0x14414114, 0x50050550, 0x41411414, 0x00141400,
    0x00041504, 0x00105410, 0x10541000, 0x04150400, 0x50410514, 0x41051450, 0x05415014, 0x14054150,
    0x41050514, 0x41505014, 0x40011554, 0x54150140, 0x50505500, 0x00555050, 0x15151010, 0x54540404,
];

const BC7_PARTITION_THREE: [u32; 64] = [
    0xaa685050, 0x6a5a5040, 0x5a5a4200, 0x5450a0a8, 0xa5a50000, 0xa0a05050, 0x5555a0a0, 0x5a5a5050,
    0xaa550000, 0xaa555500, 0xaaaa5500, 0x90909090, 0x94949494, 0xa4a4a4a4, 0xa9a59450, 0x2a0a4250,
    0xa5945040, 0x0a425054, 0xa5a5a500, 0x55a0a0a0, 0xa8a85454, 0x6a6a4040, 0xa4a45000, 0x1a1a0500,
    0x0050a4a4, 0xaaa59090, 0x14696914, 0x69691400, 0xa08585a0, 0xaa821414, 0x50a4a450, 0x6a5a0200,
    0xa9a58000, 0x5090a0a8, 0xa8a09050, 0x24242424, 0x00aa5500, 0x24924924, 0x24499224, 0x50a50a50,
    0x500aa550, 0xaaaa4444, 0x66660000, 0xa5a0a5a0, 0x50a050a0, 0x69286928, 0x44aaaa44, 0x66666600,
    0xaa444444, 0x54a854a8, 0x95809580, 0x96969600, 0xa85454a8, 0x80959580, 0xaa141414, 0x96960000,
    0xaaaa1414, 0xa05050a0, 0xa0a5a5a0, 0x96000000, 0x40804080, 0xa9a8a9a8, 0xaaaaaa44, 0x2a4a5254,
];

const BC7_ANCHOR_TWO: [u8; 64] = [
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 2, 8, 2, 2, 8, 8, 15, 2, 8,
    2, 2, 8, 8, 2, 2, 15, 15, 6, 8, 2, 8, 15, 15, 2, 8, 2, 2, 2, 15, 15, 6, 6, 2, 6, 8, 15, 15, 2,
    2, 15, 15, 15, 15, 15, 2, 2, 15,
];

const BC7_ANCHOR_THREE_SECOND: [u8; 64] = [
    3, 3, 15, 15, 8, 3, 15, 15, 8, 8, 6, 6, 6, 5, 3, 3, 3, 3, 8, 15, 3, 3, 6, 10, 5, 8, 8, 6, 8, 5,
    15, 15, 8, 15, 3, 5, 6, 10, 8, 15, 15, 3, 15, 5, 15, 15, 15, 15, 3, 15, 5, 5, 5, 8, 5, 10, 5,
    10, 8, 13, 15, 12, 3, 3,
];

const BC7_ANCHOR_THREE_THIRD: [u8; 64] = [
    15, 8, 8, 3, 15, 15, 3, 8, 15, 15, 15, 15, 15, 15, 15, 8, 15, 8, 15, 3, 15, 8, 15, 8, 3, 15, 6,
    10, 15, 15, 10, 8, 15, 3, 15, 10, 10, 8, 9, 10, 6, 15, 8, 15, 3, 6, 6, 8, 15, 3, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 3, 15, 15, 8,
];

// BC7 single-subset modes have no partition tables. Fields are consumed LSB
// first, in the order illustrated by Microsoft's BC7 Format Mode Reference:
// https://learn.microsoft.com/en-us/windows/win32/direct3d11/bc7-format-mode-reference
// The endpoint expansion, fix-up index and interpolation are specified in:
// https://learn.microsoft.com/en-us/windows/win32/direct3d11/bc7-format
fn decode_bc7(block: &[u8]) -> Result<[[u8; 4]; 16], Error> {
    let bytes: [u8; 16] = block.try_into().map_err(|_| {
        err(
            ImgErrorKind::IllegalData,
            "DDS BC7 block must contain 16 bytes",
        )
    })?;
    let mode = bytes[0].trailing_zeros();
    // The reserved mode (no set bit in the first byte) produces zero texels.
    if mode == 8 {
        return Ok([[0; 4]; 16]);
    }
    if !matches!(mode, 4..=6) {
        return Ok(decode_bc7_partitioned(bytes, mode));
    }
    let mut bits = u128::from_le_bytes(bytes) >> (mode + 1);
    let mut take = |count: u32| {
        let value = (bits & ((1u128 << count) - 1)) as u8;
        bits >>= count;
        value
    };
    let rotation = if mode == 6 { 0 } else { take(2) };
    let selector = if mode == 4 { take(1) } else { 0 };
    let color_bits = if mode == 4 { 5 } else { 7 };
    let alpha_bits = match mode {
        4 => 6,
        5 => 8,
        _ => 7,
    };
    let mut endpoints = [[0u8; 4]; 2];
    for channel in 0..4 {
        let count = if channel == 3 { alpha_bits } else { color_bits };
        for endpoint in &mut endpoints {
            endpoint[channel] = take(count);
        }
    }
    if mode == 6 {
        for endpoint in &mut endpoints {
            let pbit = take(1);
            for value in endpoint {
                *value = (*value << 1) | pbit;
            }
        }
    } else {
        for endpoint in &mut endpoints {
            for (channel, value) in endpoint.iter_mut().enumerate() {
                let count = if channel == 3 { alpha_bits } else { color_bits };
                let aligned = u16::from(*value) << (8 - count);
                *value = (aligned | (aligned >> count)) as u8;
            }
        }
    }
    let first_width = if mode == 6 { 4 } else { 2 };
    let mut first = [0u8; 16];
    for (pixel, index) in first.iter_mut().enumerate() {
        *index = take(first_width - u32::from(pixel == 0));
    }
    let second_width = if mode == 4 { 3 } else { 2 };
    let mut second = [0u8; 16];
    if mode != 6 {
        for (pixel, index) in second.iter_mut().enumerate() {
            *index = take(second_width - u32::from(pixel == 0));
        }
    }
    // Mode 4's selector assigns the first (2-bit) stream to color when zero,
    // and to alpha when one. Mode 5 always puts color in the first stream.
    let weights = |width: u32, index: u8| -> u16 {
        const TWO: [u16; 4] = [0, 21, 43, 64];
        const THREE: [u16; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
        const FOUR: [u16; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];
        match width {
            2 => TWO[index as usize],
            3 => THREE[index as usize],
            _ => FOUR[index as usize],
        }
    };
    let mut output = [[0u8; 4]; 16];
    for (pixel, rgba) in output.iter_mut().enumerate() {
        for (channel, value) in rgba.iter_mut().enumerate() {
            let use_second = mode != 6 && ((channel == 3) != (selector != 0));
            let weight = if use_second {
                weights(second_width, second[pixel])
            } else {
                weights(first_width, first[pixel])
            };
            *value = (((64 - weight) * u16::from(endpoints[0][channel])
                + weight * u16::from(endpoints[1][channel])
                + 32)
                >> 6) as u8;
        }
        if rotation != 0 {
            rgba.swap(3, usize::from(rotation - 1));
        }
    }
    Ok(output)
}

fn decode_bc7_partitioned(bytes: [u8; 16], mode: u32) -> [[u8; 4]; 16] {
    let (subsets, partition_bits, color_bits, index_bits, has_pbits) = match mode {
        0 => (3, 4, 4, 3, true),
        1 => (2, 6, 6, 3, true),
        2 => (3, 6, 5, 2, false),
        3 => (2, 6, 7, 2, true),
        7 => (2, 6, 5, 2, true),
        _ => unreachable!("single-subset and reserved BC7 modes are handled separately"),
    };
    let mut bits = u128::from_le_bytes(bytes) >> (mode + 1);
    let mut take = |count: u32| {
        let value = (bits & ((1u128 << count) - 1)) as u8;
        bits >>= count;
        value
    };
    let partition = usize::from(take(partition_bits));
    let channels = if mode == 7 { 4 } else { 3 };
    let endpoint_count = subsets * 2;
    let mut endpoints = [[0u8; 4]; 6];
    for channel in 0..channels {
        for endpoint in &mut endpoints[..endpoint_count] {
            endpoint[channel] = take(color_bits);
        }
    }
    if has_pbits {
        if mode == 1 {
            // Shared P-bits apply to both RGB endpoints of each subset.
            for pair in endpoints[..endpoint_count].chunks_exact_mut(2) {
                let pbit = take(1);
                for endpoint in pair {
                    for value in &mut endpoint[..channels] {
                        *value = (*value << 1) | pbit;
                    }
                }
            }
        } else {
            for endpoint in &mut endpoints[..endpoint_count] {
                let pbit = take(1);
                for value in &mut endpoint[..channels] {
                    *value = (*value << 1) | pbit;
                }
            }
        }
    }
    let precision = color_bits + u32::from(has_pbits);
    for endpoint in &mut endpoints[..endpoint_count] {
        for value in &mut endpoint[..channels] {
            let aligned = u16::from(*value) << (8 - precision);
            *value = (aligned | (aligned >> precision)) as u8;
        }
        if channels == 3 {
            endpoint[3] = 255;
        }
    }
    let (pattern, anchors) = if subsets == 2 {
        (
            BC7_PARTITION_TWO[partition],
            [0, BC7_ANCHOR_TWO[partition], 0],
        )
    } else {
        (
            BC7_PARTITION_THREE[partition],
            [
                0,
                BC7_ANCHOR_THREE_SECOND[partition],
                BC7_ANCHOR_THREE_THIRD[partition],
            ],
        )
    };
    const WEIGHTS_TWO: [u16; 4] = [0, 21, 43, 64];
    const WEIGHTS_THREE: [u16; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
    let mut output = [[0u8; 4]; 16];
    for (pixel, rgba) in output.iter_mut().enumerate() {
        let subset = ((pattern >> (pixel * 2)) & 3) as usize;
        let index = usize::from(take(
            index_bits - u32::from(usize::from(anchors[subset]) == pixel),
        ));
        let weight = if index_bits == 2 {
            WEIGHTS_TWO[index]
        } else {
            WEIGHTS_THREE[index]
        };
        for (channel, value) in rgba.iter_mut().enumerate() {
            *value = (((64 - weight) * u16::from(endpoints[subset * 2][channel])
                + weight * u16::from(endpoints[subset * 2 + 1][channel])
                + 32)
                >> 6) as u8;
        }
    }
    output
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
        Compression::Bc7 => decode_bc7(block),
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
        98 | 99 => Some(Compression::Bc7),
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
            output[dst..dst + 4].copy_from_slice(&[r, g, b, alpha]);
        }
    }
    Ok(output)
}

// Normalize only explicitly premultiplied/opaque images; unknown and straight
// paths keep the original callback sequence and skip this helper.
#[inline(never)]
fn finish_dx10_alpha(data: &[u8], output: &mut [u8]) -> Result<(), Error> {
    if &data[84..88] != b"DX10" || le32(data, 80)? & 4 == 0 {
        return Ok(());
    }
    // Already validated before any pixel allocation; the input is immutable.
    let mode = le32(data, 144)?;
    match mode {
        2 => {
            for pixel in output.chunks_exact_mut(4) {
                let alpha = pixel[3];
                for channel in &mut pixel[..3] {
                    *channel = unpremultiply(*channel, alpha);
                }
            }
        }
        3 => {
            for pixel in output.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
        }
        _ => {}
    }
    Ok(())
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
    let header_flags = le32(&data, 8)?;
    let pitch_or_linear = usize::try_from(le32(&data, 20)?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "DDS pitch is invalid"))?;
    let mipmaps = le32(&data, 28)?;
    if le32(&data, 76)? != 32 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "DDS pixel format size is invalid",
        ));
    }
    let pf_flags = le32(&data, 80)?;
    let fourcc = &data[84..88];
    // The callback exposes a single 2D image, not faces or volume slices.
    // DX10 has its own resource checks below; old-style files use caps2.
    if fourcc != b"DX10" || pf_flags & 0x4 == 0 {
        let caps2 = le32(&data, 112)?;
        if caps2 & 0x0000_fe00 != 0 {
            return Err(err(
                ImgErrorKind::NoSupportFormat,
                "Unsupported DDS legacy cubemap",
            ));
        }
        if caps2 & 0x0020_0000 != 0 || header_flags & 0x0080_0000 != 0 {
            return Err(err(
                ImgErrorKind::NoSupportFormat,
                "Unsupported DDS legacy volume texture",
            ));
        }
    }
    let mut data_offset = 128usize;
    let compression = if pf_flags & 0x4 != 0 {
        if fourcc == b"DX10" {
            let dxgi = le32(&data, 128)?;
            let resource_dimension = le32(&data, 132)?;
            let misc_flags = le32(&data, 136)?;
            let array_size = le32(&data, 140)?;
            // Microsoft DDS_HEADER_DXT10: the low three bits specify alpha;
            // unknown retains the legacy straight-alpha assumption. Reserved
            // bits and custom fourth-channel semantics cannot be interpreted
            // by this RGBA8 callback path.
            let flags2 = le32(&data, 144)?;
            if flags2 > 3 {
                return Err(err(
                    ImgErrorKind::NoSupportFormat,
                    "Unsupported DDS DX10 alpha mode or reserved flags",
                ));
            }
            if resource_dimension != 3 {
                return Err(err(
                    ImgErrorKind::NoSupportFormat,
                    "Unsupported DDS DX10 resource dimension",
                ));
            }
            if array_size != 1 {
                return Err(err(
                    ImgErrorKind::NoSupportFormat,
                    "Unsupported DDS DX10 array size",
                ));
            }
            if misc_flags & 0x4 != 0 {
                return Err(err(
                    ImgErrorKind::NoSupportFormat,
                    "Unsupported DDS DX10 cubemap",
                ));
            }
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
    let mut output = if matches!(compression, Compression::Uncompressed(_)) {
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
            if header_flags & 0x8 != 0 {
                pitch_or_linear
            } else {
                width.checked_mul(bytes_per_pixel).ok_or_else(|| {
                    err(ImgErrorKind::InvalidParameter, "DDS scanline size overflow")
                })?
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
        let image_end = data_offset.checked_add(image_bytes).ok_or_else(|| {
            err(
                ImgErrorKind::InvalidParameter,
                "DDS block image offset overflow",
            )
        })?;
        let compressed = data
            .get(data_offset..image_end)
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
    if &data[84..88] == b"DX10" && le32(&data, 80)? & 4 != 0 && matches!(le32(&data, 144)?, 2 | 3) {
        finish_dx10_alpha(&data, &mut output)?;
    }
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

/// Encodes one 2D image as an uncompressed legacy DDS with 32-bit BGRA storage.
///
/// The source callback supplies straight RGBA8. All four channels are preserved;
/// rows are stored top to bottom with no padding, mipmaps, or extra surfaces.
/// No compression or encoder options are currently supported.
pub fn encode(image: &mut EncodeOptions<'_>) -> Result<Vec<u8>, Error> {
    let profile = image
        .drawer
        .encode_start(None)?
        .ok_or_else(|| err(ImgErrorKind::EncodeError, "DDS source profile is missing"))?;
    let width = u32::try_from(profile.width)
        .map_err(|_| err(ImgErrorKind::InvalidParameter, "DDS width exceeds u32"))?;
    let height = u32::try_from(profile.height)
        .map_err(|_| err(ImgErrorKind::InvalidParameter, "DDS height exceeds u32"))?;
    if width == 0 || height == 0 {
        return Err(err(
            ImgErrorKind::InvalidParameter,
            "DDS dimensions must be non-zero",
        ));
    }
    let pitch = width
        .checked_mul(4)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "DDS pitch exceeds u32"))?;
    let row_len = usize::try_from(pitch)?;
    let pixels_len = checked_output_len(profile.width, profile.height)?;
    let total = pixels_len
        .checked_add(128)
        .ok_or_else(|| err(ImgErrorKind::InvalidParameter, "DDS encoded size overflow"))?;
    crate::limits::check(
        total,
        crate::limits::current().expanded_bytes,
        "encoded DDS",
    )?;
    let mut data = Vec::new();
    data.try_reserve_exact(total)
        .map_err(|_| err(ImgErrorKind::EncodeError, "DDS output allocation failed"))?;
    data.resize(128, 0);
    data[..4].copy_from_slice(b"DDS ");
    for (offset, value) in [
        (4, 124),
        (8, 0x100f),
        (12, height),
        (16, width),
        (20, pitch),
        (76, 32),
        (80, 0x41),
        (88, 32),
        (92, 0x00ff_0000),
        (96, 0x0000_ff00),
        (100, 0x0000_00ff),
        (104, 0xff00_0000),
        (108, 0x1000),
    ] {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    for y in 0..profile.height {
        let row = image
            .drawer
            .encode_pick(0, y, profile.width, 1, None)?
            .ok_or_else(|| err(ImgErrorKind::EncodeError, "DDS source row is missing"))?;
        if row.len() != row_len {
            return Err(err(
                ImgErrorKind::EncodeError,
                "DDS source row has an invalid RGBA8 length",
            ));
        }
        for pixel in row.chunks_exact(4) {
            data.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    image.drawer.encode_end(None)?;
    Ok(data)
}
