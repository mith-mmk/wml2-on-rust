#![cfg(feature = "image-buffer-ex")]

// Compile and execute the exact documented example; no copied sample logic.
#[allow(dead_code)]
#[path = "../examples/ex_callback_contracts.rs"]
mod example;
use example::contracts::{BufferedIccCallback, LegacyOnly, source, words};
use wml2::draw::*;
use wml2::metadata::DataMap;

#[test]
fn documented_five_callback_scenarios_execute() {
    example::contracts::run().unwrap();
}

#[test]
fn rgba16_to_twelve_exact_success_then_late_failure_preserves_entire_canvas() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Exact);
    image.init_ex(2, 2, source(PixelFormatEx::Rgba16)).unwrap();
    image
        .draw(0, 0, 1, 1, &words(&[4369, 21845, 65535, 34952]), None)
        .unwrap();
    image
        .draw(1, 1, 1, 1, &words(&[21845, 4369, 0, 65535]), None)
        .unwrap();
    assert_eq!(
        image.pixel_bytes(0, 0).unwrap(),
        words(&[273, 1365, 4095, 2184])
    );
    assert_eq!(
        image.pixel_bytes(1, 1).unwrap(),
        words(&[1365, 273, 0, 4095])
    );
    let before = image.bytes().unwrap().to_vec();
    for channel in 0..4 {
        let mut values = [65535; 16]; // first pixel would change before late failure
        values[12 + channel] = 1;
        for x in [0, 1, 3] {
            // last bad pixel visible, clipped, or entirely outside
            let error = image.draw(x, 0, 2, 2, &words(&values), None).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("precision conversion would lose information")
            );
            assert_eq!(image.bytes().unwrap(), before);
        }
    }
    assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba16));
    assert_eq!(image.format(), PixelFormatEx::Rgba12);
}

#[test]
fn rgba16_to_twelve_round_scales_alpha_and_uses_declared_byte_stride() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Round);
    image
        .init_ex(
            1,
            2,
            Some(InitOptionsEx {
                source_format: PixelFormatEx::Rgba16,
                source_stride: Some(9), // bytes, including final row padding; odd is legal
                legacy: None,
            }),
        )
        .unwrap();
    let mut padded = words(&[1, 32768, 65535, 32768]);
    padded.push(0xee);
    padded.extend(words(&[65535, 17, 0, 0])); // hidden color retained with alpha0
    padded.push(0xff);
    image.draw(0, 0, 1, 2, &padded, None).unwrap();
    assert_eq!(
        image.bytes().unwrap(),
        words(&[0, 2048, 4095, 2048, 4095, 1, 0, 0])
    );
    assert_eq!(image.source_stride(), Some(9));
    assert_eq!(image.stride(), 8);
    assert!(
        image
            .bytes()
            .unwrap()
            .chunks_exact(2)
            .all(|v| v[1] & 0xf0 == 0)
    );
    let before = image.bytes().unwrap().to_vec();
    assert!(image.draw(0, 0, 1, 2, &padded[..17], None).is_err());
    assert_eq!(image.bytes().unwrap(), before);
}

#[test]
fn failed_imagebufferex_initialization_invalidates_old_pixels_not_just_new_draw() {
    for legacy in [false, true] {
        let mut image =
            ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Round);
        image.init_ex(1, 1, source(PixelFormatEx::Rgba12)).unwrap();
        image
            .draw(0, 0, 1, 1, &words(&[1, 2, 3, 4095]), None)
            .unwrap();
        let failed = if legacy {
            image.init(0, 1, None)
        } else {
            image.init_ex(0, 1, source(PixelFormatEx::Rgba16))
        };
        assert!(failed.is_err());
        assert!(image.bytes().is_none());
        assert_eq!(image.source_format(), None);
        assert!(image.pixel_bytes(0, 0).is_err());
        assert!(image.encode_start_ex(None).is_err());
        assert!(image.draw(0, 0, 1, 1, &words(&[0; 4]), None).is_err());
        image.init_ex(1, 1, source(PixelFormatEx::Rgba12)).unwrap();
        assert_eq!(image.bytes().unwrap(), &[0; 8]);
    }
}

#[test]
fn legacy_default_rejects_before_legacy_init_and_preserves_old_target() {
    let mut old = LegacyOnly::default();
    old.init(1, 1, None).unwrap();
    old.draw(0, 0, 1, 1, &[1, 2, 3, 255], None).unwrap();
    for format in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Rgba16,
    ] {
        assert!(old.init_ex(2, 2, source(format)).is_err());
        assert_eq!((old.0.width, old.0.height), (1, 1));
        assert_eq!(old.0.buffer.as_deref().unwrap(), &[1, 2, 3, 255]);
    }
    assert!(
        old.init_ex(
            2,
            2,
            Some(InitOptionsEx {
                source_stride: Some(8),
                ..Default::default()
            })
        )
        .is_err()
    );
    assert_eq!(old.0.buffer.as_deref().unwrap(), &[1, 2, 3, 255]);
}

#[test]
fn buffered_metadata_survives_preinit_and_late_delivery_without_transform() {
    let mut callback = BufferedIccCallback::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    callback
        .set_metadata("before-init", DataMap::UInt(1))
        .unwrap();
    callback
        .init_ex(1, 1, source(PixelFormatEx::Rgba16))
        .unwrap();
    callback
        .draw(0, 0, 1, 1, &words(&[1, 1234, 65535, 0]), None)
        .unwrap();
    callback
        .set_metadata("Source ICC Profile", DataMap::ICCProfile(vec![1, 2, 3]))
        .unwrap();
    callback.terminate(None).unwrap();
    callback
        .after_decode(Ok(None), |pixels, tags| {
            assert_eq!(pixels.bytes().unwrap(), words(&[1, 1234, 65535, 0]));
            assert_eq!(tags["before-init"], DataMap::UInt(1));
            assert_eq!(
                tags["Source ICC Profile"],
                DataMap::ICCProfile(vec![1, 2, 3])
            );
            // Source annotations are separate; no accidental output-profile label.
            assert!(pixels.metadata.is_none());
            Ok(())
        })
        .unwrap();
}

#[test]
fn buffered_external_stage_never_runs_on_error_warning_or_missing_completion() {
    #[derive(Debug)]
    struct Warning;
    impl std::fmt::Display for Warning {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("test warning")
        }
    }
    impl wml2::warning::ImgWarning for Warning {}
    for case in 0..3 {
        let mut callback =
            BufferedIccCallback::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
        callback.init(1, 1, None).unwrap();
        callback.draw(0, 0, 1, 1, &[1, 2, 3, 255], None).unwrap();
        if case != 2 {
            callback.terminate(None).unwrap();
        }
        let status = match case {
            0 => Err("decode failed after draw".into()),
            1 => Ok(wml2::warning::ImgWarnings::add(None, Box::new(Warning))),
            _ => Ok(None),
        };
        let mut called = false;
        assert!(
            callback
                .after_decode(status, |_, _| {
                    called = true;
                    Ok(())
                })
                .is_err()
        );
        assert!(!called);
    }
}

#[cfg(any(feature = "png", feature = "tiff"))]
mod decoder_metadata_order {
    use super::*;
    use bin_rs::reader::BytesReader;
    const ICC: &[u8] = include_bytes!("fixtures/review/linear-rgb.icc");

    // Observe actual decoder callbacks, not the physical position of file tags.
    struct Probe {
        inner: BufferedIccCallback,
        events: Vec<&'static str>,
    }
    impl DrawCallback for Probe {
        fn init(&mut self, w: usize, h: usize, o: Option<InitOptions>) -> Response {
            self.inner.init(w, h, o)
        }
        fn init_ex(&mut self, w: usize, h: usize, o: Option<InitOptionsEx>) -> Response {
            self.events.push("init");
            self.inner.init_ex(w, h, o)
        }
        fn draw(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            bytes: &[u8],
            o: Option<DrawOptions>,
        ) -> Response {
            self.events.push("draw");
            self.inner.draw(x, y, w, h, bytes, o)
        }
        fn set_metadata(&mut self, key: &str, value: DataMap) -> Response {
            if key == "Source ICC Profile" {
                self.events.push("icc");
            }
            self.inner.set_metadata(key, value)
        }
        fn terminate(&mut self, o: Option<TerminateOptions>) -> Response {
            self.events.push("terminate");
            self.inner.terminate(o)
        }
        fn next(&mut self, o: Option<NextOptions>) -> Response {
            self.inner.next(o)
        }
        fn verbose(&mut self, text: &str, o: Option<VerboseOptions>) -> Response {
            self.inner.verbose(text, o)
        }
    }

    fn check(input: &[u8]) {
        let mut probe = Probe {
            inner: BufferedIccCallback::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact),
            events: Vec::new(),
        };
        let status = image_decoder_ex(
            &mut BytesReader::new(input),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut probe,
            },
        );
        assert_eq!(probe.events, ["init", "draw", "icc", "terminate"]);
        probe
            .inner
            .after_decode(status, |pixels, tags| {
                assert_eq!(pixels.bytes().unwrap(), words(&[257, 514, 771, 65535]));
                assert_eq!(
                    tags["Source ICC Profile"],
                    DataMap::ICCProfile(ICC.to_vec())
                );
                assert!(!tags.contains_key("ICC Profile"));
                Ok(())
            })
            .unwrap();
    }

    #[cfg(feature = "png")]
    #[test]
    fn png_icc_before_idat_is_still_delivered_after_draw() {
        use wml2::png::utils::CRC32;
        fn chunk(out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
            out.extend((data.len() as u32).to_be_bytes());
            out.extend(name);
            out.extend(data);
            out.extend(
                CRC32::new()
                    .crc32(&[name.as_slice(), data].concat())
                    .to_be_bytes(),
            );
        }
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        chunk(&mut png, b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
        let mut iccp = b"rgb\0\0".to_vec();
        iccp.extend(miniz_oxide::deflate::compress_to_vec_zlib(ICC, 6));
        chunk(&mut png, b"iCCP", &iccp);
        chunk(
            &mut png,
            b"IDAT",
            &miniz_oxide::deflate::compress_to_vec_zlib(&[0, 1, 2, 3, 255], 6),
        );
        chunk(&mut png, b"IEND", &[]);
        check(&png);
    }

    #[cfg(feature = "tiff")]
    #[test]
    fn tiff_icc_in_header_is_delivered_after_draw() {
        let mut image = ImageBuffer::from_buffer(1, 1, vec![1, 2, 3, 255]);
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(ICC.to_vec()))
            .unwrap();
        let tiff = image_to(&mut image, wml2::util::ImageFormat::Tiff, None).unwrap();
        check(&tiff);
    }
}
