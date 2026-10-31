use super::*;
macro_rules! intrinsic_case {
    ($n:literal) => {
        (
            include_bytes!(concat!(
                "../fixtures/review/jxl-intrinsic-orientation",
                $n,
                ".jxl"
            ))
            .as_slice(),
            include_bytes!(concat!("../fixtures/review/jxl-orientation", $n, ".rgba")).as_slice(),
            $n,
        )
    };
}
#[test]
fn intrinsic_is_metadata_and_all_eight_existing_orientations_keep_independent_pixels() {
    for (input, expected, orientation) in [
        intrinsic_case!(1),
        intrinsic_case!(2),
        intrinsic_case!(3),
        intrinsic_case!(4),
        intrinsic_case!(5),
        intrinsic_case!(6),
        intrinsic_case!(7),
        intrinsic_case!(8),
    ] {
        let output = if orientation >= 5 { (3, 2) } else { (2, 3) };
        let display = if orientation >= 5 { (11, 19) } else { (19, 11) };
        let image = image_from_with_limits(
            input,
            DecodeLimits {
                pixels: 6,
                animation_bytes: 24,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!((image.width, image.height), output);
        assert_eq!(image.buffer.as_deref().unwrap(), expected);
        let meta = image.metadata.as_ref().unwrap();
        for (key, value) in [
            ("coded_width", 2),
            ("coded_height", 3),
            ("output_width", output.0),
            ("output_height", output.1),
            ("JPEG XL Source Intrinsic Width", 19),
            ("JPEG XL Source Intrinsic Height", 11),
            ("intrinsic_display_width", display.0),
            ("intrinsic_display_height", display.1),
        ] {
            assert!(
                matches!(meta.get(key),Some(DataMap::UInt(v))if *v==value as u64),
                "{key}"
            );
        }
        assert!(matches!(meta["orientation_applied"], DataMap::Bool(true)));
        assert!(matches!(meta["JPEG XL Source Orientation"],DataMap::UInt(v)if v==orientation));
        #[cfg(feature = "image-buffer-ex")]
        {
            let native =
                image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Exact).unwrap();
            assert_eq!((native.width(), native.height()), output);
            assert_eq!(native.bytes().unwrap(), expected);
        }
        #[cfg(feature = "color-management")]
        {
            let display = wml2::jpegxl::image_from_srgb(input).unwrap();
            for (a, b) in display
                .buffer
                .unwrap()
                .chunks_exact(4)
                .zip(expected.chunks_exact(4))
            {
                for c in 0..3 {
                    assert!(a[c].abs_diff(b[c]) <= 2);
                }
                assert_eq!(a[3], b[3]);
            }
        }
    }
}
#[test]
fn absence_stays_absent_and_huge_display_never_spends_canvas_pixels() {
    let plain = image_from(include_bytes!("../fixtures/review/jxl-orientation6.jxl")).unwrap();
    assert!(
        !plain
            .metadata
            .as_ref()
            .unwrap()
            .contains_key("intrinsic_display_width")
    );
    assert!(
        !plain
            .metadata
            .as_ref()
            .unwrap()
            .contains_key("JPEG XL Source Intrinsic Width")
    );
    let input = include_bytes!("../fixtures/review/jxl-intrinsic-large-display.jxl");
    let limits = DecodeLimits {
        pixels: 6,
        frames: 1,
        animation_bytes: 24,
        expanded_bytes: 2 << 20,
        ..Default::default()
    };
    let image = image_from_with_limits(input, limits).unwrap();
    assert_eq!((image.width, image.height), (2, 3));
    assert_eq!(
        image.buffer.as_deref().unwrap(),
        include_bytes!("../fixtures/review/jxl-orientation1.rgba")
    );
    assert!(matches!(
        image.metadata.as_ref().unwrap()["intrinsic_display_width"],
        DataMap::UInt(1048576)
    ));
    for limits in [
        DecodeLimits {
            pixels: 5,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 23,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 1,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1,
            ..Default::default()
        },
    ] {
        let mut p = Probe::default();
        assert!(probe(input, &mut p, limits, false).is_err());
        assert!(p.events.is_empty());
    }
}
#[test]
fn oversized_intrinsic_fields_remain_rejected_before_publication() {
    for input in [
        include_bytes!("../fixtures/review/jxl-intrinsic-oversized-area.jxl").as_slice(),
        include_bytes!("../fixtures/review/jxl-intrinsic-oversized-axis.jxl"),
    ] {
        let mut p = Probe::default();
        assert!(probe(input, &mut p, DecodeLimits::unlimited(), false).is_err());
        assert!(p.events.is_empty());
    }
    let input = include_bytes!("../fixtures/review/jxl-intrinsic-orientation1.jxl");
    for n in 0..9 {
        let mut p = Probe::default();
        assert!(probe(&input[..n], &mut p, DecodeLimits::default(), false).is_err());
        assert!(p.events.is_empty());
    }
}
#[test]
#[cfg(feature = "image-buffer-ex")]
fn intrinsic_never_changes_native_icc_sixteen_bit_output() {
    let input = include_bytes!("../fixtures/review/jxl-intrinsic-icc16.jxl");
    let image = image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
    assert_eq!((image.width(), image.height()), (5, 7));
    assert_eq!(
        image.bytes().unwrap(),
        include_bytes!("../fixtures/review/jxl-intrinsic-icc16.rgba16le")
    );
    let ordinary = image_from(input).unwrap();
    assert_eq!(
        ordinary.buffer.as_deref().unwrap(),
        include_bytes!("../fixtures/review/jxl-intrinsic-icc16.rgba")
    );
}

#[cfg(feature = "color-management")]
#[test]
fn intrinsic_icc_managed_pixels_match_independent_littlecms_reference() {
    let input = include_bytes!("../fixtures/review/jxl-intrinsic-icc16.jxl");
    let image = wml2::jpegxl::image_from_srgb(input).unwrap();
    assert_eq!((image.width, image.height), (5, 7));
    let expected = include_bytes!("../fixtures/review/jxl-intrinsic-icc16.srgb.rgba");
    for (a, b) in image
        .buffer
        .unwrap()
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
    {
        for c in 0..3 {
            assert!(a[c].abs_diff(b[c]) <= 2);
        }
        assert_eq!(a[3], b[3]);
    }
}
