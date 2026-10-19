#![cfg(feature = "avif")]
use wml2::draw::*;
use wml2::metadata::DataMap;

fn limits() -> avif_codec::NativeDecodeLimits {
    avif_codec::NativeDecodeLimits::new(
        64 << 20,
        16384,
        16384,
        16 << 20,
        64 << 20,
        8 << 20,
        8 << 20,
        128,
        512,
        128,
        16,
        128,
    )
}

const CASES: &[(&str, &[u8], &[u8], bool)] = &[
    (
        "rgb-linear",
        include_bytes!("fixtures/review/avif-icc-gradient-linear-premult0.avif"),
        include_bytes!("fixtures/review/avif-icc-gradient-linear-premult0.raw.rgba"),
        false,
    ),
    (
        "rgb-linear-premult",
        include_bytes!("fixtures/review/avif-icc-gradient-linear-premult1.avif"),
        include_bytes!("fixtures/review/avif-icc-gradient-linear-premult1.raw.rgba"),
        true,
    ),
    (
        "coded-linear8",
        include_bytes!("fixtures/review/avif-icc-ff-linear-coded-icc.avif"),
        include_bytes!("fixtures/review/avif-icc-ff-linear-coded-icc.raw.rgba"),
        false,
    ),
    (
        "coded-linear10",
        include_bytes!("fixtures/review/avif-icc-ff-linear-coded10-icc.avif"),
        include_bytes!("fixtures/review/avif-icc-ff-linear-coded10-icc.raw.rgba"),
        false,
    ),
    (
        "coded-linear12",
        include_bytes!("fixtures/review/avif-icc-ff-linear-coded12-icc.avif"),
        include_bytes!("fixtures/review/avif-icc-ff-linear-coded12-icc.raw.rgba"),
        false,
    ),
    (
        "gray-linear",
        include_bytes!("fixtures/review/avif-icc-gray-linear.avif"),
        include_bytes!("fixtures/review/avif-icc-gray-linear.raw.rgba"),
        false,
    ),
    (
        "gray-limited8",
        include_bytes!("fixtures/review/avif-icc-gray-linear-limited.avif"),
        include_bytes!("fixtures/review/avif-icc-gray-linear-limited.raw.rgba"),
        false,
    ),
    (
        "gray-limited10",
        include_bytes!("fixtures/review/avif-icc-gray-linear-limited10.avif"),
        include_bytes!("fixtures/review/avif-icc-gray-linear-limited10.raw.rgba"),
        false,
    ),
    (
        "gray-limited12",
        include_bytes!("fixtures/review/avif-icc-gray-linear-limited12.avif"),
        include_bytes!("fixtures/review/avif-icc-gray-linear-limited12.raw.rgba"),
        false,
    ),
];

#[cfg(feature = "image-buffer-ex")]
#[test]
fn ex_retains_encoded_rgb_without_icc_or_display_transfer_and_unassociates_alpha_first() {
    for &(name, bytes, expected, premultiplied) in CASES {
        let image =
            image_from_ex(bytes, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
        let actual: Vec<u8> = image
            .bytes()
            .unwrap()
            .chunks_exact(2)
            .map(|s| {
                let value = u16::from_le_bytes([s[0], s[1]]);
                ((u32::from(value) * 255 + 32767) / 65535) as u8
            })
            .collect();
        assert_eq!(actual.len(), expected.len());
        let mut max = 0;
        for (a, b) in actual.chunks_exact(4).zip(expected.chunks_exact(4)) {
            assert_eq!(a[3], b[3], "{name}: alpha");
            for c in 0..3 {
                max = max.max(a[c].abs_diff(b[c]));
            }
        }
        // The independent reference also reconstructs16-bit RGB and quantizes
        // only at this comparison boundary, avoiding8-bit pre-CMS rounding.
        let tolerance = 1;
        assert!(
            max <= tolerance,
            "{name}: raw RGB differs from libavif by {max}"
        );
        let expected_profile = if name.starts_with("gray") {
            include_bytes!("fixtures/review/avif-icc-gray-linear.icc").as_slice()
        } else {
            include_bytes!("fixtures/review/avif-icc-linear.icc").as_slice()
        };
        let metadata = image.metadata.as_ref().unwrap();
        assert_eq!(
            metadata["Source ICC Profile"],
            DataMap::ICCProfile(expected_profile.to_vec())
        );
        assert!(!metadata.contains_key("ICC Profile"));
        assert_eq!(
            metadata["Source AVIF Alpha Premultiplied"],
            DataMap::UInt(u64::from(premultiplied))
        );
    }
}

const MANAGED: &[&[u8]] = &[
    include_bytes!("fixtures/review/avif-icc-gradient-linear-premult0.srgb.rgba"),
    include_bytes!("fixtures/review/avif-icc-gradient-linear-premult1.srgb.rgba"),
    include_bytes!("fixtures/review/avif-icc-ff-linear-coded-icc.srgb.rgba"),
    include_bytes!("fixtures/review/avif-icc-ff-linear-coded10-icc.srgb.rgba"),
    include_bytes!("fixtures/review/avif-icc-ff-linear-coded12-icc.srgb.rgba"),
    include_bytes!("fixtures/review/avif-icc-gray-linear.srgb.rgba"),
    include_bytes!("fixtures/review/avif-icc-gray-linear-limited.srgb.rgba"),
    include_bytes!("fixtures/review/avif-icc-gray-linear-limited10.srgb.rgba"),
    include_bytes!("fixtures/review/avif-icc-gray-linear-limited12.srgb.rgba"),
];
fn source_profile(name: &str) -> &'static [u8] {
    if name.starts_with("gray") {
        include_bytes!("fixtures/review/avif-icc-gray-linear.icc")
    } else {
        include_bytes!("fixtures/review/avif-icc-linear.icc")
    }
}

#[test]
fn ordinary_icc_overrides_nclx_and_runs_once_after_reconstruction_and_alpha() {
    for (&(name, bytes, _, _), &expected) in CASES.iter().zip(MANAGED) {
        let image = image_from(bytes).unwrap();
        let actual = image.buffer.as_ref().unwrap();
        assert_eq!(actual.len(), expected.len());
        let mut max = 0;
        for (a, b) in actual.chunks_exact(4).zip(expected.chunks_exact(4)) {
            assert_eq!(a[3], b[3], "{name}: alpha");
            for c in 0..3 {
                max = max.max(a[c].abs_diff(b[c]));
            }
        }
        assert!(
            max <= 1,
            "{name}: managed RGB differs from independent libavif/LittleCMS by {max}"
        );
        let metadata = image.metadata.as_ref().unwrap();
        assert_eq!(
            metadata["Source ICC Profile"],
            DataMap::ICCProfile(source_profile(name).to_vec())
        );
        assert_eq!(
            metadata["ICC Profile"],
            DataMap::ICCProfile(include_bytes!("../src/srgb.icc").to_vec())
        );
        if name.starts_with("coded-linear") {
            assert!(
                matches!(metadata.get("Source AV1 CICP"),Some(DataMap::UIntAllay(v)) if v[1]==8)
            );
        }
    }
}

#[test]
fn colr_property_order_does_not_change_effective_icc_or_pixels() {
    let original = CASES[0].1;
    let tags: Vec<_> = original
        .windows(4)
        .enumerate()
        .filter(|(i, v)| *i >= 4 && *v == b"colr")
        .map(|(i, _)| i - 4)
        .collect();
    assert_eq!(tags.len(), 2);
    let (a, b) = (tags[0], tags[1]);
    let na = bin_rs::io::read_u32_be(original, a) as usize;
    let nb = bin_rs::io::read_u32_be(original, b) as usize;
    let mut swapped = original.to_vec();
    let replacement = [
        &original[b..b + nb],
        &original[a + na..b],
        &original[a..a + na],
    ]
    .concat();
    swapped.splice(a..b + nb, replacement);
    let first = avif_codec::parse_native_info(original, &limits()).unwrap();
    let second = avif_codec::parse_native_info(&swapped, &limits()).unwrap();
    assert_eq!(first.color_information(), second.color_information());
    assert!(first.color_information().nclx.is_some());
    assert_eq!(
        image_from(original).unwrap().buffer,
        image_from(&swapped).unwrap().buffer
    );
}

#[cfg(feature = "image-buffer-ex")]
#[test]
fn malformed_profile_is_retained_by_ex_and_rejected_by_the_display_cms() {
    let mut bytes = CASES[0].1.to_vec();
    let profile = source_profile(CASES[0].0);
    let at = bytes
        .windows(profile.len())
        .position(|v| v == profile)
        .unwrap();
    bytes[at + 36..at + 40].copy_from_slice(b"xxxx");
    assert!(image_from(&bytes).is_err());
    let native = image_from_ex(&bytes, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
    let Some(DataMap::ICCProfile(retained)) =
        native.metadata.as_ref().unwrap().get("Source ICC Profile")
    else {
        panic!("source ICC missing")
    };
    assert_eq!(retained, &bytes[at..at + profile.len()]);
    assert!(
        !native
            .metadata
            .as_ref()
            .unwrap()
            .contains_key("ICC Profile")
    );
}

#[test]
fn metadata_limits_reject_before_display_initialization() {
    use bin_rs::reader::BytesReader;
    use wml2::limits::DecodeLimits;
    let bytes = CASES[0].1;
    assert!(
        image_from_with_limits(
            bytes,
            DecodeLimits {
                metadata_bytes: 1 << 20,
                ..Default::default()
            }
        )
        .is_ok()
    );
    for budget in [0, 1, source_profile(CASES[0].0).len() - 1] {
        let mut image = ImageBuffer::new();
        assert!(
            image_decoder_with_limits(
                &mut BytesReader::new(bytes),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut image
                },
                DecodeLimits {
                    metadata_bytes: budget,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            image.buffer.is_none(),
            "metadata budget failure must precede pixel initialization"
        );
    }
}

#[test]
fn icc_display_callbacks_stop_on_abort_and_preserve_callback_errors() {
    use bin_rs::reader::BytesReader;
    #[derive(Default)]
    struct Probe {
        target: &'static str,
        fail: bool,
        stopped: bool,
        hits: usize,
        terms: usize,
        last: String,
    }
    impl Probe {
        fn event(&mut self, name: &str) -> Response {
            assert!(!self.stopped, "callback after Abort");
            self.last = name.into();
            if name == self.target {
                self.hits += 1;
                if self.fail {
                    return Err(std::io::Error::other("AVIF display callback failure").into());
                }
                self.stopped = true;
                return Ok(Some(CallbackResponse::abort()));
            }
            Ok(None)
        }
    }
    impl DrawCallback for Probe {
        fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
            self.event("init")
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
            self.event("draw")
        }
        fn next(&mut self, _: Option<NextOptions>) -> Response {
            self.event("next")
        }
        fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
            self.event("verbose")
        }
        fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
            self.terms += 1;
            Ok(None)
        }
        fn set_metadata(&mut self, key: &str, _: DataMap) -> Response {
            self.event(key)
        }
    }
    for target in [
        "verbose",
        "init",
        "draw",
        "Source ICC Profile",
        "ICC Profile",
        "Source AVIF nclx",
        "Source AV1 CICP",
    ] {
        for direct in [false, true] {
            for fail in [false, true] {
                let mut probe = Probe {
                    target,
                    fail,
                    ..Default::default()
                };
                let mut reader = BytesReader::new(CASES[1].1);
                let mut options = DecodeOptions {
                    debug_flag: 1,
                    drawer: &mut probe,
                };
                let result = if direct {
                    wml2::avif::decoder::decode(&mut reader, &mut options)
                } else {
                    image_decoder(&mut reader, &mut options)
                };
                if fail {
                    assert_eq!(
                        result.unwrap_err().to_string(),
                        "AVIF display callback failure"
                    );
                } else {
                    assert!(result.is_ok());
                    assert_eq!(probe.terms, 1);
                }
                assert_eq!(probe.hits, 1);
                assert_eq!(probe.last, target);
            }
        }
    }
}

#[cfg(feature = "image-buffer-ex")]
#[test]
fn prem_reference_must_name_the_decoded_alpha_item() {
    let mut bytes = CASES[1].1.to_vec();
    let prem = bytes.windows(4).position(|v| v == b"prem").unwrap();
    bytes[prem + 8..prem + 10].copy_from_slice(&3u16.to_be_bytes());
    assert!(image_from_ex(&bytes, PixelFormatEx::Rgba16, PrecisionConversion::Exact).is_err());
    assert!(image_from(&bytes).is_err());
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn actual_prem_reference_corrects_native_association_without_changing_planes() {
    use wml2::highres::{AlphaAssociation, PixelBuffer};
    for &(_, bytes, _, premultiplied) in &CASES[..2] {
        let limits = limits();
        let baseline = avif_codec::decode_frame_bytes_strict_with_limits(bytes, &limits).unwrap();
        let (baseline, _) = baseline.into_frame_and_rich();
        // Both were projected as straight by the published codec; the second
        // independent fixture has an actual meta/iref/prem relation.
        assert!(!baseline.alpha_premultiplied);
        let frame = wml2::highres::avif::decode_native(bytes, &limits).unwrap();
        assert_eq!(
            frame.descriptor().alpha(),
            if premultiplied {
                AlphaAssociation::Premultiplied
            } else {
                AlphaAssociation::Straight
            }
        );
        let PixelBuffer::U16(planes) = frame.pixels() else {
            panic!("native integer planes")
        };
        for (plane, original) in planes.as_slice().iter().zip(baseline.buffers.planes.iter()) {
            assert_eq!(plane.samples(), original.samples);
        }
    }
}
