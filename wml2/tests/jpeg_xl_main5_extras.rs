#![cfg(feature = "jpeg-xl")]
use JxlExtraChannelKind::{Alpha, Depth, SelectionMask};
use wml2::{jpegxl::*, limits::DecodeLimits};
fn decode(input: &[u8]) -> JxlExtraImage {
    image_with_extra_channels_from_with_resource_limits_and_cancel(
        input,
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| false,
    )
    .unwrap()
}
#[test]
fn all_main5_planes_and_straight_alpha_match_independent_c() {
    for (input, color, planes, bits, kinds, o, cb) in include!("jpeg_xl/main5_extra_cases.rs") {
        let im = decode(input);
        let size = im.dimensions.output.unwrap();
        assert_eq!(
            (size.width, size.height),
            if o >= 5 { (5, 7) } else { (7, 5) }
        );
        assert_eq!(im.source_orientation, o);
        assert_eq!(im.color_bits_per_sample, cb);
        assert_eq!(im.extra_planes.len(), planes.len());
        assert!(
            input
                .windows(im.source_header.len())
                .any(|b| b == im.source_header)
        );
        for (i, gold) in planes.iter().enumerate() {
            let p = &im.extra_planes[i];
            assert_eq!(p.source_index, i);
            assert_eq!(p.kind, kinds[i]);
            assert_eq!(p.bits_per_sample, bits[i]);
            assert_eq!(p.dim_shift, 0);
            assert_eq!(
                (p.source_dimensions.width, p.source_dimensions.height),
                (7, 5)
            );
            assert_eq!(p.dimensions, size);
            let expected: Vec<_> = gold
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
                .collect();
            assert_eq!(p.samples, expected);
        }
        for (i, p) in im.extra_planes.iter().enumerate() {
            for q in &im.extra_planes[i + 1..] {
                assert_ne!(p.samples.as_ptr(), q.samples.as_ptr());
            }
        }
        let color: Vec<_> = color
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(im.rgba16.len(), color.len());
        for (a, b) in im.rgba16.iter().zip(color) {
            assert!(a.abs_diff(b) <= 1, "{cb}/{o}: {a}/{b}")
        }
        assert_eq!(wml2::draw::image_from(input).is_ok(), kinds == [Alpha]);
    }
    for (input, count) in [
        (
            include_bytes!("fixtures/review/jxl-alpha-refused-multiple.jxl").as_slice(),
            2,
        ),
        (
            include_bytes!("fixtures/review/jxl-two-refused-three.jxl").as_slice(),
            3,
        ),
    ] {
        assert_eq!(decode(input).extra_planes.len(), count);
    }
}
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
#[test]
fn both_outputs_and_retained_metadata_have_joint_limits() {
    let input = include_bytes!("fixtures/review/jxl-main5-3c8-AMDM-12-16-8-9-o4.jxl");
    let run = |limits, policy| {
        image_with_extra_channels_from_with_resource_limits_and_cancel(
            input,
            limits,
            policy,
            &|| false,
        )
    };
    for limits in [
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            pixels: 34,
            ..Default::default()
        },
        DecodeLimits {
            frames: 0,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 559,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(run(limits, JxlResourceLimits::default()).is_err());
    }
    assert!(
        run(
            DecodeLimits {
                animation_bytes: 560,
                ..Default::default()
            },
            JxlResourceLimits::default()
        )
        .is_ok()
    );
    let (mut lo, mut hi) = (0, 64 << 20);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if run(
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: mid,
                ..Default::default()
            },
        )
        .is_ok()
        {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    assert!(
        run(
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: lo,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        run(
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: lo - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        run(
            DecodeLimits::default(),
            JxlResourceLimits {
                internal_layers: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    for end in 0..input.len() {
        assert!(
            image_with_extra_channels_from_with_resource_limits_and_cancel(
                &input[..end],
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
    assert_eq!(allocation_tracking::measure(|| drop(decode(input))), 0);
}
#[test]
fn source_profiles_do_not_transform_raw_alpha_or_other_planes() {
    for input in [
        include_bytes!("fixtures/review/jxl-main5-3c8-AMDM-12-16-8-9-o4.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-main5-3c16-DMDM-8-9-12-16-o5.jxl").as_slice(),
    ] {
        let im = decode(input);
        assert_eq!(
            im.source_icc,
            include_bytes!("fixtures/review/jxl-gamma2.icc")
        );
        assert_eq!(im.output_icc, im.source_icc);
    }
}
#[test]
fn all_unrepresented_channels_and_unqualified_modes_reject() {
    let selection_input = include_bytes!("fixtures/review/jxl-main5-3c8-AMDM-12-16-8-9-o4.jxl");
    #[cfg(feature = "color-management")]
    assert!(image_from_srgb(selection_input).is_err());
    assert!(
        image_f32_from_with_resource_limits_and_cancel(
            selection_input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert!(
        image_f16_as_f32_from_with_resource_limits_and_cancel(
            selection_input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert!(
        image_hdr_encoded_from_with_resource_limits_and_cancel(
            selection_input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false
        )
        .is_err()
    );
    #[cfg(feature = "image-buffer-ex")]
    assert!(
        wml2::draw::image_from_ex(
            selection_input,
            wml2::draw::PixelFormatEx::Rgba16,
            wml2::draw::PrecisionConversion::Exact
        )
        .is_err()
    );

    for input in [
        include_bytes!("fixtures/review/jxl-main5-refused-five.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-main5-refused-multiple-alpha.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-main5-refused-spot.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-main5-refused-float.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-main5-refused-subsampled.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-main5-refused-layers.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-alpha-refused-spot.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-alpha-refused-multiple-alpha.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-alpha-refused-float-alpha.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-hdr.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-selection-refused-selection-float.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-selection-refused-selection-subsampled.jxl").as_slice(),
    ] {
        assert!(
            image_with_extra_channels_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
}
#[test]
fn cancellation_never_publishes_half_color_or_half_plane() {
    use std::cell::Cell;
    let input = include_bytes!("fixtures/review/jxl-main5-3c8-AMDM-12-16-8-9-o4.jxl");
    let count = Cell::new(0);
    let run = |cancel: &dyn Fn() -> bool| {
        image_with_extra_channels_from_with_resource_limits_and_cancel(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            cancel,
        )
    };
    drop(
        run(&|| {
            count.set(count.get() + 1);
            false
        })
        .unwrap(),
    );
    let n = count.get();
    assert!(n > 10);
    eprintln!("Main5 extras/Alpha cancellation checkpoints: {n}");
    for stop in 1..=n {
        let c = Cell::new(0);
        assert_eq!(
            allocation_tracking::measure(|| {
                let e = run(&|| {
                    c.set(c.get() + 1);
                    c.get() == stop
                })
                .unwrap_err();
                assert!(e.is::<DecodeCancelled>());
                drop(e);
            }),
            0
        );
    }
    let e = image_with_extra_channels_from_with_resource_limits_and_cancel(
        &[],
        DecodeLimits::default(),
        JxlResourceLimits {
            workspace_bytes: usize::MAX,
            ..Default::default()
        },
        &|| true,
    )
    .unwrap_err();
    assert!(e.is::<DecodeCancelled>());
}
