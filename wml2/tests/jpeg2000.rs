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
            (cod + 9, 2),
            (cod + 10, 3),
            (cod + 11, 3),
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
            assert!(
                b.chunks_exact(4)
                    .all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255)
            );
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
