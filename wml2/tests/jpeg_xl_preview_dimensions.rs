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
