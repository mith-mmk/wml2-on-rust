#![cfg(feature = "png")]

use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;

const ICC: &[u8] = b"opaque source profile retained without CMS";

fn chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let mut crc = !0u32;
    for &byte in kind.iter().chain(data) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    png.extend_from_slice(&(!crc).to_be_bytes());
}

fn png(icc: bool, gray: bool) -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = [0; 13];
    ihdr[3] = 1;
    ihdr[7] = 1;
    ihdr[8] = 8;
    ihdr[9] = if gray { 0 } else { 2 };
    chunk(&mut png, b"IHDR", &ihdr);
    if icc {
        let mut payload = b"test\0\0".to_vec();
        payload.extend(miniz_oxide::deflate::compress_to_vec_zlib(ICC, 6));
        chunk(&mut png, b"iCCP", &payload);
    }
    // EX PNG emits this metadata before initialization.
    chunk(
        &mut png,
        b"bKGD",
        if gray { &[0, 7] } else { &[0, 7, 0, 7, 0, 7] },
    );
    let samples: &[u8] = if gray { &[0, 123] } else { &[0, 12, 34, 56] };
    chunk(
        &mut png,
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(samples, 6),
    );
    chunk(&mut png, b"IEND", &[]);
    png
}

fn decode(bytes: &[u8], image: &mut dyn DrawCallback) {
    image_decoder(
        &mut BytesReader::new(bytes),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: image,
        },
    )
    .unwrap();
}

fn no_old_tags(metadata: &Option<std::collections::HashMap<String, DataMap>>) {
    let metadata = metadata.as_ref().unwrap();
    for key in ["ICC Profile", "Source ICC Profile", "old custom tag"] {
        assert!(!metadata.contains_key(key), "stale {key}");
    }
}

#[test]
fn ordinary_reuse_clears_profiles_and_other_image_metadata() {
    let mut image = ImageBuffer::new();
    for gray in [false, true] {
        decode(&png(true, gray), &mut image);
        assert!(
            image
                .metadata
                .as_ref()
                .unwrap()
                .values()
                .any(|v| matches!(v, DataMap::ICCProfile(b) if b == ICC))
        );
        image
            .set_metadata("old custom tag", DataMap::Ascii("old".into()))
            .unwrap();
        decode(&png(false, false), &mut image);
        no_old_tags(&image.metadata);
        assert_eq!(image.buffer.as_deref().unwrap(), &[12, 34, 56, 255]);
    }
}

#[test]
fn failed_new_decode_clears_metadata_before_input_validation() {
    let mut image = ImageBuffer::new();
    for limited in [false, true] {
        decode(&png(true, false), &mut image);
        let result = image_decoder_with_limits(
            &mut BytesReader::new(if limited { b"too large" } else { b"" }),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
            DecodeLimits {
                input_bytes: if limited { 1 } else { usize::MAX },
                ..DecodeLimits::default()
            },
        );
        assert!(result.is_err());
        assert!(image.metadata.is_none());
    }
}

#[cfg(feature = "image-buffer-ex")]
#[test]
fn ex_reuse_retains_new_pre_init_metadata_and_storage_configuration() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    for gray in [true, false, true] {
        image_decoder_ex(
            &mut BytesReader::new(&png(true, gray)),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
        )
        .unwrap();
        assert!(
            image
                .metadata
                .as_ref()
                .unwrap()
                .contains_key("PNG Background")
        );
        assert!(
            image
                .metadata
                .as_ref()
                .unwrap()
                .contains_key("Source ICC Profile")
        );
        assert_eq!(image.format(), PixelFormatEx::Rgba16);
        image
            .set_metadata("old custom tag", DataMap::Ascii("old".into()))
            .unwrap();
        image_decoder_ex(
            &mut BytesReader::new(&png(false, false)),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
        )
        .unwrap();
        no_old_tags(&image.metadata);
        assert!(
            image
                .metadata
                .as_ref()
                .unwrap()
                .contains_key("PNG Background")
        );
        assert_eq!(image.bytes().unwrap(), &[12, 12, 34, 34, 56, 56, 255, 255]);
    }
    assert!(
        image_decoder_ex(
            &mut BytesReader::new(b""),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image
            }
        )
        .is_err()
    );
    assert!(image.metadata.is_none());
    assert!(image.bytes().is_none());
}

struct Probe {
    events: Vec<&'static str>,
    abort: bool,
    fail: bool,
    image: ImageBuffer,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            events: vec![],
            abort: false,
            fail: false,
            image: ImageBuffer::new(),
        }
    }
}
impl DrawCallback for Probe {
    fn begin_image(&mut self) -> Response {
        self.events.push("begin");
        if self.fail {
            return Err(std::io::Error::other("begin failure").into());
        }
        self.image.begin_image()?;
        Ok(self.abort.then(CallbackResponse::abort))
    }
    fn init(&mut self, w: usize, h: usize, option: Option<InitOptions>) -> Response {
        self.events.push("init");
        self.image.init(w, h, option)
    }
    fn draw(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        data: &[u8],
        option: Option<DrawOptions>,
    ) -> Response {
        self.events.push("draw");
        self.image.draw(x, y, w, h, data, option)
    }
    fn next(&mut self, option: Option<NextOptions>) -> Response {
        self.events.push("next");
        assert!(
            self.image
                .metadata
                .as_ref()
                .unwrap()
                .contains_key("animation global")
        );
        self.image.next(option)
    }
    fn terminate(&mut self, option: Option<TerminateOptions>) -> Response {
        self.events.push("terminate");
        self.image.terminate(option)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        self.events.push("verbose");
        Ok(None)
    }
    fn set_metadata(&mut self, key: &str, value: DataMap) -> Response {
        self.events.push("metadata");
        self.image.set_metadata(key, value)
    }
}

#[test]
fn begin_abort_and_error_stop_before_all_other_callbacks() {
    for bytes in [png(false, false), b"unsupported input".to_vec()] {
        for fail in [false, true] {
            let mut probe = Probe {
                abort: true,
                fail,
                ..Default::default()
            };
            let result = image_decoder(
                &mut BytesReader::new(&bytes),
                &mut DecodeOptions {
                    debug_flag: 1,
                    drawer: &mut probe,
                },
            );
            if fail {
                assert!(result.unwrap_err().to_string().contains("begin failure"));
                assert_eq!(probe.events, ["begin"]);
            } else {
                assert!(result.is_ok());
                assert_eq!(probe.events, ["begin", "terminate"]);
            }
        }
    }
}

#[cfg(feature = "gif")]
#[test]
fn animation_next_does_not_restart_metadata_lifecycle() {
    let mut bytes = b"GIF89a\x01\0\x01\0\x80\0\0\xff\0\0\0\0\xff".to_vec();
    for index in [0, 1, 0] {
        bytes.extend_from_slice(b"!\xf9\x04\0\x02\0\0\0,\0\0\0\0\x01\0\x01\0\0\x02\x02");
        bytes.extend_from_slice(&[0x44 | (index << 3), 1, 0]);
    }
    bytes.push(b';');
    // Inject a global marker once at init to check each actual frame boundary.
    struct Animation(Probe);
    impl DrawCallback for Animation {
        fn begin_image(&mut self) -> Response {
            self.0.begin_image()
        }
        fn init(&mut self, w: usize, h: usize, o: Option<InitOptions>) -> Response {
            self.0
                .image
                .set_metadata("animation global", DataMap::Ascii("retained".into()))?;
            self.0.init(w, h, o)
        }
        fn draw(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            d: &[u8],
            o: Option<DrawOptions>,
        ) -> Response {
            self.0.draw(x, y, w, h, d, o)
        }
        fn next(&mut self, o: Option<NextOptions>) -> Response {
            self.0.next(o)
        }
        fn terminate(&mut self, o: Option<TerminateOptions>) -> Response {
            self.0.terminate(o)
        }
        fn verbose(&mut self, t: &str, o: Option<VerboseOptions>) -> Response {
            self.0.verbose(t, o)
        }
        fn set_metadata(&mut self, k: &str, v: DataMap) -> Response {
            self.0.set_metadata(k, v)
        }
    }
    let mut probe = Animation(Probe::default());
    decode(&bytes, &mut probe);
    assert_eq!(probe.0.events.iter().filter(|&&e| e == "begin").count(), 1);
    assert_eq!(probe.0.events.iter().filter(|&&e| e == "next").count(), 3);
    assert_eq!(probe.0.image.animation.as_ref().unwrap().len(), 3);
    assert_eq!(
        probe
            .0
            .image
            .animation
            .as_ref()
            .unwrap()
            .iter()
            .map(|f| f.buffer.as_slice())
            .collect::<Vec<_>>(),
        vec![
            &[255, 0, 0, 255][..],
            &[0, 0, 255, 255][..],
            &[255, 0, 0, 255][..]
        ]
    );
    assert!(
        probe
            .0
            .image
            .metadata
            .as_ref()
            .unwrap()
            .contains_key("animation global")
    );
}

#[cfg(feature = "ico-png")]
#[test]
fn embedded_png_guard_does_not_restart_outer_image() {
    let png = png(true, false);
    let mut ico = vec![0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 32, 0];
    ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&png);
    let mut probe = Probe::default();
    decode(&ico, &mut probe);
    assert_eq!(probe.events.iter().filter(|&&e| e == "begin").count(), 1);
    assert!(
        probe
            .image
            .metadata
            .as_ref()
            .unwrap()
            .contains_key("ICC Profile")
    );
}
