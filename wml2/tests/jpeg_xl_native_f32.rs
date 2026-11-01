#![cfg(feature = "jpeg-xl")]
use wml2::jpegxl::{
    DecodeCancelled, JxlResourceLimits, image_f32_from_with_resource_limits_and_cancel as decode,
};
use wml2::limits::DecodeLimits;
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
fn image(b: &[u8]) -> Result<wml2::jpegxl::JxlF32Image, Box<dyn std::error::Error>> {
    decode(
        b,
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| false,
    )
}
#[test]
fn native_f32_matches_independent_oracle_without_clipping_or_relabelling() {
    for (input, expected, w, h, channels, orientation) in [
        (
            include_bytes!("fixtures/review/jxl-native-f32-rgb.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-native-f32-rgb.f32le.zlib").as_slice(),
            5,
            3,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-native-f32-rgba.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-native-f32-rgba.f32le.zlib").as_slice(),
            5,
            3,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-native-f32-gray.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-native-f32-gray.f32le.zlib").as_slice(),
            5,
            3,
            1,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-native-f32-grayalpha.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-native-f32-grayalpha.f32le.zlib").as_slice(),
            5,
            3,
            1,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-native-f32-grayicc.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-native-f32-grayicc.f32le.zlib").as_slice(),
            5,
            3,
            1,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-native-f32-icc.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-native-f32-icc.f32le.zlib").as_slice(),
            5,
            3,
            3,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-native-f32-oriented.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-native-f32-oriented.f32le.zlib").as_slice(),
            3,
            5,
            3,
            6,
        ),
    ] {
        let im = image(input).unwrap();
        let size = im.dimensions.output.unwrap();
        assert_eq!(
            (
                size.width,
                size.height,
                im.color_channels,
                im.source_orientation
            ),
            (w, h, channels, orientation)
        );
        let expected = miniz_oxide::inflate::decompress_to_vec_zlib(expected).unwrap();
        let actual: Vec<u8> = im.rgba.iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(actual, expected);
        assert!(
            im.rgba
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|&v| v < 0.0))
        );
        assert!(
            im.rgba
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|&v| v > 1.0))
        );
        assert!(im.rgba.iter().any(|v| v.is_subnormal()));
        assert!(
            im.rgba
                .chunks_exact(4)
                .all(|p| p[3].is_finite() && (0.0..=1.0).contains(&p[3]))
        );
        assert_eq!(im.source_icc, im.output_icc);
        assert_eq!(&im.source_icc[36..40], b"acsp");
        if input == include_bytes!("fixtures/review/jxl-native-f32-icc.jxl") {
            assert_eq!(
                im.source_icc,
                include_bytes!("fixtures/review/jxl-gamma2.icc")
            );
        }
        if input == include_bytes!("fixtures/review/jxl-native-f32-grayicc.jxl") {
            assert_eq!(
                im.source_icc,
                include_bytes!("fixtures/review/jxl-gray-gamma2.icc")
            );
        }
        assert!(wml2::draw::image_from(input).is_err());
        #[cfg(feature = "color-management")]
        assert!(wml2::jpegxl::image_from_srgb(input).is_err());
    }
}
#[test]
fn f32_rejects_unqualified_formats_nonfinite_and_invalid_alpha() {
    for input in [
        include_bytes!("fixtures/review/jxl-native-f32-half-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-native-f32-pq-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-native-f32-hlg-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-native-f32-nonfinite-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-native-f32-alpha-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-rgba8.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-alpha-refused-associated.jxl").as_slice(),
    ] {
        let e = image(input).unwrap_err();
        assert!(!e.is::<DecodeCancelled>());
        if input == include_bytes!("fixtures/review/jxl-native-f32-nonfinite-refused.jxl")
            || input == include_bytes!("fixtures/review/jxl-native-f32-alpha-refused.jxl")
        {
            assert!(e.to_string().contains("finite RGB"), "{e}");
        }
    }
}
#[test]
fn f32_output_metadata_and_workspace_are_admitted_before_publication() {
    let input = include_bytes!("fixtures/review/jxl-native-f32-icc.jxl");
    for end in 0..input.len() {
        assert!(image(&input[..end]).is_err(), "prefix {end}");
    }
    for limits in [
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            pixels: 14,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 239,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(decode(input, limits, JxlResourceLimits::default(), &|| false).is_err());
    }
    assert!(
        decode(
            input,
            DecodeLimits {
                animation_bytes: 240,
                ..Default::default()
            },
            JxlResourceLimits::default(),
            &|| false
        )
        .is_ok()
    );
    for policy in [
        JxlResourceLimits {
            icc_coded_bytes: 0,
            ..Default::default()
        },
        JxlResourceLimits {
            icc_expanded_bytes: 0,
            ..Default::default()
        },
        JxlResourceLimits {
            internal_pixels: 0,
            ..Default::default()
        },
        JxlResourceLimits {
            toc_entries: 0,
            ..Default::default()
        },
    ] {
        assert!(decode(input, DecodeLimits::default(), policy, &|| false).is_err());
    }
    let (mut low, mut high) = (0, 64 << 20);
    while low < high {
        let mid = low + (high - low) / 2;
        if decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: mid,
                ..Default::default()
            },
            &|| false,
        )
        .is_ok()
        {
            high = mid
        } else {
            low = mid + 1
        }
    }
    assert!(
        decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: low,
                ..Default::default()
            },
            &|| false
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
            },
            &|| false
        )
        .is_err()
    );
}
#[test]
fn f32_cancellation_is_typed_and_releases_owned_storage_at_every_checkpoint() {
    use std::cell::Cell;
    let input = include_bytes!("fixtures/review/jxl-native-f32-icc.jxl");
    let count = Cell::new(0);
    drop(
        decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| {
                count.set(count.get() + 1);
                false
            },
        )
        .unwrap(),
    );
    let probes = count.get();
    assert!(probes > 10);
    eprintln!("native F32 checkpoints: {probes}");
    for stop in 1..=probes {
        let count = Cell::new(0);
        let typed = Cell::new(false);
        let live = allocation_tracking::measure(|| {
            let e = decode(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| {
                    count.set(count.get() + 1);
                    count.get() == stop
                },
            )
            .unwrap_err();
            typed.set(e.is::<DecodeCancelled>());
            drop(e);
        });
        assert!(typed.get(), "probe {stop}");
        assert_eq!(live, 0, "probe {stop}");
    }
    assert_eq!(
        allocation_tracking::measure(|| drop(image(input).unwrap())),
        0
    );
    for bad in [
        include_bytes!("fixtures/review/jxl-native-f32-nonfinite-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-native-f32-alpha-refused.jxl").as_slice(),
    ] {
        drop(image(bad).unwrap_err());
        assert_eq!(allocation_tracking::measure(|| drop(image(bad))), 0);
    }
    assert!(image(input).is_ok());
}
