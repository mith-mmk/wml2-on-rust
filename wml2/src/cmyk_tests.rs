use super::*;
#[cfg(feature = "color-management")]
#[path = "../tests/common/cmyk_profile.rs"]
mod profiles;

#[test]
fn legacy_policies_remain_distinct_and_alpha_is_not_colour() {
    let input = [64, 96, 128, 32, 0, 0, 0, 255];
    for (policy, expected) in [
        (Approximation::Additive, [159, 127, 95, 13]),
        (Approximation::Multiplicative, [167, 139, 111, 13]),
    ] {
        let (mut converter, warning) = CmykConverter::prepare(None, policy, 0).unwrap();
        assert!(warning.is_none());
        assert!(!converter.has_icc());
        let mut output = [0; 8];
        converter
            .convert_u8(&input, Some(&[13, 0]), &mut output)
            .unwrap();
        assert_eq!(&output[..4], &expected);
        assert_eq!(&output[4..], &[0, 0, 0, 0]);
    }
}

#[test]
fn layout_errors_leave_borrowed_destination_untouched() {
    let (mut converter, _) =
        CmykConverter::prepare(None, Approximation::Multiplicative, 0).unwrap();
    for (input, alpha) in [(&[1u8, 2, 3][..], None), (&[1, 2, 3, 4][..], Some(&[][..]))] {
        let mut output = [31; 4];
        assert!(converter.convert_u8(input, alpha, &mut output).is_err());
        assert_eq!(output, [31; 4]);
    }
    let mut output = [43u16; 4];
    assert!(converter.convert_u16(&[], None, &mut output).is_err());
    assert_eq!(output, [43; 4]);
    let mut output = [53u8; 4];
    assert!(
        converter
            .convert_u32(&[0; 4], 0, None, &mut output)
            .is_err()
    );
    assert_eq!(output, [53; 4]);
    converter.convert_u8(&[], Some(&[]), &mut []).unwrap();
}

#[test]
fn native_precision_and_packed_integer_fallback_preserve_tiff_rounding() {
    let (mut converter, _) =
        CmykConverter::prepare(None, Approximation::Multiplicative, 0).unwrap();
    for max in [1u32, 63, 255, 65535, u32::MAX] {
        let input = [max / 3, max / 5, max / 7, max / 11];
        let mut output = [0; 4];
        converter
            .convert_u32(&input, max, Some(&[max / 2]), &mut output)
            .unwrap();
        let reference = |c| {
            let m = u128::from(max);
            let value = ((m - u128::from(c)) * (m - u128::from(input[3])) + m / 2) / m;
            ((value * 255 + m / 2) / m) as u8
        };
        assert_eq!(
            output,
            [
                reference(input[0]),
                reference(input[1]),
                reference(input[2]),
                quantize(max / 2, max)
            ]
        );
    }
    let mut output = [0u16; 4];
    converter
        .convert_u16(&[1, 2, 3, 0], Some(&[12345]), &mut output)
        .unwrap();
    assert_eq!(output, [65534, 65533, 65532, 12345]);
}

#[test]
fn unusable_profile_falls_back_with_one_preparation_warning() {
    let (mut converter, warning) = CmykConverter::prepare(
        Some(b"not an ICC profile"),
        Approximation::Multiplicative,
        4 << 20,
    )
    .unwrap();
    assert!(warning.is_some());
    assert!(!converter.has_icc());
    let mut output = [0; 4];
    converter.convert_u8(&[0; 4], None, &mut output).unwrap();
    assert_eq!(output, [255; 4]);
}

#[cfg(feature = "color-management")]
#[test]
fn wrong_device_model_is_not_accepted_by_channel_count() {
    let (_, warning) = CmykConverter::prepare(
        Some(include_bytes!("srgb.icc")),
        Approximation::Multiplicative,
        4 << 20,
    )
    .unwrap();
    assert!(warning.unwrap().to_string().contains("CMYK source profile"));
}

#[cfg(feature = "color-management")]
#[test]
fn resource_limits_are_errors_not_profile_fallbacks() {
    let profile = profiles::cmyk_profile();
    assert!(CmykConverter::prepare(Some(&profile), Approximation::Multiplicative, 1).is_err());
    let limits = crate::limits::DecodeLimits {
        metadata_bytes: 100,
        ..Default::default()
    };
    crate::limits::scope(limits, || {
        assert!(
            CmykConverter::prepare(Some(&profile), Approximation::Multiplicative, 4 << 20).is_err()
        );
    });
}

#[cfg(feature = "color-management")]
#[test]
fn prepared_transform_reuses_chunks_preserves_u16_and_separate_alpha() {
    use icc_profile::{Profile, Transform, TransformOptions};
    let profile = profiles::cmyk_profile();
    if !profiles::supports_cmyk() {
        let (mut converter, warning) =
            CmykConverter::prepare(Some(&profile), Approximation::Multiplicative, 4 << 20).unwrap();
        assert!(warning.is_some());
        assert!(!converter.has_icc());
        let mut output = [0; 4];
        converter
            .convert_u16(&[1, 2, 3, 0], Some(&[12345]), &mut output)
            .unwrap();
        assert_eq!(output, [65534, 65533, 65532, 12345]);
        return;
    }
    let reference = Transform::new(
        &Profile::parse(&profile).unwrap(),
        &Profile::parse(include_bytes!("srgb.icc")).unwrap(),
        TransformOptions::default(),
    )
    .unwrap();
    let (mut converter, warning) =
        CmykConverter::prepare(Some(&profile), Approximation::Multiplicative, 4 << 20).unwrap();
    assert!(converter.has_icc());
    assert!(warning.is_none());
    let input: Vec<u16> = (0..1031)
        .flat_map(|i| [i * 37, i * 17, i * 13, i * 3])
        .collect();
    let alpha: Vec<u16> = (0..1031).map(|i| i * 61).collect();
    let mut expected = vec![0; 1031 * 3];
    reference.transform_u16(&input, &mut expected).unwrap();
    let mut output = vec![0; input.len()];
    for ((input, alpha), output) in input
        .chunks(333 * 4)
        .zip(alpha.chunks(333))
        .zip(output.chunks_mut(333 * 4))
    {
        converter.convert_u16(input, Some(alpha), output).unwrap();
    }
    for (i, actual) in output.chunks_exact(4).enumerate() {
        assert_eq!(&actual[..3], &expected[i * 3..i * 3 + 3]);
        assert_eq!(actual[3], alpha[i]);
    }
    assert!(output.iter().any(|v| v % 257 != 0));
    // A malformed caller buffer is an error, never a state transition to the
    // fallback route. The same prepared transform remains usable afterwards.
    assert!(converter.convert_u8(&[0; 3], None, &mut [0; 4]).is_err());
    assert!(converter.has_icc());
    let mut endpoints = [0; 8];
    converter
        .convert_u8(&[0, 0, 0, 0, 0, 0, 0, 255], None, &mut endpoints)
        .unwrap();
    assert!(endpoints[..3].iter().all(|v| *v >= 254));
    assert_eq!(&endpoints[4..], &[0, 0, 0, 255]);
}

#[cfg(all(test, feature = "color-management"))]
mod native_precision_tests {
    use super::{Approximation, CmykConverter};

    mod profiles {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/common/cmyk_profile.rs"
        ));
    }

    // Independent analytic reference for the synthetic profile's device-CMYK
    // products, followed by the standard piecewise sRGB transfer function.
    fn srgb(linear: f64) -> f64 {
        if linear <= 0.0031308 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        }
    }

    #[test]
    fn icc_u16_retains_sub_u8_differences_across_batch_boundary() {
        assert!(profiles::supports_cmyk());
        let (mut converter, warning) = CmykConverter::prepare(
            Some(&profiles::cmyk_profile()),
            Approximation::Multiplicative,
            4 << 20,
        )
        .unwrap();
        assert!(warning.is_none());
        assert!(converter.has_icc());
        let mut input: Vec<u16> = (0..514u32)
            .flat_map(|i| [60000 + (i / 2) * 7 + i % 2, i * 101, i * 79, 0].map(|v| v as u16))
            .collect();
        // Put a discriminating adjacent pair on opposite sides of the internal
        // 256-pixel boundary, not only within one scratch batch.
        input[255 * 4..256 * 4].copy_from_slice(&[65000, 12000, 24000, 0]);
        input[256 * 4..257 * 4].copy_from_slice(&[65001, 12000, 24000, 0]);
        let alpha: Vec<u16> = (0..514u32).map(|i| ((i * 127) % 65536) as u16).collect();
        let mut output = vec![0; input.len()];
        converter
            .convert_u16(&input, Some(&alpha), &mut output)
            .unwrap();
        let q8 = |v: u16| ((u32::from(v) + 128) / 257) as u8;
        for channel in 0..4 {
            assert_eq!(q8(input[255 * 4 + channel]), q8(input[256 * 4 + channel]));
        }
        assert_ne!(
            output[255 * 4],
            output[256 * 4],
            "u16 input must not be rounded to u8 before ICC"
        );
        for (i, (source, actual)) in input
            .chunks_exact(4)
            .zip(output.chunks_exact(4))
            .enumerate()
        {
            assert_eq!(actual[3], alpha[i], "alpha at pixel {i}");
            for channel in 0..3 {
                let linear = (1.0 - f64::from(source[channel]) / 65535.0)
                    * (1.0 - f64::from(source[3]) / 65535.0);
                let expected = (srgb(linear) * 65535.0).round() as i32;
                // ICC XYZ nodes are stored in u1.15 and the destination matrix
                // is fixed-point. Allow < one display-u8 step for that error,
                // independently of the strict low-bit discrimination above.
                assert!(
                    (i32::from(actual[channel]) - expected).abs() <= 160,
                    "pixel={i}, channel={channel}, actual={}, expected={expected}",
                    actual[channel]
                );
            }
        }
    }

    #[test]
    fn icc_u32_native_normalization_differs_from_early_u8_rounding() {
        assert!(profiles::supports_cmyk());
        let (mut converter, warning) = CmykConverter::prepare(
            Some(&profiles::cmyk_profile()),
            Approximation::Multiplicative,
            4 << 20,
        )
        .unwrap();
        assert!(warning.is_none());
        let input: Vec<u32> = (0..513u32)
            .flat_map(|i| [u32::MAX - (i + 1) * 32768, u32::MAX - (i + 1) * 16384, 0, 0])
            .collect();
        let alpha: Vec<u32> = (0..513u32).map(|i| i * (u32::MAX / 512)).collect();
        let mut actual = vec![0; input.len()];
        converter
            .convert_u32(&input, u32::MAX, Some(&alpha), &mut actual)
            .unwrap();
        let rounded: Vec<u8> = input
            .iter()
            .map(|v| super::quantize(*v, u32::MAX))
            .collect();
        let mut early_u8 = vec![0; input.len()];
        converter.convert_u8(&rounded, None, &mut early_u8).unwrap();
        let mut distinguishable = 0;
        for (i, (source, pixel)) in input
            .chunks_exact(4)
            .zip(actual.chunks_exact(4))
            .enumerate()
        {
            assert_eq!(pixel[3], super::quantize(alpha[i], u32::MAX));
            for channel in 0..3 {
                let linear = 1.0 - f64::from(source[channel]) / f64::from(u32::MAX);
                let expected = (srgb(linear) * 255.0).round() as i32;
                assert!(
                    (i32::from(pixel[channel]) - expected).abs() <= 1,
                    "pixel={i}, channel={channel}, actual={}, expected={expected}",
                    pixel[channel]
                );
                if pixel[channel].abs_diff(early_u8[i * 4 + channel]) >= 2 {
                    distinguishable += 1;
                }
            }
        }
        assert!(
            distinguishable > 32,
            "native normalization must measurably differ from early u8 rounding"
        );
    }
}
