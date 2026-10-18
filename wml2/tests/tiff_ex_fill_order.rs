#![cfg(all(feature = "tiff", feature = "image-buffer-ex"))]

use wml2::draw::{PixelFormatEx, PrecisionConversion, image_from_ex};

fn fixture(depth: u16, endian: &str, codec: &str, fill: u16, predictor: u16) -> Vec<u8> {
    let name = format!("ex-review-fill{fill}-gray{depth}-{endian}-{codec}-p{predictor}.tif");
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn check_codec(codec: &str) {
    for depth in [8, 16] {
        let values: [u16; 4] = if depth == 8 {
            [1, 2, 128, 255]
        } else {
            [1, 0x1234, 0x8001, 65535]
        };
        let format = if depth == 8 {
            PixelFormatEx::Gray8
        } else {
            PixelFormatEx::Gray16
        };
        let expected: Vec<u8> = if depth == 8 {
            values.iter().map(|v| *v as u8).collect()
        } else {
            values.iter().flat_map(|v| v.to_le_bytes()).collect()
        };
        for endian in ["le", "be"] {
            for predictor in if matches!(codec, "lzw" | "deflate") {
                &[1, 2][..]
            } else {
                &[1][..]
            } {
                for fill in [1, 2] {
                    let input = fixture(depth, endian, codec, fill, *predictor);
                    let image = image_from_ex(&input, format, PrecisionConversion::Exact)
                        .unwrap_or_else(|err| {
                            panic!("{depth}/{endian}/{codec}/fill{fill}/p{predictor}: {err}")
                        });
                    assert_eq!(
                        image.bytes().unwrap(),
                        expected,
                        "{depth}/{endian}/{codec}/fill{fill}/p{predictor}"
                    );
                }
            }
        }
    }
}

#[test]
fn raw_native_sample_fill_order_is_independent_of_word_endianness() {
    check_codec("raw");
}
#[test]
fn packbits_fill_order_normalizes_literal_header_and_pixels_before_decode() {
    check_codec("packbits");
}
#[test]
fn standard_lzw_fill_order_normalizes_codes_before_predictor() {
    check_codec("lzw");
}
#[test]
fn deflate_fill_order_normalizes_stream_before_predictor() {
    check_codec("deflate");
}

#[test]
fn canonical_old_lsb_lzw_is_normalized_even_when_wire_has_lsb_signature() {
    // first=64/65/127/255 leave [0, second & 1] in the bit-reversed
    // canonical wire too. A prefix alone cannot select the original-byte
    // nonportable compatibility interpretation. Oracle: libtiff 4.7.0.
    for first in [1u8, 64, 65, 127, 128, 255] {
        let name = format!("ex-review-fill2-gray8-le-old-lsb-first{first}.tif");
        let input = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap();
        let image =
            image_from_ex(&input, PixelFormatEx::Gray8, PrecisionConversion::Exact).unwrap();
        assert_eq!(image.bytes().unwrap(), [first, 2, 128, 255]);
    }
}

#[test]
fn fill_order_requires_one_short_with_a_defined_value() {
    for (ty, count, values) in [
        (3u16, 1u32, [0u16, 0u16]),
        (3, 1, [3, 0]),
        (3, 1, [65535, 0]),
        (3, 0, [1, 0]),
        (3, 2, [1, 2]),
        (4, 1, [1, 0]),
        (2, 1, [1, 0]),
    ] {
        let mut input = fixture(8, "le", "raw", 1, 1);
        let fields = u16::from_le_bytes(input[8..10].try_into().unwrap()) as usize;
        let offset = (0..fields)
            .map(|i| 10 + i * 12)
            .find(|i| u16::from_le_bytes(input[*i..*i + 2].try_into().unwrap()) == 266)
            .unwrap();
        input[offset + 2..offset + 4].copy_from_slice(&ty.to_le_bytes());
        input[offset + 4..offset + 8].copy_from_slice(&count.to_le_bytes());
        input[offset + 8..offset + 10].copy_from_slice(&values[0].to_le_bytes());
        input[offset + 10..offset + 12].copy_from_slice(&values[1].to_le_bytes());
        assert!(
            image_from_ex(&input, PixelFormatEx::Gray8, PrecisionConversion::Exact).is_err(),
            "accepted invalid FillOrder type={ty},count={count},values={values:?}"
        );
    }
}

#[test]
fn explicit_old_and_wml2_lsb_lzw_keep_their_natural_sample_compatibility() {
    // These original-byte streams deliberately preserve WML2's pre-existing
    // nonportable FillOrder=2 compatibility, separately from the canonical
    // reversed standard-MSB fixtures validated by TIFFReadScanline.
    for encoder in [
        wml2::encoder::lzw::encode_tiff_libtiff_compat,
        wml2::encoder::lzw::encode_tiff_wml2_lsb,
    ] {
        for depth in [8, 16] {
            let values: [u16; 4] = if depth == 8 {
                [1, 2, 128, 255]
            } else {
                [1, 0x1234, 0x8001, 65535]
            };
            let format = if depth == 8 {
                PixelFormatEx::Gray8
            } else {
                PixelFormatEx::Gray16
            };
            let expected: Vec<u8> = if depth == 8 {
                values.iter().map(|v| *v as u8).collect()
            } else {
                values.iter().flat_map(|v| v.to_le_bytes()).collect()
            };
            for endian in ["le", "be"] {
                let raw: Vec<u8> = if depth == 8 {
                    expected.clone()
                } else {
                    values
                        .iter()
                        .flat_map(|v| {
                            if endian == "le" {
                                v.to_le_bytes()
                            } else {
                                v.to_be_bytes()
                            }
                        })
                        .collect()
                };
                let compressed = encoder(&raw).unwrap();
                assert_eq!(compressed[0], 0);
                assert_ne!(compressed[1] & 1, 0);
                let mut input = fixture(depth, endian, "lzw", 2, 1);
                let read16 = |bytes: &[u8]| {
                    if endian == "le" {
                        u16::from_le_bytes(bytes.try_into().unwrap())
                    } else {
                        u16::from_be_bytes(bytes.try_into().unwrap())
                    }
                };
                let read32 = |bytes: &[u8]| {
                    if endian == "le" {
                        u32::from_le_bytes(bytes.try_into().unwrap())
                    } else {
                        u32::from_be_bytes(bytes.try_into().unwrap())
                    }
                };
                let fields = read16(&input[8..10]) as usize;
                let strip = (0..fields)
                    .map(|i| 10 + i * 12)
                    .find(|i| read16(&input[*i..*i + 2]) == 273)
                    .unwrap();
                let strip_at = read32(&input[strip + 8..strip + 12]) as usize;
                let count = (0..fields)
                    .map(|i| 10 + i * 12)
                    .find(|i| read16(&input[*i..*i + 2]) == 279)
                    .unwrap();
                let len = compressed.len() as u32;
                input[count + 8..count + 12].copy_from_slice(&if endian == "le" {
                    len.to_le_bytes()
                } else {
                    len.to_be_bytes()
                });
                input.truncate(strip_at);
                input.extend(compressed);
                let image = image_from_ex(&input, format, PrecisionConversion::Exact).unwrap();
                assert_eq!(image.bytes().unwrap(), expected, "{depth}/{endian}");
            }
        }
    }
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn highres_uses_the_same_corrected_native_reader() {
    for endian in ["le", "be"] {
        for codec in ["raw", "packbits", "lzw", "deflate"] {
            let input = fixture(16, endian, codec, 2, 1);
            let image =
                wml2::highres::tiff::decode_native(&input, &wml2::limits::DecodeLimits::default())
                    .unwrap();
            assert_eq!(
                image.pixels().u16_planes().unwrap()[0].samples(),
                [1, 0x1234, 0x8001, 65535]
            );
        }
    }
}
