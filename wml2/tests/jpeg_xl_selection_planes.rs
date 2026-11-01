#![cfg(feature = "jpeg-xl")]
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
fn selection_codes_and_separate_color_match_independent_c() {
    for (input, color, selection, cb, db, o) in include!("jpeg_xl/selection_cases.rs") {
        let im = decode(input);
        let size = im.dimensions.output.unwrap();
        assert_eq!(
            (size.width, size.height),
            if o >= 5 { (5, 7) } else { (7, 5) }
        );
        assert_eq!(im.source_orientation, o);
        assert_eq!(im.color_bits_per_sample, cb);
        assert!(
            input
                .windows(im.source_header.len())
                .any(|b| b == im.source_header)
        );
        assert!(!im.source_header.is_empty());
        assert_eq!(im.extra_planes.len(), 1);
        let p = &im.extra_planes[0];
        assert_eq!(p.kind, JxlExtraChannelKind::SelectionMask);
        assert_eq!(p.source_index, 0);
        assert_eq!(p.bits_per_sample, db);
        assert_eq!(p.dim_shift, 0);
        assert_eq!(p.dimensions, size);
        let expected: Vec<_> = selection
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(p.samples, expected);
        let expected: Vec<_> = color
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(im.rgba16.len(), expected.len());
        for (i, (a, b)) in im.rgba16.iter().zip(&expected).enumerate() {
            if i % 4 == 3 {
                assert_eq!(*a, 65535);
            } else {
                assert!(a.abs_diff(*b) <= 1, "{cb}/{db}/{o}/{i}: {a}/{b}");
            }
        }
        assert!(wml2::draw::image_from(input).is_err());
    }
}

#[test]
fn zero_extra_controls_keep_color_and_no_false_plane() {
    for (input, expected, channels) in [
        (
            include_bytes!("fixtures/review/jxl-gray16.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-gray16.bin").as_slice(),
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-rgb16.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-rgb16.bin").as_slice(),
            3,
        ),
    ] {
        let im = decode(input);
        assert!(im.extra_planes.is_empty());
        assert_eq!(im.color_channels, channels as u8);
        let expected: Vec<_> = expected
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        for (i, p) in im.rgba16.chunks_exact(4).enumerate() {
            assert_eq!(p[3], 65535);
            for c in 0..3 {
                assert_eq!(
                    p[c],
                    expected[i * channels + if channels == 1 { 0 } else { c }]
                );
            }
        }
    }
}
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
#[test]
fn both_outputs_and_retained_metadata_have_joint_limits() {
    let input = include_bytes!("fixtures/review/jxl-selection-3c8d16o5.jxl");
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
            animation_bytes: 349,
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
                animation_bytes: 350,
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
fn source_profiles_are_not_applied_to_selection_codes() {
    for input in [
        include_bytes!("fixtures/review/jxl-selection-3c8d16o5.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-selection-3c16d8o5.jxl").as_slice(),
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
    let selection_input = include_bytes!("fixtures/review/jxl-selection-3c8d16o5.jxl");
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
        include_bytes!("fixtures/review/jxl-rgba8.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-alpha-refused-multiple.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-alpha-refused-spot.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-alpha-refused-multiple-alpha.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-alpha-refused-float-alpha.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-preview-icc16.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-hdr.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-selection-refused-selection-multiple.jxl").as_slice(),
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
    let input = include_bytes!("fixtures/review/jxl-selection-3c8d16o5.jxl");
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
    eprintln!("SelectionMask output cancellation checkpoints: {n}");
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
