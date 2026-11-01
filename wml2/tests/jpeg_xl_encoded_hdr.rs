#![cfg(feature = "jpeg-xl")]
use wml2::{
    jpegxl::{
        DecodeCancelled, JxlHdrTransfer, JxlResourceLimits,
        image_hdr_encoded_from_with_resource_limits_and_cancel as decode,
    },
    limits::DecodeLimits,
};
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
fn image(b: &[u8]) -> Result<wml2::jpegxl::JxlEncodedHdrImage, Box<dyn std::error::Error>> {
    decode(
        b,
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| false,
    )
}
#[test]
fn encoded_pq_hlg_match_independent_codes_and_signal_metadata() {
    for (input, expected, transfer, w, h, intensity, min_nits, relative, linear) in [
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-rgb.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-rgb.f32le.zlib").as_slice(),
            JxlHdrTransfer::Pq,
            5,
            3,
            255.0f32,
            0.0f32,
            false,
            0.0f32,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-rgba.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-rgba.f32le.zlib").as_slice(),
            JxlHdrTransfer::Pq,
            5,
            3,
            1000.0f32,
            0.0199890137f32,
            true,
            0.5f32,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-oriented.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-encoded-hdr-pq-oriented.f32le.zlib").as_slice(),
            JxlHdrTransfer::Pq,
            3,
            5,
            2000.0f32,
            0.00499725342f32,
            false,
            10.0f32,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-rgb.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-rgb.f32le.zlib").as_slice(),
            JxlHdrTransfer::Hlg,
            5,
            3,
            255.0f32,
            0.0f32,
            false,
            0.0f32,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-rgba.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-rgba.f32le.zlib").as_slice(),
            JxlHdrTransfer::Hlg,
            5,
            3,
            1000.0f32,
            0.0199890137f32,
            true,
            0.5f32,
        ),
        (
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-oriented.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-oriented.f32le.zlib").as_slice(),
            JxlHdrTransfer::Hlg,
            3,
            5,
            2000.0f32,
            0.00499725342f32,
            false,
            10.0f32,
        ),
    ] {
        let im = image(input).unwrap();
        let size = im.dimensions.output.unwrap();
        assert_eq!((size.width, size.height), (w, h));
        let expected = miniz_oxide::inflate::decompress_to_vec_zlib(expected).unwrap();
        assert_eq!(
            im.rgba_encoded
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(im.signal.transfer, transfer);
        assert_eq!(
            (
                im.signal.intensity_target,
                im.signal.min_nits,
                im.signal.relative_to_max_display,
                im.signal.linear_below
            ),
            (intensity, min_nits, relative, linear)
        );
        assert_eq!(
            im.signal.primaries_xy,
            [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]]
        );
        assert_eq!(im.signal.white_xy, [0.3127, 0.3290]);
        assert!(
            im.rgba_encoded
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        );
        assert!(im.rgba_encoded.contains(&0.0));
        assert!(im.rgba_encoded.contains(&1.0));
        assert!(wml2::draw::image_from(input).is_err());
        #[cfg(feature = "color-management")]
        assert!(wml2::jpegxl::image_from_srgb(input).is_err());
        assert!(
            wml2::jpegxl::image_f32_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_err()
        );
        assert!(
            wml2::jpegxl::image_f16_as_f32_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
    let pq = image(include_bytes!(
        "fixtures/review/jxl-encoded-hdr-pq-rgba.jxl"
    ))
    .unwrap();
    let hlg = image(include_bytes!(
        "fixtures/review/jxl-encoded-hdr-hlg-rgba.jxl"
    ))
    .unwrap();
    assert_eq!(pq.rgba_encoded, hlg.rgba_encoded);
    assert_ne!(pq.signal.transfer, hlg.signal.transfer);
}
#[test]
fn hdr_limits_and_truncation_never_publish_partial_output() {
    let input = include_bytes!("fixtures/review/jxl-encoded-hdr-pq-rgba.jxl");
    for end in 0..input.len() {
        assert!(image(&input[..end]).is_err());
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
    let (mut lo, mut hi) = (0, 64 << 20);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
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
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    assert!(
        decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: lo,
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
                workspace_bytes: lo - 1,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    for bad in [
        include_bytes!("fixtures/review/jxl-encoded-hdr-p3-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-encoded-hdr-associated-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-encoded-hdr-icc-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-encoded-hdr-tone-infinite-bad.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-encoded-hdr-tone-zero-bad.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-rgba8.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-native-f32-pq-refused.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-native-f32-icc.jxl").as_slice(),
    ] {
        assert!(image(bad).is_err());
    }
}
#[test]
fn hdr_cancellation_releases_owned_storage_at_every_checkpoint() {
    use std::cell::Cell;
    let input = include_bytes!("fixtures/review/jxl-encoded-hdr-hlg-rgba.jxl");
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
    let n = count.get();
    assert!(n > 10);
    eprintln!("HDR preservation checkpoints: {n}");
    for stop in 1..=n {
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
        assert!(typed.get());
        assert_eq!(live, 0);
    }
    assert_eq!(
        allocation_tracking::measure(|| drop(image(input).unwrap())),
        0
    );
}
