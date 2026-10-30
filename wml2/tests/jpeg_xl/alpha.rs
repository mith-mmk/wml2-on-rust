use super::*;
#[test]
fn integer_full_resolution_alpha_depth_matrix_matches_independent_libjxl() {
    let cases = include!("alpha_cases.rs");
    for (input, expected, color_bits, alpha_bits, orientation) in cases {
        let image = image_from(input).unwrap();
        assert_eq!(
            (image.width, image.height),
            if orientation >= 5 { (5, 7) } else { (7, 5) }
        );
        let expected = samples(expected, 16);
        for (actual, oracle) in image.buffer.unwrap().iter().zip(&expected) {
            assert!(actual.abs_diff(((u32::from(*oracle) * 255 + 32767) / 65535) as u8) <= 1);
        }
        #[cfg(feature = "image-buffer-ex")]
        {
            let native_bits = if color_bits != alpha_bits || matches!(color_bits, 9 | 10) {
                16
            } else {
                color_bits
            };
            let format = match native_bits {
                8 => PixelFormatEx::Rgba8,
                12 => PixelFormatEx::Rgba12,
                _ => PixelFormatEx::Rgba16,
            };
            let native = image_from_ex(input, format, PrecisionConversion::Exact).unwrap();
            assert_eq!(native.source_format(), Some(format));
            for (i, (actual, oracle)) in samples(native.bytes().unwrap(), native_bits)
                .iter()
                .zip(&expected)
                .enumerate()
            {
                let expected = if native_bits == 16 {
                    u32::from(*oracle)
                } else {
                    (u32::from(*oracle) * ((1 << native_bits) - 1) + 32767) / 65535
                };
                if i % 4 == 3 {
                    let max = (1u32 << alpha_bits) - 1;
                    let storage = (1u32 << native_bits) - 1;
                    assert_eq!(
                        (u32::from(*actual) * max + storage / 2) / storage,
                        (u32::from(*oracle) * max + 32767) / 65535
                    );
                }
                assert!(
                    u32::from(*actual).abs_diff(expected) <= 1,
                    "{color_bits}/{alpha_bits}/{orientation}: {actual}/{expected}"
                );
            }
            if color_bits != alpha_bits {
                assert!(
                    matches!(native.metadata.as_ref().unwrap()["JPEG XL Source Alpha Bit Depth"],DataMap::UInt(v)if v==alpha_bits.into())
                );
                assert!(
                    image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_err()
                );
            }
        }
    }
}
#[cfg(feature = "color-management")]
#[test]
fn managed_mixed_alpha_matches_independent_c_and_littlecms_without_alpha_transform() {
    for (input, expected, color_bits, alpha_bits, _) in include!("alpha_cases.rs") {
        if color_bits == alpha_bits {
            continue;
        }
        let image = wml2::jpegxl::image_from_srgb(input).unwrap();
        let expected = samples(expected, 16);
        for (i, actual) in image.buffer.unwrap().iter().enumerate() {
            let oracle = ((u32::from(expected[i]) * 255 + 32767) / 65535) as u8;
            if i % 4 == 3 {
                assert_eq!(*actual, oracle, "alpha {color_bits}/{alpha_bits}");
            } else {
                assert!(actual.abs_diff(oracle) <= 2);
            }
        }
    }
    for (input, raw, managed, color_bits, alpha_bits) in [
        (
            include_bytes!("../fixtures/review/jxl-alpha-icc-c8a16o6.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-alpha-icc-c8a16o6.rgba16le").as_slice(),
            include_bytes!("../fixtures/review/jxl-alpha-icc-c8a16o6.srgb.rgba").as_slice(),
            8,
            16,
        ),
        (
            include_bytes!("../fixtures/review/jxl-alpha-icc-c16a8o8.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-alpha-icc-c16a8o8.rgba16le").as_slice(),
            include_bytes!("../fixtures/review/jxl-alpha-icc-c16a8o8.srgb.rgba").as_slice(),
            16,
            8,
        ),
    ] {
        let image = wml2::jpegxl::image_from_srgb(input).unwrap();
        for (a, b) in image
            .buffer
            .unwrap()
            .chunks_exact(4)
            .zip(managed.chunks_exact(4))
        {
            for c in 0..3 {
                assert!(a[c].abs_diff(b[c]) <= 2);
            }
            assert_eq!(a[3], b[3]);
        }
        #[cfg(feature = "image-buffer-ex")]
        {
            let native =
                image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert_eq!(native.bytes().unwrap(), raw);
            assert!(
                matches!(native.metadata.as_ref().unwrap()["JPEG XL Source Bit Depth"],DataMap::UInt(v)if v==color_bits)
            );
            assert!(
                matches!(native.metadata.as_ref().unwrap()["JPEG XL Source Alpha Bit Depth"],DataMap::UInt(v)if v==alpha_bits)
            );
        }
        let _ = (raw, color_bits, alpha_bits);
    }
}
#[test]
fn alpha_limits_admit_the_common_sixteen_bit_canvas_before_any_callbacks() {
    let input = include_bytes!("../fixtures/review/jxl-alpha-3c8a16o5.jxl");
    let limits = DecodeLimits {
        pixels: 35,
        frames: 1,
        animation_bytes: 280,
        expanded_bytes: 2 << 20,
        ..Default::default()
    };
    assert!(image_from_with_limits(input, limits).is_ok());
    for limits in [
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..limits
        },
        DecodeLimits {
            pixels: 34,
            ..limits
        },
        DecodeLimits {
            frames: 0,
            ..limits
        },
        DecodeLimits {
            animation_bytes: 279,
            ..limits
        },
        DecodeLimits {
            expanded_bytes: 1,
            ..limits
        },
        DecodeLimits {
            metadata_bytes: 1,
            ..limits
        },
    ] {
        let mut p = Probe::default();
        assert!(probe(input, &mut p, limits, false).is_err());
        assert!(p.events.is_empty());
    }
    #[cfg(feature = "image-buffer-ex")]
    {
        let image = image_from_ex_with_limits(
            input,
            PixelFormatEx::Rgba16,
            PrecisionConversion::Exact,
            limits,
        )
        .unwrap();
        assert_eq!(image.bytes().unwrap().len(), 280);
        assert!(
            image_from_ex_with_limits(
                input,
                PixelFormatEx::Rgba16,
                PrecisionConversion::Exact,
                DecodeLimits {
                    animation_bytes: 279,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(image_from_ex(input, PixelFormatEx::Rgba12, PrecisionConversion::Exact).is_err());
        let round = image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Round).unwrap();
        assert_eq!(
            round.bytes().unwrap(),
            image_from(input).unwrap().buffer.unwrap()
        );
    }
}
#[test]
fn associated_subsampled_float_and_extra_planes_remain_explicit_refusals() {
    for input in [
        include_bytes!("../fixtures/review/jxl-alpha-refused-associated.jxl").as_slice(),
        include_bytes!("../fixtures/review/jxl-alpha-refused-subsampled.jxl"),
        include_bytes!("../fixtures/review/jxl-alpha-refused-float-alpha.jxl"),
        include_bytes!("../fixtures/review/jxl-alpha-refused-depth.jxl"),
        include_bytes!("../fixtures/review/jxl-alpha-refused-spot.jxl"),
        include_bytes!("../fixtures/review/jxl-alpha-refused-multiple.jxl"),
        include_bytes!("../fixtures/review/jxl-alpha-refused-multiple-alpha.jxl"),
        include_bytes!("../fixtures/review/jxl-alpha-refused-alpha4.jxl"),
    ] {
        for limits in [DecodeLimits::default(), DecodeLimits::unlimited()] {
            let mut p = Probe::default();
            let e = probe(input, &mut p, limits, false).unwrap_err();
            assert!(
                e.to_string()
                    .contains("at most one full-resolution straight integer"),
                "{e}"
            );
            assert!(p.events.is_empty());
        }
        #[cfg(feature = "image-buffer-ex")]
        assert!(image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).is_err());
        #[cfg(feature = "color-management")]
        assert!(wml2::jpegxl::image_from_srgb(input).is_err());
    }
}
#[cfg(feature = "image-buffer-ex")]
#[test]
fn original_mixed_depth_fixture_is_accepted_with_exact_normalized_output() {
    let image = image_from_ex(
        include_bytes!("../fixtures/review/jxl-mixed-alpha.jxl"),
        PixelFormatEx::Rgba16,
        PrecisionConversion::Exact,
    )
    .unwrap();
    assert_eq!(
        image.bytes().unwrap(),
        include_bytes!("../fixtures/review/jxl-alpha-original-mixed.rgba16le")
    );
}
