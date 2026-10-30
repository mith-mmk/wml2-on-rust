#![cfg(feature = "jpeg-xl")]
use wml2::draw::*;
use wml2::metadata::DataMap;
macro_rules! orientation {
    ($n:literal) => {
        (
            include_bytes!(concat!("fixtures/review/jxl-orientation", $n, ".jxl")).as_slice(),
            include_bytes!(concat!("fixtures/review/jxl-orientation", $n, ".rgba")).as_slice(),
            $n,
        )
    };
}
#[test]
fn all_eight_orientations_match_independent_djxl_after_one_reorientation() {
    for (input, expected, orientation) in [
        orientation!(1),
        orientation!(2),
        orientation!(3),
        orientation!(4),
        orientation!(5),
        orientation!(6),
        orientation!(7),
        orientation!(8),
    ] {
        let size = if orientation <= 4 { (2, 3) } else { (3, 2) };
        let image = image_from(input).unwrap();
        let metadata = image.metadata.as_ref().unwrap();
        assert!(matches!(
            metadata["orientation_applied"],
            DataMap::Bool(true)
        ));
        assert!(
            matches!(metadata["JPEG XL Source Orientation"], DataMap::UInt(v) if v == orientation)
        );
        assert_eq!(
            (image.width, image.height),
            size,
            "orientation {orientation}"
        );
        assert_eq!(
            image.buffer.as_deref().unwrap(),
            expected,
            "orientation {orientation}"
        );
        #[cfg(feature = "color-management")]
        {
            let image = wml2::jpegxl::image_from_srgb_with_limits_and_cancel(
                input,
                Default::default(),
                &|| false,
            )
            .unwrap();
            assert_eq!((image.width, image.height), size);
            let metadata = image.metadata.as_ref().unwrap();
            assert!(matches!(
                metadata["orientation_applied"],
                DataMap::Bool(true)
            ));
            assert!(
                matches!(metadata["JPEG XL Source Orientation"], DataMap::UInt(v) if v == orientation)
            );
            for (actual, reference) in image
                .buffer
                .unwrap()
                .chunks_exact(4)
                .zip(expected.chunks_exact(4))
            {
                for c in 0..3 {
                    assert!(actual[c].abs_diff(reference[c]) <= 2);
                }
                assert_eq!(actual[3], reference[3]);
            }
        }
        #[cfg(feature = "image-buffer-ex")]
        {
            let image =
                image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Exact).unwrap();
            assert!(matches!(
                image.metadata.as_ref().unwrap()["orientation_applied"],
                DataMap::Bool(true)
            ));
            assert!(
                matches!(image.metadata.as_ref().unwrap()["JPEG XL Source Orientation"], DataMap::UInt(v) if v == orientation)
            );
            assert_eq!((image.width(), image.height()), size);
            assert_eq!(
                image.bytes().unwrap(),
                expected,
                "EX orientation {orientation}"
            );
        }
    }
}
#[test]
fn nine_and_ten_bit_sources_promote_to_normalized_sixteen_without_losing_codes() {
    for (input, expected, source, bits) in [
        (
            include_bytes!("fixtures/review/jxl-rgba9.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-rgba9.rgba16le").as_slice(),
            include_bytes!("fixtures/review/jxl-rgba9.source16le").as_slice(),
            9,
        ),
        (
            include_bytes!("fixtures/review/jxl-rgba10.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-rgba10.rgba16le").as_slice(),
            include_bytes!("fixtures/review/jxl-rgba10.source16le").as_slice(),
            10,
        ),
    ] {
        let image = image_from(input).unwrap();
        let metadata = image.metadata.unwrap();
        assert!(matches!(metadata["JPEG XL Source Bit Depth"],DataMap::UInt(v) if v==bits));
        assert!(matches!(
            metadata["JPEG XL Output Bit Depth"],
            DataMap::UInt(8)
        ));
        let rgba8: Vec<u8> = expected
            .chunks_exact(2)
            .map(|b| ((u32::from(u16::from_le_bytes([b[0], b[1]])) * 255 + 32767) / 65535) as u8)
            .collect();
        assert_eq!(image.buffer.unwrap(), rgba8);
        #[cfg(feature = "image-buffer-ex")]
        {
            let image =
                image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba16));
            let actual = image.bytes().unwrap();
            assert_eq!(actual.len(), expected.len());
            for (a, b) in actual.chunks_exact(2).zip(expected.chunks_exact(2)) {
                assert!(
                    u16::from_le_bytes([a[0], a[1]]).abs_diff(u16::from_le_bytes([b[0], b[1]]))
                        <= 1
                );
            }
            let max = (1u32 << bits) - 1;
            for (output, input) in actual.chunks_exact(2).zip(source.chunks_exact(2)) {
                let promoted = u32::from(u16::from_le_bytes([output[0], output[1]]));
                let original = u32::from(u16::from_le_bytes([input[0], input[1]]));
                assert_eq!((promoted * max + 32767) / 65535, original);
            }
            assert!(matches!(
                image.metadata.as_ref().unwrap()["JPEG XL Output Bit Depth"],
                DataMap::UInt(16)
            ));
        }
        #[cfg(not(feature = "image-buffer-ex"))]
        let _ = source;
    }
}
#[cfg(feature = "color-management")]
#[test]
fn explicit_srgb_display_uses_the_actual_reconstruction_profile_once() {
    let bench = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "fixtures/review/jxl-jbrd-bench.srgb.rgba.zlib"
    ))
    .unwrap();
    let gray = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "fixtures/review/jxl-jbrd-gray.srgb.rgba.zlib"
    ))
    .unwrap();
    for (input, expected, original, size) in [
        (
            include_bytes!("fixtures/review/jxl-icc-gray16.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-icc-gray16.srgb.rgba").as_slice(),
            include_bytes!("fixtures/review/jxl-gray-gamma2.icc").as_slice(),
            (7, 5),
        ),
        (
            include_bytes!("fixtures/review/jxl-icc-lossless.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-icc-lossless.srgb.rgba").as_slice(),
            include_bytes!("fixtures/review/jxl-gamma2.icc").as_slice(),
            (7, 5),
        ),
        (
            include_bytes!("fixtures/review/jxl-icc-lossy.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-icc-lossy.srgb.rgba").as_slice(),
            include_bytes!("fixtures/review/jxl-gamma2.icc").as_slice(),
            (7, 5),
        ),
        (
            include_bytes!("fixtures/review/jxl-jbrd-bench.jxl").as_slice(),
            bench.as_slice(),
            include_bytes!("fixtures/review/jxl-jbrd-bench.icc").as_slice(),
            (606, 500),
        ),
        (
            include_bytes!("fixtures/review/jxl-jbrd-gray.jxl").as_slice(),
            gray.as_slice(),
            include_bytes!("fixtures/review/jxl-jbrd-gray.icc").as_slice(),
            (200, 200),
        ),
    ] {
        for image in [
            wml2::jpegxl::image_from_srgb(input).unwrap(),
            wml2::jpegxl::image_from_srgb_with_limits_and_cancel(
                input,
                Default::default(),
                &|| false,
            )
            .unwrap(),
        ] {
            assert_eq!((image.width, image.height), size);
            let pixels = image.buffer.unwrap();
            assert_eq!(pixels.len(), expected.len());
            for (actual, reference) in pixels.chunks_exact(4).zip(expected.chunks_exact(4)) {
                for c in 0..3 {
                    assert!(
                        actual[c].abs_diff(reference[c]) <= 2,
                        "actual={actual:?}, reference={reference:?}"
                    );
                }
                assert_eq!(actual[3], reference[3]);
            }
            let meta = image.metadata.unwrap();
            assert!(
                matches!(&meta["ICC Profile"],DataMap::ICCProfile(p) if p==include_bytes!("../src/srgb.icc"))
            );
            assert!(
                matches!(&meta["JPEG XL Source ICC Profile"],DataMap::ICCProfile(p) if p==original)
            );
            assert!(
                matches!(&meta["JPEG XL Reconstruction ICC Profile"],DataMap::ICCProfile(p) if p.len()>=128)
            );
        }
        assert!(
            wml2::jpegxl::image_from_srgb_with_limits(
                input,
                wml2::limits::DecodeLimits {
                    metadata_bytes: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            wml2::jpegxl::image_from_srgb_with_limits(
                input,
                wml2::limits::DecodeLimits {
                    expanded_bytes: 1024,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}

#[cfg(feature = "color-management")]
mod cancellation {
    use std::cell::Cell;
    use wml2::jpegxl::{DecodeCancelled, image_from_srgb_with_limits_and_cancel as decode};
    use wml2::limits::DecodeLimits;
    const INPUT: &[u8] = include_bytes!("fixtures/review/jxl-icc-lossless.jxl");

    #[test]
    fn managed_cancellable_alpha_matches_independent_djxl_at_all_depths() {
        for (input, oracle, channels, bits) in [
            (
                include_bytes!("fixtures/review/jxl-rgba8.jxl").as_slice(),
                include_bytes!("fixtures/review/jxl-rgba8-djxl.bin").as_slice(),
                4,
                8,
            ),
            (
                include_bytes!("fixtures/review/jxl-rgba12.jxl").as_slice(),
                include_bytes!("fixtures/review/jxl-rgba12-djxl.bin").as_slice(),
                4,
                12,
            ),
            (
                include_bytes!("fixtures/review/jxl-rgba16.jxl").as_slice(),
                include_bytes!("fixtures/review/jxl-rgba16-djxl.bin").as_slice(),
                4,
                16,
            ),
            (
                include_bytes!("fixtures/review/jxl-grayalpha8.jxl").as_slice(),
                include_bytes!("fixtures/review/jxl-grayalpha8-djxl.bin").as_slice(),
                2,
                8,
            ),
            (
                include_bytes!("fixtures/review/jxl-grayalpha12.jxl").as_slice(),
                include_bytes!("fixtures/review/jxl-grayalpha12-djxl.bin").as_slice(),
                2,
                12,
            ),
            (
                include_bytes!("fixtures/review/jxl-grayalpha16.jxl").as_slice(),
                include_bytes!("fixtures/review/jxl-grayalpha16-djxl.bin").as_slice(),
                2,
                16,
            ),
        ] {
            let image = decode(input, DecodeLimits::default(), &|| false).unwrap();
            let source: Vec<u32> = if bits == 8 {
                oracle.iter().map(|&v| v as u32).collect()
            } else {
                oracle
                    .chunks_exact(2)
                    .map(|v| u16::from_le_bytes([v[0], v[1]]) as u32)
                    .collect()
            };
            let max = (1u32 << bits) - 1;
            let expected: Vec<u8> = source
                .chunks_exact(channels)
                .map(|p| ((p[channels - 1] * 255 + max / 2) / max) as u8)
                .collect();
            let actual: Vec<u8> = image
                .buffer
                .unwrap()
                .chunks_exact(4)
                .map(|p| p[3])
                .collect();
            assert_eq!(actual, expected, "{bits}-bit, {channels} channels");
            assert!(actual.iter().any(|&a| a != 255));
        }
    }

    #[test]
    fn cancellation_at_every_checkpoint_never_publishes_an_image() {
        let calls = Cell::new(0);
        decode(INPUT, DecodeLimits::default(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
        let checkpoints = calls.get();
        assert!(checkpoints > 10, "must include intermediate work");
        for stop in 1..=checkpoints {
            calls.set(0);
            let mut published = None;
            let result = decode(INPUT, DecodeLimits::default(), &|| {
                calls.set(calls.get() + 1);
                calls.get() == stop
            })
            .map(|image| published = Some(image));
            assert!(
                result.err().unwrap().is::<DecodeCancelled>(),
                "checkpoint {stop}"
            );
            assert!(published.is_none());
            assert_eq!(calls.get(), stop, "cancellation is latched");
        }
    }

    #[test]
    fn precancellation_wins_but_encountered_limit_and_decode_errors_are_preserved() {
        assert!(
            decode(
                b"invalid",
                DecodeLimits {
                    input_bytes: 0,
                    ..Default::default()
                },
                &|| true
            )
            .err()
            .unwrap()
            .is::<DecodeCancelled>()
        );
        let calls = Cell::new(0);
        for limits in [
            DecodeLimits {
                input_bytes: 1,
                ..Default::default()
            },
            DecodeLimits {
                pixels: 1,
                ..Default::default()
            },
            DecodeLimits {
                expanded_bytes: 1,
                ..Default::default()
            },
            DecodeLimits {
                metadata_bytes: 1,
                ..Default::default()
            },
            DecodeLimits {
                frames: 0,
                ..Default::default()
            },
            DecodeLimits {
                animation_bytes: 1,
                ..Default::default()
            },
        ] {
            calls.set(0);
            let err = decode(INPUT, limits, &|| {
                calls.set(calls.get() + 1);
                false
            })
            .err()
            .unwrap();
            assert!(!err.is::<DecodeCancelled>());
            assert!(err.to_string().contains("decode limit"), "{err}");
            let count = calls.get();
            calls.set(0);
            // A cancellation observable only after the failing limit must not
            // mask that failure, or run another probe after returning it.
            let err = decode(INPUT, limits, &|| {
                calls.set(calls.get() + 1);
                calls.get() > count
            })
            .err()
            .unwrap();
            assert!(!err.is::<DecodeCancelled>());
            assert_eq!(calls.get(), count);
        }
        let err = decode(b"invalid", DecodeLimits::default(), &|| false)
            .err()
            .unwrap();
        assert!(!err.is::<DecodeCancelled>());
        assert!(!err.to_string().contains("decode limit"));
        assert!(decode(INPUT, DecodeLimits::default(), &|| false).is_ok());
    }
}
