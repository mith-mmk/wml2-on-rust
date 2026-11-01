#![cfg(feature = "jpeg-xl")]
use wml2::{draw::image_from, jpegxl::*, limits::DecodeLimits, metadata::DataMap};
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
type Case = (&'static [u8], &'static [u8], u8, u8, u8, usize, usize, u8);
fn cases() -> Vec<Case> {
    let mut cases = include!("jpeg_xl/subsampled_cases.rs").to_vec();
    cases.extend(include!("jpeg_xl/associated_subsampling_cases.rs"));
    cases
}
#[test]
fn subsampled_pixels_dimensions_and_source_metadata_match_independent_c() {
    #[cfg(feature = "image-buffer-ex")]
    let (mut rgb_diff, mut alpha_diff) = (0, 0);
    for (input, expected, color, alpha, shift, w, h, orientation) in cases() {
        let expected: Vec<_> = expected
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let im = image_from(input)
            .unwrap_or_else(|e| panic!("{color}/{alpha}/s{shift}/o{orientation}: {e}"));
        assert_eq!(
            (im.width, im.height),
            if orientation >= 5 { (h, w) } else { (w, h) }
        );
        assert!(
            matches!(im.metadata.as_ref().unwrap()["JPEG XL Source Alpha Dim Shift"],DataMap::UInt(v)if v==u64::from(shift))
        );
        for (key, value) in [
            ("JPEG XL Source Alpha Width", w.div_ceil(1usize << shift)),
            ("JPEG XL Source Alpha Height", h.div_ceil(1usize << shift)),
        ] {
            assert!(
                matches!(im.metadata.as_ref().unwrap()[key],DataMap::UInt(v) if v==value as u64)
            );
        }
        assert!(
            preview_from_with_resource_limits(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default()
            )
            .unwrap()
            .is_none()
        );
        let raw = im.buffer.unwrap();
        assert_eq!(raw.len(), expected.len());
        for (a, b) in raw.iter().zip(&expected) {
            assert!(a.abs_diff(((u32::from(*b) * 255 + 32767) / 65535) as u8) <= 1);
        }
        #[cfg(feature = "image-buffer-ex")]
        {
            use wml2::draw::{PixelFormatEx, PrecisionConversion, image_from_ex};
            let im =
                image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            let actual: Vec<_> = im
                .bytes()
                .unwrap()
                .chunks_exact(2)
                .map(|v| u16::from_le_bytes(v.try_into().unwrap()))
                .collect();
            assert_eq!(actual.len(), expected.len());
            let flat = expected.iter().skip(3).step_by(4).all(|&a| a == 0)
                || expected.iter().skip(3).step_by(4).all(|&a| a == 65535);
            for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
                if flat && i % 4 == 3 {
                    assert_eq!(a, b);
                }
                let d = a.abs_diff(*b);
                if i % 4 == 3 {
                    alpha_diff = alpha_diff.max(d)
                } else {
                    rgb_diff = rgb_diff.max(d)
                };
                assert!(
                    d <= 1,
                    "{color}/{alpha}/s{shift}/o{orientation}/i{i}: {a}/{b}"
                );
            }
            let exactly_representable = actual.iter().all(|v| v % 257 == 0);
            assert_eq!(
                image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_ok(),
                exactly_representable
            );
        }
    }
    #[cfg(feature = "image-buffer-ex")]
    eprintln!("subsampled native maximum RGB/alpha differences: {rgb_diff}/{alpha_diff}");
}

#[cfg(feature = "color-management")]
#[test]
fn cms_transforms_rgb_and_copies_interpolated_alpha_without_change() {
    for (input, expected) in [
        (
            include_bytes!("fixtures/review/jxl-subsampled-3c9a16s1o6.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-subsampled-3c9a16s1o6.srgb.rgba").as_slice(),
        ),
        (
            include_bytes!("fixtures/review/jxl-subsampled-3c16a8s3o5.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-subsampled-3c16a8s3o5.srgb.rgba").as_slice(),
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-subsampled-3c9a16s1o6.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-associated-subsampled-3c9a16s1o6.srgb.rgba")
                .as_slice(),
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-subsampled-3c16a8s3o5.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-associated-subsampled-3c16a8s3o5.srgb.rgba")
                .as_slice(),
        ),
    ] {
        let ordinary = image_from(input).unwrap().buffer.unwrap();
        let im = image_from_srgb(input).unwrap();
        for key in [
            "JPEG XL Source ICC Profile",
            "JPEG XL Reconstruction ICC Profile",
        ] {
            assert!(
                matches!(&im.metadata.as_ref().unwrap()[key],DataMap::ICCProfile(b)if b.as_slice()==include_bytes!("fixtures/review/jxl-gamma2.icc"))
            );
        }
        let actual = im.buffer.unwrap();
        assert_eq!(actual.len(), expected.len());
        for (i, (a, b)) in actual.iter().zip(expected).enumerate() {
            if i % 4 == 3 {
                assert_eq!(*a, ordinary[i]);
                assert!(a.abs_diff(*b) <= 1);
            } else {
                assert!(a.abs_diff(*b) <= 2);
            }
        }
    }
}
#[test]
fn reduced_alpha_limits_truncation_and_unsupported_combinations_are_explicit() {
    let input = include_bytes!("fixtures/review/jxl-associated-subsampled-3c16a8s3o5.jxl");
    let pixels = 33 * 17;
    for limits in [
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            pixels: pixels - 1,
            ..Default::default()
        },
        DecodeLimits {
            frames: 0,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: pixels * 4 - 1,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(
            image_from_with_resource_limits(input, limits, JxlResourceLimits::default()).is_err()
        );
    }
    for (cap, accepted) in [(959, false), (960, true)] {
        assert_eq!(
            image_from_with_resource_limits(
                input,
                DecodeLimits::default(),
                JxlResourceLimits {
                    internal_pixels: cap,
                    ..Default::default()
                }
            )
            .is_ok(),
            accepted
        );
    }
    let (mut lo, mut hi) = (0, 64 << 20);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if image_from_with_resource_limits(
            input,
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
        image_from_with_resource_limits(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: lo,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        image_from_with_resource_limits(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: lo - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        image_from_with_resource_limits(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                internal_layers: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    for end in 0..input.len() {
        assert!(image_from(&input[..end]).is_err());
    }
    for input in [
        include_bytes!("fixtures/review/jxl-associated-subsampled-layers-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-subsampled-refused-subsampled-layers.jxl").as_slice(),
    ] {
        assert!(image_from(input).is_err());
    }
    assert_eq!(
        allocation_tracking::measure(|| drop(image_from(input).unwrap())),
        0
    );
}
#[cfg(feature = "color-management")]
#[test]
fn each_subsampled_decode_cms_checkpoint_releases_kernel_and_owned_buffers() {
    use std::cell::Cell;
    for input in [
        include_bytes!("fixtures/review/jxl-subsampled-3c9a16s1o6.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-subsampled-3c16a8s3o5.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-associated-subsampled-3c9a16s1o6.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-associated-subsampled-3c16a8s3o5.jxl").as_slice(),
    ] {
        let decode = |cancel: &dyn Fn() -> bool| {
            image_from_srgb_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                cancel,
            )
        };
        let n = Cell::new(0);
        drop(
            decode(&|| {
                n.set(n.get() + 1);
                false
            })
            .unwrap(),
        );
        assert!(n.get() > 10);
        eprintln!("subsampled cancellation checkpoints: {}", n.get());
        for stop in 1..=n.get() {
            let count = Cell::new(0);
            assert_eq!(
                allocation_tracking::measure(|| {
                    let e = decode(&|| {
                        count.set(count.get() + 1);
                        count.get() == stop
                    })
                    .err()
                    .unwrap();
                    assert!(e.is::<DecodeCancelled>());
                    drop(e);
                }),
                0
            );
        }
    }
}

#[test]
fn association_descriptor_and_rounding_remain_explicit() {
    for (input, _, _, _, _, _, _, _) in include!("jpeg_xl/associated_subsampling_cases.rs") {
        let ordinary = image_from(input).unwrap();
        assert!(matches!(
            ordinary.metadata.as_ref().unwrap()["JPEG XL Source Alpha Associated"],
            DataMap::Bool(true)
        ));
        #[cfg(feature = "image-buffer-ex")]
        {
            use wml2::draw::{PixelFormatEx, PrecisionConversion, image_from_ex};
            let im =
                image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Round).unwrap();
            assert_eq!(im.bytes().unwrap(), ordinary.buffer.as_ref().unwrap());
        }
    }
}
