#![cfg(all(feature = "tga", not(feature = "noretoro")))]
use std::collections::HashMap;
use wml2::draw::{EncodeOptions, ImageBuffer, image_encoder, image_load};
use wml2::metadata::DataMap;
use wml2::util::{ImageFormat, encoder_supports_format};

fn encode(
    width: usize,
    height: usize,
    pixels: Vec<u8>,
    depth: u64,
    compression: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut image = ImageBuffer::from_buffer(width, height, pixels);
    image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut image,
            options: Some(HashMap::from([
                ("depth".into(), DataMap::UInt(depth)),
                ("compression".into(), DataMap::Ascii(compression.into())),
            ])),
        },
        ImageFormat::Tga,
    )
}

#[test]
fn raw_and_rle_true_color_preserve_channels_and_orientation() {
    let pixels = vec![1, 2, 3, 255, 11, 22, 33, 127, 4, 5, 6, 255, 44, 55, 66, 255];
    for compression in ["none", "rle"] {
        for depth in [24, 32] {
            let data = encode(2, 2, pixels.clone(), depth, compression).unwrap();
            assert_eq!(data[16], depth as u8);
            assert_eq!(data[17], if depth == 32 { 0x28 } else { 0x20 });
            assert!(data.ends_with(b"TRUEVISION-XFILE.\0"));
            let decoded = image_load(&data).unwrap();
            let mut expected = pixels.clone();
            if depth == 24 {
                for pixel in expected.chunks_exact_mut(4) {
                    pixel[3] = 255;
                }
            }
            assert_eq!(decoded.buffer.unwrap(), expected);
        }
    }
    assert!(encoder_supports_format(&ImageFormat::Tga));
}

#[test]
fn rle_packets_are_limited_to_128_and_do_not_cross_rows() {
    let pixels = [7, 8, 9, 255].repeat(129 * 2);
    let data = encode(129, 2, pixels.clone(), 32, "rle").unwrap();
    assert_eq!(&data[18..28], &[255, 9, 8, 7, 255, 0, 9, 8, 7, 255]);
    assert_eq!(&data[28..38], &data[18..28]);
    assert_eq!(image_load(&data).unwrap().buffer.unwrap(), pixels);
    let varied: Vec<_> = (0..260)
        .flat_map(|x| [x as u8, (x / 256) as u8, 1, 255])
        .collect();
    let data = encode(260, 1, varied.clone(), 32, "rle").unwrap();
    assert_eq!(data[18], 127);
    assert_eq!(data[18 + 1 + 128 * 4], 127);
    assert_eq!(image_load(&data).unwrap().buffer.unwrap(), varied);
}

#[test]
fn options_and_dimensions_are_validated() {
    assert!(encode(0, 1, vec![], 32, "rle").is_err());
    assert!(encode(65536, 1, vec![], 32, "rle").is_err());
    assert!(encode(1, 1, vec![0; 4], 16, "rle").is_err());
    assert!(encode(1, 1, vec![0; 4], 32, "zip").is_err());
}

#[test]
fn default_keeps_alpha_and_transparent_rgb_on_wire() {
    let mut image = ImageBuffer::from_buffer(1, 1, vec![17, 29, 43, 0]);
    let data = image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut image,
            options: None,
        },
        ImageFormat::Tga,
    )
    .unwrap();
    assert_eq!(&data[18..23], &[0, 43, 29, 17, 0]);
    assert_eq!(image_load(&data).unwrap().buffer.unwrap(), vec![0; 4]);
}

#[test]
fn legacy_interleave_respects_origin_and_odd_height() {
    for height in 1..=9usize {
        for (flag, step) in [(0x40u8, 2usize), (0x80, 4)] {
            for origin in [0u8, 0x10, 0x20, 0x30] {
                for rle in [false, true] {
                    let mut data = vec![0; 18];
                    data[2] = if rle { 10 } else { 2 };
                    data[12..14].copy_from_slice(&2u16.to_le_bytes());
                    data[14..16].copy_from_slice(&(height as u16).to_le_bytes());
                    data[16] = 24;
                    data[17] = flag | origin;
                    for phase in 0..step {
                        for row in (phase..height).step_by(step) {
                            if rle {
                                data.push(1);
                            }
                            for col in 0..2 {
                                let y = if origin & 0x20 != 0 {
                                    row
                                } else {
                                    height - 1 - row
                                };
                                let x = if origin & 0x10 != 0 { 1 - col } else { col };
                                data.extend_from_slice(&[0, x as u8, y as u8]);
                            }
                        }
                    }
                    let expected: Vec<_> = (0..height)
                        .flat_map(|y| (0..2).flat_map(move |x| [y as u8, x as u8, 0, 255]))
                        .collect();
                    assert_eq!(image_load(&data).unwrap().buffer.unwrap(), expected);
                    data[17] |= 0xc0;
                    assert!(image_load(&data).is_err());
                }
            }
        }
    }
}

struct Source {
    width: usize,
    row: Option<Vec<u8>>,
    ended: bool,
}
impl wml2::draw::PickCallback for Source {
    fn encode_start(
        &mut self,
        _: Option<wml2::draw::EncoderOptions>,
    ) -> Result<Option<wml2::draw::ImageProfiles>, Box<dyn std::error::Error>> {
        Ok(Some(wml2::draw::ImageProfiles {
            width: self.width,
            height: 1,
            background: None,
            metadata: None,
        }))
    }
    fn encode_pick(
        &mut self,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
        _: Option<wml2::draw::PickOptions>,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        Ok(self.row.clone())
    }
    fn encode_end(
        &mut self,
        _: Option<wml2::draw::EndOptions>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.ended = true;
        Ok(())
    }
    fn metadata(&mut self) -> Result<Option<HashMap<String, DataMap>>, Box<dyn std::error::Error>> {
        Ok(None)
    }
}
#[test]
fn malformed_callback_rows_are_rejected_and_success_finalizes() {
    for row in [None, Some(vec![0; 3]), Some(vec![0; 5])] {
        let mut source = Source {
            width: 1,
            row,
            ended: false,
        };
        assert!(
            image_encoder(
                &mut EncodeOptions {
                    debug_flag: 0,
                    drawer: &mut source,
                    options: None
                },
                ImageFormat::Tga
            )
            .is_err()
        );
        assert!(!source.ended);
    }
    let mut source = Source {
        width: 1,
        row: Some(vec![1, 2, 3, 255]),
        ended: false,
    };
    image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut source,
            options: None,
        },
        ImageFormat::Tga,
    )
    .unwrap();
    assert!(source.ended);
}

#[test]
#[ignore = "writes interoperability/display samples and measures encoder/decoder throughput; set WML2_TGA_OUTPUT to an existing directory"]
fn external_samples_and_benchmark() {
    let folder = std::path::PathBuf::from(std::env::var("WML2_TGA_OUTPUT").unwrap());
    let width = 257;
    let height = 193;
    let pixels: Vec<u8> = (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                [
                    (x * 3) as u8,
                    (y * 5) as u8,
                    (x + y) as u8,
                    ((x * 7 + y * 11) % 256) as u8,
                ]
            })
        })
        .collect();
    std::fs::write(folder.join("expected.rgba"), &pixels).unwrap();
    for depth in [24, 32] {
        for compression in ["none", "rle"] {
            let data = encode(width, height, pixels.clone(), depth, compression).unwrap();
            std::fs::write(folder.join(format!("{depth}-{compression}.tga")), &data).unwrap();
            let start = std::time::Instant::now();
            for _ in 0..100 {
                std::hint::black_box(
                    encode(width, height, pixels.clone(), depth, compression).unwrap(),
                );
            }
            let encode_ms = start.elapsed().as_secs_f64() * 10.0;
            let start = std::time::Instant::now();
            for _ in 0..100 {
                std::hint::black_box(image_load(&data).unwrap());
            }
            eprintln!(
                "TGA depth={depth} compression={compression}: encode {encode_ms:.3} ms/image, decode {:.3} ms/image bytes={}",
                start.elapsed().as_secs_f64() * 10.0,
                data.len()
            );
        }
    }
}

fn encode_kind(
    width: usize,
    height: usize,
    pixels: Vec<u8>,
    kind: &str,
    depth: u64,
    palette_depth: u64,
    compression: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut image = ImageBuffer::from_buffer(width, height, pixels);
    let mut settings = HashMap::from([
        ("color_type".into(), DataMap::Ascii(kind.into())),
        ("depth".into(), DataMap::UInt(depth)),
        ("compression".into(), DataMap::Ascii(compression.into())),
    ]);
    if kind == "indexed" {
        settings.insert("palette_depth".into(), DataMap::UInt(palette_depth));
    }
    image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut image,
            options: Some(settings),
        },
        ImageFormat::Tga,
    )
}
#[test]
fn grayscale8_and16_raw_rle_preserve_gray_alpha() {
    for compression in ["none", "rle"] {
        for depth in [8, 16] {
            let pixels: Vec<_> = [0, 1, 17, 255]
                .into_iter()
                .flat_map(|g| [g, g, g, if depth == 8 { 255 } else { 127 }])
                .collect();
            let data =
                encode_kind(2, 2, pixels.clone(), "grayscale", depth, 32, compression).unwrap();
            assert_eq!(data[2], if compression == "rle" { 11 } else { 3 });
            assert_eq!(data[16], depth as u8);
            assert_eq!(image_load(&data).unwrap().buffer.unwrap(), pixels);
        }
    }
    assert!(encode_kind(1, 1, vec![1, 2, 3, 255], "grayscale", 16, 32, "none").is_err());
    assert!(encode_kind(1, 1, vec![1, 1, 1, 127], "grayscale", 8, 32, "none").is_err());
}
#[test]
fn indexed_exact_palette_depth_indices_and_alpha() {
    for compression in ["none", "rle"] {
        for depth in [8] {
            for palette_depth in [24, 32] {
                let color = [11, 22, 33, if palette_depth == 24 { 255 } else { 127 }];
                let pixels = [color, [99, 88, 77, 255], color, color].concat();
                let data = encode_kind(
                    2,
                    2,
                    pixels.clone(),
                    "indexed",
                    depth,
                    palette_depth,
                    compression,
                )
                .unwrap();
                assert_eq!(data[1], 1);
                assert_eq!(data[5..7], [2, 0]);
                assert_eq!(data[7], palette_depth as u8);
                assert_eq!(&data[18..21], &[33, 22, 11]);
                assert_eq!(image_load(&data).unwrap().buffer.unwrap(), pixels);
            }
        }
    }
    assert!(encode_kind(1, 1, vec![1, 2, 3, 127], "indexed", 8, 24, "none").is_err());
    let colors: Vec<u8> = (0..257u16)
        .flat_map(|v| [v as u8, (v >> 8) as u8, 0, 255])
        .collect();
    assert!(encode_kind(257, 1, colors.clone(), "indexed", 8, 32, "none").is_err());
    assert!(encode_kind(1, 1, vec![0, 0, 0, 255], "indexed", 16, 32, "none").is_err());
}
#[test]
fn grayscale_and_indexed_invalid_options_are_rejected() {
    for (kind, depth) in [("gray", 8), ("grayscale", 24), ("indexed", 32)] {
        assert!(encode_kind(1, 1, vec![0, 0, 0, 255], kind, depth, 32, "none").is_err());
    }
    assert!(encode_kind(1, 1, vec![0, 0, 0, 255], "indexed", 8, 16, "none").is_err());
}
#[test]
#[ignore = "writes grayscale/indexed interoperability samples; set WML2_TGA_OUTPUT to an existing directory"]
fn external_gray_indexed_samples() {
    let folder = std::path::PathBuf::from(std::env::var("WML2_TGA_OUTPUT").unwrap());
    for mode in ["none", "rle"] {
        let rgba: Vec<u8> = (0..3999usize)
            .flat_map(|i| [(i % 128) as u8, (i % 128 * 3) as u8, 17, 255])
            .collect();
        let data = encode_kind(129, 31, rgba.clone(), "indexed", 8, 24, mode).unwrap();
        std::fs::write(folder.join(format!("indexed24-8-{mode}.tga")), data).unwrap();
        std::fs::write(folder.join(format!("indexed24-8-{mode}.rgba")), rgba).unwrap();
    }

    let folder = std::path::PathBuf::from(std::env::var("WML2_TGA_OUTPUT").unwrap());
    for kind in ["grayscale", "indexed"] {
        for depth in if kind == "indexed" {
            vec![8]
        } else {
            vec![8, 16]
        } {
            for mode in ["none", "rle"] {
                let pixels: Vec<u8> = (0..31usize)
                    .flat_map(|y| {
                        (0..129usize).flat_map(move |x| {
                            let g = ((x / 4 + y * 7) % 128) as u8;
                            if kind == "grayscale" {
                                [g, g, g, if depth == 8 { 255 } else { g * 2 }]
                            } else {
                                [g, g.wrapping_mul(3), g.wrapping_mul(7), g * 2]
                            }
                        })
                    })
                    .collect();
                let data = encode_kind(129, 31, pixels.clone(), kind, depth, 32, mode).unwrap();
                let name = format!("{kind}-{depth}-{mode}");
                std::fs::write(folder.join(format!("{name}.tga")), &data).unwrap();
                std::fs::write(folder.join(format!("{name}.rgba")), &pixels).unwrap();
                let start = std::time::Instant::now();
                for _ in 0..100 {
                    std::hint::black_box(
                        encode_kind(129, 31, pixels.clone(), kind, depth, 32, mode).unwrap(),
                    );
                }
                eprintln!(
                    "{name}: {:.3} ms/image",
                    start.elapsed().as_secs_f64() * 10.0
                );
            }
        }
    }
}

#[test]
fn indexed_256_colors_and_palette_budget_are_checked() {
    let pixels: Vec<u8> = (0..256u16).flat_map(|v| [v as u8, 11, 23, 255]).collect();
    let data = encode_kind(256, 1, pixels.clone(), "indexed", 8, 24, "none").unwrap();
    assert_eq!(&data[5..7], &[0, 1]);
    assert_eq!(image_load(&data).unwrap().buffer.unwrap(), pixels);
    let mut image = ImageBuffer::from_buffer(1, 1, vec![1, 2, 3, 255]);
    let mut options = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: Some(HashMap::from([
            ("color_type".into(), DataMap::Ascii("indexed".into())),
            ("compression".into(), DataMap::Ascii("none".into())),
        ])),
    };
    let mut limits = wml2::limits::DecodeLimits::unlimited();
    limits.expanded_bytes = 48; // 18 header + 1 index + 4 palette + 26 footer = 49.
    assert!(wml2::draw::image_encoder_with_limits(&mut options, ImageFormat::Tga, limits).is_err());
    limits.expanded_bytes = 49;
    assert_eq!(
        wml2::draw::image_encoder_with_limits(&mut options, ImageFormat::Tga, limits)
            .unwrap()
            .len(),
        49
    );
}
