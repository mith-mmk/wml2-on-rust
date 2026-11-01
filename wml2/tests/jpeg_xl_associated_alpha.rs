#![cfg(feature = "jpeg-xl")]
use wml2::{draw::image_from, jpegxl::*, limits::DecodeLimits};
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
fn cases() -> Vec<(&'static [u8], &'static [u8], u8, u8)> {
    let mut cases: Vec<(&'static [u8], &'static [u8], u8, u8)> = vec![
        (
            include_bytes!("fixtures/review/jxl-associated-1c8o1.jxl"),
            include_bytes!("fixtures/review/jxl-associated-1c8o1.rgba16le"),
            8,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-1c8o6.jxl"),
            include_bytes!("fixtures/review/jxl-associated-1c8o6.rgba16le"),
            8,
            6,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-1c8o8.jxl"),
            include_bytes!("fixtures/review/jxl-associated-1c8o8.rgba16le"),
            8,
            8,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-1c16o1.jxl"),
            include_bytes!("fixtures/review/jxl-associated-1c16o1.rgba16le"),
            16,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-1c16o6.jxl"),
            include_bytes!("fixtures/review/jxl-associated-1c16o6.rgba16le"),
            16,
            6,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-1c16o8.jxl"),
            include_bytes!("fixtures/review/jxl-associated-1c16o8.rgba16le"),
            16,
            8,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-3c8o1.jxl"),
            include_bytes!("fixtures/review/jxl-associated-3c8o1.rgba16le"),
            8,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-3c8o6.jxl"),
            include_bytes!("fixtures/review/jxl-associated-3c8o6.rgba16le"),
            8,
            6,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-3c8o8.jxl"),
            include_bytes!("fixtures/review/jxl-associated-3c8o8.rgba16le"),
            8,
            8,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-3c16o1.jxl"),
            include_bytes!("fixtures/review/jxl-associated-3c16o1.rgba16le"),
            16,
            1,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-3c16o6.jxl"),
            include_bytes!("fixtures/review/jxl-associated-3c16o6.rgba16le"),
            16,
            6,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-3c16o8.jxl"),
            include_bytes!("fixtures/review/jxl-associated-3c16o8.rgba16le"),
            16,
            8,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-colored-zero-8.jxl"),
            include_bytes!("fixtures/review/jxl-associated-colored-zero-8.rgba16le"),
            8,
            6,
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-colored-zero-16.jxl"),
            include_bytes!("fixtures/review/jxl-associated-colored-zero-16.rgba16le"),
            16,
            6,
        ),
        (
            include_bytes!("fixtures/review/jxl-premult.jxl"),
            include_bytes!("fixtures/review/jxl-associated-original-premult.rgba16le"),
            16,
            1,
        ),
    ];
    cases.extend(
        include!("jpeg_xl/associated_depth_cases.rs")
            .into_iter()
            .map(|(i, e, c, _a, o)| (i, e, c, o)),
    );
    cases
}
#[test]
fn straight_output_matches_independent_unpremultiply_at_source_precision() {
    for (input, expected, bits, orientation) in cases() {
        let expected: Vec<_> = expected
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let im = image_from(input).unwrap();
        assert_eq!(
            (im.width, im.height),
            if orientation >= 5 { (5, 7) } else { (7, 5) }
        );
        let raw = im.buffer.unwrap();
        assert_eq!(raw.len(), expected.len());
        for (i, (a, b)) in raw.iter().zip(&expected).enumerate() {
            let b = ((u32::from(*b) * 255 + 32767) / 65535) as u8;
            if i % 4 == 3 {
                assert_eq!(*a, b)
            } else {
                assert!(a.abs_diff(b) <= 1, "legacy {i}: {a}/{b}");
            }
        }
        #[cfg(feature = "image-buffer-ex")]
        {
            use wml2::draw::{PixelFormatEx, PrecisionConversion, image_from_ex};
            let im =
                image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert!(
                image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_err()
            );
            let actual: Vec<u16> = im
                .bytes()
                .unwrap()
                .chunks_exact(2)
                .map(|v| u16::from_le_bytes(v.try_into().unwrap()))
                .collect();
            assert_eq!(actual.len(), expected.len());
            for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
                if i % 4 == 3 {
                    assert_eq!(a, b)
                } else {
                    assert!(a.abs_diff(*b) <= 1, "native {bits} {i}: {a}/{b}");
                }
            }
        }
        #[cfg(not(feature = "image-buffer-ex"))]
        let _ = bits;
    }
}

#[cfg(feature = "color-management")]
#[test]
fn cms_sees_straight_rgb_and_never_transforms_alpha() {
    for (input, expected) in [
        (
            include_bytes!("fixtures/review/jxl-associated-3c8o8.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-associated-3c8o8.srgb.rgba").as_slice(),
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-3c16o8.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-associated-3c16o8.srgb.rgba").as_slice(),
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-depth-3c9a16o2.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-associated-depth-3c9a16o2.srgb.rgba").as_slice(),
        ),
        (
            include_bytes!("fixtures/review/jxl-associated-depth-3c16a10o7.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-associated-depth-3c16a10o7.srgb.rgba").as_slice(),
        ),
    ] {
        let im = image_from_srgb(input).unwrap();
        assert!(matches!(
            im.metadata.as_ref().unwrap()["JPEG XL Source Alpha Associated"],
            wml2::metadata::DataMap::Bool(true)
        ));
        for key in [
            "JPEG XL Source ICC Profile",
            "JPEG XL Reconstruction ICC Profile",
        ] {
            assert!(
                matches!(&im.metadata.as_ref().unwrap()[key],wml2::metadata::DataMap::ICCProfile(bytes) if bytes.as_slice()==include_bytes!("fixtures/review/jxl-gamma2.icc"))
            );
        }
        let raw = im.buffer.unwrap();
        assert_eq!(raw.len(), expected.len());
        for (i, (a, b)) in raw.iter().zip(expected).enumerate() {
            if i % 4 == 3 {
                assert_eq!(a, b)
            } else {
                assert!(a.abs_diff(*b) <= 2, "CMS {i}: {a}/{b}");
            }
        }
    }
}
#[test]
fn limits_and_truncation_preserve_atomic_result_publication() {
    let input = include_bytes!("fixtures/review/jxl-associated-depth-3c9a16o2.jxl");
    let decode = |limits, resources| image_from_with_resource_limits(input, limits, resources);
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
            animation_bytes: 139,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(decode(limits, JxlResourceLimits::default()).is_err());
    }
    let (mut lo, mut hi) = (0, 64 << 20);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if decode(
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
        decode(
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: lo,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        decode(
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: lo - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        decode(
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
    assert_eq!(
        allocation_tracking::measure(|| drop(image_from(input).unwrap())),
        0
    );
}
#[cfg(feature = "color-management")]
#[test]
fn every_associated_decode_and_cms_cancellation_checkpoint_releases_storage() {
    use std::cell::Cell;
    for input in [
        include_bytes!("fixtures/review/jxl-associated-3c16o8.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-associated-depth-3c16a10o7.jxl").as_slice(),
    ] {
        let n = Cell::new(0);
        let decode = |cancel: &dyn Fn() -> bool| {
            image_from_srgb_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                cancel,
            )
        };
        drop(
            decode(&|| {
                n.set(n.get() + 1);
                false
            })
            .unwrap(),
        );
        assert!(n.get() > 10);
        eprintln!(
            "associated decode/CMS cancellation checkpoints: {}",
            n.get()
        );
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
        drop(decode(&|| false).unwrap());
    }
}

#[test]
fn associated_internal_layers_remain_outside_first_qualified_subset() {
    assert!(
        image_from(include_bytes!(
            "fixtures/review/jxl-associated-layers-refused.jxl"
        ))
        .is_err()
    );
}

#[test]
fn mixed_associated_metadata_retains_both_depths_and_native_precision() {
    for (input, _, color, alpha, _) in include!("jpeg_xl/associated_depth_cases.rs") {
        let im = image_from(input).unwrap();
        let metadata = im.metadata.as_ref().unwrap();
        assert!(
            matches!(metadata["JPEG XL Source Bit Depth"],wml2::metadata::DataMap::UInt(v) if v==u64::from(color))
        );
        if color != alpha {
            assert!(
                matches!(metadata["JPEG XL Source Alpha Bit Depth"],wml2::metadata::DataMap::UInt(v) if v==u64::from(alpha))
            );
        }
        // No preview is a successful absence, not an unsupported-source error.
        assert!(
            preview_from_with_resource_limits(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
            )
            .unwrap()
            .is_none()
        );
    }
}
