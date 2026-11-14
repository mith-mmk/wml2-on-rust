#![cfg(feature = "jpeg")]
use wml2::draw::{image_from, image_load_with_warnings};
use wml2::metadata::DataMap;
const BASE: &[u8] = include_bytes!("fixtures/review/jpeg-cmyk-blocks.jpg");
const PROGRESSIVE: &[u8] = include_bytes!("fixtures/review/jpeg-cmyk-blocks-progressive.jpg");
fn append_app(jpeg: &[u8], marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = jpeg[..2].to_vec();
    out.extend([255, marker]);
    out.extend(((payload.len() + 2) as u16).to_be_bytes());
    out.extend(payload);
    out.extend(&jpeg[2..]);
    out
}
#[test]
fn nonuniform_cmyk_baseline_and_progressive_declared_ids_match() {
    for jpeg in [BASE, PROGRESSIVE] {
        let image = image_from(jpeg).unwrap();
        assert_eq!((image.width, image.height), (16, 16));
        let data = image.buffer.unwrap();
        for y in 0..16 {
            for x in 0..16 {
                let block = (y / 8 * 2 + x / 8) as u8;
                assert_eq!(
                    &data[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4],
                    &[165 + block * 12, 185 - block * 8, 195 - block * 10, 255]
                );
            }
        }
        assert!(
            matches!(image.metadata.unwrap().get("Color Space"), Some(DataMap::Ascii(s)) if s == "RGB")
        );
    }
}
#[test]
fn short_adobe_payloads_are_ancillary_not_panics() {
    let adobe = b"Adobe\0\x64\0\0\0\0\0";
    for n in 0..12 {
        let image = image_from(&append_app(BASE, 0xee, &adobe[..n])).unwrap();
        assert_eq!(image.buffer, image_from(BASE).unwrap().buffer);
    }
}
#[test]
fn malformed_icc_association_warns_and_preserves_legacy_pixels() {
    let malformed = append_app(BASE, 0xe2, b"ICC_PROFILE\0\x02\x01bad");
    let (image, warnings) = image_load_with_warnings(&malformed).unwrap();
    assert_eq!(image.buffer, image_from(BASE).unwrap().buffer);
    assert!(format!("{:?}", warnings).contains("Invalid ICC APP2"));
}
#[test]
fn existing_real_cmyk_progressive_fixture_decodes() {
    let image = image_from(include_bytes!(
        "fixtures/review/jpeg-icc-cmyk-progressive.jpg"
    ))
    .unwrap();
    assert!(
        image
            .buffer
            .as_ref()
            .unwrap()
            .chunks_exact(4)
            .all(|p| p == [159, 127, 95, 255])
    );
}

#[cfg(feature = "color-management")]
#[path = "common/cmyk_profile.rs"]
mod profiles;

#[cfg(feature = "color-management")]
fn tagged(jpeg: &[u8]) -> Vec<u8> {
    let mut payload = b"ICC_PROFILE\0\x01\x01".to_vec();
    payload.extend(profiles::cmyk_profile());
    append_app(jpeg, 0xe2, &payload)
}

#[cfg(feature = "color-management")]
#[test]
fn embedded_icc_transforms_canonical_adobe_and_unmarked_samples() {
    use icc_profile::{Profile, Transform, TransformOptions};
    let profile = profiles::cmyk_profile();
    let source = Profile::parse(&profile).unwrap();
    let destination = Profile::parse(include_bytes!("../src/srgb.icc")).unwrap();
    if !profiles::supports_cmyk() {
        for jpeg in [BASE, PROGRESSIVE] {
            for adobe in [true, false] {
                let unmarked = [&jpeg[..2], &jpeg[18..]].concat();
                let plain = if adobe { jpeg } else { &unmarked };
                let (image, warnings) = image_load_with_warnings(&tagged(plain)).unwrap();
                assert_eq!(
                    warnings
                        .unwrap()
                        .iter()
                        .filter(|w| w.to_string().starts_with("CMYK color management:"))
                        .count(),
                    1
                );
                assert_eq!(image.buffer, image_from(plain).unwrap().buffer);
                assert!(!image.metadata.unwrap().contains_key("ICC Profile"));
            }
        }
        return;
    }
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    for jpeg in [BASE, PROGRESSIVE] {
        for adobe in [true, false] {
            // Generated APP14 is the first segment, 16 bytes including marker.
            let unmarked = [&jpeg[..2], &jpeg[18..]].concat();
            let data = tagged(if adobe { jpeg } else { &unmarked });
            let image = image_from(&data).unwrap();
            let pixels = image.buffer.as_ref().unwrap();
            for y in 0..16 {
                for x in 0..16 {
                    let b = (y / 8 * 2 + x / 8) as u8;
                    let mut ink = [180 + b * 12, 200 - b * 8, 210 - b * 10, 240];
                    if adobe {
                        ink.iter_mut().for_each(|v| *v = 255 - *v);
                    }
                    let mut rgb = [0; 3];
                    transform.transform_u8(&ink, &mut rgb).unwrap();
                    assert_eq!(&pixels[(y * 16 + x) * 4..(y * 16 + x) * 4 + 3], &rgb);
                    assert_eq!(pixels[(y * 16 + x) * 4 + 3], 255);
                }
            }
            let metadata = image.metadata.unwrap();
            assert!(
                matches!(metadata.get("Source ICC Profile"),Some(DataMap::ICCProfile(p)) if p == &profile)
            );
            assert!(!metadata.contains_key("ICC Profile"));
        }
    }
}

#[cfg(feature = "color-management")]
#[test]
fn ycck_icc_normalizes_before_transform() {
    use icc_profile::{Profile, Transform, TransformOptions};
    // Existing independently encoded constant YCCK fixture has DCT samples
    // Y=90, Cb=149, Cr=109, K=223 (quantized from its original CMYK patch).
    let jpeg = include_bytes!("fixtures/review/jpeg-icc-ycck.jpg");
    if !profiles::supports_cmyk() {
        let (image, warnings) = image_load_with_warnings(&tagged(jpeg)).unwrap();
        assert_eq!(
            warnings
                .unwrap()
                .iter()
                .filter(|w| w.to_string().starts_with("CMYK color management:"))
                .count(),
            1
        );
        assert_eq!(image.buffer, image_from(jpeg).unwrap().buffer);
        return;
    }
    let image = image_from(&tagged(jpeg)).unwrap();
    let bytes = profiles::cmyk_profile();
    let source = Profile::parse(&bytes).unwrap();
    let dest = Profile::parse(include_bytes!("../src/srgb.icc")).unwrap();
    let transform = Transform::new(&source, &dest, TransformOptions::default()).unwrap();
    let mut rgb = [0; 3];
    transform
        .transform_u8(&[64, 96, 127, 32], &mut rgb)
        .unwrap();
    // Integer IDCT/chroma rounding permits one ink code from the encoded source.
    for p in image.buffer.unwrap().chunks_exact(4) {
        assert!(
            p[..3].iter().zip(rgb).all(|(a, b)| a.abs_diff(b) <= 2),
            "{p:?} vs {rgb:?}"
        );
        assert_eq!(p[3], 255);
    }
}

#[cfg(all(feature = "color-management", feature = "png"))]
#[test]
#[ignore = "manual visual artifact and timing generation"]
fn jpeg_cmyk_visual_benchmark() {
    use std::time::Instant;
    std::fs::create_dir_all("../.test-jpeg-validation").unwrap();
    for (name, jpeg) in [
        ("baseline", BASE.to_vec()),
        ("progressive", PROGRESSIVE.to_vec()),
        ("icc-baseline", tagged(BASE)),
        ("icc-progressive", tagged(PROGRESSIVE)),
        (
            "ycck",
            include_bytes!("fixtures/review/jpeg-icc-ycck.jpg").to_vec(),
        ),
        (
            "icc-ycck",
            tagged(include_bytes!("fixtures/review/jpeg-icc-ycck.jpg")),
        ),
    ] {
        let start = Instant::now();
        for _ in 0..100 {
            std::hint::black_box(image_from(&jpeg).unwrap());
        }
        eprintln!("{name}: {} us/decode", start.elapsed().as_micros() / 100);
        let mut image = image_from(&jpeg).unwrap();
        let png = wml2::draw::image_to(&mut image, wml2::util::ImageFormat::Png, None).unwrap();
        std::fs::write(format!("../.test-jpeg-validation/{name}.png"), png).unwrap();
    }
}

#[cfg(feature = "color-management")]
fn analytic_srgb_from_ink(ink: [f64; 4]) -> [u8; 3] {
    std::array::from_fn(|c| {
        let linear = (1.0 - ink[c]) * (1.0 - ink[3]);
        let encoded = if linear <= 0.0031308 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (encoded * 255.0).round() as u8
    })
}

#[cfg(feature = "color-management")]
#[test]
fn cmyk_and_ycck_match_independent_analytic_srgb_not_backend_self_comparison() {
    for jpeg in [BASE, PROGRESSIVE] {
        for adobe in [true, false] {
            let unmarked = [&jpeg[..2], &jpeg[18..]].concat();
            let (image, warnings) =
                image_load_with_warnings(&tagged(if adobe { jpeg } else { &unmarked })).unwrap();
            assert!(!format!("{warnings:?}").contains("CMYK color management:"));
            for (index, pixel) in image.buffer.unwrap().chunks_exact(4).enumerate() {
                let b = (index / 16 / 8 * 2 + index % 16 / 8) as u8;
                let samples = [180 + b * 12, 200 - b * 8, 210 - b * 10, 240];
                let ink = samples.map(|v| {
                    if adobe {
                        f64::from(255 - v) / 255.0
                    } else {
                        f64::from(v) / 255.0
                    }
                });
                let expected = analytic_srgb_from_ink(ink);
                for c in 0..3 {
                    assert!(
                        pixel[c].abs_diff(expected[c]) <= 3,
                        "adobe={adobe}, block={b}, {pixel:?} vs {expected:?}"
                    );
                }
                assert_eq!(pixel[3], 255);
            }
        }
    }
    let ycck = include_bytes!("fixtures/review/jpeg-icc-ycck.jpg");
    let image = image_from(&tagged(ycck)).unwrap();
    let expected =
        analytic_srgb_from_ink([64.0 / 255.0, 96.0 / 255.0, 127.0 / 255.0, 32.0 / 255.0]);
    for pixel in image.buffer.unwrap().chunks_exact(4) {
        for c in 0..3 {
            assert!(
                pixel[c].abs_diff(expected[c]) <= 3,
                "YCCK {pixel:?} vs {expected:?}"
            );
        }
        assert_eq!(pixel[3], 255);
    }
}

#[cfg(feature = "color-management")]
#[test]
fn unusable_embedded_cmyk_profile_warns_once_and_preserves_all_legacy_pixels() {
    let valid = profiles::cmyk_profile();
    for jpeg in [
        BASE,
        PROGRESSIVE,
        include_bytes!("fixtures/review/jpeg-icc-ycck.jpg").as_slice(),
    ] {
        let plain = image_from(jpeg).unwrap();
        for profile in [
            b"broken".as_slice(),
            &valid[..100],
            include_bytes!("../src/srgb.icc").as_slice(),
        ] {
            let mut payload = b"ICC_PROFILE\0\x01\x01".to_vec();
            payload.extend(profile);
            let (image, warnings) =
                image_load_with_warnings(&append_app(jpeg, 0xe2, &payload)).unwrap();
            assert_eq!(image.buffer, plain.buffer);
            assert_eq!((image.width, image.height), (plain.width, plain.height));
            assert_eq!(
                warnings
                    .unwrap()
                    .iter()
                    .filter(|w| w.to_string().starts_with("CMYK color management:"))
                    .count(),
                1
            );
            assert!(!image.metadata.unwrap().contains_key("ICC Profile"));
        }
    }
}

#[cfg(feature = "color-management")]
#[test]
fn cmyk_profile_budget_failure_is_an_error_before_any_pixel_callback() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::*;
    use wml2::limits::DecodeLimits;
    #[derive(Default)]
    struct Probe {
        draws: usize,
    }
    impl DrawCallback for Probe {
        fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
            Ok(None)
        }
        fn draw(
            &mut self,
            _: usize,
            _: usize,
            _: usize,
            _: usize,
            _: &[u8],
            _: Option<DrawOptions>,
        ) -> Response {
            self.draws += 1;
            Ok(None)
        }
        fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
            Ok(None)
        }
        fn next(&mut self, _: Option<NextOptions>) -> Response {
            Ok(None)
        }
        fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
            Ok(None)
        }
        fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
            Ok(None)
        }
    }
    for jpeg in [BASE, PROGRESSIVE] {
        let limits = DecodeLimits {
            expanded_bytes: 8192,
            ..DecodeLimits::unlimited()
        };
        assert!(
            image_from_with_limits(jpeg, limits).is_ok(),
            "plain fixture fits the same resource budget"
        );
        let encoded = tagged(jpeg);
        let mut reader = BytesReader::new(&encoded);
        let mut probe = Probe::default();
        let error = image_decoder_with_limits(
            &mut reader,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut probe,
            },
            limits,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("CMYK converter workspace"),
            "{error}"
        );
        assert_eq!(
            probe.draws, 0,
            "resource failure must not become pixel fallback"
        );
    }
}

#[test]
fn cmyk_outcomes_distinguish_missing_and_malformed_profiles() {
    use wml2::color_outcome::*;
    for jpeg in [BASE, PROGRESSIVE] {
        let image = image_from(jpeg).unwrap();
        let outcome = ColorOutcome::from_metadata(
            image
                .metadata
                .as_ref()
                .and_then(|m| m.get(&metadata_key(0))),
        );
        assert_eq!(
            outcome,
            ColorOutcome {
                conversion: Conversion::Approximation(
                    ApproximationReason::MissingProfile,
                    ApproximationMethod::AdditiveCmyk
                ),
                output: OutputColorSpace::DeviceApproximation
            }
        );
        let malformed = append_app(jpeg, 0xe2, b"ICC_PROFILE\0\x02\x01bad");
        let image = image_from(&malformed).unwrap();
        let outcome = ColorOutcome::from_metadata(
            image
                .metadata
                .as_ref()
                .and_then(|m| m.get(&metadata_key(0))),
        );
        assert_eq!(
            outcome.conversion,
            Conversion::Approximation(
                ApproximationReason::InvalidProfile,
                ApproximationMethod::AdditiveCmyk
            )
        );
    }
}

#[cfg(feature = "color-management")]
#[test]
fn cmyk_outcomes_report_applied_only_for_completed_managed_pixels() {
    use wml2::color_outcome::*;
    for jpeg in [BASE, PROGRESSIVE] {
        let image = image_from(&tagged(jpeg)).unwrap();
        let outcome = ColorOutcome::from_metadata(
            image
                .metadata
                .as_ref()
                .and_then(|m| m.get(&metadata_key(0))),
        );
        assert_eq!(outcome.conversion, Conversion::AppliedToSrgb);
        assert_eq!(outcome.output, OutputColorSpace::Srgb);
    }
}

#[test]
fn incomplete_cmyk_jpeg_never_reports_completed_color_processing() {
    use wml2::color_outcome::*;
    for jpeg in [BASE, PROGRESSIVE] {
        for length in 2..jpeg.len() - 2 {
            if let Ok(image) = image_from(&jpeg[..length]) {
                assert_eq!(
                    ColorOutcome::from_metadata(
                        image
                            .metadata
                            .as_ref()
                            .and_then(|m| m.get(&metadata_key(0)))
                    ),
                    ColorOutcome::default(),
                    "length={length}"
                );
            }
        }
    }
}
