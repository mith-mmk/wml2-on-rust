use std::{
    cell::Cell,
    sync::atomic::{AtomicU32, Ordering},
};
use wml2::{
    jpegxl::{
        DecodeCancelled, JxlEncodedHdrImage, JxlHdrLinearization, JxlHdrTransfer,
        JxlLinearHdrImage, JxlResourceLimits,
        image_hdr_encoded_from_with_resource_limits_and_cancel as decode,
    },
    limits::DecodeLimits,
};
static CHECKS: AtomicU32 = AtomicU32::new(0);
static PROBES: AtomicU32 = AtomicU32::new(0);
#[unsafe(no_mangle)]
pub extern "C" fn checks() -> u32 {
    CHECKS.load(Ordering::Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn probes() -> u32 {
    PROBES.load(Ordering::Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn run() -> u32 {
    let mut checks = 0;
    for (input, expected, linear_expected, transfer, intensity, min_nits, relative, linear) in [
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-pq-rgb.jxl")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-pq-rgb.f32le.zlib")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-linear-hdr-pq-rgb.f32le.zlib")
                .as_slice(),
            JxlHdrTransfer::Pq,
            255.0f32,
            0.0f32,
            false,
            0.0f32,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-pq-rgba.jxl")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-pq-rgba.f32le.zlib")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-linear-hdr-pq-rgba.f32le.zlib")
                .as_slice(),
            JxlHdrTransfer::Pq,
            1000.0f32,
            0.0199890137f32,
            true,
            0.5f32,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-pq-oriented.jxl")
                .as_slice(),
            include_bytes!(
                "../../wml2/tests/fixtures/review/jxl-encoded-hdr-pq-oriented.f32le.zlib"
            )
            .as_slice(),
            include_bytes!(
                "../../wml2/tests/fixtures/review/jxl-linear-hdr-pq-oriented.f32le.zlib"
            )
            .as_slice(),
            JxlHdrTransfer::Pq,
            2000.0f32,
            0.00499725342f32,
            false,
            10.0f32,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-hlg-rgb.jxl")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-hlg-rgb.f32le.zlib")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-linear-hdr-hlg-rgb.f32le.zlib")
                .as_slice(),
            JxlHdrTransfer::Hlg,
            255.0f32,
            0.0f32,
            false,
            0.0f32,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-hlg-rgba.jxl")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-hlg-rgba.f32le.zlib")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-linear-hdr-hlg-rgba.f32le.zlib")
                .as_slice(),
            JxlHdrTransfer::Hlg,
            1000.0f32,
            0.0199890137f32,
            true,
            0.5f32,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-hlg-oriented.jxl")
                .as_slice(),
            include_bytes!(
                "../../wml2/tests/fixtures/review/jxl-encoded-hdr-hlg-oriented.f32le.zlib"
            )
            .as_slice(),
            include_bytes!(
                "../../wml2/tests/fixtures/review/jxl-linear-hdr-hlg-oriented.f32le.zlib"
            )
            .as_slice(),
            JxlHdrTransfer::Hlg,
            2000.0f32,
            0.00499725342f32,
            false,
            10.0f32,
        ),
    ] {
        let expected = miniz_oxide::inflate::decompress_to_vec_zlib(expected).unwrap();
        let im = match decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        ) {
            Ok(v) => v,
            Err(_) => return 1,
        };
        if im.rgba_encoded.len() * 4 != expected.len()
            || im
                .rgba_encoded
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .zip(&expected)
                .any(|(a, &b)| a != b)
        {
            return 2;
        }
        if (
            im.signal.transfer,
            im.signal.intensity_target,
            im.signal.min_nits,
            im.signal.relative_to_max_display,
            im.signal.linear_below,
        ) != (transfer, intensity, min_nits, relative, linear)
        {
            return 3;
        }
        checks += 1;
        let mode = if transfer == JxlHdrTransfer::Pq {
            JxlHdrLinearization::PqDisplayNits
        } else {
            JxlHdrLinearization::HlgSceneRelative
        };
        let ptr = im.rgba_encoded.as_ptr();
        let linear_expected =
            miniz_oxide::inflate::decompress_to_vec_zlib(linear_expected).unwrap();
        let result = im
            .into_linear_with_resource_limits_and_cancel(
                mode,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false,
            )
            .unwrap();
        if ptr != result.rgba_linear.as_ptr()
            || result
                .rgba_linear
                .iter()
                .zip(linear_expected.chunks_exact(4))
                .any(|(a, b)| {
                    a.to_bits()
                        .abs_diff(f32::from_le_bytes(b.try_into().unwrap()).to_bits())
                        > 2
                })
        {
            return 12;
        }
        checks += 1;
        match decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        )
        .unwrap()
        .into_linear_with_resource_limits_and_cancel(
            mode,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| true,
        ) {
            Err(e) if e.is::<DecodeCancelled>() => checks += 1,
            _ => return 13,
        }
        match decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| true,
        ) {
            Err(e) if e.is::<DecodeCancelled>() => checks += 1,
            _ => return 4,
        }
        if wml2::draw::image_from(input).is_ok() {
            return 5;
        }
        checks += 1;
        if wml2::jpegxl::image_f32_from_with_resource_limits_and_cancel(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        )
        .is_ok()
        {
            return 6;
        }
        checks += 1;
    }
    for input in [
        include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-p3-refused.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-icc-refused.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-associated-refused.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-tone-infinite-bad.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-tone-zero-bad.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f32-pq-refused.jxl").as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba8.jxl").as_slice(),
    ] {
        if decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        )
        .is_ok()
        {
            return 7;
        }
        checks += 1;
    }
    let input = include_bytes!("../../wml2/tests/fixtures/review/jxl-encoded-hdr-hlg-rgba.jxl");
    let count = Cell::new(0);
    if decode(
        input,
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| {
            count.set(count.get() + 1);
            false
        },
    )
    .is_err()
    {
        return 8;
    }
    let n = count.get();
    PROBES.store(n, Ordering::Relaxed);
    for stop in 1..=n {
        let count = Cell::new(0);
        match decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| {
                count.set(count.get() + 1);
                count.get() == stop
            },
        ) {
            Err(e) if e.is::<DecodeCancelled>() => checks += 1,
            _ => return 9,
        }
    }
    if decode(
        input,
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| false,
    )
    .is_err()
    {
        return 10;
    }
    checks += 1;
    for limits in [
        DecodeLimits {
            animation_bytes: 239,
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
        if decode(input, limits, JxlResourceLimits::default(), &|| false).is_ok() {
            return 11;
        }
        checks += 1;
    }
    let make = || {
        let mut im = decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        )
        .unwrap();
        im.dimensions.coded.width = 600;
        im.dimensions.coded.height = 1;
        im.dimensions.output = Some(im.dimensions.coded);
        im.rgba_encoded = vec![0.5; 2400];
        im
    };
    let count = Cell::new(0);
    drop(
        make()
            .into_linear_with_resource_limits_and_cancel(
                JxlHdrLinearization::HlgSceneRelative,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| {
                    count.set(count.get() + 1);
                    false
                },
            )
            .unwrap(),
    );
    let linear_probes = count.get();
    if linear_probes != 6 {
        return 14;
    }
    PROBES.store(n + linear_probes, Ordering::Relaxed);
    for stop in 1..=linear_probes {
        let count = Cell::new(0);
        match make().into_linear_with_resource_limits_and_cancel(
            JxlHdrLinearization::HlgSceneRelative,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| {
                count.set(count.get() + 1);
                count.get() == stop
            },
        ) {
            Err(e) if e.is::<DecodeCancelled>() => checks += 1,
            _ => return 15,
        }
    }
    for delta in [0, 1] {
        let im = make();
        let bytes = im.rgba_encoded.capacity() * 4
            + std::mem::size_of::<JxlEncodedHdrImage>()
                .max(std::mem::size_of::<JxlLinearHdrImage>())
            + 128;
        let result = im.into_linear_with_resource_limits_and_cancel(
            JxlHdrLinearization::HlgSceneRelative,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: bytes - delta,
                ..Default::default()
            },
            &|| false,
        );
        if result.is_ok() != (delta == 0) {
            return 16;
        }
        checks += 1;
    }
    if make()
        .into_linear_with_resource_limits_and_cancel(
            JxlHdrLinearization::PqDisplayNits,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        )
        .is_ok()
    {
        return 17;
    }
    checks += 1;
    let mut im = make();
    im.rgba_encoded[3] = f32::NAN;
    if im
        .into_linear_with_resource_limits_and_cancel(
            JxlHdrLinearization::HlgSceneRelative,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        )
        .is_ok()
    {
        return 18;
    }
    checks += 1;
    CHECKS.store(checks, Ordering::Relaxed);
    0
}
