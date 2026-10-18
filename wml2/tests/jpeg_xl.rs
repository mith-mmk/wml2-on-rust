use wml2::util::{ImageFormat, format_check};

#[test]
fn feature_controls_detection_and_extension_reporting() {
    let raw = b"\xff\x0a";
    let boxed = b"\0\0\0\x0cJXL \r\n\x87\n";
    #[cfg(feature = "jpeg-xl")]
    {
        for data in [raw.as_slice(), boxed.as_slice()] {
            assert!(matches!(format_check(data), ImageFormat::JpegXl));
            assert!(wml2::get_can_decode(data).unwrap());
        }
        assert!(wml2::get_decoder_extentions().contains(&"jxl".into()));
        assert!(!wml2::get_encode_extentions().contains(&"jxl".into()));
    }
    #[cfg(not(feature = "jpeg-xl"))]
    {
        for data in [raw.as_slice(), boxed.as_slice()] {
            assert!(matches!(format_check(data), ImageFormat::Unknown));
            assert!(!wml2::get_can_decode(data).unwrap());
            assert!(wml2::draw::image_from(data).is_err());
        }
        assert!(!wml2::get_decoder_extentions().contains(&"jxl".into()));
    }
    for n in 0..boxed.len() {
        let _ = format_check(&boxed[..n]);
    }
}

#[cfg(feature = "jpeg-xl")]
mod enabled {
    use bin_rs::reader::BytesReader;
    use wml2::draw::*;
    use wml2::limits::DecodeLimits;
    use wml2::metadata::DataMap;
    const RGBA8: &[u8] = include_bytes!("fixtures/review/jxl-rgba8.jxl");
    const RGBA16: &[u8] = include_bytes!("fixtures/review/jxl-rgba16.jxl");
    type Error = Box<dyn std::error::Error>;
    fn samples(bytes: &[u8], bits: u8) -> Vec<u16> {
        if bits == 8 {
            bytes.iter().map(|&v| v.into()).collect()
        } else {
            bytes
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect()
        }
    }
    fn rgba(source: &[u16], channels: usize, bits: u8) -> Vec<u16> {
        let max = ((1u32 << bits) - 1) as u16;
        source
            .chunks_exact(channels)
            .flat_map(|p| match channels {
                1 => [p[0], p[0], p[0], max],
                2 => [p[0], p[0], p[0], p[1]],
                3 => [p[0], p[1], p[2], max],
                _ => [p[0], p[1], p[2], p[3]],
            })
            .collect()
    }
    macro_rules! fixture {
        ($name:literal,$channels:literal,$bits:literal) => {
            (
                include_bytes!(concat!("fixtures/review/jxl-", $name, ".jxl")).as_slice(),
                include_bytes!(concat!("fixtures/review/jxl-", $name, ".bin")).as_slice(),
                include_bytes!(concat!("fixtures/review/jxl-", $name, "-djxl.bin")).as_slice(),
                $channels,
                $bits,
            )
        };
    }
    type Case = (&'static [u8], &'static [u8], &'static [u8], usize, u8);
    fn cases() -> Vec<Case> {
        vec![
            fixture!("gray8", 1, 8),
            fixture!("gray12", 1, 12),
            fixture!("gray16", 1, 16),
            fixture!("grayalpha8", 2, 8),
            fixture!("grayalpha12", 2, 12),
            fixture!("grayalpha16", 2, 16),
            fixture!("rgba8", 4, 8),
            fixture!("rgba12", 4, 12),
            fixture!("rgba16", 4, 16),
        ]
    }
    #[test]
    fn ordinary_rgba8_matches_independent_source_and_djxl() {
        for (data, expected, oracle, channels, bits) in cases() {
            assert_eq!(expected, oracle, "independent lossless libjxl roundtrip");
            let image = image_from(data).unwrap();
            assert_eq!((image.width, image.height), (7, 5));
            let values = rgba(&samples(expected, bits), channels, bits);
            let max = (1u32 << bits) - 1;
            let expected: Vec<u8> = values
                .iter()
                .map(|&v| ((u32::from(v) * 255 + max / 2) / max) as u8)
                .collect();
            assert_eq!(
                image.buffer.unwrap(),
                expected,
                "channels={channels} bits={bits}"
            );
        }
    }
    #[cfg(feature = "image-buffer-ex")]
    #[test]
    fn ex_native_low_bits_transparent_rgb_and_alpha_are_exact() {
        for (data, expected, oracle, channels, bits) in cases() {
            let format = match (channels == 1, bits) {
                (true, 8) => PixelFormatEx::Gray8,
                (true, 12) => PixelFormatEx::Gray12,
                (true, 16) => PixelFormatEx::Gray16,
                (false, 8) => PixelFormatEx::Rgba8,
                (false, 12) => PixelFormatEx::Rgba12,
                _ => PixelFormatEx::Rgba16,
            };
            let image = image_from_ex(data, format, PrecisionConversion::Exact).unwrap();
            assert_eq!(image.source_format(), Some(format));
            assert_eq!((image.width(), image.height()), (7, 5));
            let expected = if channels == 1 {
                samples(expected, bits)
            } else {
                rgba(&samples(expected, bits), channels, bits)
            };
            let oracle = if channels == 1 {
                samples(oracle, bits)
            } else {
                rgba(&samples(oracle, bits), channels, bits)
            };
            assert_eq!(samples(image.bytes().unwrap(), bits), expected);
            assert_eq!(expected, oracle);
        }
    }
    #[cfg(feature = "image-buffer-ex")]
    #[test]
    fn precision_policy_is_explicit_including_widening() {
        let data = include_bytes!("fixtures/review/jxl-rgba12.jxl");
        assert!(image_from_ex(data, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_err());
        assert!(image_from_ex(data, PixelFormatEx::Rgba16, PrecisionConversion::Exact).is_err());
        for format in [PixelFormatEx::Rgba8, PixelFormatEx::Rgba16] {
            let image = image_from_ex(data, format, PrecisionConversion::Round).unwrap();
            let max = if format == PixelFormatEx::Rgba8 {
                255u32
            } else {
                65535
            };
            let expected: Vec<u16> = samples(include_bytes!("fixtures/review/jxl-rgba12.bin"), 12)
                .into_iter()
                .map(|v| ((u32::from(v) * max + 2047) / 4095) as u16)
                .collect();
            assert_eq!(samples(image.bytes().unwrap(), format.bits()), expected);
        }
        assert!(image_from_ex(RGBA8, PixelFormatEx::Gray8, PrecisionConversion::Round).is_err());
    }
    #[test]
    fn source_and_output_icc_profiles_are_distinct_and_describe_output() {
        for (data, changed) in [
            (
                include_bytes!("fixtures/review/jxl-icc-lossless.jxl").as_slice(),
                false,
            ),
            (
                include_bytes!("fixtures/review/jxl-icc-lossy.jxl").as_slice(),
                true,
            ),
        ] {
            let image = image_from(data).unwrap();
            let meta = image.metadata.unwrap();
            let DataMap::ICCProfile(source) = &meta["JPEG XL Source ICC Profile"] else {
                panic!()
            };
            let DataMap::ICCProfile(output) = &meta["ICC Profile"] else {
                panic!()
            };
            assert_eq!(source, include_bytes!("fixtures/review/jxl-gamma2.icc"));
            assert_eq!(source != output, changed);
            assert_eq!(&output[16..20], b"RGB ");
            assert!(matches!(&meta["JPEG XL Source Header"],DataMap::Raw(v) if !v.is_empty()));
        }
        let image = image_from(include_bytes!("fixtures/review/jxl-gray8.jxl")).unwrap();
        let meta = image.metadata.unwrap();
        let DataMap::ICCProfile(output) = &meta["ICC Profile"] else {
            panic!()
        };
        let DataMap::ICCProfile(source) = &meta["JPEG XL Source ICC Profile"] else {
            panic!()
        };
        assert_eq!(&source[16..20], b"GRAY");
        assert_eq!(&output[16..20], b"RGB ");
    }
    #[test]
    fn lossy_matches_independent_djxl_with_two_lsb_tolerance() {
        let image = image_from(include_bytes!("fixtures/review/jxl-lossy.jxl")).unwrap();
        let expected = rgba(
            &samples(include_bytes!("fixtures/review/jxl-lossy-djxl.bin"), 8),
            3,
            8,
        );
        let max = image
            .buffer
            .unwrap()
            .iter()
            .zip(expected)
            .map(|(&a, b)| u16::from(a).abs_diff(b))
            .max()
            .unwrap();
        assert!(max <= 2, "max 8bit error {max}");
    }
    #[cfg(feature = "image-buffer-ex")]
    #[test]
    fn high_precision_icc_lossless_is_exact_and_lossy_matches_djxl() {
        for (data, expected, tolerance) in [
            (
                include_bytes!("fixtures/review/jxl-icc-lossless.jxl").as_slice(),
                include_bytes!("fixtures/review/jxl-rgba16.bin").as_slice(),
                0,
            ),
            (
                include_bytes!("fixtures/review/jxl-icc-lossy.jxl").as_slice(),
                include_bytes!("fixtures/review/jxl-icc-lossy-djxl.bin").as_slice(),
                32,
            ),
        ] {
            let image =
                image_from_ex(data, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            let max = samples(image.bytes().unwrap(), 16)
                .iter()
                .zip(samples(expected, 16))
                .map(|(a, b)| a.abs_diff(b))
                .max()
                .unwrap();
            assert!(max <= tolerance, "max 16bit error {max}");
        }
    }
    fn box_bytes(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        [
            ((data.len() + 8) as u32).to_be_bytes().as_slice(),
            kind,
            data,
        ]
        .concat()
    }
    fn container(chunks: Vec<Vec<u8>>) -> Vec<u8> {
        let mut data = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
        data.extend(box_bytes(b"ftyp", b"jxl \0\0\0\0jxl "));
        for chunk in chunks {
            data.extend(chunk);
        }
        data
    }
    #[test]
    fn raw_container_chunks_and_source_boxes_decode_identically() {
        let single = container(vec![
            box_bytes(b"jxlc", RGBA8),
            box_bytes(b"xml ", b"<source>test</source>"),
        ]);
        let mut chunks = Vec::new();
        for (i, part) in RGBA8.chunks(RGBA8.len().div_ceil(3)).enumerate() {
            let index = if i == 2 {
                (i as u32) | 0x8000_0000
            } else {
                i as u32
            };
            chunks.push(box_bytes(
                b"jxlp",
                &[index.to_be_bytes().as_slice(), part].concat(),
            ));
        }
        let expected = image_from(RGBA8).unwrap().buffer;
        assert_eq!(image_from(&single).unwrap().buffer, expected);
        assert_eq!(image_from(&container(chunks)).unwrap().buffer, expected);
        assert!(
            image_from(&single)
                .unwrap()
                .metadata
                .unwrap()
                .values()
                .any(|v| matches!(v,DataMap::Raw(b) if b==b"<source>test</source>"))
        );
        let mut extended = Vec::new();
        extended.extend(1u32.to_be_bytes());
        extended.extend(b"jxlc");
        extended.extend(((RGBA8.len() + 16) as u64).to_be_bytes());
        extended.extend(RGBA8);
        assert_eq!(
            image_from(&container(vec![extended])).unwrap().buffer,
            expected
        );
        let mut zero = box_bytes(b"jxlc", RGBA8);
        zero[..4].copy_from_slice(&[0; 4]);
        assert_eq!(image_from(&container(vec![zero])).unwrap().buffer, expected);
    }
    #[derive(Default)]
    struct Probe {
        events: Vec<&'static str>,
        abort: Option<&'static str>,
    }
    impl Probe {
        fn event(&mut self, e: &'static str) -> Response {
            self.events.push(e);
            Ok((self.abort == Some(e)).then(CallbackResponse::abort))
        }
    }
    impl DrawCallback for Probe {
        fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
            self.event("init")
        }
        #[cfg(feature = "image-buffer-ex")]
        fn init_ex(&mut self, _: usize, _: usize, _: Option<InitOptionsEx>) -> Response {
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
        fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
            self.event("metadata")
        }
        fn next(&mut self, _: Option<NextOptions>) -> Response {
            panic!("static only")
        }
        fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
            self.event("end")
        }
        fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
            Ok(None)
        }
    }
    fn probe(data: &[u8], p: &mut Probe, limits: DecodeLimits, native: bool) -> Result<(), Error> {
        let mut reader = BytesReader::new(data);
        let mut options = DecodeOptions {
            debug_flag: 0,
            drawer: p,
        };
        #[cfg(feature = "image-buffer-ex")]
        if native {
            return image_decoder_ex_with_limits(&mut reader, &mut options, limits).map(|_| ());
        }
        let _ = native;
        image_decoder_with_limits(&mut reader, &mut options, limits).map(|_| ())
    }
    #[test]
    fn abort_stops_forwarding_and_terminates_once() {
        for native in [false, true] {
            for abort in ["init", "draw", "metadata", "end"] {
                let mut p = Probe {
                    abort: Some(abort),
                    ..Default::default()
                };
                probe(RGBA8, &mut p, DecodeLimits::default(), native).unwrap();
                let expected = match abort {
                    "init" => vec!["init", "end"],
                    "draw" => vec!["init", "draw", "end"],
                    "metadata" => vec!["init", "draw", "metadata", "end"],
                    _ => {
                        assert_eq!(p.events.iter().filter(|&&s| s == "end").count(), 1);
                        continue;
                    }
                };
                assert_eq!(p.events, expected);
            }
        }
    }
    #[test]
    fn limits_fail_before_callbacks_and_restore_scope() {
        for data in [
            RGBA16,
            include_bytes!("fixtures/review/jxl-icc-lossless.jxl"),
        ] {
            for limits in [
                DecodeLimits {
                    input_bytes: data.len() - 1,
                    ..Default::default()
                },
                DecodeLimits {
                    pixels: 34,
                    ..Default::default()
                },
                DecodeLimits {
                    expanded_bytes: 1,
                    ..Default::default()
                },
                DecodeLimits {
                    metadata_bytes: 1,
                    ..Default::default()
                },
                DecodeLimits {
                    frames: 0,
                    ..Default::default()
                },
                DecodeLimits {
                    animation_bytes: 1,
                    ..Default::default()
                },
            ] {
                for native in [false, true] {
                    let mut p = Probe::default();
                    assert!(probe(data, &mut p, limits, native).is_err());
                    assert!(p.events.is_empty());
                }
            }
            assert!(
                image_from_with_limits(
                    data,
                    DecodeLimits {
                        pixels: 35,
                        ..Default::default()
                    }
                )
                .is_ok()
            );
        }
    }
    #[test]
    fn rejects_unsupported_and_malformed_inputs_before_callbacks() {
        let unsupported = [
            include_bytes!("fixtures/review/jxl-float.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-mixed-alpha.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-depth10.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-premult.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-animation.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-hdr.jxl").as_slice(),
        ];
        for (i, data) in unsupported.into_iter().enumerate() {
            let mut p = Probe::default();
            assert!(
                probe(data, &mut p, DecodeLimits::default(), false).is_err(),
                "accepted unsupported case {i}"
            );
            assert!(p.events.is_empty());
        }
        let mut malformed = vec![
            container(vec![]),
            container(vec![box_bytes(b"jxlc", RGBA8), box_bytes(b"jxlc", RGBA8)]),
            container(vec![box_bytes(b"jxlp", b"\0\0\0\x01junk")]),
            container(vec![
                box_bytes(b"jxlc", RGBA8),
                box_bytes(b"brob", b"xml junk"),
            ]),
            container(vec![box_bytes(b"jxlc", RGBA8), box_bytes(b"jbrd", b"junk")]),
        ];
        let mut trailing = RGBA8.to_vec();
        trailing.extend(b"junk");
        malformed.push(trailing);
        let mut truncatedbox = container(vec![box_bytes(b"jxlc", RGBA8)]);
        truncatedbox.extend(b"123");
        malformed.push(truncatedbox);
        for n in 2..RGBA8.len() {
            malformed.push(RGBA8[..n].to_vec());
        }
        for data in malformed {
            for native in [false, true] {
                let mut p = Probe::default();
                assert!(
                    probe(&data, &mut p, DecodeLimits::default(), native).is_err(),
                    "accepted truncated/malformed {} bytes",
                    data.len()
                );
                assert!(p.events.is_empty());
            }
        }
        assert!(image_from(RGBA8).is_ok());
    }
    #[test]
    fn orientation_is_applied_once() {
        let image = image_from(include_bytes!("fixtures/review/jxl-orientation.jxl")).unwrap();
        assert_eq!((image.width, image.height), (5, 7));
        let src = rgba(
            &samples(include_bytes!("fixtures/review/jxl-rgba16.bin"), 16),
            4,
            16,
        );
        let bytes = image.buffer.unwrap();
        for y in 0..5 {
            for x in 0..7 {
                for c in 0..4 {
                    let v = u32::from(src[(y * 7 + x) * 4 + c]);
                    assert_eq!(
                        bytes[(x * 5 + (4 - y)) * 4 + c],
                        ((v * 255 + 32767) / 65535) as u8
                    );
                }
            }
        }
    }
}
