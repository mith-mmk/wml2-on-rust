use wml2::util::{ImageFormat, decoder_supports_format, format_check};
#[test]
fn detection() {
    for b in [&[255, 79][..], b"\x00\x00\x00\x0cjP  \x0d\x0a\x87\x0a"] {
        assert!(matches!(format_check(b), ImageFormat::Jpeg2000));
        assert_eq!(
            decoder_supports_format(&format_check(b)),
            cfg!(feature = "jpeg2000")
        );
    }
    for b in [&[][..], &[255][..], b"\x00\x00\x00\x0cjP  "] {
        assert!(!matches!(format_check(b), ImageFormat::Jpeg2000));
    }
}
#[cfg(feature = "jpeg2000")]
#[test]
fn oracle_pixels() {
    for (w, h) in [(1, 1), (2, 2), (3, 5), (8, 8), (15, 13), (16, 16)] {
        for level in 0..if w >= 2 { 2 } else { 1 } {
            for pattern in ["noise", "ramp", "flat", "checker"] {
                let stem = format!(
                    "{}/tests/fixtures/review/jp2_{w}x{h}_d{level}_{pattern}",
                    env!("CARGO_MANIFEST_DIR")
                );
                let gray = std::fs::read(format!("{stem}.gray")).unwrap();
                for ext in ["j2k", "jp2"] {
                    let data = std::fs::read(format!("{stem}.{ext}")).unwrap();
                    let image = wml2::draw::image_load(&data)
                        .unwrap_or_else(|e| panic!("{stem}.{ext}: {e}"));
                    assert_eq!((image.width, image.height), (w, h));
                    let expected: Vec<_> = gray.iter().flat_map(|&v| [v, v, v, 255]).collect();
                    assert_eq!(image.buffer.unwrap(), expected, "{stem}.{ext}");
                }
            }
        }
    }
}
#[cfg(feature = "jpeg2000")]
mod enabled {
    use bin_rs::reader::BytesReader;
    use wml2::draw::*;
    use wml2::limits::DecodeLimits;
    use wml2::metadata::DataMap;
    type Error = Box<dyn std::error::Error>;
    const RAW: &[u8] = include_bytes!("fixtures/review/jp2_16x16_d1_noise.j2k");
    const JP2: &[u8] = include_bytes!("fixtures/review/jp2_16x16_d1_noise.jp2");
    fn marker(data: &[u8], m: u8) -> usize {
        data.windows(2).position(|b| b == [255, m]).unwrap()
    }
    #[test]
    fn limits() {
        let mut limits = DecodeLimits::unlimited();
        limits.input_bytes = RAW.len();
        limits.pixels = 256;
        limits.expanded_bytes = 256 * 64 + 8192;
        limits.frames = 1;
        limits.metadata_bytes = 64;
        assert!(image_from_with_limits(RAW, limits).is_ok());
        for field in 0..5 {
            let mut l = limits;
            match field {
                0 => l.input_bytes -= 1,
                1 => l.pixels -= 1,
                2 => l.expanded_bytes -= 1,
                3 => l.frames = 0,
                _ => l.metadata_bytes = 0,
            };
            assert!(image_from_with_limits(RAW, l).is_err(), "limit {field}");
        }
    }
    #[test]
    fn unsupported() {
        let cod = marker(RAW, 0x52);
        let siz = marker(RAW, 0x51);
        let qcd = marker(RAW, 0x5c);
        for (offset, value) in [
            (cod + 4, 1),
            (cod + 5, 1),
            (cod + 7, 2),
            (cod + 8, 1),
            (cod + 9, 6),
            (cod + 10, 9),
            (cod + 11, 9),
            (cod + 12, 1),
            (cod + 13, 0),
            (siz + 40, 15),
            (siz + 41, 2),
            (qcd + 4, 0x42),
        ] {
            let mut data = RAW.to_vec();
            data[offset] = value;
            let e = image_load(&data).err().unwrap().to_string();
            assert!(
                e.contains("Unsupported") || e.contains("Illegal"),
                "{offset}: {e}"
            );
        }
        let mut data = RAW.to_vec();
        data[siz + 6..siz + 10].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(image_load(&data).is_err());
        let mut data = JP2.to_vec();
        let p = data.windows(4).position(|b| b == b"colr").unwrap();
        data[p + 10] = 16;
        assert!(image_load(&data).is_err());
    }
    #[test]
    fn structural_rejection() {
        for input in [RAW, JP2] {
            for n in 0..input.len() {
                assert!(image_load(&input[..n]).is_err(), "prefix {n}");
            }
            let mut data = input.to_vec();
            data.push(0);
            assert!(image_load(&data).is_err());
        }
        for m in [0x51, 0x52, 0x5c, 0x90] {
            for value in [0u16, 1, 65535] {
                let mut data = RAW.to_vec();
                let p = marker(&data, m);
                data[p + 2..p + 4].copy_from_slice(&value.to_be_bytes());
                assert!(image_load(&data).is_err());
            }
        }
        let mut data = RAW.to_vec();
        let p = marker(&data, 0x90);
        data[p + 6..p + 10].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(image_load(&data).is_err());
        let mut data = JP2.to_vec();
        data[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(image_load(&data).is_err());
    }
    #[derive(Default)]
    struct Drawer {
        events: Vec<&'static str>,
        abort: Option<&'static str>,
        fail: Option<&'static str>,
        rgb: bool,
    }
    impl Drawer {
        fn event(&mut self, name: &'static str) -> Result<Option<CallbackResponse>, Error> {
            self.events.push(name);
            if self.fail == Some(name) {
                return Err(std::io::Error::other(name).into());
            }
            Ok(self
                .abort
                .filter(|&s| s == name)
                .map(|_| CallbackResponse::abort()))
        }
    }
    impl DrawCallback for Drawer {
        fn init(
            &mut self,
            w: usize,
            h: usize,
            _: Option<InitOptions>,
        ) -> Result<Option<CallbackResponse>, Error> {
            assert_eq!((w, h), (16, 16));
            self.event("init")
        }
        fn draw(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            b: &[u8],
            _: Option<DrawOptions>,
        ) -> Result<Option<CallbackResponse>, Error> {
            assert_eq!((x, y, w, h, b.len()), (0, 0, 16, 16, 1024));
            if self.rgb {
                let source = include_bytes!("fixtures/review/jp2_rgb_16x16_d1_noise.rgb");
                let expected: Vec<_> = source
                    .chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect();
                assert_eq!(b, expected);
            } else {
                assert!(
                    b.chunks_exact(4)
                        .all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255)
                );
            }
            self.event("draw")
        }
        fn terminate(
            &mut self,
            _: Option<TerminateOptions>,
        ) -> Result<Option<CallbackResponse>, Error> {
            self.event("terminate")
        }
        fn next(&mut self, _: Option<NextOptions>) -> Result<Option<CallbackResponse>, Error> {
            panic!("unexpected next")
        }
        fn verbose(
            &mut self,
            _: &str,
            _: Option<VerboseOptions>,
        ) -> Result<Option<CallbackResponse>, Error> {
            panic!("unexpected verbose")
        }
        fn set_metadata(&mut self, _: &str, _: DataMap) -> Result<Option<CallbackResponse>, Error> {
            panic!("unexpected metadata")
        }
    }
    fn decode(data: &[u8], drawer: &mut Drawer) -> Result<(), Error> {
        image_decoder(
            &mut BytesReader::new(data),
            &mut DecodeOptions {
                drawer,
                debug_flag: 0,
            },
        )
        .map(|_| ())
    }
    #[test]
    fn callbacks() {
        for abort in [None, Some("init"), Some("draw")] {
            let mut drawer = Drawer {
                abort,
                ..Default::default()
            };
            decode(RAW, &mut drawer).unwrap();
            assert_eq!(
                drawer.events,
                if abort == Some("init") {
                    vec!["init", "terminate"]
                } else {
                    vec!["init", "draw", "terminate"]
                }
            );
        }
        for fail in ["init", "draw", "terminate"] {
            let mut drawer = Drawer {
                fail: Some(fail),
                ..Default::default()
            };
            assert!(decode(RAW, &mut drawer).is_err());
            assert_eq!(drawer.events.last(), Some(&fail));
        }
        let mut drawer = Drawer::default();
        assert!(decode(&RAW[..RAW.len() - 1], &mut drawer).is_err());
        assert!(drawer.events.is_empty());
    }
    #[test]
    fn rgb_callbacks_and_invalid_inputs_emit_no_callbacks() {
        for raw in [
            &include_bytes!("fixtures/review/jp2_rgb_16x16_d1_noise_mct0.j2k")[..],
            &include_bytes!("fixtures/review/jp2_rgb_16x16_d1_noise_mct1.j2k")[..],
        ] {
            for abort in [None, Some("init"), Some("draw")] {
                let mut drawer = Drawer {
                    rgb: true,
                    abort,
                    ..Default::default()
                };
                decode(raw, &mut drawer).unwrap();
                assert_eq!(
                    drawer.events,
                    if abort == Some("init") {
                        vec!["init", "terminate"]
                    } else {
                        vec!["init", "draw", "terminate"]
                    }
                );
            }
            for fail in ["init", "draw", "terminate"] {
                let mut drawer = Drawer {
                    rgb: true,
                    fail: Some(fail),
                    ..Default::default()
                };
                assert!(decode(raw, &mut drawer).is_err());
                assert_eq!(drawer.events.last(), Some(&fail));
            }
            let cod = marker(raw, 0x52);
            for (offset, value) in [(cod + 8, 2), (cod + 13, 0)] {
                let mut bad = raw.to_vec();
                bad[offset] = value;
                let mut drawer = Drawer::default();
                assert!(decode(&bad, &mut drawer).is_err());
                assert!(drawer.events.is_empty());
            }
            let mut drawer = Drawer::default();
            assert!(decode(&raw[..raw.len() - 1], &mut drawer).is_err());
            assert!(drawer.events.is_empty());
        }
    }
    #[test]
    fn deterministic_mutations_do_not_panic() {
        for data in [RAW, JP2] {
            for i in 0..data.len() {
                for mask in [1, 128, 255] {
                    let mut mutated = data.to_vec();
                    mutated[i] ^= mask;
                    let _ = image_load(&mutated);
                }
            }
        }
    }
}
#[cfg(not(feature = "jpeg2000"))]
#[test]
fn feature_off() {
    assert!(
        wml2::draw::image_load(include_bytes!("fixtures/review/jp2_16x16_d1_noise.j2k")).is_err()
    );
    assert!(!wml2::get_decoder_extentions().iter().any(|v| v == "jp2"));
}
#[cfg(feature = "jpeg2000")]
#[test]
fn multiple_blocks_match_oracle() {
    for (w, h) in [
        (17, 19),
        (31, 33),
        (65, 49),
        (257, 129),
        (1, 513),
        (513, 1),
        (1024, 1024),
    ] {
        for level in 0..if w > 1 && h > 1 { 2 } else { 1 } {
            let patterns: &[&str] = if w == 1024 {
                &["ramp"]
            } else {
                &["noise", "mixed"]
            };
            for pattern in patterns {
                let stem = format!(
                    "{}/tests/fixtures/review/jp2_multi_{w}x{h}_d{level}_{pattern}",
                    env!("CARGO_MANIFEST_DIR")
                );
                let gray = std::fs::read(format!("{stem}.gray")).unwrap();
                let expected: Vec<_> = gray.iter().flat_map(|&v| [v, v, v, 255]).collect();
                for ext in ["j2k", "jp2"] {
                    let data = std::fs::read(format!("{stem}.{ext}")).unwrap();
                    let image = wml2::draw::image_load(&data)
                        .unwrap_or_else(|e| panic!("{stem}.{ext}: {e}"));
                    assert_eq!((image.width, image.height), (w, h));
                    assert_eq!(image.buffer.unwrap(), expected, "{stem}.{ext}");
                }
            }
        }
    }
}
#[cfg(feature = "jpeg2000")]
#[test]
fn multiple_block_bounds_and_limits() {
    use wml2::draw::{image_from_with_limits, image_load};
    use wml2::limits::DecodeLimits;
    let raw = include_bytes!("fixtures/review/jp2_multi_65x49_d1_noise.j2k");
    let mut limits = DecodeLimits::unlimited();
    limits.pixels = 65 * 49;
    limits.expanded_bytes = 65 * 49 * 64 + 8192;
    assert!(image_from_with_limits(raw, limits).is_ok());
    for field in 0..2 {
        let mut limited = limits;
        if field == 0 {
            limited.pixels -= 1;
        } else {
            limited.expanded_bytes -= 1;
        }
        assert!(image_from_with_limits(raw, limited).is_err());
    }
    let siz = raw.windows(2).position(|b| b == [255, 0x51]).unwrap();
    let cod = raw.windows(2).position(|b| b == [255, 0x52]).unwrap();
    let sot = raw.windows(2).position(|b| b == [255, 0x90]).unwrap();
    for dimension in [0u32, 1025, u32::MAX] {
        let mut bad = raw.to_vec();
        bad[siz + 6..siz + 10].copy_from_slice(&dimension.to_be_bytes());
        bad[siz + 22..siz + 26].copy_from_slice(&dimension.to_be_bytes());
        assert!(image_load(&bad).is_err());
    }
    for level in [6, 32, 255] {
        let mut bad = raw.to_vec();
        bad[cod + 9] = level;
        assert!(
            image_load(&bad)
                .err()
                .unwrap()
                .to_string()
                .contains("Unsupported")
        );
    }
    // Repair outer Psot/EOC so short packet headers and bodies reach the parser.
    for n in [0, 1, 2, 3, 4, 8, 16, 32] {
        let mut bad = raw[..sot + 14 + n].to_vec();
        bad[sot + 6..sot + 10].copy_from_slice(&((14 + n) as u32).to_be_bytes());
        bad.extend_from_slice(&[255, 217]);
        assert!(image_load(&bad).is_err(), "tile payload {n}");
    }
    // Body/header corruption may encode another legal image; always require safety.
    for n in 0..128 {
        let mut bad = raw.to_vec();
        bad[sot + 14 + n] ^= 255;
        let _ = image_load(&bad);
    }
}
#[cfg(feature = "jpeg2000")]
#[test]
fn multilevel_oracles() {
    for (w, h, level) in [
        (4, 4, 2),
        (5, 7, 2),
        (8, 8, 3),
        (9, 11, 3),
        (16, 16, 4),
        (17, 19, 4),
        (32, 32, 5),
        (33, 35, 5),
        (65, 49, 2),
        (65, 49, 5),
        (257, 129, 3),
        (257, 129, 5),
        (1024, 1024, 5),
    ] {
        let patterns: &[&str] = if w == 1024 {
            &["ramp"]
        } else {
            &["noise", "mixed"]
        };
        for pattern in patterns {
            let stem = format!(
                "{}/tests/fixtures/review/jp2_deep_{w}x{h}_d{level}_{pattern}",
                env!("CARGO_MANIFEST_DIR")
            );
            let gray = std::fs::read(format!("{stem}.gray")).unwrap();
            let expected: Vec<_> = gray.iter().flat_map(|&v| [v, v, v, 255]).collect();
            for ext in ["j2k", "jp2"] {
                let data = std::fs::read(format!("{stem}.{ext}")).unwrap();
                let image =
                    wml2::draw::image_load(&data).unwrap_or_else(|e| panic!("{stem}.{ext}: {e}"));
                assert_eq!((image.width, image.height), (w, h));
                assert_eq!(image.buffer.unwrap(), expected, "{stem}.{ext}");
            }
        }
    }
}
#[cfg(feature = "jpeg2000")]
#[test]
fn multilevel_boundaries_and_corruption() {
    use wml2::draw::{image_from_with_limits, image_load};
    use wml2::limits::DecodeLimits;
    let raw = include_bytes!("fixtures/review/jp2_deep_33x35_d5_noise.j2k");
    let siz = raw.windows(2).position(|b| b == [255, 0x51]).unwrap();
    let cod = raw.windows(2).position(|b| b == [255, 0x52]).unwrap();
    let sot = raw.windows(2).position(|b| b == [255, 0x90]).unwrap();
    let qcd = raw.windows(2).position(|b| b == [255, 0x5c]).unwrap();
    let mut limits = DecodeLimits::unlimited();
    limits.pixels = 33 * 35;
    limits.expanded_bytes = 33 * 35 * 64 + 8192;
    assert!(image_from_with_limits(raw, limits).is_ok());
    for field in 0..2 {
        let mut short = limits;
        if field == 0 {
            short.pixels -= 1;
        } else {
            short.expanded_bytes -= 1;
        }
        assert!(image_from_with_limits(raw, short).is_err());
    }
    for (w, h) in [(31u32, 35u32), (33, 31)] {
        let mut data = raw.to_vec();
        for (p, v) in [(siz + 6, w), (siz + 10, h), (siz + 22, w), (siz + 26, h)] {
            data[p..p + 4].copy_from_slice(&v.to_be_bytes());
        }
        assert!(
            image_load(&data)
                .err()
                .unwrap()
                .to_string()
                .contains("2^decomposition")
        );
    }
    for level in [6, 32, 255] {
        let mut data = raw.to_vec();
        data[cod + 9] = level;
        assert!(
            image_load(&data)
                .err()
                .unwrap()
                .to_string()
                .contains("Unsupported")
        );
    }
    for level in [0, 1, 2, 3, 4] {
        let mut data = raw.to_vec();
        data[cod + 9] = level;
        assert!(
            image_load(&data)
                .err()
                .unwrap()
                .to_string()
                .contains("QCD band count")
        );
    }
    let mut data = raw.to_vec();
    data[qcd + 4] = 0xe0;
    data[qcd + 5] = 0xf8;
    assert!(
        image_load(&data)
            .err()
            .unwrap()
            .to_string()
            .contains("precision")
    );
    for n in [0, 1, 2, 4, 8, 16, 32, 64] {
        let mut data = raw[..sot + 14 + n].to_vec();
        data[sot + 6..sot + 10].copy_from_slice(&((14 + n) as u32).to_be_bytes());
        data.extend_from_slice(&[255, 217]);
        assert!(image_load(&data).is_err(), "short multilevel packet {n}");
    }
    for n in 0..128 {
        let mut data = raw.to_vec();
        data[sot + 14 + n] ^= 255;
        let _ = image_load(&data);
    }
}

#[cfg(feature = "jpeg2000")]
#[test]
fn rgb_oracle_pixels() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = include_str!("fixtures/review/jpeg2000-rgb-manifest.txt");
    let mut checked = 0;
    for line in manifest.lines().skip(1) {
        let name = line.split_whitespace().nth(1).unwrap();
        if !name.ends_with(".j2k") && !name.ends_with(".jp2") {
            continue;
        }
        let stem = name.split("_mct").next().unwrap();
        let source = std::fs::read(format!("{root}{stem}.rgb")).unwrap();
        let data = std::fs::read(format!("{root}{name}")).unwrap();
        let image = wml2::draw::image_load(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
        let shape = stem.split('_').nth(2).unwrap();
        let (width, height) = shape.split_once('x').unwrap();
        let expected_shape = (
            width.parse::<usize>().unwrap(),
            height.parse::<usize>().unwrap(),
        );
        assert_eq!((image.width, image.height), expected_shape, "{name}");
        let expected: Vec<_> = source
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect();
        assert_eq!(image.width * image.height * 3, source.len(), "{name}");
        assert_eq!(image.buffer.unwrap(), expected, "{name}");
        checked += 1;
    }
    assert_eq!(checked, 184);
}

#[cfg(feature = "jpeg2000")]
mod rgb_bounds {
    use wml2::draw::{image_from_with_limits, image_load};
    use wml2::limits::DecodeLimits;
    const RAW: &[u8] = include_bytes!("fixtures/review/jp2_rgb_16x16_d1_noise_mct1.j2k");
    const JP2: &[u8] = include_bytes!("fixtures/review/jp2_rgb_16x16_d1_noise_mct1.jp2");
    fn marker(data: &[u8], value: u8) -> usize {
        data.windows(2).position(|b| b == [255, value]).unwrap()
    }
    fn with_qcc(data: &[u8], component: u8, quant: &[u8]) -> Vec<u8> {
        let sot = marker(data, 0x90);
        let mut result = data[..sot].to_vec();
        result.extend_from_slice(&[255, 0x5d]);
        result.extend_from_slice(&((quant.len() + 3) as u16).to_be_bytes());
        result.push(component);
        result.extend_from_slice(quant);
        result.extend_from_slice(&data[sot..]);
        result
    }
    #[test]
    fn exact_rgb_resource_limits() {
        for raw in [
            RAW,
            &include_bytes!("fixtures/review/jp2_rgb_16x16_d1_noise_mct0.j2k")[..],
            &include_bytes!("fixtures/review/jp2_rgb_33x35_d5_noise_mct1.j2k")[..],
        ] {
            let siz = marker(raw, 0x51);
            let w = u32::from_be_bytes(raw[siz + 6..siz + 10].try_into().unwrap()) as usize;
            let h = u32::from_be_bytes(raw[siz + 10..siz + 14].try_into().unwrap()) as usize;
            let mut limit = DecodeLimits::unlimited();
            limit.input_bytes = raw.len();
            limit.pixels = w * h;
            limit.expanded_bytes = w * h * 64 * 3 + 8192;
            limit.frames = 1;
            limit.metadata_bytes = 64;
            assert!(image_from_with_limits(raw, limit).is_ok());
            for field in 0..5 {
                let mut short = limit;
                match field {
                    0 => short.input_bytes -= 1,
                    1 => short.pixels -= 1,
                    2 => short.expanded_bytes -= 1,
                    3 => short.frames = 0,
                    _ => short.metadata_bytes = 0,
                }
                assert!(image_from_with_limits(raw, short).is_err(), "limit {field}");
            }
        }
    }
    #[test]
    fn qcc_overrides_and_rejection() {
        let qcd = marker(RAW, 0x5c);
        let length = usize::from(u16::from_be_bytes(
            RAW[qcd + 2..qcd + 4].try_into().unwrap(),
        ));
        let quant = &RAW[qcd + 4..qcd + 2 + length];
        let expected = image_load(RAW).unwrap().buffer.unwrap();
        let mut data = RAW.to_vec();
        for component in 0..3 {
            data = with_qcc(&data, component, quant);
            assert_eq!(image_load(&data).unwrap().buffer.unwrap(), expected);
        }
        let mut invalid_default = data.clone();
        invalid_default[qcd + 4] = 0x42;
        assert!(image_load(&invalid_default).is_err());
        assert!(image_load(&with_qcc(&data, 1, quant)).is_err());
        for component in [3, 255] {
            assert!(image_load(&with_qcc(RAW, component, quant)).is_err());
        }
        assert!(image_load(&with_qcc(RAW, 1, &[])).is_err());
        assert!(image_load(&with_qcc(RAW, 1, &quant[..quant.len() - 1])).is_err());
        for (index, value) in [(0, 0x42), (1, 0xff), (1, 0xf8)] {
            let mut bad_quant = quant.to_vec();
            bad_quant[index] = value;
            assert!(image_load(&with_qcc(RAW, 1, &bad_quant)).is_err());
        }
        let gray = include_bytes!("fixtures/review/jp2_16x16_d1_noise.j2k");
        assert!(image_load(&with_qcc(gray, 0, quant)).is_err());
    }
    #[test]
    fn qcc_different_component_precision_matches_oracle() {
        // QCC raises this component's Mb from 9 to 10; its zero-plane tag
        // compensates so the coded bit depth and RGB pixels remain unchanged.
        // Ignoring QCC makes the full coding-pass check fail for these inputs.
        for data in [
            &include_bytes!("fixtures/review/jp2_qcc_1x1_d0_c0.j2k")[..],
            &include_bytes!("fixtures/review/jp2_qcc_1x1_d0_c1.j2k")[..],
            &include_bytes!("fixtures/review/jp2_qcc_1x1_d0_c2.j2k")[..],
        ] {
            let image = image_load(data).unwrap();
            assert_eq!((image.width, image.height), (1, 1));
            assert_eq!(image.buffer.unwrap(), [31, 83, 197, 255]);
        }
    }
    #[test]
    fn component_profiles_and_container_consistency() {
        let siz = marker(RAW, 0x51);
        let cod = marker(RAW, 0x52);
        for component in 0..3 {
            for (field, value) in [(0, 15), (0, 135), (1, 0), (1, 2), (2, 0), (2, 2)] {
                let mut data = RAW.to_vec();
                data[siz + 40 + 3 * component + field] = value;
                assert!(image_load(&data).is_err());
            }
        }
        for count in [0u16, 1, 2, 4, 65535] {
            let mut data = RAW.to_vec();
            data[siz + 38..siz + 40].copy_from_slice(&count.to_be_bytes());
            assert!(image_load(&data).is_err());
        }
        for (offset, value) in [(cod + 8, 2), (cod + 9, 6), (cod + 13, 0)] {
            let mut data = RAW.to_vec();
            data[offset] = value;
            assert!(image_load(&data).is_err());
        }
        for (offset, values) in [
            (siz + 6, [0u32, 1025, u32::MAX]),
            (siz + 10, [0u32, 1025, u32::MAX]),
        ] {
            for value in values {
                let mut data = RAW.to_vec();
                data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
                assert!(image_load(&data).is_err());
            }
        }
        let ihdr = JP2.windows(4).position(|b| b == b"ihdr").unwrap();
        let colr = JP2.windows(4).position(|b| b == b"colr").unwrap();
        for (offset, value) in [(ihdr + 13, 1), (ihdr + 11, 17), (colr + 10, 17)] {
            let mut data = JP2.to_vec();
            data[offset] = value;
            assert!(image_load(&data).is_err());
        }
    }
    #[test]
    fn rgb_truncation_and_mutations() {
        for raw in [RAW, JP2] {
            for end in 0..raw.len() {
                assert!(image_load(&raw[..end]).is_err(), "prefix {end}");
            }
            let mut extra = raw.to_vec();
            extra.push(0);
            assert!(image_load(&extra).is_err());
            for offset in 0..raw.len() {
                for mask in [1, 128, 255] {
                    let mut data = raw.to_vec();
                    data[offset] ^= mask;
                    let _ = image_load(&data);
                }
            }
        }
        let raw = include_bytes!("fixtures/review/jp2_rgb_33x35_d5_noise_mct1.j2k");
        let sot = marker(raw, 0x90);
        for n in [0, 1, 2, 4, 8, 16, 32, 64] {
            let mut data = raw[..sot + 14 + n].to_vec();
            data[sot + 6..sot + 10].copy_from_slice(&((14 + n) as u32).to_be_bytes());
            data.extend_from_slice(&[255, 217]);
            assert!(image_load(&data).is_err());
        }
        for n in 0..128 {
            let mut data = raw.to_vec();
            data[sot + 14 + n] ^= 255;
            let _ = image_load(&data);
        }
    }
}

#[cfg(feature = "jpeg2000")]
#[test]
fn code_block_geometry_matches_oracle() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!("{root}jpeg2000-blocks-manifest.txt")).unwrap();
    let mut checked = 0;
    for line in manifest.lines().skip(1) {
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 11);
        let name = fields[1];
        let source = std::fs::read(format!("{root}{}", fields[3])).unwrap();
        let number = |index: usize| fields[index].parse::<usize>().unwrap();
        let (w, h, bw, bh, levels, components, mct) = (
            number(4),
            number(5),
            number(6),
            number(7),
            number(8),
            number(9),
            number(10),
        );
        let data = std::fs::read(format!("{root}{name}")).unwrap();
        let cod = data.windows(2).position(|b| b == [255, 0x52]).unwrap();
        assert_eq!(
            (
                1usize << (data[cod + 10] + 2),
                1usize << (data[cod + 11] + 2)
            ),
            (bw, bh)
        );
        assert_eq!(
            (usize::from(data[cod + 9]), usize::from(data[cod + 8])),
            (levels, mct)
        );
        let image = wml2::draw::image_load(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((image.width, image.height), (w, h), "{name}");
        assert_eq!(source.len(), w * h * components);
        let expected: Vec<_> = source
            .chunks_exact(components)
            .flat_map(|p| {
                if components == 1 {
                    [p[0], p[0], p[0], 255]
                } else {
                    [p[0], p[1], p[2], 255]
                }
            })
            .collect();
        assert_eq!(image.buffer.unwrap(), expected, "{name}");
        // The same geometry-scaled budget covers all accepted block shapes.
        let mut limits = wml2::limits::DecodeLimits::unlimited();
        limits.expanded_bytes = w * h * components * 64 + 8192;
        limits.pixels = w * h;
        assert!(
            wml2::draw::image_from_with_limits(&data, limits).is_ok(),
            "{name}"
        );
        limits.expanded_bytes -= 1;
        assert!(
            wml2::draw::image_from_with_limits(&data, limits).is_err(),
            "{name}"
        );
        checked += 1;
    }
    assert_eq!(checked, 318);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn invalid_code_block_exponents_are_rejected_before_callbacks() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder};
    for input in [
        &include_bytes!("fixtures/review/jp2_16x16_d1_noise.j2k")[..],
        &include_bytes!("fixtures/review/jp2_rgb_16x16_d1_noise_mct1.j2k")[..],
    ] {
        let cod = input.windows(2).position(|b| b == [255, 0x52]).unwrap();
        for (x, y) in [(9, 0), (0, 9), (255, 255), (5, 4), (8, 1), (1, 8)] {
            let mut bad = input.to_vec();
            bad[cod + 10] = x;
            bad[cod + 11] = y;
            let mut image = ImageBuffer::new();
            let result = image_decoder(
                &mut BytesReader::new(&bad),
                &mut DecodeOptions {
                    drawer: &mut image,
                    debug_flag: 0,
                },
            );
            assert!(result.is_err());
            assert!(image.buffer.is_none(), "invalid block initialized output");
        }
    }
}

#[cfg(feature = "jpeg2000")]
#[test]
fn variable_code_block_corruption_stays_bounded() {
    use wml2::draw::image_load;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!("{root}jpeg2000-blocks-manifest.txt")).unwrap();
    let mut checked = 0;
    for line in manifest.lines().skip(1) {
        let fields: Vec<_> = line.split_whitespace().collect();
        let shape = (
            fields[6].parse::<usize>().unwrap(),
            fields[7].parse::<usize>().unwrap(),
        );
        if !fields[1].ends_with(".j2k")
            || fields[8] != "0"
            || !matches!(shape, (4, 4) | (64, 64) | (1024, 4) | (4, 1024))
            || (fields[9] == "3" && fields[10] != "1")
        {
            continue;
        }
        // Select the full-block/partial-edge matrix, excluding the separate
        // 257x129 multilevel cases that share the same nominal block shape.
        if fields[4].parse::<usize>().unwrap() != (shape.0 + 3).min(1024)
            || fields[5].parse::<usize>().unwrap() != (shape.1 + 5).min(1024)
        {
            continue;
        }
        let data = std::fs::read(format!("{root}{}", fields[1])).unwrap();
        let marker = |m| data.windows(2).position(|b| b == [255, m]).unwrap();
        let sot = marker(0x90);
        for n in [0, 1, 2, 4, 8, 16, 32] {
            let mut bad = data[..sot + 14 + n].to_vec();
            bad[sot + 6..sot + 10].copy_from_slice(&((14 + n) as u32).to_be_bytes());
            bad.extend_from_slice(&[255, 217]);
            assert!(image_load(&bad).is_err(), "{} short {n}", fields[1]);
        }
        for m in [0x51, 0x52, 0x5c, 0x90] {
            for length in [0u16, 1, 65535] {
                let mut bad = data.clone();
                let p = marker(m);
                bad[p + 2..p + 4].copy_from_slice(&length.to_be_bytes());
                assert!(image_load(&bad).is_err());
            }
        }
        for n in 0..32 {
            for mask in [1, 128, 255] {
                let mut bad = data.clone();
                bad[sot + 14 + n] ^= mask;
                let _ = image_load(&bad);
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 8);
}
