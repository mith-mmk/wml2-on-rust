use std::{
    cell::Cell,
    sync::atomic::{AtomicU32, Ordering},
};
use wml2::{
    jpegxl::{
        DecodeCancelled, JxlResourceLimits,
        image_f16_as_f32_from_with_resource_limits_and_cancel as decode,
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
    for (input, expected) in [
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-rgb.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-rgb.f32le.zlib")
                .as_slice(),
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-rgba.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-rgba.f32le.zlib")
                .as_slice(),
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-gray.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-gray.f32le.zlib")
                .as_slice(),
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-grayalpha.jxl")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-grayalpha.f32le.zlib")
                .as_slice(),
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-grayicc.jxl")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-grayicc.f32le.zlib")
                .as_slice(),
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-icc.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-icc.f32le.zlib")
                .as_slice(),
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-oriented.jxl")
                .as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-oriented.f32le.zlib")
                .as_slice(),
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
        if im.rgba.len() * 4 != expected.len()
            || im
                .rgba
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .zip(&expected)
                .any(|(a, &b)| a != b)
        {
            return 2;
        }
        checks += 1;
        match decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| true,
        ) {
            Err(e) if e.is::<DecodeCancelled>() => checks += 1,
            _ => return 3,
        };
    }
    for input in [
        include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-float32-refused.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-pq-refused.jxl").as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-hlg-refused.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-nonfinite-refused.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-alpha-refused.jxl")
            .as_slice(),
    ] {
        if decode(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
        )
        .is_ok()
        {
            return 4;
        }
        checks += 1;
    }
    let input = include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-icc.jxl");
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
        return 5;
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
            _ => return 6,
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
        return 7;
    }
    checks += 1;
    if wml2::jpegxl::image_f32_from_with_resource_limits_and_cancel(
        include_bytes!("../../wml2/tests/fixtures/review/jxl-native-f16-float32-refused.jxl"),
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| false,
    )
    .is_err()
    {
        return 8;
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
        return 9;
    }
    checks += 1;
    CHECKS.store(checks, Ordering::Relaxed);
    0
}
