use wml2::util::{ImageFormat, decoder_supports_format, format_check};
#[cfg(feature = "jpeg2000")]
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
    for magic in [&[255, 79][..], b"\x00\x00\x00\x0cjP  \x0d\x0a\x87\x0a"] {
        assert!(matches!(format_check(magic), ImageFormat::Unknown));
        assert!(!decoder_supports_format(&format_check(magic)));
    }
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
        // Small axes are supported now; this mutated header still disagrees
        // with the original packed coefficient/packet layout.
        assert!(image_load(&data).is_err());
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

#[cfg(feature = "jpeg2000")]
#[test]
fn tiled_images_match_oracle_and_canvas_limits() {
    use wml2::draw::image_from_with_limits;
    use wml2::limits::DecodeLimits;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!("{root}jpeg2000-tiles-manifest.txt")).unwrap();
    let mut modes = std::collections::BTreeSet::new();
    let mut levels_seen = std::collections::BTreeSet::new();
    let mut maximum_tiles = 0;
    let mut checked = 0;
    for line in manifest.lines().skip(1) {
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 14);
        let n = |i: usize| fields[i].parse::<usize>().unwrap();
        let (w, h, components, count) = (n(4), n(5), n(9), n(13));
        assert_eq!(count, w.div_ceil(n(11)) * h.div_ceil(n(12)));
        assert!(count > 1 && count <= 256);
        let name = fields[1];
        let data = std::fs::read(format!("{root}{name}")).unwrap();
        let source = std::fs::read(format!("{root}{}", fields[3])).unwrap();
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
        let mut limits = DecodeLimits::unlimited();
        limits.input_bytes = data.len();
        limits.pixels = w * h;
        limits.expanded_bytes = w * h * components * 64 + 8192;
        limits.frames = 1; // Tiles are portions of one frame.
        let image = image_from_with_limits(&data, limits).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((image.width, image.height), (w, h), "{name}");
        assert_eq!(image.buffer.unwrap(), expected, "{name}");
        for which in 0..4 {
            let mut short = limits;
            match which {
                0 => short.input_bytes -= 1,
                1 => short.pixels -= 1,
                2 => short.expanded_bytes -= 1,
                _ => short.frames = 0,
            }
            assert!(
                image_from_with_limits(&data, short).is_err(),
                "{name}: limit {which}"
            );
        }
        modes.insert((components, n(10)));
        levels_seen.insert(n(8));
        maximum_tiles = maximum_tiles.max(count);
        checked += 1;
    }
    assert_eq!(checked, 162);
    assert_eq!(modes, [(1, 0), (3, 0), (3, 1)].into_iter().collect());
    assert_eq!(levels_seen, (0..=5).collect());
    assert_eq!(maximum_tiles, 256);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn invalid_tile_structure_is_rejected_before_callbacks() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!("{root}jpeg2000-tiles-manifest.txt")).unwrap();
    let reject = |data: &[u8]| {
        let mut image = ImageBuffer::new();
        assert!(
            image_decoder(
                &mut BytesReader::new(data),
                &mut DecodeOptions {
                    drawer: &mut image,
                    debug_flag: 0,
                }
            )
            .is_err()
        );
        assert!(image.buffer.is_none(), "invalid tiling initialized output");
    };
    let mut selected = std::collections::BTreeSet::new();
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        let levels = f[8].parse::<usize>().unwrap();
        let mode = (f[9], f[10]);
        if !f[1].ends_with(".j2k")
            || levels == 0
            || f[13].parse::<usize>().unwrap() < 4
            || selected.contains(&mode)
        {
            continue;
        }
        selected.insert(mode);
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let mut p = 2;
        while data[p..p + 2] != [255, 0x90] {
            p += usize::from(u16::from_be_bytes([data[p + 2], data[p + 3]])) + 2;
        }
        let mut parts = Vec::new();
        while data[p..p + 2] == [255, 0x90] {
            let len = u32::from_be_bytes(data[p + 6..p + 10].try_into().unwrap()) as usize;
            parts.push((p, p + len));
            p += len;
        }
        assert_eq!(parts.len(), f[13].parse::<usize>().unwrap());
        let (first, first_end) = parts[0];
        let (second, second_end) = parts[1];
        let (last, _) = *parts.last().unwrap();
        for start in [first, second, last] {
            // Invalid IDs/part counts/length/SOD on an early or late tile must
            // not emit a partially reconstructed canvas.
            for (offset, bytes) in [
                (4, vec![255, 255]),
                (10, vec![1]),
                (11, vec![19]),
                (11, vec![2]),
                (2, vec![0, 9]),
                (2, vec![0, 11]),
                (6, vec![0, 0, 0, 13]),
                (6, vec![255, 255, 255, 255]),
                (12, vec![255, 0x52]),
            ] {
                let mut bad = data.clone();
                bad[start + offset..start + offset + bytes.len()].copy_from_slice(&bytes);
                reject(&bad);
            }
            for end in [start, start + 1, start + 12, start + 14] {
                reject(&data[..end]);
            }
        }
        let mut duplicate = data.clone();
        duplicate[second + 4..second + 6].copy_from_slice(&[0, 0]);
        reject(&duplicate);
        let mut swapped = data[..first].to_vec();
        swapped.extend_from_slice(&data[second..second_end]);
        swapped.extend_from_slice(&data[first..first_end]);
        swapped.extend_from_slice(&data[second_end..]);
        reject(&swapped);
        let mut missing = data[..second].to_vec();
        missing.extend_from_slice(&data[second_end..]);
        reject(&missing);
        let mut early_eoc = data[..second].to_vec();
        early_eoc.extend_from_slice(&[255, 217]);
        reject(&early_eoc);
        let mut extra = data[..data.len() - 2].to_vec();
        extra.extend_from_slice(&data[first..first_end]);
        extra.extend_from_slice(&[255, 217]);
        reject(&extra);
        let mut trailing = data.clone();
        trailing.push(0);
        reject(&trailing);
        // SIZ starts immediately after SOC; tile dimensions are at 24/28.
        for offset in [24, 28] {
            for value in [0u32, 1025, u32::MAX] {
                let mut bad = data.clone();
                bad[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
                reject(&bad);
            }
        }
        let mut excessive = data.clone();
        excessive[24..28].copy_from_slice(&1u32.to_be_bytes());
        excessive[28..32].copy_from_slice(&1u32.to_be_bytes());
        reject(&excessive);
        // Repaired tile lengths reach the packet parser instead of only
        // failing outer framing. Require rejection of missing packet bodies.
        for n in [0, 1, 2, 4] {
            if first + 14 + n >= first_end {
                continue;
            }
            let mut short = data[..first + 14 + n].to_vec();
            short[first + 6..first + 10].copy_from_slice(&((14 + n) as u32).to_be_bytes());
            short.extend_from_slice(&data[first_end..]);
            reject(&short);
        }
        for (start, end) in [parts[0], parts[1], *parts.last().unwrap()] {
            for offset in 14..(end - start).min(46) {
                for mask in [1, 128, 255] {
                    let mut changed = data.clone();
                    changed[start + offset] ^= mask;
                    // A mutation may encode different valid pixels. It must
                    // remain bounded and never panic.
                    let _ = wml2::draw::image_load(&changed);
                }
            }
        }
    }
    assert_eq!(selected.len(), 3);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn previous_tile_origin_rejections_become_pixel_oracles() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder, image_from_with_limits};
    use wml2::limits::DecodeLimits;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest =
        std::fs::read_to_string(format!("{root}jpeg2000-tiles-unsupported-manifest.txt")).unwrap();
    let (mut accepted, mut rejected) = (0, 0);
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        assert_eq!(f.len(), 15);
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        if f[14] == "count" {
            let mut image = ImageBuffer::new();
            assert!(
                image_decoder(
                    &mut BytesReader::new(&data),
                    &mut DecodeOptions {
                        drawer: &mut image,
                        debug_flag: 0,
                    }
                )
                .is_err()
            );
            assert!(image.buffer.is_none());
            rejected += 1;
        } else {
            let n = |i: usize| f[i].parse::<usize>().unwrap();
            let (w, h, c) = (n(4), n(5), n(9));
            let source = std::fs::read(format!("{root}{}", f[3])).unwrap();
            let expected: Vec<_> = source
                .chunks_exact(c)
                .flat_map(|p| {
                    if c == 1 {
                        [p[0], p[0], p[0], 255]
                    } else {
                        [p[0], p[1], p[2], 255]
                    }
                })
                .collect();
            let mut limits = DecodeLimits::unlimited();
            limits.pixels = w * h;
            limits.expanded_bytes = w * h * c * 64 + 8192;
            limits.frames = 1;
            let image =
                image_from_with_limits(&data, limits).unwrap_or_else(|e| panic!("{}: {e}", f[1]));
            assert_eq!((image.width, image.height), (w, h));
            assert_eq!(image.buffer.unwrap(), expected, "{}", f[1]);
            limits.expanded_bytes -= 1;
            assert!(image_from_with_limits(&data, limits).is_err());
            accepted += 1;
        }
    }
    assert_eq!((accepted, rejected), (18, 6));
}

#[cfg(feature = "jpeg2000")]
#[test]
fn arbitrary_tile_origins_match_oracles_and_limits() {
    use wml2::draw::image_from_with_limits;
    use wml2::limits::DecodeLimits;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!("{root}jpeg2000-origins-manifest.txt")).unwrap();
    let mut count = 0;
    let mut modes = std::collections::BTreeSet::new();
    let mut levels = std::collections::BTreeSet::new();
    let mut one_sample_edges = 0;
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        assert_eq!(f.len(), 14);
        let n = |i: usize| f[i].parse::<usize>().unwrap();
        let (w, h, c, tw, th) = (n(4), n(5), n(9), n(11), n(12));
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let source = std::fs::read(format!("{root}{}", f[3])).unwrap();
        assert_eq!(source.len(), w * h * c);
        let expected: Vec<_> = source
            .chunks_exact(c)
            .flat_map(|p| {
                if c == 1 {
                    [p[0], p[0], p[0], 255]
                } else {
                    [p[0], p[1], p[2], 255]
                }
            })
            .collect();
        let mut limits = DecodeLimits::unlimited();
        limits.pixels = w * h;
        limits.input_bytes = data.len();
        limits.expanded_bytes = w * h * c * 64 + 8192;
        limits.frames = 1;
        let image =
            image_from_with_limits(&data, limits).unwrap_or_else(|e| panic!("{}: {e}", f[1]));
        assert_eq!((image.width, image.height), (w, h));
        assert_eq!(image.buffer.unwrap(), expected, "{}", f[1]);
        for field in 0..4 {
            let mut short = limits;
            match field {
                0 => short.input_bytes -= 1,
                1 => short.pixels -= 1,
                2 => short.expanded_bytes -= 1,
                _ => short.frames = 0,
            }
            assert!(
                image_from_with_limits(&data, short).is_err(),
                "{} limit {field}",
                f[1]
            );
        }
        assert_eq!(n(13), w.div_ceil(tw) * h.div_ceil(th));
        modes.insert((c, n(10)));
        levels.insert(n(8));
        if w % tw == 1 || h % th == 1 {
            one_sample_edges += 1;
        }
        count += 1;
    }
    assert_eq!(count, 210);
    assert_eq!(modes, [(1, 0), (3, 0), (3, 1)].into_iter().collect());
    assert_eq!(levels, (0..=5).collect());
    assert!(one_sample_edges >= 60);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn unaligned_tile_corruption_stays_bounded_and_before_callbacks() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder, image_load};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!("{root}jpeg2000-origins-manifest.txt")).unwrap();
    let reject = |data: &[u8]| {
        let mut image = ImageBuffer::new();
        assert!(
            image_decoder(
                &mut BytesReader::new(data),
                &mut DecodeOptions {
                    drawer: &mut image,
                    debug_flag: 0,
                }
            )
            .is_err()
        );
        assert!(image.buffer.is_none());
    };
    let mut modes = std::collections::BTreeSet::new();
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        let levels = f[8].parse::<usize>().unwrap();
        let mode = (f[9], f[10]);
        if !f[1].ends_with(".j2k")
            || levels < 2
            || modes.contains(&mode)
            || f[13].parse::<usize>().unwrap() < 4
            || f[11].parse::<usize>().unwrap() % (1 << levels) == 0
        {
            continue;
        }
        modes.insert(mode);
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let mut p = 2;
        while data[p..p + 2] != [255, 0x90] {
            p += usize::from(u16::from_be_bytes([data[p + 2], data[p + 3]])) + 2;
        }
        let mut parts = Vec::new();
        while data[p..p + 2] == [255, 0x90] {
            let end = p + u32::from_be_bytes(data[p + 6..p + 10].try_into().unwrap()) as usize;
            parts.push((p, end));
            p = end;
        }
        for (start, end) in [parts[1], *parts.last().unwrap()] {
            for n in [start, start + 1, start + 12, start + 14, end - 1] {
                reject(&data[..n]);
            }
            for (offset, bytes) in [
                (2, vec![0, 9]),
                (4, vec![255, 255]),
                (10, vec![1]),
                (11, vec![2]),
                (12, vec![255, 0x52]),
                (6, vec![255, 255, 255, 255]),
            ] {
                let mut bad = data.clone();
                bad[start + offset..start + offset + bytes.len()].copy_from_slice(&bytes);
                reject(&bad);
            }
            let mut no_body = data[..start + 14].to_vec();
            no_body[start + 6..start + 10].copy_from_slice(&14u32.to_be_bytes());
            no_body.extend_from_slice(&data[end..]);
            reject(&no_body);
            for offset in start + 14..end.min(start + 46) {
                for mask in [1, 128, 255] {
                    let mut changed = data.clone();
                    changed[offset] ^= mask;
                    let _ = image_load(&changed);
                }
            }
        }
        // Oversized image/tile-grid coordinates remain invalid.
        for offset in [16, 20, 32, 36] {
            let mut bad = data.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
            reject(&bad);
        }
    }
    assert_eq!(modes.len(), 3);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn odd_negative_singleton_matches_independent_oracle() {
    let data =
        include_bytes!("fixtures/review/oracle_origins_22x11_d4_gray_b8x4_t20x22_singleton.j2k");
    let expected = include_bytes!(
        "fixtures/review/oracle_origins_22x11_d4_gray_b8x4_t20x22_singleton.expected.gray"
    );
    let image = wml2::draw::image_load(data).unwrap();
    assert_eq!((image.width, image.height), (22, 11));
    let rgba: Vec<_> = expected.iter().flat_map(|&v| [v, v, v, 255]).collect();
    assert_eq!(image.buffer.unwrap(), rgba);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn global_origin_oracles_and_cropped_limits() {
    use wml2::draw::image_from_with_limits;
    use wml2::limits::DecodeLimits;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest =
        std::fs::read_to_string(format!("{root}jpeg2000-global-origins-manifest.txt")).unwrap();
    let mut count = 0;
    let mut modes = std::collections::BTreeSet::new();
    let mut levels = std::collections::BTreeSet::new();
    let mut large_origin = false;
    let mut clipped_first = false;
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        assert_eq!(f.len(), 18);
        let n = |i: usize| f[i].parse::<usize>().unwrap();
        let (w, h, c, x0, y0, tx0, ty0) = (n(4), n(5), n(9), n(14), n(15), n(16), n(17));
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let source = std::fs::read(format!("{root}{}", f[3])).unwrap();
        assert_eq!(source.len(), w * h * c);
        let expected: Vec<_> = source
            .chunks_exact(c)
            .flat_map(|p| {
                if c == 1 {
                    [p[0], p[0], p[0], 255]
                } else {
                    [p[0], p[1], p[2], 255]
                }
            })
            .collect();
        let mut limits = DecodeLimits::unlimited();
        limits.input_bytes = data.len();
        limits.pixels = w * h;
        limits.expanded_bytes = w * h * c * 64 + 8192;
        limits.frames = 1;
        let image =
            image_from_with_limits(&data, limits).unwrap_or_else(|e| panic!("{}: {e}", f[1]));
        assert_eq!((image.width, image.height), (w, h), "{}", f[1]);
        assert_eq!(image.buffer.unwrap(), expected, "{}", f[1]);
        for field in 0..4 {
            let mut short = limits;
            match field {
                0 => short.input_bytes -= 1,
                1 => short.pixels -= 1,
                2 => short.expanded_bytes -= 1,
                _ => short.frames = 0,
            }
            assert!(
                image_from_with_limits(&data, short).is_err(),
                "{} limit{field}",
                f[1]
            );
        }
        assert_eq!(
            n(13),
            (x0 + w - tx0).div_ceil(n(11)) * (y0 + h - ty0).div_ceil(n(12))
        );
        large_origin |= x0 > 30000 || y0 > 30000;
        clipped_first |= x0 != tx0 || y0 != ty0;
        modes.insert((c, n(10)));
        levels.insert(n(8));
        count += 1;
    }
    assert_eq!(count, 126);
    assert!(large_origin && clipped_first);
    assert_eq!(modes, [(1, 0), (3, 0), (3, 1)].into_iter().collect());
    assert_eq!(levels, (0..=5).collect());
}

#[cfg(feature = "jpeg2000")]
#[test]
fn invalid_global_geometry_rejected_before_callbacks() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder, image_load};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest =
        std::fs::read_to_string(format!("{root}jpeg2000-global-origins-manifest.txt")).unwrap();
    let reject = |data: &[u8]| {
        let mut image = ImageBuffer::new();
        assert!(
            image_decoder(
                &mut BytesReader::new(data),
                &mut DecodeOptions {
                    drawer: &mut image,
                    debug_flag: 0
                }
            )
            .is_err()
        );
        assert!(image.buffer.is_none());
    };
    let mut modes = std::collections::BTreeSet::new();
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        let mode = (f[9], f[10]);
        if !f[1].ends_with(".j2k")
            || modes.contains(&mode)
            || f[8] == "0"
            || f[13].parse::<usize>().unwrap() < 4
        {
            continue;
        }
        modes.insert(mode);
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let read = |p: usize| u32::from_be_bytes(data[p..p + 4].try_into().unwrap());
        for p in [8, 12, 16, 20, 24, 28, 32, 36] {
            let mut huge = data.clone();
            huge[p..p + 4].copy_from_slice(&u32::MAX.to_be_bytes());
            reject(&huge);
        }
        for (extent, origin, grid, tile) in [(8, 16, 32, 24), (12, 20, 36, 28)] {
            for (p, value) in [
                (origin, read(extent)),
                (origin, read(extent) + 1),
                (grid, read(origin) + 1),
                (tile, 0),
                (extent, read(origin) + 1025),
                (extent, 32769),
            ] {
                let mut bad = data.clone();
                bad[p..p + 4].copy_from_slice(&value.to_be_bytes());
                reject(&bad);
            }
            // First nominal tile must intersect the image; equality fails too.
            let mut uncovered = data.clone();
            uncovered[origin..origin + 4].copy_from_slice(&(read(grid) + read(tile)).to_be_bytes());
            reject(&uncovered);
        }
        let mut p = 2;
        while data[p..p + 2] != [255, 0x90] {
            p += usize::from(u16::from_be_bytes([data[p + 2], data[p + 3]])) + 2;
        }
        let mut parts = Vec::new();
        while data[p..p + 2] == [255, 0x90] {
            let end = p + u32::from_be_bytes(data[p + 6..p + 10].try_into().unwrap()) as usize;
            parts.push((p, end));
            p = end;
        }
        for (start, end) in [parts[0], *parts.last().unwrap()] {
            for n in [start, start + 1, start + 12, start + 14, end - 1] {
                reject(&data[..n]);
            }
            for (offset, bytes) in [
                (4, vec![255, 255]),
                (6, vec![255, 255, 255, 255]),
                (10, vec![1]),
                (11, vec![2]),
                (12, vec![255, 0x52]),
            ] {
                let mut bad = data.clone();
                bad[start + offset..start + offset + bytes.len()].copy_from_slice(&bytes);
                reject(&bad);
            }
            let mut short = data[..start + 14].to_vec();
            short[start + 6..start + 10].copy_from_slice(&14u32.to_be_bytes());
            short.extend_from_slice(&data[end..]);
            reject(&short);
            for offset in start + 14..end.min(start + 38) {
                for mask in [1, 128, 255] {
                    let mut changed = data.clone();
                    changed[offset] ^= mask;
                    let _ = image_load(&changed);
                }
            }
        }
    }
    assert_eq!(modes.len(), 3);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn global_reference_extent_bound_rejects_valid_inputs_before_callbacks() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!(
        "{root}jpeg2000-global-origins-unsupported-manifest.txt"
    ))
    .unwrap();
    let mut count = 0;
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        assert_eq!(f.len(), 19);
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let mut image = ImageBuffer::new();
        let error = image_decoder(
            &mut BytesReader::new(&data),
            &mut DecodeOptions {
                drawer: &mut image,
                debug_flag: 0,
            },
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("reference extents"),
            "{}: {error}",
            f[1]
        );
        assert!(image.buffer.is_none());
        count += 1;
    }
    assert_eq!(count, 12);
}

#[cfg(feature = "jpeg2000")]
fn tile_part_oracles(manifest_name: &str, expected_count: usize, first_level: usize) {
    use wml2::draw::image_from_with_limits;
    use wml2::limits::DecodeLimits;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!("{root}{manifest_name}")).unwrap();
    let mut modes = std::collections::BTreeSet::new();
    let mut levels = std::collections::BTreeSet::new();
    let mut maximum_parts = 0;
    let mut count = 0;
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        assert_eq!(f.len(), 20);
        let n = |i: usize| f[i].parse::<usize>().unwrap();
        let (w, h, c, extra) = (n(4), n(5), n(9), n(19));
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let source = std::fs::read(format!("{root}{}", f[3])).unwrap();
        assert_eq!(source.len(), w * h * c);
        let expected: Vec<_> = source
            .chunks_exact(c)
            .flat_map(|p| {
                if c == 1 {
                    [p[0], p[0], p[0], 255]
                } else {
                    [p[0], p[1], p[2], 255]
                }
            })
            .collect();
        let mut limits = DecodeLimits::unlimited();
        limits.input_bytes = data.len();
        limits.pixels = w * h;
        limits.expanded_bytes = w * h * c * 64 + 8192 + extra;
        limits.frames = 1;
        let image =
            image_from_with_limits(&data, limits).unwrap_or_else(|e| panic!("{}: {e}", f[1]));
        assert_eq!((image.width, image.height), (w, h), "{}", f[1]);
        assert_eq!(image.buffer.unwrap(), expected, "{}", f[1]);
        for field in 0..4 {
            let mut short = limits;
            match field {
                0 => short.input_bytes -= 1,
                1 => short.pixels -= 1,
                2 => short.expanded_bytes -= 1,
                _ => short.frames = 0,
            }
            assert!(
                image_from_with_limits(&data, short).is_err(),
                "{} limit{field}",
                f[1]
            );
        }
        assert_eq!(extra > 0, n(18) > 1, "{}", f[1]);
        modes.insert((c, n(10)));
        levels.insert(n(8));
        maximum_parts = maximum_parts.max(n(18));
        count += 1;
    }
    assert_eq!(count, expected_count);
    assert_eq!(maximum_parts, 18);
    assert_eq!(modes, [(1, 0), (3, 0), (3, 1)].into_iter().collect());
    assert_eq!(levels, (first_level..=5).collect());
}

#[cfg(feature = "jpeg2000")]
#[test]
fn multiple_tile_parts_match_source_and_exact_workspace() {
    tile_part_oracles("jpeg2000-parts-manifest.txt", 126, 1);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn unknown_and_mixed_tile_counts_match_source_and_exact_workspace() {
    tile_part_oracles("jpeg2000-unknown-parts-manifest.txt", 156, 1);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn unsupported_and_malformed_tile_parts_rejected_before_callbacks() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder, image_load};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let reject = |data: &[u8]| {
        let mut image = ImageBuffer::new();
        assert!(
            image_decoder(
                &mut BytesReader::new(data),
                &mut DecodeOptions {
                    drawer: &mut image,
                    debug_flag: 0,
                }
            )
            .is_err()
        );
        assert!(image.buffer.is_none());
    };
    let manifest =
        std::fs::read_to_string(format!("{root}jpeg2000-parts-unsupported-manifest.txt")).unwrap();
    let mut reasons = std::collections::BTreeSet::new();
    let mut count = 0;
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        assert_eq!(f.len(), 21);
        if f[20] == "unsupported_unknown_TNsot" {
            // Promoted into the unknown-part positive manifest without rewriting
            // the original saved fixture or its source.
            continue;
        }
        reject(&std::fs::read(format!("{root}{}", f[1])).unwrap());
        reasons.insert(f[20]);
        count += 1;
    }
    assert!(count >= 12 && reasons.len() >= 4);
    let manifest = std::fs::read_to_string(format!("{root}jpeg2000-parts-manifest.txt")).unwrap();
    let mut modes = std::collections::BTreeSet::new();
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        let mode = (f[9], f[10]);
        if !f[1].ends_with(".j2k") || modes.contains(&mode) {
            continue;
        }
        modes.insert(mode);
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let mut p = 2;
        while data[p..p + 2] != [255, 0x90] {
            p += usize::from(u16::from_be_bytes([data[p + 2], data[p + 3]])) + 2;
        }
        let mut parts = Vec::new();
        while data[p..p + 2] == [255, 0x90] {
            let end = p + u32::from_be_bytes(data[p + 6..p + 10].try_into().unwrap()) as usize;
            parts.push((p, end));
            p = end;
        }
        for (start, end) in [parts[0], *parts.last().unwrap()] {
            for n in [start, start + 1, start + 12, start + 14, end - 1] {
                reject(&data[..n]);
            }
            for (offset, bytes) in [
                (4, vec![255, 255]),
                (6, vec![255, 255, 255, 255]),
                (10, vec![255]),
                (11, vec![19]),
                (12, vec![255, 0x52]),
            ] {
                let mut bad = data.clone();
                bad[start + offset..start + offset + bytes.len()].copy_from_slice(&bytes);
                reject(&bad);
            }
            let mut short = data[..start + 14].to_vec();
            short[start + 6..start + 10].copy_from_slice(&14u32.to_be_bytes());
            short.extend_from_slice(&data[end..]);
            reject(&short);
            for offset in start + 14..end.min(start + 38) {
                for mask in [1, 128, 255] {
                    let mut changed = data.clone();
                    changed[offset] ^= mask;
                    let _ = image_load(&changed);
                }
            }
        }
    }
    assert_eq!(modes.len(), 3);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn unknown_tile_count_corruption_rejected_before_callbacks() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!(
        "{root}jpeg2000-unknown-parts-unsupported-manifest.txt"
    ))
    .unwrap();
    let mut count = 0;
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        assert_eq!(f.len(), 21);
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let mut image = ImageBuffer::new();
        assert!(
            image_decoder(
                &mut BytesReader::new(&data),
                &mut DecodeOptions {
                    drawer: &mut image,
                    debug_flag: 0,
                }
            )
            .is_err(),
            "{}",
            f[1]
        );
        assert!(image.buffer.is_none(), "{}", f[1]);
        count += 1;
    }
    assert_eq!(count, 54);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn terminal_zero_length_parts_match_source_and_limits() {
    tile_part_oracles("jpeg2000-terminal-parts-manifest.txt", 54, 0);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn malformed_terminal_parts_reject_before_callbacks() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    let manifest = std::fs::read_to_string(format!(
        "{root}jpeg2000-terminal-parts-unsupported-manifest.txt"
    ))
    .unwrap();
    let mut count = 0;
    for line in manifest.lines().skip(1) {
        let f: Vec<_> = line.split_whitespace().collect();
        assert_eq!(f.len(), 21);
        let data = std::fs::read(format!("{root}{}", f[1])).unwrap();
        let mut image = ImageBuffer::new();
        assert!(
            image_decoder(
                &mut BytesReader::new(&data),
                &mut DecodeOptions {
                    drawer: &mut image,
                    debug_flag: 0,
                }
            )
            .is_err(),
            "{}",
            f[1]
        );
        assert!(image.buffer.is_none(), "{}", f[1]);
        count += 1;
    }
    assert_eq!(count, 18);
}

#[cfg(feature = "jpeg2000")]
#[test]
fn metadata_accumulates_across_comments_and_boxes() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder_with_limits, image_load};
    use wml2::limits::DecodeLimits;
    let raw = include_bytes!("fixtures/review/jp2_16x16_d1_noise.j2k");
    let mut p = 2;
    let mut used = 0;
    while raw[p..p + 2] != [255, 144] {
        let length = u16::from_be_bytes(raw[p + 2..p + 4].try_into().unwrap()) as usize;
        if raw[p..p + 2] == [255, 100] {
            used += length - 2;
        }
        p += length + 2;
    }
    let mut comments = raw.to_vec();
    let comment = [255, 100, 0, 9, 0, 1, b'A', b'B', b'C', b'D', b'E'];
    comments.splice(p..p, comment.into_iter().chain(comment));
    let mut boxes = include_bytes!("fixtures/review/jp2_16x16_d1_noise.jp2").to_vec();
    let mut box_start = 12;
    while &boxes[box_start + 4..box_start + 8] != b"jp2c" {
        box_start +=
            u32::from_be_bytes(boxes[box_start..box_start + 4].try_into().unwrap()) as usize;
    }
    let mut sot = box_start + 10;
    while boxes[sot..sot + 2] != [255, 144] {
        sot += usize::from(u16::from_be_bytes(
            boxes[sot + 2..sot + 4].try_into().unwrap(),
        )) + 2;
    }
    boxes[sot + 6..sot + 10].fill(0);
    // Psot=0 ends at the codestream box's EOC, even with trailing free boxes.
    let free = [0, 0, 0, 13, b'f', b'r', b'e', b'e', 1, 2, 3, 4, 5];
    boxes.extend_from_slice(&free);
    boxes.extend_from_slice(&free);
    let expected = image_load(raw).unwrap().buffer;
    for (data, extra) in [(comments, 14), (boxes, 10)] {
        for short in [false, true] {
            let limits = DecodeLimits {
                metadata_bytes: used + extra - usize::from(short),
                ..DecodeLimits::default()
            };
            let mut image = ImageBuffer::new();
            let result = image_decoder_with_limits(
                &mut BytesReader::new(&data),
                &mut DecodeOptions {
                    drawer: &mut image,
                    debug_flag: 0,
                },
                limits,
            );
            assert_eq!(result.is_err(), short);
            if short {
                assert!(image.buffer.is_none());
            } else {
                assert_eq!(image.buffer, expected);
            }
        }
    }
}

#[cfg(feature = "jpeg2000")]
#[test]
fn public_decoder_applies_default_metadata_scope() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_load};
    let raw = include_bytes!("fixtures/review/jp2_16x16_d1_noise.j2k");
    let mut p = 2;
    while raw[p..p + 2] != [255, 144] {
        p += usize::from(u16::from_be_bytes(raw[p + 2..p + 4].try_into().unwrap())) + 2;
    }
    let mut oversized = raw[..p].to_vec();
    // Every COM fits its u16 length and the default metadata budget individually;
    // together they exceed 32MiB. This failed through the public direct API.
    let mut comment = vec![0; 65537];
    comment[..6].copy_from_slice(&[255, 100, 255, 255, 0, 1]);
    for _ in 0..513 {
        oversized.extend_from_slice(&comment);
    }
    oversized.extend_from_slice(&raw[p..]);
    assert!(image_load(&oversized).is_err());
    let mut image = ImageBuffer::new();
    let error = wml2::jpeg2000::decode(
        &mut BytesReader::new(&oversized),
        &mut DecodeOptions {
            drawer: &mut image,
            debug_flag: 0,
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("metadata exceeds decode limit"));
    assert!(image.buffer.is_none());
    // The failed scope must not poison a later direct decode.
    wml2::jpeg2000::decode(
        &mut BytesReader::new(raw),
        &mut DecodeOptions {
            drawer: &mut image,
            debug_flag: 0,
        },
    )
    .unwrap();
    assert_eq!(image.buffer, image_load(raw).unwrap().buffer);
}

#[cfg(feature = "jpeg2000")]
mod irreversible_tests {
    use bin_rs::reader::BytesReader;
    use wml2::draw::*;
    use wml2::limits::DecodeLimits;
    use wml2::metadata::DataMap;
    type Error = Box<dyn std::error::Error>;
    const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/review/");
    fn pixels(expected: &[u8], actual: &[u8], components: usize, tolerance: u8) -> bool {
        actual.len() == expected.len() / components * 4
            && actual
                .chunks_exact(4)
                .zip(expected.chunks_exact(components))
                .all(|(a, e)| {
                    a[3] == 255
                        && (0..3).all(|c| {
                            a[c].abs_diff(e[if components == 1 { 0 } else { c }]) <= tolerance
                        })
                })
    }
    struct Probe<'a> {
        expected: &'a [u8],
        w: usize,
        h: usize,
        components: usize,
        tolerance: u8,
        abort: u8,
        events: Vec<u8>,
    }
    impl DrawCallback for Probe<'_> {
        fn init(
            &mut self,
            w: usize,
            h: usize,
            _: Option<InitOptions>,
        ) -> Result<Option<CallbackResponse>, Error> {
            assert_eq!((w, h), (self.w, self.h));
            self.events.push(1);
            Ok((self.abort == 1).then(CallbackResponse::abort))
        }
        fn draw(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            a: &[u8],
            _: Option<DrawOptions>,
        ) -> Result<Option<CallbackResponse>, Error> {
            assert_eq!((x, y, w, h), (0, 0, self.w, self.h));
            assert!(pixels(self.expected, a, self.components, self.tolerance));
            self.events.push(2);
            Ok((self.abort == 2).then(CallbackResponse::abort))
        }
        fn terminate(
            &mut self,
            _: Option<TerminateOptions>,
        ) -> Result<Option<CallbackResponse>, Error> {
            self.events.push(3);
            Ok(None)
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
    #[test]
    fn fixed_oracle_pixels_limits_and_callbacks() {
        let mut text =
            std::fs::read_to_string(format!("{ROOT}jpeg2000-irreversible-manifest.txt")).unwrap();
        let variants =
            std::fs::read_to_string(format!("{ROOT}jpeg2000-irreversible-variants-manifest.txt"))
                .unwrap();
        text.extend(variants.lines().skip(1).flat_map(|line| [line, "\n"]));
        let mut modes = std::collections::BTreeSet::new();
        let mut levels = std::collections::BTreeSet::new();
        let mut count = 0;
        for line in text.lines().skip(1) {
            let f: Vec<_> = line.split_whitespace().collect();
            assert_eq!(f.len(), 23);
            let n = |i: usize| f[i].parse::<usize>().unwrap();
            let (w, h, c) = (n(4), n(5), n(9));
            let tolerance = n(22) as u8;
            assert_eq!(tolerance, 1);
            let data = std::fs::read(format!("{ROOT}{}", f[1])).unwrap();
            let expected = std::fs::read(format!("{ROOT}{}", f[21])).unwrap();
            assert_eq!(expected.len(), w * h * c);
            let image = image_load(&data).unwrap_or_else(|e| panic!("{}: {e}", f[1]));
            assert_eq!((image.width, image.height), (w, h));
            assert!(
                pixels(&expected, image.buffer.as_ref().unwrap(), c, tolerance),
                "{}",
                f[1]
            );
            let limit = DecodeLimits {
                input_bytes: data.len(),
                pixels: w * h,
                expanded_bytes: w * h * c * 64 + 8192 + n(19),
                frames: 1,
                metadata_bytes: 64,
                ..DecodeLimits::unlimited()
            };
            let limited = image_from_with_limits(&data, limit)
                .unwrap_or_else(|e| panic!("{} limit: {e}", f[1]));
            assert!(pixels(
                &expected,
                limited.buffer.as_ref().unwrap(),
                c,
                tolerance
            ));
            for field in 0..5 {
                let mut short = limit;
                match field {
                    0 => short.input_bytes -= 1,
                    1 => short.pixels -= 1,
                    2 => short.expanded_bytes -= 1,
                    3 => short.frames = 0,
                    _ => short.metadata_bytes = 0,
                }
                let mut image = ImageBuffer::new();
                assert!(
                    image_decoder_with_limits(
                        &mut BytesReader::new(&data),
                        &mut DecodeOptions {
                            drawer: &mut image,
                            debug_flag: 0
                        },
                        short
                    )
                    .is_err(),
                    "{} limit{field}",
                    f[1]
                );
                assert!(image.buffer.is_none());
            }
            for abort in 0..=2 {
                let mut probe = Probe {
                    expected: &expected,
                    w,
                    h,
                    components: c,
                    tolerance,
                    abort,
                    events: Vec::new(),
                };
                image_decoder(
                    &mut BytesReader::new(&data),
                    &mut DecodeOptions {
                        drawer: &mut probe,
                        debug_flag: 0,
                    },
                )
                .unwrap();
                assert_eq!(
                    probe.events,
                    if abort == 1 {
                        vec![1, 3]
                    } else {
                        vec![1, 2, 3]
                    },
                    "{}",
                    f[1]
                );
            }
            let mut image = ImageBuffer::new();
            assert!(
                image_decoder(
                    &mut BytesReader::new(&data[..data.len() - 1]),
                    &mut DecodeOptions {
                        drawer: &mut image,
                        debug_flag: 0
                    }
                )
                .is_err()
            );
            assert!(image.buffer.is_none());
            assert!(image_load(&data).is_ok()); // finite/error scopes restored
            modes.insert((c, n(10)));
            levels.insert(n(8));
            count += 1;
        }
        assert_eq!(count, 256);
        assert_eq!(modes.len(), 3);
        assert_eq!(levels.len(), 6);
    }
    #[test]
    fn malformed_and_unsupported_profiles_reject_before_init() {
        let text = std::fs::read_to_string(format!(
            "{ROOT}jpeg2000-irreversible-unsupported-manifest.txt"
        ))
        .unwrap();
        let mut count = 0;
        for line in text.lines().skip(1) {
            let f: Vec<_> = line.split_whitespace().collect();
            assert_eq!(f.len(), 4);
            let data = std::fs::read(format!("{ROOT}{}", f[1])).unwrap();
            let mut image = ImageBuffer::new();
            assert!(
                image_decoder(
                    &mut BytesReader::new(&data),
                    &mut DecodeOptions {
                        drawer: &mut image,
                        debug_flag: 0
                    }
                )
                .is_err(),
                "{} {}",
                f[1],
                f[3]
            );
            assert!(image.buffer.is_none(), "{}", f[1]);
            count += 1;
        }
        assert!(count >= 32);
    }
}
