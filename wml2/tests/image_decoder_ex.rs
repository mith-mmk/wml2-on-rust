#![cfg(feature = "image-buffer-ex")]
use wml2::draw::*;

#[cfg(any(feature = "png", feature = "tiff"))]
fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[cfg(feature = "png")]
mod png {
    use super::*;
    use wml2::png::utils::CRC32;
    fn chunk(out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        out.extend_from_slice(
            &CRC32::new()
                .crc32(&[name.as_slice(), data].concat())
                .to_be_bytes(),
        );
    }
    pub(super) fn fixture(
        size: (u32, u32),
        depth: u8,
        color: u8,
        interlace: u8,
        raw: &[u8],
        extra: &[(&[u8; 4], &[u8])],
    ) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut header = size.0.to_be_bytes().to_vec();
        header.extend_from_slice(&size.1.to_be_bytes());
        header.extend_from_slice(&[depth, color, 0, 0, interlace]);
        chunk(&mut out, b"IHDR", &header);
        for &(name, data) in extra {
            chunk(&mut out, name, data);
        }
        chunk(
            &mut out,
            b"IDAT",
            &miniz_oxide::deflate::compress_to_vec_zlib(raw, 6),
        );
        chunk(&mut out, b"IEND", &[]);
        out
    }
    fn decode(bytes: &[u8], format: PixelFormatEx) -> ImageBufferEx {
        image_from_ex(bytes, format, PrecisionConversion::Exact).unwrap()
    }
    #[test]
    fn gray_and_rgba_eight_sixteen_preserve_low_bits_and_legacy_high_bytes() {
        for (depth, gray, rgba, g, c) in [
            (
                8,
                PixelFormatEx::Gray8,
                PixelFormatEx::Rgba8,
                vec![0, 1, 128, 255],
                vec![1, 18, 171, 128, 255, 0, 127, 0],
            ),
            (
                16,
                PixelFormatEx::Gray16,
                PixelFormatEx::Rgba16,
                vec![0, 0, 0, 1, 0x12, 0x34, 255, 255],
                vec![
                    0, 1, 0x12, 0x34, 0xab, 0xcd, 0x80, 1, 255, 255, 0, 0, 0x7f, 255, 0, 0,
                ],
            ),
        ] {
            let mut raw = vec![0];
            raw.extend(g);
            let input = fixture((4, 1), depth, 0, 0, &raw, &[]);
            let image = decode(&input, gray);
            let expected = if depth == 8 {
                vec![0, 1, 128, 255]
            } else {
                words(&[0, 1, 0x1234, 65535])
            };
            assert_eq!(image.bytes().unwrap(), expected);
            assert_eq!(image.stride(), expected.len());
            let legacy = image_from(&input).unwrap();
            assert_eq!(
                legacy.buffer.unwrap(),
                if depth == 8 {
                    vec![
                        0, 0, 0, 255, 1, 1, 1, 255, 128, 128, 128, 255, 255, 255, 255, 255,
                    ]
                } else {
                    vec![
                        0, 0, 0, 255, 0, 0, 0, 255, 18, 18, 18, 255, 255, 255, 255, 255,
                    ]
                }
            );
            let mut raw = vec![0];
            raw.extend(c);
            let input = fixture((2, 1), depth, 6, 0, &raw, &[]);
            let image = decode(&input, rgba);
            assert_eq!(
                image.bytes().unwrap(),
                if depth == 8 {
                    vec![1, 18, 171, 128, 255, 0, 127, 0]
                } else {
                    words(&[1, 0x1234, 0xabcd, 0x8001, 65535, 0, 0x7fff, 0])
                }
            );
            assert_eq!(
                image_from(&input).unwrap().buffer.unwrap(),
                if depth == 8 {
                    vec![1, 18, 171, 128, 255, 0, 127, 0]
                } else {
                    vec![0, 18, 171, 128, 255, 0, 127, 0]
                }
            );
        }
    }
    #[test]
    fn gray_alpha_rgb_transparency_and_palette_expansion_are_sample_based() {
        for depth in [8, 16] {
            let (ga, rgb, expected_ga, expected_rgb) = if depth == 8 {
                (
                    vec![0, 18, 128, 171, 0],
                    vec![0, 1, 18, 171],
                    vec![18, 18, 18, 128, 171, 171, 171, 0],
                    vec![1, 18, 171, 255],
                )
            } else {
                (
                    vec![0, 0x12, 0x34, 0x80, 1, 0xab, 0xcd, 0, 0],
                    vec![0, 0, 1, 0x12, 0x34, 0xab, 0xcd],
                    words(&[0x1234, 0x1234, 0x1234, 0x8001, 0xabcd, 0xabcd, 0xabcd, 0]),
                    words(&[1, 0x1234, 0xabcd, 65535]),
                )
            };
            let f = if depth == 8 {
                PixelFormatEx::Rgba8
            } else {
                PixelFormatEx::Rgba16
            };
            assert_eq!(
                decode(&fixture((2, 1), depth, 4, 0, &ga, &[]), f)
                    .bytes()
                    .unwrap(),
                expected_ga
            );
            assert_eq!(
                decode(&fixture((1, 1), depth, 2, 0, &rgb, &[]), f)
                    .bytes()
                    .unwrap(),
                expected_rgb
            );
        }
        let input = fixture(
            (2, 1),
            16,
            0,
            0,
            &[0, 0x12, 0x34, 0x12, 0x35],
            &[(b"tRNS", &[0x12, 0x34])],
        );
        assert_eq!(
            decode(&input, PixelFormatEx::Rgba16).bytes().unwrap(),
            words(&[0x1234, 0x1234, 0x1234, 0, 0x1235, 0x1235, 0x1235, 65535])
        );
        let input = fixture(
            (2, 1),
            8,
            3,
            0,
            &[0, 0, 1],
            &[(b"PLTE", &[1, 2, 3, 4, 5, 6]), (b"tRNS", &[0, 128])],
        );
        assert_eq!(
            decode(&input, PixelFormatEx::Rgba8).bytes().unwrap(),
            [1, 2, 3, 0, 4, 5, 6, 128]
        );
        assert_eq!(
            decode(
                &fixture((4, 1), 2, 0, 0, &[0, 0b00011011], &[]),
                PixelFormatEx::Gray8
            )
            .bytes()
            .unwrap(),
            [0, 85, 170, 255]
        );
    }
    #[test]
    fn filters_and_adam7_preserve_sixteen_bit_samples() {
        // PNG filters operate on big-endian encoded bytes, not decoded samples.
        let row0 = [0, 1, 0x12, 0x34, 0xab, 0xcd, 0x80, 1];
        let row1: [u8; 8] = [255, 255, 0x13, 0x35, 0, 2, 0, 0];
        for filter in 0..=4 {
            let mut raw = vec![0];
            raw.extend(row0);
            raw.push(filter);
            for i in 0..row1.len() {
                let a = 0i32;
                let b = i32::from(row0[i]);
                let c = 0i32;
                let predict = match filter {
                    0 => 0,
                    1 => a,
                    2 => b,
                    3 => (a + b) / 2,
                    _ => {
                        let p = a + b - c;
                        let distances = [(p - a).abs(), (p - b).abs(), (p - c).abs()];
                        if distances[0] <= distances[1] && distances[0] <= distances[2] {
                            a
                        } else if distances[1] <= distances[2] {
                            b
                        } else {
                            c
                        }
                    }
                };
                raw.push(row1[i].wrapping_sub(predict as u8));
            }
            let input = fixture((1, 2), 16, 6, 0, &raw, &[]);
            assert_eq!(
                decode(&input, PixelFormatEx::Rgba16).bytes().unwrap(),
                words(&[1, 0x1234, 0xabcd, 0x8001, 65535, 0x1335, 2, 0])
            );
        }
        let passes = [
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ];
        let mut raw = Vec::new();
        for (sx, sy, dx, dy) in passes {
            for y in (sy..9).step_by(dy) {
                raw.push(0);
                for x in (sx..9).step_by(dx) {
                    raw.extend_from_slice(&((y * 9 + x) * 257 + 1u16).to_be_bytes());
                }
            }
        }
        let image = decode(&fixture((9, 9), 16, 0, 1, &raw, &[]), PixelFormatEx::Gray16);
        assert_eq!(
            image.bytes().unwrap(),
            words(&(0..81).map(|i| i * 257 + 1).collect::<Vec<_>>())
        );
    }
    #[test]
    fn explicit_precision_picker_stride_background_and_limits() {
        let input = fixture(
            (1, 1),
            16,
            6,
            0,
            &[0, 0, 1, 0x12, 0x34, 0xab, 0xcd, 0x80, 1],
            &[(b"bKGD", &[0x12, 0x34, 0xab, 0xcd, 0, 1])],
        );
        assert!(image_from_ex(&input, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_err());
        let mut image = decode(&input, PixelFormatEx::Rgba16);
        assert!(
            image
                .encode_start_ex(None)
                .unwrap()
                .unwrap()
                .legacy
                .background
                .is_none()
        );
        let metadata = image.metadata_ex().unwrap().unwrap();
        assert!(
            matches!(metadata.get("PNG Background"),Some(wml2::metadata::DataMap::Raw(v)) if v==&[0x12,0x34,0xab,0xcd,0,1])
        );
        let picked = image
            .encode_pick_ex(
                0,
                0,
                1,
                1,
                PixelOutputEx {
                    format: PixelFormatEx::Rgba8,
                    stride: Some(5),
                    conversion: PrecisionConversion::Round,
                },
                None,
            )
            .unwrap()
            .unwrap();
        assert_eq!(picked.bytes(), [0, 18, 171, 128, 0]);
        let rounded =
            image_from_ex(&input, PixelFormatEx::Rgba8, PrecisionConversion::Round).unwrap();
        assert_eq!(rounded.bytes().unwrap(), [0, 18, 171, 128]);
        // Normalized rounding differs from the unchanged legacy high-byte rule.
        let near = fixture((2, 1), 16, 0, 0, &[0, 0, 255, 1, 0], &[]);
        assert_eq!(
            image_from_ex(&near, PixelFormatEx::Gray8, PrecisionConversion::Round)
                .unwrap()
                .bytes()
                .unwrap(),
            [1, 1]
        );
        assert_eq!(
            image_from(&near).unwrap().buffer.unwrap(),
            [0, 0, 0, 255, 1, 1, 1, 255]
        );
        // ICC stays opaque source metadata even if Gray is mechanically replicated.
        let profile = b"uninterpreted source profile";
        let mut iccp = b"profile\0\0".to_vec();
        iccp.extend(miniz_oxide::deflate::compress_to_vec_zlib(profile, 6));
        let tagged = fixture((1, 1), 16, 0, 0, &[0, 0x12, 0x34], &[(b"iCCP", &iccp)]);
        let mut native = decode(&tagged, PixelFormatEx::Rgba16);
        assert_eq!(
            native.bytes().unwrap(),
            words(&[0x1234, 0x1234, 0x1234, 65535])
        );
        let tags = native.metadata_ex().unwrap().unwrap();
        assert!(matches!(tags.get("Source ICC Profile"),
            Some(wml2::metadata::DataMap::ICCProfile(bytes)) if bytes == profile));
        assert!(!tags.contains_key("ICC Profile"));
        let limits = wml2::limits::DecodeLimits {
            expanded_bytes: 4,
            ..wml2::limits::DecodeLimits::unlimited()
        };
        assert!(
            image_from_ex_with_limits(
                &input,
                PixelFormatEx::Rgba8,
                PrecisionConversion::Round,
                limits
            )
            .is_err()
        );
        let limits = wml2::limits::DecodeLimits {
            input_bytes: input.len() - 1,
            ..wml2::limits::DecodeLimits::unlimited()
        };
        assert!(
            image_from_ex_with_limits(
                &input,
                PixelFormatEx::Rgba16,
                PrecisionConversion::Exact,
                limits
            )
            .is_err()
        );
        assert!(
            image_from_ex(
                &fixture((1, 1), 16, 0, 0, &[0, 1], &[]),
                PixelFormatEx::Gray16,
                PrecisionConversion::Exact
            )
            .is_err()
        );
        let apng = fixture(
            (1, 1),
            8,
            6,
            0,
            &[0, 1, 2, 3, 255],
            &[(b"acTL", &[0, 0, 0, 1, 0, 0, 0, 0])],
        );
        assert!(image_from_ex(&apng, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_err());
    }
}

#[cfg(feature = "tiff")]
mod tiff {
    use super::*;
    fn put16(n: u16, be: bool) -> [u8; 2] {
        if be { n.to_be_bytes() } else { n.to_le_bytes() }
    }
    fn put32(n: u32, be: bool) -> [u8; 4] {
        if be { n.to_be_bytes() } else { n.to_le_bytes() }
    }
    // Independent classic-TIFF writer: only tags and sample bytes, no WML2 encoder.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fixture(
        size: (usize, usize),
        depth: u16,
        photo: u16,
        alpha: Option<u16>,
        be: bool,
        planar: bool,
        predictor: bool,
        tiled: bool,
        compressed: bool,
        values: &[u16],
    ) -> Vec<u8> {
        let (width, height) = size;
        let channels = if photo == 2 { 3 } else { 1 } + usize::from(alpha.is_some());
        assert_eq!(values.len(), width * height * channels);
        let stored_w = if tiled { 2 } else { width };
        let stored_h = if tiled { 2 } else { height };
        let planes = if planar { channels } else { 1 };
        let blocks = if tiled { width.div_ceil(2) } else { 1 };
        let block_channels = if planar { 1 } else { channels };
        let mut payloads = Vec::new();
        for plane in 0..planes {
            for block in 0..blocks {
                let mut data = Vec::new();
                for y in 0..stored_h {
                    let mut previous = vec![0u16; block_channels];
                    for x in 0..stored_w {
                        for c in 0..block_channels {
                            let px = if tiled { block * 2 + x } else { x };
                            let value = if px < width && y < height {
                                values[(y * width + px) * channels + if planar { plane } else { c }]
                            } else {
                                0
                            };
                            let encoded = if predictor {
                                value.wrapping_sub(previous[c])
                            } else {
                                value
                            };
                            previous[c] = value;
                            if depth == 8 {
                                data.push(encoded as u8);
                            } else {
                                data.extend_from_slice(&put16(encoded, be));
                            }
                        }
                    }
                }
                payloads.push(if compressed {
                    miniz_oxide::deflate::compress_to_vec_zlib(&data, 6)
                } else {
                    data
                });
            }
        }
        let shorts = |v: &[u16]| v.iter().flat_map(|n| put16(*n, be)).collect::<Vec<_>>();
        let longs = |v: &[u32]| v.iter().flat_map(|n| put32(*n, be)).collect::<Vec<_>>();
        let mut tags = vec![
            (256, 4, 1, longs(&[width as u32])),
            (257, 4, 1, longs(&[height as u32])),
            (258, 3, channels, shorts(&vec![depth; channels])),
            (259, 3, 1, shorts(&[if compressed { 8 } else { 1 }])),
            (262, 3, 1, shorts(&[photo])),
            (274, 3, 1, shorts(&[6])),
            (277, 3, 1, shorts(&[channels as u16])),
            (284, 3, 1, shorts(&[if planar { 2 } else { 1 }])),
            (317, 3, 1, shorts(&[if predictor { 2 } else { 1 }])),
            (339, 3, channels, shorts(&vec![1; channels])),
        ];
        if let Some(alpha) = alpha {
            tags.push((338, 3, 1, shorts(&[alpha])));
        }
        let offset_tag = if tiled { 324 } else { 273 };
        tags.push((
            offset_tag,
            4,
            payloads.len(),
            longs(&vec![0; payloads.len()]),
        ));
        tags.push((
            if tiled { 325 } else { 279 },
            4,
            payloads.len(),
            longs(&payloads.iter().map(|d| d.len() as u32).collect::<Vec<_>>()),
        ));
        if tiled {
            tags.push((322, 4, 1, longs(&[2])));
            tags.push((323, 4, 1, longs(&[2])));
        } else {
            tags.push((278, 4, 1, longs(&[height as u32])));
        }
        tags.sort_by_key(|t| t.0);
        let directory_end = 8 + 2 + 12 * tags.len() + 4;
        let pixel_start = directory_end
            + tags
                .iter()
                .filter(|t| t.3.len() > 4)
                .map(|t| t.3.len())
                .sum::<usize>();
        let mut offset = pixel_start;
        let offsets = payloads
            .iter()
            .map(|p| {
                let n = offset as u32;
                offset += p.len();
                n
            })
            .collect::<Vec<_>>();
        tags.iter_mut().find(|t| t.0 == offset_tag).unwrap().3 = longs(&offsets);
        let mut out = if be { b"MM".to_vec() } else { b"II".to_vec() };
        out.extend_from_slice(&put16(42, be));
        out.extend_from_slice(&put32(8, be));
        out.extend_from_slice(&put16(tags.len() as u16, be));
        let mut extras = Vec::new();
        for (tag, kind, count, data) in tags {
            out.extend_from_slice(&put16(tag, be));
            out.extend_from_slice(&put16(kind, be));
            out.extend_from_slice(&put32(count as u32, be));
            if data.len() <= 4 {
                out.extend_from_slice(&data);
                out.resize(out.len() + 4 - data.len(), 0);
            } else {
                out.extend_from_slice(&put32((directory_end + extras.len()) as u32, be));
                extras.extend(data);
            }
        }
        out.extend_from_slice(&put32(0, be));
        out.extend(extras);
        for p in payloads {
            out.extend(p);
        }
        out
    }
    #[test]
    fn native_gray_rgba_endian_planar_predictor_deflate_and_edge_tiles() {
        for depth in [8, 16] {
            for be in [false, true] {
                for planar in [false, true] {
                    for tiled in [false, true] {
                        for predicted in [false, true] {
                            let values = if depth == 8 {
                                vec![
                                    1, 18, 171, 128, 255, 0, 127, 0, 4, 5, 6, 7, 8, 9, 10, 11, 12,
                                    13, 14, 15, 16, 17, 18, 19,
                                ]
                            } else {
                                vec![
                                    1, 0x1234, 0xabcd, 0x8001, 65535, 0, 0x7fff, 0, 4, 5, 6, 7, 8,
                                    9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
                                ]
                            };
                            let data = fixture(
                                (3, 2),
                                depth,
                                2,
                                Some(2),
                                be,
                                planar,
                                predicted,
                                tiled,
                                predicted,
                                &values,
                            );
                            let format = if depth == 8 {
                                PixelFormatEx::Rgba8
                            } else {
                                PixelFormatEx::Rgba16
                            };
                            let mut image =
                                image_from_ex(&data, format, PrecisionConversion::Exact).unwrap();
                            let expected = if depth == 8 {
                                values.iter().map(|v| *v as u8).collect()
                            } else {
                                words(&values)
                            };
                            assert_eq!(
                                image.bytes().unwrap(),
                                expected,
                                "{depth} BE{be} planar{planar} tiled{tiled} predictor{predicted}"
                            );
                            assert!(matches!(
                                image.metadata_ex().unwrap().unwrap().get("Orientation"),
                                Some(wml2::metadata::DataMap::UInt(6))
                            ));
                            let picked = image
                                .encode_pick_ex(
                                    2,
                                    1,
                                    1,
                                    1,
                                    PixelOutputEx {
                                        format,
                                        stride: Some(format.bytes_per_pixel() + 1),
                                        conversion: PrecisionConversion::Exact,
                                    },
                                    None,
                                )
                                .unwrap()
                                .unwrap();
                            let mut last = if depth == 8 {
                                vec![16, 17, 18, 19]
                            } else {
                                words(&[16, 17, 18, 19])
                            };
                            last.push(0);
                            assert_eq!(picked.bytes(), last);
                        }
                    }
                }
            }
        }
        for depth in [8, 16] {
            for photo in [0, 1] {
                let values = if depth == 8 {
                    vec![0, 1, 18, 255]
                } else {
                    vec![0, 1, 0x1234, 65535]
                };
                let format = if depth == 8 {
                    PixelFormatEx::Gray8
                } else {
                    PixelFormatEx::Gray16
                };
                let data = fixture(
                    (2, 2),
                    depth,
                    photo,
                    None,
                    false,
                    false,
                    true,
                    false,
                    true,
                    &values,
                );
                let image = image_from_ex(&data, format, PrecisionConversion::Exact).unwrap();
                let expected = match (depth, photo) {
                    (8, 0) => vec![255, 254, 237, 0],
                    (8, _) => vec![0, 1, 18, 255],
                    (16, 0) => words(&[65535, 65534, 0xedcb, 0]),
                    _ => words(&[0, 1, 0x1234, 65535]),
                };
                assert_eq!(image.bytes().unwrap(), expected);
            }
        }
    }
    #[test]
    fn gray_alpha_and_rgb_expand_without_changing_samples() {
        for depth in [8, 16] {
            let (values, expected) = if depth == 8 {
                (
                    vec![18, 128, 171, 0],
                    vec![18, 18, 18, 128, 171, 171, 171, 0],
                )
            } else {
                (
                    vec![0x1234, 0x8001, 0xabcd, 0],
                    words(&[0x1234, 0x1234, 0x1234, 0x8001, 0xabcd, 0xabcd, 0xabcd, 0]),
                )
            };
            let format = if depth == 8 {
                PixelFormatEx::Rgba8
            } else {
                PixelFormatEx::Rgba16
            };
            let image = image_from_ex(
                &fixture(
                    (2, 1),
                    depth,
                    1,
                    Some(2),
                    true,
                    true,
                    true,
                    false,
                    true,
                    &values,
                ),
                format,
                PrecisionConversion::Exact,
            )
            .unwrap();
            assert_eq!(image.bytes().unwrap(), expected);
            let vals = if depth == 8 {
                vec![1, 18, 171]
            } else {
                vec![1, 0x1234, 0xabcd]
            };
            let image = image_from_ex(
                &fixture(
                    (1, 1),
                    depth,
                    2,
                    None,
                    false,
                    false,
                    false,
                    false,
                    false,
                    &vals,
                ),
                format,
                PrecisionConversion::Exact,
            )
            .unwrap();
            assert_eq!(
                image.bytes().unwrap(),
                if depth == 8 {
                    vec![1, 18, 171, 255]
                } else {
                    words(&[1, 0x1234, 0xabcd, 65535])
                }
            );
        }
    }
    #[test]
    fn unsupported_layouts_malformed_data_and_precision_are_explicit() {
        for alpha in [1, 0] {
            assert!(
                image_from_ex(
                    &fixture(
                        (1, 1),
                        16,
                        2,
                        Some(alpha),
                        false,
                        false,
                        false,
                        false,
                        false,
                        &[1, 2, 3, 4]
                    ),
                    PixelFormatEx::Rgba16,
                    PrecisionConversion::Exact
                )
                .is_err()
            );
        }
        let data = fixture(
            (1, 1),
            16,
            2,
            Some(2),
            false,
            false,
            false,
            false,
            false,
            &[1, 0x1234, 0xabcd, 0x8001],
        );
        assert!(image_from_ex(&data, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_err());
        let rounded =
            image_from_ex(&data, PixelFormatEx::Rgba8, PrecisionConversion::Round).unwrap();
        assert_eq!(rounded.bytes().unwrap(), [0, 18, 171, 128]);
        assert!(
            image_from_ex(
                &data[..data.len() - 1],
                PixelFormatEx::Rgba16,
                PrecisionConversion::Exact
            )
            .is_err()
        );
        let limits = wml2::limits::DecodeLimits {
            expanded_bytes: 4,
            ..wml2::limits::DecodeLimits::unlimited()
        };
        assert!(
            image_from_ex_with_limits(
                &data,
                PixelFormatEx::Rgba8,
                PrecisionConversion::Round,
                limits
            )
            .is_err()
        );
        assert_eq!(
            image_from(&data).unwrap().buffer.unwrap(),
            [0, 18, 171, 128]
        );
        let count = u16::from_le_bytes([data[8], data[9]]) as usize;
        let entry = (0..count)
            .map(|i| 10 + i * 12)
            .find(|i| u16::from_le_bytes([data[*i], data[*i + 1]]) == 339)
            .unwrap();
        let payload = u32::from_le_bytes(data[entry + 8..entry + 12].try_into().unwrap()) as usize;
        let mut signed = data.clone();
        signed[payload..payload + 2].copy_from_slice(&2u16.to_le_bytes());
        assert!(image_from_ex(&signed, PixelFormatEx::Rgba16, PrecisionConversion::Exact).is_err());
        let mut pages = data.clone();
        let next = 10 + count * 12;
        pages[next..next + 4].copy_from_slice(&(data.len() as u32).to_le_bytes());
        pages.extend_from_slice(&data[8..next + 4]);
        assert!(
            image_from_ex(&pages, PixelFormatEx::Rgba16, PrecisionConversion::Exact)
                .err()
                .unwrap()
                .to_string()
                .contains("multi-page")
        );
    }
}

#[cfg(any(feature = "png", feature = "tiff"))]
fn callback_fixtures() -> Vec<(Vec<u8>, PixelFormatEx)> {
    let mut fixtures = Vec::new();
    #[cfg(feature = "png")]
    {
        fixtures.push((
            png::fixture(
                (2, 2),
                16,
                6,
                0,
                &[
                    0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 0, 9, 0, 10, 0, 11, 0,
                    12, 0, 13, 0, 14, 0, 15, 0, 16,
                ],
                &[],
            ),
            PixelFormatEx::Rgba16,
        ));
        fixtures.push((
            png::fixture((2, 2), 16, 0, 0, &[0, 0, 1, 0, 2, 0, 0, 3, 0, 4], &[]),
            PixelFormatEx::Gray16,
        ));
        fixtures.push((
            png::fixture(
                (2, 2),
                8,
                6,
                0,
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 0, 9, 10, 11, 12, 13, 14, 15, 16],
                &[],
            ),
            PixelFormatEx::Rgba8,
        ));
        fixtures.push((
            png::fixture((2, 2), 8, 0, 0, &[0, 1, 2, 0, 3, 4], &[]),
            PixelFormatEx::Gray8,
        ));
    }
    #[cfg(feature = "tiff")]
    {
        fixtures.push((
            tiff::fixture(
                (2, 2),
                16,
                2,
                Some(2),
                true,
                true,
                true,
                false,
                true,
                &(1..=16).collect::<Vec<_>>(),
            ),
            PixelFormatEx::Rgba16,
        ));
        fixtures.push((
            tiff::fixture(
                (2, 2),
                16,
                1,
                None,
                true,
                false,
                true,
                false,
                true,
                &[1, 2, 3, 4],
            ),
            PixelFormatEx::Gray16,
        ));
        fixtures.push((
            tiff::fixture(
                (2, 2),
                8,
                2,
                Some(2),
                true,
                false,
                false,
                false,
                false,
                &(1..=16).collect::<Vec<_>>(),
            ),
            PixelFormatEx::Rgba8,
        ));
        fixtures.push((
            tiff::fixture(
                (2, 2),
                8,
                1,
                None,
                true,
                false,
                false,
                false,
                false,
                &[1, 2, 3, 4],
            ),
            PixelFormatEx::Gray8,
        ));
    }
    fixtures
}

#[cfg(any(feature = "png", feature = "tiff"))]
struct Probe {
    image: ImageBufferEx,
    source: Option<PixelFormatEx>,
    init: usize,
    draws: usize,
    metadata: usize,
    ends: usize,
    stop: Option<u8>,
}
#[cfg(any(feature = "png", feature = "tiff"))]
impl Probe {
    fn new(stop: Option<u8>) -> Self {
        Self {
            image: ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact),
            source: None,
            init: 0,
            draws: 0,
            metadata: 0,
            ends: 0,
            stop,
        }
    }
    fn abort() -> Response {
        Ok(Some(CallbackResponse {
            response: ResponseCommand::Abort,
        }))
    }
}
#[cfg(any(feature = "png", feature = "tiff"))]
impl DrawCallback for Probe {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        panic!("EX decoder must declare its source")
    }
    fn init_ex(&mut self, w: usize, h: usize, option: Option<InitOptionsEx>) -> Response {
        self.init += 1;
        let option = option.unwrap();
        self.source = Some(option.source_format);
        assert!(option.source_stride.is_none());
        assert!(!option.legacy.as_ref().unwrap().animation);
        if self.stop == Some(0) {
            return Self::abort();
        }
        self.image.init_ex(w, h, Some(option))
    }
    fn draw(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        data: &[u8],
        option: Option<DrawOptions>,
    ) -> Response {
        self.draws += 1;
        assert_eq!(data.len(), w * h * self.source.unwrap().bytes_per_pixel());
        if self.stop == Some(1) {
            return Self::abort();
        }
        self.image.draw(x, y, w, h, data, option)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        panic!("static EX decoder must not request animation")
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.ends += 1;
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, key: &str, value: wml2::metadata::DataMap) -> Response {
        self.metadata += 1;
        if self.stop == Some(2) {
            return Self::abort();
        }
        self.image.set_metadata(key, value)
    }
}
#[cfg(any(feature = "png", feature = "tiff"))]
#[test]
fn callbacks_declare_packed_source_and_do_not_silently_narrow_legacy_drawers() {
    use bin_rs::reader::BytesReader;
    for (data, format) in callback_fixtures() {
        let mut probe = Probe::new(None);
        image_decoder_ex(
            &mut BytesReader::new(&data),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut probe,
            },
        )
        .unwrap();
        assert_eq!(probe.source, Some(format));
        assert_eq!(probe.init, 1);
        assert_eq!(probe.draws, 2);
        assert_eq!(probe.ends, 1);
        let mut legacy = ImageBuffer::new();
        let result = image_decoder_ex(
            &mut BytesReader::new(&data),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut legacy,
            },
        );
        if format == PixelFormatEx::Rgba8 {
            result.unwrap();
            assert_eq!(legacy.buffer, image_from(&data).unwrap().buffer);
        } else {
            assert!(result.is_err());
            assert!(legacy.buffer.is_none());
        }
    }
}
#[cfg(any(feature = "png", feature = "tiff"))]
#[test]
fn ex_abort_suppresses_callbacks_for_png_and_tiff_at_all_stages() {
    use bin_rs::reader::BytesReader;
    for (data, _) in callback_fixtures() {
        for stop in 0..=2 {
            let mut probe = Probe::new(Some(stop));
            image_decoder_ex(
                &mut BytesReader::new(&data),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut probe,
                },
            )
            .unwrap();
            assert_eq!(probe.ends, 1);
            match stop {
                0 => {
                    assert_eq!(probe.init, 1);
                    assert_eq!(probe.draws, 0);
                    assert_eq!(probe.metadata, 0);
                }
                1 => {
                    assert_eq!(probe.draws, 1);
                    assert_eq!(probe.metadata, 0);
                }
                _ => assert_eq!(probe.metadata, 1),
            }
        }
    }
}
#[test]
fn unsupported_codec_does_not_fall_back_to_rgba8() {
    assert!(image_from_ex(b"GIF89a", PixelFormatEx::Rgba16, PrecisionConversion::Exact).is_err());
}
