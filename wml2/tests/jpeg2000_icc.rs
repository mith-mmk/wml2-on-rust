#![cfg(feature = "jpeg2000")]
use bin_rs::reader::{BinaryReader, BytesReader};
use wml2::{draw::*, limits::DecodeLimits, metadata::DataMap};
const RGB: &[u8] = include_bytes!("fixtures/review/avif-icc-linear.icc");
const GRAY: &[u8] = include_bytes!("fixtures/review/avif-icc-gray-linear.icc");
const RGB8: &[u8] = include_bytes!("fixtures/review/jp2_rgb_16x16_d1_noise_mct1.jp2");
const GRAY8: &[u8] = include_bytes!("fixtures/review/jp2_16x16_d1_noise.jp2");
fn boxed(kind: &[u8], data: &[u8]) -> Vec<u8> {
    let mut v = ((data.len() + 8) as u32).to_be_bytes().to_vec();
    v.extend_from_slice(kind);
    v.extend_from_slice(data);
    v
}
fn boxes(data: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut r = BytesReader::new(data);
    let mut v = Vec::new();
    while r.offset().unwrap() < (data.len() as u64) {
        let p = r.offset().unwrap() as usize;
        let n = r.read_u32_be().unwrap() as usize;
        assert!(n >= 8);
        let end = p + n;
        v.push((&data[p + 4..p + 8], &data[p + 8..end]));
        r.skip_ptr(n - 4).unwrap();
    }
    v
}
fn colour(input: &[u8], records: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for (kind, payload) in boxes(input) {
        if kind == b"jp2h" {
            let mut h = Vec::new();
            for (k, p) in boxes(payload) {
                if k == b"colr" {
                    for record in records {
                        h.extend(boxed(b"colr", record));
                    }
                } else {
                    h.extend(boxed(k, p));
                }
            }
            out.extend(boxed(kind, &h));
        } else {
            out.extend(boxed(kind, payload));
        }
    }
    out
}
fn icc(profile: &[u8]) -> Vec<u8> {
    let mut v = vec![2, 0, 0];
    v.extend_from_slice(profile);
    v
}
fn check_profile(meta: &std::collections::HashMap<String, DataMap>, profile: &[u8], active: bool) {
    assert!(matches!(meta.get("Source ICC Profile"),Some(DataMap::ICCProfile(p)) if p==profile));
    assert_eq!(meta.contains_key("ICC Profile"), active);
    if active {
        assert!(matches!(meta.get("ICC Profile"),Some(DataMap::ICCProfile(p)) if p==profile));
    }
}
#[test]
fn ordinary_rgb_gray_and_general_alpha_retain_selected_icc_without_changing_pixels() {
    for (input, profile) in [
        (RGB8, RGB),
        (GRAY8, GRAY),
        (
            include_bytes!("fixtures/review/jp2_general_RGBA.jp2").as_slice(),
            RGB,
        ),
        (
            include_bytes!("fixtures/review/jp2_general_LA.jp2").as_slice(),
            GRAY,
        ),
    ] {
        let encoded = colour(input, &[icc(profile)]);
        let plain = image_from(input).unwrap();
        let image = image_from(&encoded).unwrap();
        assert_eq!(image.buffer, plain.buffer);
        check_profile(image.metadata.as_ref().unwrap(), profile, profile == RGB);
    }
}
#[test]
fn first_understood_colr_wins_and_part1_ignores_unregistered_methods() {
    let enumeration = vec![1, 0, 0, 0, 0, 0, 16];
    let with_icc = colour(RGB8, &[icc(RGB), enumeration.clone()]);
    check_profile(
        image_from(&with_icc).unwrap().metadata.as_ref().unwrap(),
        RGB,
        true,
    );
    let with_enum = colour(RGB8, &[enumeration.clone(), icc(RGB)]);
    assert!(image_from(&with_enum).unwrap().metadata.is_none());
    let mut part2 = icc(RGB);
    part2[0] = 3;
    let with_ignored = colour(RGB8, &[part2, enumeration]);
    assert_eq!(
        image_from(&with_ignored).unwrap().buffer,
        image_from(RGB8).unwrap().buffer
    );
    assert!(image_from(&colour(RGB8, &[vec![3, 0, 0, 1, 2, 3]])).is_err());
}
#[test]
fn opaque_unsupported_transform_tags_do_not_gate_native_profile_retention() {
    let mut profile = RGB.to_vec();
    let table = icc_profile::Profile::parse(&profile).unwrap();
    assert_eq!(table.color_space(), icc_profile::ColorSpace::Rgb);
    drop(table);
    let mut r = BytesReader::new(&profile);
    r.skip_ptr(128).unwrap();
    let count = r.read_u32_be().unwrap();
    let mut curve = None;
    for _ in 0..count {
        let tag = r.read_u32_be().unwrap();
        let offset = r.read_u32_be().unwrap() as usize;
        let _size = r.read_u32_be().unwrap();
        if tag == u32::from_be_bytes(*b"rTRC") {
            curve = Some(offset);
        }
    }
    profile[curve.unwrap()..curve.unwrap() + 4].copy_from_slice(b"zzzz");
    assert!(icc_profile::Profile::parse(&profile).is_ok());
    assert!(icc_profile::Profile::new(&profile).is_err());
    let image = image_from(&colour(RGB8, &[icc(&profile)])).unwrap();
    assert_eq!(image.buffer, image_from(RGB8).unwrap().buffer);
    check_profile(image.metadata.as_ref().unwrap(), &profile, true);
}
#[derive(Default)]
struct Probe {
    events: Vec<String>,
    abort: bool,
    fail: bool,
    at: &'static str,
}
impl DrawCallback for Probe {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        self.events.push("init".into());
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
        self.events.push("draw".into());
        Ok(None)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.events.push("terminate".into());
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, key: &str, _: DataMap) -> Response {
        self.events.push(key.into());
        if key == self.at && self.fail {
            return Err(std::io::Error::other("JP2 caller metadata failure").into());
        }
        Ok((key == self.at && self.abort).then_some(CallbackResponse {
            response: ResponseCommand::Abort,
        }))
    }
}
#[test]
fn profile_errors_limits_and_abort_precede_display_callbacks() {
    let encoded = colour(RGB8, &[icc(RGB)]);
    for direct in [false, true] {
        for at in ["Source ICC Profile", "ICC Profile"] {
            for fail in [false, true] {
                let mut probe = Probe {
                    abort: !fail,
                    fail,
                    at,
                    ..Default::default()
                };
                let mut reader = BytesReader::new(&encoded);
                let mut options = DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut probe,
                };
                let result = if direct {
                    wml2::jpeg2000::decode(&mut reader, &mut options)
                } else {
                    image_decoder(&mut reader, &mut options)
                };
                let mut expected = vec!["Source ICC Profile"];
                if at == "ICC Profile" {
                    expected.push("ICC Profile");
                }
                if fail {
                    assert!(
                        result
                            .unwrap_err()
                            .to_string()
                            .contains("JP2 caller metadata failure")
                    );
                } else {
                    result.unwrap();
                    expected.push("terminate");
                }
                assert_eq!(probe.events, expected);
            }
        }
    }
    // The independent OpenJPEG input also carries a COM segment. Determine
    // its existing public metadata requirement, then verify cumulative ICC
    // copies/keys on top rather than assuming an empty metadata carrier.
    let baseline = (0..1024)
        .find(|&n| {
            image_from_with_limits(
                RGB8,
                DecodeLimits {
                    metadata_bytes: n,
                    ..Default::default()
                },
            )
            .is_ok()
        })
        .unwrap();
    let exact = baseline + RGB.len() * 2 + "Source ICC Profile".len() + "ICC Profile".len();
    for budget in [0, RGB.len() - 1, exact - 1, exact] {
        let mut probe = Probe::default();
        let result = image_decoder_with_limits(
            &mut BytesReader::new(&encoded),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut probe,
            },
            DecodeLimits {
                metadata_bytes: budget,
                ..Default::default()
            },
        );
        assert_eq!(result.is_ok(), budget == exact);
        if budget != exact {
            assert!(probe.events.is_empty());
        }
    }
    let mut malformed = RGB.to_vec();
    malformed[36..40].fill(0);
    let mut mismatch = RGB.to_vec();
    mismatch[16..20].copy_from_slice(b"CMYK");
    for profile in [&malformed[..], &mismatch[..], &RGB[..20], GRAY] {
        let data = colour(RGB8, &[icc(profile)]);
        let mut probe = Probe::default();
        assert!(
            image_decoder(
                &mut BytesReader::new(&data),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut probe
                }
            )
            .is_err()
        );
        assert!(probe.events.is_empty());
    }
}
#[cfg(feature = "image-buffer-ex")]
#[test]
fn native_owned_callbacks_storage_conversion_and_reduced_profiles_are_consistent() {
    for (input, profile, format) in [
        (
            include_bytes!("fixtures/review/jp2_ex_gray12_ramp.jp2").as_slice(),
            GRAY,
            PixelFormatEx::Gray12,
        ),
        (
            include_bytes!("fixtures/review/jp2_ex_gray16_ramp.jp2").as_slice(),
            GRAY,
            PixelFormatEx::Gray16,
        ),
        (
            include_bytes!("fixtures/review/jp2_ex_rgb12_edges_mct1.jp2").as_slice(),
            RGB,
            PixelFormatEx::Rgba12,
        ),
        (
            include_bytes!("fixtures/review/jp2_ex_rgba16_edges_mct1.jp2").as_slice(),
            RGB,
            PixelFormatEx::Rgba16,
        ),
    ] {
        let data = colour(input, &[icc(profile)]);
        let plain = image_from_ex(input, format, PrecisionConversion::Exact).unwrap();
        let owned = image_from_ex(&data, format, PrecisionConversion::Exact).unwrap();
        assert_eq!(owned.bytes(), plain.bytes());
        check_profile(owned.metadata.as_ref().unwrap(), profile, true);
        let mut callback = ImageBufferEx::with_storage(format, PrecisionConversion::Exact);
        image_decoder_ex(
            &mut BytesReader::new(&data),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut callback,
            },
        )
        .unwrap();
        assert_eq!(callback.bytes(), owned.bytes());
        assert_eq!(callback.metadata, owned.metadata);
        let converted =
            image_from_ex(&data, PixelFormatEx::Rgba8, PrecisionConversion::Round).unwrap();
        check_profile(
            converted.metadata.as_ref().unwrap(),
            profile,
            profile == RGB,
        );
    }
    for (input, profile) in [
        (
            include_bytes!("fixtures/review/jp2_reduced_odd_8b_1c_53.jp2").as_slice(),
            GRAY,
        ),
        (
            include_bytes!("fixtures/review/jp2_reduced_odd_8b_3c_53.jp2").as_slice(),
            RGB,
        ),
    ] {
        let data = colour(input, &[icc(profile)]);
        let mut plain = ImageBuffer::new();
        let mut image = ImageBuffer::new();
        for (encoded, target) in [(input, &mut plain), (&data[..], &mut image)] {
            wml2::jpeg2000::decode_with_reduction(
                &mut BytesReader::new(encoded),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: target,
                },
                1,
                DecodeLimits::default(),
            )
            .unwrap();
        }
        assert_eq!(image.buffer, plain.buffer);
        check_profile(image.metadata.as_ref().unwrap(), profile, profile == RGB);
    }
    let source = include_bytes!("fixtures/review/jp2_reduced_odd_12b_1c_53.jp2");
    let data = colour(source, &[icc(GRAY)]);
    let plain = wml2::jpeg2000::image_from_ex_with_reduction(
        source,
        PixelFormatEx::Gray12,
        PrecisionConversion::Exact,
        1,
        DecodeLimits::default(),
    )
    .unwrap();
    let image = wml2::jpeg2000::image_from_ex_with_reduction(
        &data,
        PixelFormatEx::Gray12,
        PrecisionConversion::Exact,
        1,
        DecodeLimits::default(),
    )
    .unwrap();
    assert_eq!(plain.bytes(), image.bytes());
    check_profile(image.metadata.as_ref().unwrap(), GRAY, true);
}
