use wml2::draw::*;

#[test]
fn image_buffer_reinitialization_and_failed_next_are_atomic() {
    let mut image = ImageBuffer::new();
    let init = || {
        Some(InitOptions {
            animation: true,
            loop_count: 2,
            background: None,
        })
    };
    image.init(2, 2, init()).unwrap();
    image
        .set_metadata("before-init", wml2::metadata::DataMap::UInt(7))
        .unwrap();
    image.next(Some(NextOptions::wait(10))).unwrap();
    image.init(2, 2, init()).unwrap();
    image.draw(0, 0, 2, 2, &[11; 16], None).unwrap();
    assert_eq!(image.current, None);
    assert_eq!(image.first_wait_time, None);
    assert!(image.metadata.as_ref().unwrap().contains_key("before-init"));
    let mut next = NextOptions::new();
    next.image_rect = Some(ImageRect {
        start_x: 0,
        start_y: 0,
        width: usize::MAX,
        height: 2,
    });
    assert!(image.next(Some(next)).is_err());
    assert_eq!(image.current, None);
    image.next(Some(NextOptions::wait(20))).unwrap();
    image.draw(0, 0, 2, 2, &[22; 16], None).unwrap();
    assert_eq!(image.current, Some(0));
    image.current = Some(99);
    assert!(image.draw(0, 0, 1, 1, &[0; 4], None).is_err());
    image.init(1, 1, None).unwrap();
    assert!(image.animation.is_none());
    image.draw(0, 0, 1, 1, &[33; 4], None).unwrap();
}

#[test]
fn rectangles_preserve_stride_padding_and_reject_before_mutation() {
    let mut image = ImageBuffer::from_buffer(3, 2, vec![0; 24]);
    let input: Vec<_> = (0..32).collect();
    image.draw(1, 0, 4, 2, &input, None).unwrap();
    assert_eq!(
        image.encode_pick(1, 0, 4, 3, None).unwrap().unwrap(),
        [
            input[0..8].to_vec(),
            vec![0; 8],
            input[16..24].to_vec(),
            vec![0; 24]
        ]
        .concat()
    );
    let before = image.buffer.clone();
    assert!(image.draw(0, 0, 3, 2, &[255; 16], None).is_err());
    assert_eq!(image.buffer, before);
    assert!(image.draw(0, 0, usize::MAX, 2, &[], None).is_err());
    assert!(image.encode_pick(3, 0, 1, 1, None).unwrap().is_none());
}

#[cfg(feature = "png")]
mod png {
    use super::*;
    use wml2::png::utils::CRC32;
    pub fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], bytes: &[u8]) {
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(bytes);
        let crc = CRC32::new().crc32(&[kind.as_slice(), bytes].concat());
        out.extend_from_slice(&crc.to_be_bytes());
    }
    fn fixture(
        width: u32,
        height: u32,
        depth: u8,
        color: u8,
        interlace: u8,
        raw: &[u8],
    ) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut header = width.to_be_bytes().to_vec();
        header.extend_from_slice(&height.to_be_bytes());
        header.extend_from_slice(&[depth, color, 0, 0, interlace]);
        chunk(&mut out, b"IHDR", &header);
        if color == 3 {
            chunk(
                &mut out,
                b"PLTE",
                &[0, 0, 0, 85, 85, 85, 170, 170, 170, 255, 255, 255],
            );
        }
        chunk(
            &mut out,
            b"IDAT",
            &miniz_oxide::deflate::compress_to_vec_zlib(raw, 6),
        );
        chunk(&mut out, b"IEND", &[]);
        out
    }
    #[test]
    fn truncated_scanline_is_error() {
        assert!(image_from(&fixture(2, 2, 8, 0, 0, &[0, 1, 2])).is_err());
    }
    #[test]
    fn gray_alpha_16_all_filters_are_bytewise() {
        let rows: [[u8; 8]; 5] = [
            [1, 255, 2, 128, 3, 0, 4, 255],
            [5, 12, 6, 34, 7, 56, 8, 78],
            [9, 90, 10, 123, 11, 234, 12, 255],
            [13, 128, 14, 99, 15, 88, 16, 77],
            [17, 66, 18, 55, 19, 44, 20, 33],
        ];
        let mut raw = Vec::new();
        for (filter, row) in rows.iter().enumerate() {
            raw.push(filter as u8);
            for i in 0..8 {
                let a = if i >= 4 { row[i - 4] } else { 0 } as i32;
                let b = if filter > 0 { rows[filter - 1][i] } else { 0 } as i32;
                let c = if filter > 0 && i >= 4 {
                    rows[filter - 1][i - 4]
                } else {
                    0
                } as i32;
                let p = a + b - c;
                let predictor = match filter {
                    0 => 0,
                    1 => a,
                    2 => b,
                    3 => (a + b) / 2,
                    _ => {
                        if (p - a).abs() <= (p - b).abs() && (p - a).abs() <= (p - c).abs() {
                            a
                        } else if (p - b).abs() <= (p - c).abs() {
                            b
                        } else {
                            c
                        }
                    }
                };
                raw.push(row[i].wrapping_sub(predictor as u8));
            }
        }
        let expected: Vec<_> = rows
            .iter()
            .flat_map(|row| row.chunks_exact(4).flat_map(|p| [p[0], p[0], p[0], p[2]]))
            .collect();
        assert_eq!(
            image_from(&fixture(2, 5, 16, 4, 0, &raw))
                .unwrap()
                .buffer
                .unwrap(),
            expected
        );
    }
    #[test]
    fn adam7_gray_palette_alpha_and_empty_passes() {
        for (width, height) in [(9, 9), (1, 9), (9, 1), (1, 1)] {
            for (color, depth) in [
                (0, 1),
                (0, 2),
                (0, 4),
                (0, 8),
                (0, 16),
                (3, 2),
                (4, 8),
                (4, 16),
                (2, 8),
                (6, 8),
            ] {
                let channels = match color {
                    4 => 2,
                    2 => 3,
                    6 => 4,
                    _ => 1,
                };
                let max = if color == 3 { 3 } else { (1u32 << depth) - 1 };
                let value = |x: usize, y: usize, c: usize| {
                    ((x * 3 + y * 5 + c * 7) as u32 % (max + 1)) as u16
                };
                let mut raw = Vec::new();
                for (sx, sy, dx, dy) in [
                    (0, 0, 8, 8),
                    (4, 0, 8, 8),
                    (0, 4, 4, 8),
                    (2, 0, 4, 4),
                    (0, 2, 2, 4),
                    (1, 0, 2, 2),
                    (0, 1, 1, 2),
                ] {
                    if sx >= width || sy >= height {
                        continue;
                    }
                    let mut previous = Vec::new();
                    for y in (sy..height).step_by(dy) {
                        let mut row = Vec::new();
                        let mut packed = 0u8;
                        let mut bits = 0;
                        for x in (sx..width).step_by(dx) {
                            for c in 0..channels {
                                let v = value(x, y, c);
                                if depth == 16 {
                                    row.extend_from_slice(&v.to_be_bytes());
                                } else if depth == 8 {
                                    row.push(v as u8);
                                } else {
                                    packed = (packed << depth) | v as u8;
                                    bits += depth;
                                    if bits == 8 {
                                        row.push(packed);
                                        packed = 0;
                                        bits = 0;
                                    }
                                }
                            }
                        }
                        if bits > 0 {
                            row.push(packed << (8 - bits));
                        }
                        let filter = (y + sx + sy) % 5;
                        let bpp = (channels * depth as usize).div_ceil(8);
                        raw.push(filter as u8);
                        for (i, &v) in row.iter().enumerate() {
                            let a = if i >= bpp { row[i - bpp] as i32 } else { 0 };
                            let b = previous.get(i).copied().unwrap_or(0) as i32;
                            let c = if i >= bpp {
                                previous.get(i - bpp).copied().unwrap_or(0) as i32
                            } else {
                                0
                            };
                            let p = a + b - c;
                            let predictor = match filter {
                                0 => 0,
                                1 => a,
                                2 => b,
                                3 => (a + b) / 2,
                                _ => {
                                    if (p - a).abs() <= (p - b).abs()
                                        && (p - a).abs() <= (p - c).abs()
                                    {
                                        a
                                    } else if (p - b).abs() <= (p - c).abs() {
                                        b
                                    } else {
                                        c
                                    }
                                }
                            };
                            raw.push(v.wrapping_sub(predictor as u8));
                        }
                        previous = row;
                    }
                }
                let mut expected = Vec::new();
                for y in 0..height {
                    for x in 0..width {
                        let sample = |c| {
                            let v = value(x, y, c);
                            if color == 3 {
                                (v * 85) as u8
                            } else if depth == 16 {
                                (v >> 8) as u8
                            } else {
                                (v as u32 * 255 / max) as u8
                            }
                        };
                        let pixel = match color {
                            2 => [sample(0), sample(1), sample(2), 255],
                            6 => [sample(0), sample(1), sample(2), sample(3)],
                            4 => [sample(0), sample(0), sample(0), sample(1)],
                            _ => [sample(0), sample(0), sample(0), 255],
                        };
                        expected.extend_from_slice(&pixel);
                    }
                }
                let result =
                    image_from(&fixture(width as u32, height as u32, depth, color, 1, &raw))
                        .unwrap();
                assert_eq!(
                    result.buffer.unwrap(),
                    expected,
                    "{width}x{height} color={color} depth={depth}"
                );
            }
        }
    }
    #[test]
    fn short_fdat_is_error() {
        for length in 0..4 {
            let mut png = fixture(1, 1, 8, 0, 0, &[0, 42]);
            png.truncate(png.len() - 12);
            chunk(&mut png, b"fdAT", &vec![0; length]);
            chunk(&mut png, b"IEND", &[]);
            assert!(image_from(&png).is_err());
        }
    }

    #[test]
    fn transparent_16_bit_gray_compares_before_reducing_precision() {
        let source = fixture(2, 1, 16, 0, 0, &[0, 0x12, 0x34, 0x12, 0x35]);
        let mut bytes = source[..33].to_vec();
        chunk(&mut bytes, b"tRNS", &[0x12, 0x34]);
        bytes.extend_from_slice(&source[33..]);
        assert_eq!(
            image_from(&bytes).unwrap().buffer.unwrap(),
            [18, 18, 18, 0, 18, 18, 18, 255]
        );
    }

    #[test]
    fn image_and_metadata_limits_have_exact_boundaries() {
        use wml2::limits::DecodeLimits;
        let image = fixture(2, 2, 8, 0, 0, &[0, 1, 2, 0, 3, 4]);
        let exact = DecodeLimits {
            input_bytes: image.len(),
            pixels: 4,
            expanded_bytes: 16,
            animation_bytes: 16,
            ..Default::default()
        };
        assert!(image_from_with_limits(&image, exact).is_ok());
        assert!(image_from_with_limits(&image, DecodeLimits { pixels: 3, ..exact }).is_err());
        assert!(
            image_from_with_limits(
                &image,
                DecodeLimits {
                    input_bytes: image.len() - 1,
                    ..exact
                }
            )
            .is_err()
        );
        assert!(
            image_from_with_limits(
                &image,
                DecodeLimits {
                    expanded_bytes: 15,
                    ..exact
                }
            )
            .is_err()
        );
        let mut with_text = image[..image.len() - 12].to_vec();
        let payload = [
            b"key\0\0".to_vec(),
            miniz_oxide::deflate::compress_to_vec_zlib(&[b'x'; 32], 6),
        ]
        .concat();
        chunk(&mut with_text, b"zTXt", &payload);
        chunk(&mut with_text, b"zTXt", &payload);
        chunk(&mut with_text, b"IEND", &[]);
        assert!(
            image_from_with_limits(
                &with_text,
                DecodeLimits {
                    metadata_bytes: 64,
                    ..Default::default()
                }
            )
            .is_ok()
        );
        assert!(
            image_from_with_limits(
                &with_text,
                DecodeLimits {
                    metadata_bytes: 63,
                    ..Default::default()
                }
            )
            .is_err()
        );
        // iCCP is inflated later, while publishing metadata. It must share the
        // same cumulative budget as text decoded while reading the chunks.
        let icc = vec![42; 4096];
        let mut with_icc = image[..33].to_vec();
        let profile = [
            b"profile\0\0".to_vec(),
            miniz_oxide::deflate::compress_to_vec_zlib(&icc, 6),
        ]
        .concat();
        chunk(&mut with_icc, b"iCCP", &profile);
        with_icc.extend_from_slice(&with_text[33..]);
        let exact = DecodeLimits {
            metadata_bytes: 4096 + 64,
            ..Default::default()
        };
        let decoded = image_from_with_limits(&with_icc, exact).unwrap();
        assert!(matches!(
            decoded.metadata.unwrap().get("ICC Profile"),
            Some(wml2::metadata::DataMap::ICCProfile(bytes)) if bytes == &icc
        ));
        assert!(
            image_from_with_limits(
                &with_icc,
                DecodeLimits {
                    metadata_bytes: exact.metadata_bytes - 1,
                    ..exact
                }
            )
            .is_err()
        );
    }

    #[test]
    #[ignore = "bounded mutation campaign; run explicitly or in scheduled CI"]
    fn bounded_png_mutation_corpus() {
        let seed = fixture(2, 2, 8, 0, 0, &[0, 1, 2, 0, 3, 4]);
        let limits = wml2::limits::DecodeLimits {
            input_bytes: 4096,
            pixels: 4096,
            expanded_bytes: 16384,
            metadata_bytes: 4096,
            frames: 4,
            animation_bytes: 32768,
        };
        let mut state = 0x12345678u32;
        for _ in 0..10000 {
            let mut bytes = seed.clone();
            for _ in 0..3 {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let index = state as usize % bytes.len();
                bytes[index] ^= (state >> 16) as u8;
            }
            let _ = image_from_with_limits(&bytes, limits);
        }
    }
    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn file_conversion_preserves_existing_files_on_failure_and_same_path() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wml2-review-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("image.png");
        let name = path.to_string_lossy().into_owned();
        let png = fixture(2, 1, 8, 0, 0, &[0, 33, 99]);
        fs::write(&path, &png).unwrap();
        convert(name.clone(), name.clone(), None).unwrap();
        assert_eq!(
            image_from_file(name.clone()).unwrap().buffer.unwrap(),
            [33, 33, 33, 255, 99, 99, 99, 255]
        );
        let before = fs::read(&path).unwrap();
        assert!(
            convert(
                root.join("missing.png").to_string_lossy().into_owned(),
                name.clone(),
                None
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        let mut invalid = ImageBuffer::new();
        assert!(image_to_file(name, &mut invalid, wml2::util::ImageFormat::Png).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        let alias = root.join("alias.png");
        fs::hard_link(&path, &alias).unwrap();
        let mut replacement = ImageBuffer::from_buffer(1, 1, vec![1, 2, 3, 255]);
        image_to_file(
            alias.to_string_lossy().into_owned(),
            &mut replacement,
            wml2::util::ImageFormat::Png,
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::remove_file(alias).unwrap();
        let blocked = root.join("blocked.png");
        fs::create_dir(&blocked).unwrap();
        assert!(
            image_to_file(
                blocked.to_string_lossy().into_owned(),
                &mut replacement,
                wml2::util::ImageFormat::Png
            )
            .is_err()
        );
        fs::remove_dir(blocked).unwrap();
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let locked = fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&path)
                .unwrap();
            assert!(
                image_to_file(
                    path.to_string_lossy().into_owned(),
                    &mut replacement,
                    wml2::util::ImageFormat::Png
                )
                .is_err()
            );
            drop(locked);
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
