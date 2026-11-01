#![cfg(feature = "jpeg-xl")]
use wml2::jpegxl::{JxlResourceLimits, preview_dimensions_with_resource_limits as inspect};
use wml2::limits::DecodeLimits;
fn check(
    bytes: &[u8],
) -> Result<Option<wml2::image_info::ImageDimensions>, Box<dyn std::error::Error>> {
    inspect(bytes, DecodeLimits::default(), JxlResourceLimits::default())
}
#[test]
fn preview_geometry_matches_frozen_independent_libjxl_headers() {
    // libjxl reports coded preview geometry; output applies its shared orientation.
    let d = check(include_bytes!("fixtures/review/jxl-preview-rgba8.jxl"))
        .unwrap()
        .unwrap();
    assert_eq!((d.coded.width, d.coded.height), (2, 3));
    let o = d.output.unwrap();
    assert_eq!((o.width, o.height), (2, 3));
    assert!(d.intrinsic_display.is_none());
    let d = check(include_bytes!("fixtures/review/jxl-preview-icc16.jxl"))
        .unwrap()
        .unwrap();
    assert_eq!((d.coded.width, d.coded.height), (2, 3));
    let o = d.output.unwrap();
    assert_eq!((o.width, o.height), (2, 3));
    assert!(d.intrinsic_display.is_none());
    let d = check(include_bytes!(
        "fixtures/review/jxl-preview-oriented-icc16.jxl"
    ))
    .unwrap()
    .unwrap();
    assert_eq!((d.coded.width, d.coded.height), (2, 3));
    let o = d.output.unwrap();
    assert_eq!((o.width, o.height), (3, 2));
    assert!(d.intrinsic_display.is_none());
    let d = check(include_bytes!("fixtures/review/jxl-preview-grayalpha8.jxl"))
        .unwrap()
        .unwrap();
    assert_eq!((d.coded.width, d.coded.height), (2, 3));
    let o = d.output.unwrap();
    assert_eq!((o.width, o.height), (2, 3));
    assert!(d.intrinsic_display.is_none());
    let d = check(include_bytes!("fixtures/review/jxl-preview-larger.jxl"))
        .unwrap()
        .unwrap();
    assert_eq!((d.coded.width, d.coded.height), (32, 32));
    let o = d.output.unwrap();
    assert_eq!((o.width, o.height), (32, 32));
    assert!(d.intrinsic_display.is_none());
    let d = check(include_bytes!("fixtures/review/jxl-preview-lossy.jxl"))
        .unwrap()
        .unwrap();
    assert_eq!((d.coded.width, d.coded.height), (8, 8));
    let o = d.output.unwrap();
    assert_eq!((o.width, o.height), (8, 8));
    assert!(d.intrinsic_display.is_none());
    assert!(
        check(include_bytes!("fixtures/review/jxl-rgba8.jxl"))
            .unwrap()
            .is_none()
    );
}
#[test]
fn malformed_truncated_and_resource_refusals_never_report_absence() {
    let input = include_bytes!("fixtures/review/jxl-preview-icc16.jxl");
    for end in 0..input.len() {
        assert!(check(&input[..end]).is_err(), "prefix {end}");
    }
    assert!(check(b"garbage").is_err());
    assert!(
        check(include_bytes!(
            "fixtures/review/jxl-preview-oversized-header.jxl"
        ))
        .is_err()
    );
    for r in [
        JxlResourceLimits {
            workspace_bytes: 0,
            ..Default::default()
        },
        JxlResourceLimits {
            workspace_bytes: usize::MAX,
            ..Default::default()
        },
        JxlResourceLimits {
            internal_pixels: 0,
            ..Default::default()
        },
        JxlResourceLimits {
            internal_layers: 0,
            ..Default::default()
        },
        JxlResourceLimits {
            toc_entries: 0,
            ..Default::default()
        },
        JxlResourceLimits {
            icc_coded_bytes: 0,
            ..Default::default()
        },
        JxlResourceLimits {
            icc_expanded_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(inspect(input, DecodeLimits::default(), r).is_err());
    }
    for l in [
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(inspect(input, l, JxlResourceLimits::default()).is_err());
    }
    assert!(check(input).unwrap().is_some());
}

#[test]
fn preview_pixels_match_independent_c_hash_anchored_vectors() {
    use wml2::jpegxl::preview_from_with_resource_limits as decode;
    for (input, expected, width, height, orientation) in [
        (
            include_bytes!("fixtures/review/jxl-preview-rgba8.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-preview-rgba8.preview.rgba16le.zlib").as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-preview-icc16.preview.rgba16le.zlib").as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-oriented-icc16.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-preview-icc16.preview.rgba16le.zlib").as_slice(),
            2,
            3,
            6,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-grayalpha8.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-preview-grayalpha8.preview.rgba16le.zlib")
                .as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-larger.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-preview-larger.preview.rgba16le.zlib").as_slice(),
            32,
            32,
            1,
        ),
    ] {
        let p = decode(input, DecodeLimits::default(), JxlResourceLimits::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            (p.width, p.height, p.orientation),
            (width, height, orientation)
        );
        let expected = miniz_oxide::inflate::decompress_to_vec_zlib(expected).unwrap();
        let actual: Vec<u8> = p.rgba16.iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(actual, expected);
        assert_eq!(&p.icc_profile[36..40], b"acsp");
        assert_eq!(
            &p.icc_profile[16..20],
            if p.color_channels == 1 {
                b"GRAY"
            } else {
                b"RGB "
            }
        );
    }
}
#[test]
fn preview_pixels_have_independent_output_and_combined_workspace_limits() {
    use wml2::jpegxl::preview_from_with_resource_limits as decode;
    let input = include_bytes!("fixtures/review/jxl-preview-icc16.jxl");
    assert!(
        decode(
            include_bytes!("fixtures/review/jxl-rgba8.jxl"),
            DecodeLimits::default(),
            JxlResourceLimits::default()
        )
        .unwrap()
        .is_none()
    );
    for end in 0..input.len() {
        assert!(
            decode(
                &input[..end],
                DecodeLimits::default(),
                JxlResourceLimits::default()
            )
            .is_err()
        );
    }
    for limits in [
        DecodeLimits {
            pixels: 5,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 47,
            ..Default::default()
        },
    ] {
        assert!(decode(input, limits, JxlResourceLimits::default()).is_err());
    }
    let mut low = 0;
    let mut high = 64 << 20;
    while low < high {
        let mid = low + (high - low) / 2;
        let r = decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: mid,
                ..Default::default()
            },
        );
        if r.is_ok() { high = mid } else { low = mid + 1 }
    }
    assert!(
        decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: low,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: low - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        decode(
            include_bytes!("fixtures/review/jxl-preview-oversized-header.jxl"),
            DecodeLimits::default(),
            JxlResourceLimits::default()
        )
        .is_err()
    );
}

#[test]
fn damaged_preview_entropy_fails_extraction_but_main_skip_still_matches() {
    use wml2::jpegxl::preview_from_with_resource_limits as decode;
    let input = include_bytes!("fixtures/review/jxl-preview-rgba8.jxl");
    let mut damaged = input.to_vec();
    damaged[15..64].fill(0xff);
    assert!(
        decode(
            &damaged,
            DecodeLimits::default(),
            JxlResourceLimits::default()
        )
        .is_err()
    );
    let healthy = wml2::draw::image_from(input).unwrap();
    let skipped = wml2::draw::image_from(&damaged).unwrap();
    assert_eq!(healthy.buffer, skipped.buffer);
    assert!(decode(input, DecodeLimits::default(), JxlResourceLimits::default()).is_ok());
}

#[test]
fn lossy_preview_rgb_has_independent_tolerance_and_exact_alpha() {
    use wml2::jpegxl::preview_from_with_resource_limits as decode;
    let p = decode(
        include_bytes!("fixtures/review/jxl-preview-lossy.jxl"),
        DecodeLimits::default(),
        JxlResourceLimits::default(),
    )
    .unwrap()
    .unwrap();
    let expected = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "fixtures/review/jxl-preview-lossy.preview.rgba16le.zlib"
    ))
    .unwrap();
    assert_eq!((p.width, p.height, p.orientation), (8, 8, 1));
    for (i, (&actual, bytes)) in p.rgba16.iter().zip(expected.chunks_exact(2)).enumerate() {
        let expected = u16::from_le_bytes([bytes[0], bytes[1]]);
        assert!(actual.abs_diff(expected) <= if i % 4 == 3 { 0 } else { 514 });
    }
}

#[test]
#[cfg(feature = "color-management")]
fn managed_preview_all_orientations_match_independent_c_and_littlecms() {
    use wml2::jpegxl::{
        preview_from_srgb_with_resource_limits as display, preview_from_with_resource_limits as raw,
    };
    for (source, bit, orientation, expected, w, h, tolerance) in [
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            33,
            1,
            include_bytes!("fixtures/review/jxl-preview-icc16-o1.srgb-preview.rgba.zlib")
                .as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            33,
            2,
            include_bytes!("fixtures/review/jxl-preview-icc16-o2.srgb-preview.rgba.zlib")
                .as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            33,
            3,
            include_bytes!("fixtures/review/jxl-preview-icc16-o3.srgb-preview.rgba.zlib")
                .as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            33,
            4,
            include_bytes!("fixtures/review/jxl-preview-icc16-o4.srgb-preview.rgba.zlib")
                .as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            33,
            5,
            include_bytes!("fixtures/review/jxl-preview-icc16-o5.srgb-preview.rgba.zlib")
                .as_slice(),
            3,
            2,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            33,
            6,
            include_bytes!("fixtures/review/jxl-preview-icc16-o6.srgb-preview.rgba.zlib")
                .as_slice(),
            3,
            2,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            33,
            7,
            include_bytes!("fixtures/review/jxl-preview-icc16-o7.srgb-preview.rgba.zlib")
                .as_slice(),
            3,
            2,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
            33,
            8,
            include_bytes!("fixtures/review/jxl-preview-icc16-o8.srgb-preview.rgba.zlib")
                .as_slice(),
            3,
            2,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-rgba8.jxl").as_slice(),
            33,
            1,
            include_bytes!("fixtures/review/jxl-preview-rgba8-o1.srgb-preview.rgba.zlib")
                .as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-grayalpha8.jxl").as_slice(),
            33,
            1,
            include_bytes!("fixtures/review/jxl-preview-grayalpha8-o1.srgb-preview.rgba.zlib")
                .as_slice(),
            2,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-larger.jxl").as_slice(),
            44,
            1,
            include_bytes!("fixtures/review/jxl-preview-larger-o1.srgb-preview.rgba.zlib")
                .as_slice(),
            32,
            32,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-preview-lossy.jxl").as_slice(),
            27,
            1,
            include_bytes!("fixtures/review/jxl-preview-lossy-o1.srgb-preview.rgba.zlib")
                .as_slice(),
            8,
            8,
            2,
        ),
    ] {
        let mut input = source.to_vec();
        for k in 0..3 {
            let j = bit + k;
            input[j / 8] =
                (input[j / 8] & !(1 << (j % 8))) | ((((orientation - 1) >> k) & 1) << (j % 8));
        }
        let original = raw(
            &input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
        )
        .unwrap()
        .unwrap();
        let image = display(
            &input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!((image.width, image.height), (w, h));
        let expected = miniz_oxide::inflate::decompress_to_vec_zlib(expected).unwrap();
        let actual = image.buffer.unwrap();
        assert_eq!(actual.len(), expected.len());
        for (i, (&a, &b)) in actual.iter().zip(&expected).enumerate() {
            assert!(
                a.abs_diff(b) <= if i % 4 == 3 { 0 } else { tolerance },
                "orientation {orientation}, byte {i}: {a} != {b}"
            );
        }
        let metadata = image.metadata.unwrap();
        assert!(matches!(
            metadata["orientation_applied"],
            wml2::metadata::DataMap::Bool(true)
        ));
        assert!(
            matches!(metadata["JPEG XL Source Orientation"],wml2::metadata::DataMap::UInt(n) if n==u64::from(orientation))
        );
        assert!(
            matches!(&metadata["ICC Profile"],wml2::metadata::DataMap::ICCProfile(p) if p==include_bytes!("../src/srgb.icc"))
        );
        assert!(
            matches!(&metadata["JPEG XL Reconstruction ICC Profile"],wml2::metadata::DataMap::ICCProfile(p) if p==&original.icc_profile)
        );
        let after = raw(
            &input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(original.rgba16, after.rgba16);
        assert_eq!(after.orientation, orientation);
    }
}
#[test]
#[cfg(feature = "color-management")]
fn managed_preview_absence_errors_and_cumulative_budget_boundary() {
    use wml2::jpegxl::{
        preview_from_srgb_with_resource_limits as display, preview_from_with_resource_limits as raw,
    };
    let input = include_bytes!("fixtures/review/jxl-preview-icc16.jxl");
    assert!(
        display(
            include_bytes!("fixtures/review/jxl-rgba8.jxl"),
            DecodeLimits::default(),
            JxlResourceLimits::default()
        )
        .unwrap()
        .is_none()
    );
    for end in 0..input.len() {
        assert!(
            display(
                &input[..end],
                DecodeLimits::default(),
                JxlResourceLimits::default()
            )
            .is_err()
        );
    }
    let mut low = 0;
    let mut high = 64 << 20;
    while low < high {
        let mid = low + (high - low) / 2;
        if display(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: mid,
                ..Default::default()
            },
        )
        .is_ok()
        {
            high = mid
        } else {
            low = mid + 1
        }
    }
    assert!(
        display(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: low,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        display(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: low - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        raw(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: low - 1,
                ..Default::default()
            }
        )
        .is_ok()
    );
    for limits in [
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 47,
            ..Default::default()
        },
        DecodeLimits {
            pixels: 5,
            ..Default::default()
        },
    ] {
        assert!(display(input, limits, JxlResourceLimits::default()).is_err());
    }
    let mut damaged = include_bytes!("fixtures/review/jxl-preview-rgba8.jxl").to_vec();
    damaged[15..64].fill(0xff);
    assert!(
        display(
            &damaged,
            DecodeLimits::default(),
            JxlResourceLimits::default()
        )
        .is_err()
    );
    assert!(display(input, DecodeLimits::default(), JxlResourceLimits::default()).is_ok());
}
