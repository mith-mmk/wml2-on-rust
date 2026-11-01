#![cfg(feature = "jpeg-xl")]
use wml2::{jpegxl::*, limits::DecodeLimits};
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
fn image(h: bool) -> JxlEncodedHdrImage {
    let b = if h {
        include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-rgba.jxl").as_slice()
    } else {
        include_bytes!("fixtures/review/jxl-encoded-hdr-pq-rgba.jxl").as_slice()
    };
    image_hdr_encoded_from_with_resource_limits_and_cancel(
        b,
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn convert(im: JxlEncodedHdrImage, h: bool) -> JxlLinearHdrImage {
    im.into_linear_with_resource_limits_and_cancel(
        if h {
            JxlHdrLinearization::HlgSceneRelative
        } else {
            JxlHdrLinearization::PqDisplayNits
        },
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| false,
    )
    .unwrap()
}
#[test]
fn transfer_matches_high_precision_independent_oracle_without_reallocation() {
    for (input, expected, h) in [
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-rgb.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-linear-hdr-pq-rgb.f32le.zlib").as_slice(),
            false,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-rgba.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-linear-hdr-pq-rgba.f32le.zlib").as_slice(),
            false,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-oriented.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-linear-hdr-pq-oriented.f32le.zlib").as_slice(),
            false,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-rgb.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-linear-hdr-hlg-rgb.f32le.zlib").as_slice(),
            true,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-rgba.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-linear-hdr-hlg-rgba.f32le.zlib").as_slice(),
            true,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-oriented.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-linear-hdr-hlg-oriented.f32le.zlib").as_slice(),
            true,
        ),
    ] {
        let im = image_hdr_encoded_from_with_resource_limits_and_cancel(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        )
        .unwrap();
        let ptr = im.rgba_encoded.as_ptr();
        let alpha: Vec<_> = im
            .rgba_encoded
            .chunks_exact(4)
            .map(|p| p[3].to_bits())
            .collect();
        let dimensions = im.dimensions;
        let signal = im.signal;
        let linear = convert(im, h);
        assert_eq!(ptr, linear.rgba_linear.as_ptr());
        assert_eq!(dimensions, linear.dimensions);
        assert_eq!(signal, linear.source_signal);
        let expected = miniz_oxide::inflate::decompress_to_vec_zlib(expected).unwrap();
        for (i, (actual, bytes)) in linear
            .rgba_linear
            .iter()
            .zip(expected.chunks_exact(4))
            .enumerate()
        {
            let reference = f32::from_le_bytes(bytes.try_into().unwrap());
            assert!(
                actual.to_bits().abs_diff(reference.to_bits()) <= 2,
                "sample{i}: {actual} vs{reference}"
            );
            if i % 4 == 3 {
                assert_eq!(actual.to_bits(), alpha[i / 4]);
            }
        }
    }
}
#[test]
fn anchors_breakpoint_and_source_hints_do_not_select_a_display() {
    for h in [false, true] {
        let mut im = image(h);
        for p in im.rgba_encoded.chunks_exact_mut(4) {
            p.copy_from_slice(&[0.0, 0.5, 1.0, -0.0]);
        }
        let out = convert(im, h);
        let expected = if h {
            [0.0, 1.0 / 12.0, 1.0]
        } else {
            [0.0, 92.24571, 10000.0]
        };
        for (a, b) in out.rgba_linear[..3].iter().zip(expected) {
            assert!((a - b).abs() <= b.abs() * 2e-7 + 1e-9);
        }
        assert_eq!(out.rgba_linear[3].to_bits(), (-0.0f32).to_bits());
        let mut a = image(h);
        a.signal.intensity_target = 10000.0;
        a.signal.min_nits = 0.0;
        a.signal.linear_below = 0.0;
        assert_eq!(convert(a, h).rgba_linear, convert(image(h), h).rgba_linear);
        let mut a = image(h);
        a.rgba_encoded[..3].copy_from_slice(&[
            f32::from_bits(0x3effffff),
            0.5,
            f32::from_bits(0x3f000001),
        ]);
        let a = convert(a, h);
        assert!(a.rgba_linear[0] < a.rgba_linear[1] && a.rgba_linear[1] < a.rgba_linear[2]);
    }
}
#[test]
fn malformed_public_carrier_and_exact_capacity_budget_are_checked() {
    let rejected = |im: JxlEncodedHdrImage| {
        assert!(
            im.into_linear_with_resource_limits_and_cancel(
                JxlHdrLinearization::PqDisplayNits,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_err()
        )
    };
    rejected(image(true));
    let mut late = image(false);
    *late.rgba_encoded.last_mut().unwrap() = f32::NAN;
    rejected(late);
    for bad in [f32::NAN, f32::INFINITY, -0.01, 1.001] {
        let mut im = image(false);
        im.rgba_encoded[0] = bad;
        rejected(im);
        let mut im = image(false);
        im.rgba_encoded[3] = bad;
        rejected(im);
    }
    let mut im = image(false);
    im.dimensions.coded.width = u32::MAX;
    rejected(im);
    let mut im = image(false);
    im.source_orientation = 0;
    rejected(im);
    let mut im = image(false);
    im.rgba_encoded.pop();
    rejected(im);
    let mut im = image(false);
    im.signal.white_xy[0] = 0.5;
    rejected(im);
    let mut im = image(false);
    im.signal.intensity_target = f32::NAN;
    rejected(im);
    let mut im = image(false);
    im.signal.relative_to_max_display = true;
    im.signal.linear_below = 2.0;
    rejected(im);
    for extra in [0, 1024] {
        let mut im = image(false);
        im.rgba_encoded.reserve_exact(extra);
        let bytes = im.rgba_encoded.capacity() * 4
            + std::mem::size_of::<JxlEncodedHdrImage>()
                .max(std::mem::size_of::<JxlLinearHdrImage>())
            + 128;
        let run = |im: JxlEncodedHdrImage, b| {
            im.into_linear_with_resource_limits_and_cancel(
                JxlHdrLinearization::PqDisplayNits,
                DecodeLimits::default(),
                JxlResourceLimits {
                    workspace_bytes: b,
                    ..Default::default()
                },
                &|| false,
            )
        };
        assert!(run(im, bytes).is_ok());
        let mut im = image(false);
        im.rgba_encoded.reserve_exact(extra);
        assert!(run(im, bytes - 1).is_err());
    }
    for limits in [
        DecodeLimits {
            pixels: 14,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 239,
            ..Default::default()
        },
        DecodeLimits {
            frames: 0,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(
            image(false)
                .into_linear_with_resource_limits_and_cancel(
                    JxlHdrLinearization::PqDisplayNits,
                    limits,
                    JxlResourceLimits::default(),
                    &|| false
                )
                .is_err()
        );
    }
}
#[test]
fn every_conversion_checkpoint_drops_consumed_storage() {
    use std::cell::Cell;
    // More than2 batches, independently of decoder fixture size.
    let make = || {
        let mut im = image(false);
        im.dimensions.coded.width = 600;
        im.dimensions.coded.height = 1;
        im.dimensions.output = Some(im.dimensions.coded);
        im.rgba_encoded = vec![0.5; 2400];
        im
    };
    let n = Cell::new(0);
    drop(
        make()
            .into_linear_with_resource_limits_and_cancel(
                JxlHdrLinearization::PqDisplayNits,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| {
                    n.set(n.get() + 1);
                    false
                },
            )
            .unwrap(),
    );
    assert_eq!(n.get(), 6);
    for stop in 1..=n.get() {
        let count = Cell::new(0);
        assert_eq!(
            allocation_tracking::measure(|| {
                let error = make()
                    .into_linear_with_resource_limits_and_cancel(
                        JxlHdrLinearization::PqDisplayNits,
                        DecodeLimits::default(),
                        JxlResourceLimits::default(),
                        &|| {
                            count.set(count.get() + 1);
                            count.get() == stop
                        },
                    )
                    .unwrap_err();
                assert!(error.is::<DecodeCancelled>());
                drop(error);
            }),
            0
        );
    }
}

#[test]
fn already_oriented_geometry_is_retained_for_all_eight_tags() {
    for orientation in 1..=8 {
        let mut im = image(false);
        im.source_orientation = orientation;
        if orientation >= 5 {
            let coded = im.dimensions.coded;
            im.dimensions.output = Some(wml2::image_info::ImageSize {
                width: coded.height,
                height: coded.width,
            });
        }
        // Carrier values are already in output order; conversion must not permute them.
        let expected = convert(image(false), false).rgba_linear;
        let dimensions = im.dimensions;
        let im = convert(im, false);
        assert_eq!(im.dimensions, dimensions);
        assert_eq!(im.source_orientation, orientation);
        assert_eq!(im.rgba_linear, expected);
    }
}
