#![cfg(any(feature = "png", feature = "jpeg", feature = "gif", feature = "bmp"))]
use wml2::draw::*;
use wml2::metadata::DataMap;
#[cfg(any(feature = "png", feature = "jpeg", feature = "gif"))]
use wml2::util::ImageFormat;
struct Probe {
    target: &'static str,
    aborted: bool,
    draws: usize,
    nexts: usize,
    terms: usize,
    allow_stopped_metadata: bool,
    term_error: bool,
}
impl Probe {
    fn event(&mut self, name: &str) -> Response {
        assert!(
            !self.aborted || (self.allow_stopped_metadata && name == "metadata"),
            "callback {name} after Abort"
        );
        if name == self.target {
            self.aborted = true;
            Ok(Some(CallbackResponse::abort()))
        } else {
            Ok(Some(CallbackResponse::cont()))
        }
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
        self.draws += 1;
        self.event("draw")
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        self.nexts += 1;
        self.event("next")
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.terms += 1;
        if self.term_error {
            Err(std::io::Error::other("deliberate termination failure").into())
        } else {
            Ok(None)
        }
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        self.event("verbose")
    }
    fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
        self.event("metadata")
    }
}
#[cfg(any(feature = "png", feature = "jpeg", feature = "gif"))]
fn exercise(format: ImageFormat, animation: bool) {
    let mut image = ImageBuffer::new();
    image
        .init(
            64,
            64,
            Some(InitOptions {
                background: None,
                animation,
                loop_count: 0,
            }),
        )
        .unwrap();
    if animation {
        for _ in 0..3 {
            image.next(Some(NextOptions::wait(10))).unwrap();
            image.draw(0, 0, 64, 64, &[42; 64 * 64 * 4], None).unwrap();
        }
    }
    let bytes = image_to(&mut image, format, None).unwrap();
    for target in ["init", "draw", "metadata", "verbose", "next"] {
        if target == "next" && !animation {
            continue;
        }
        let mut probe = Probe {
            target,
            aborted: false,
            draws: 0,
            nexts: 0,
            terms: 0,
            allow_stopped_metadata: false,
            term_error: false,
        };
        image_loader(
            &bytes,
            &mut DecodeOptions {
                debug_flag: 1,
                drawer: &mut probe,
            },
        )
        .unwrap();
        assert!(probe.aborted, "did not reach {target}");
        assert_eq!(probe.terms, 1);
        if target == "init" {
            assert_eq!(probe.draws, 0);
        }
        if target == "draw" {
            assert_eq!(probe.draws, 1);
        }
    }
    // Exercise cancellation repeatedly with enough MCUs to fill bounded queues.
    for _ in 0..16 {
        let mut probe = Probe {
            target: "draw",
            aborted: false,
            draws: 0,
            nexts: 0,
            terms: 0,
            allow_stopped_metadata: false,
            term_error: false,
        };
        image_loader(
            &bytes,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut probe,
            },
        )
        .unwrap();
        assert_eq!((probe.draws, probe.terms), (1, 1));
    }
    for length in [bytes.len() / 2, bytes.len() * 3 / 4, bytes.len() - 2] {
        let mut image = ImageBuffer::new();
        let _ = image_loader(
            &bytes[..length],
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
        );
    }
}
#[cfg(feature = "png")]
#[test]
fn png_abort_stops_at_every_callback() {
    exercise(ImageFormat::Png, true);
}
#[cfg(feature = "jpeg")]
#[test]
fn jpeg_abort_stops_and_joins_workers() {
    exercise(ImageFormat::Jpeg, false);
}
#[cfg(feature = "gif")]
#[test]
fn gif_abort_stops_at_every_callback() {
    exercise(ImageFormat::Gif, true);
}

#[cfg(feature = "bmp")]
#[test]
fn bmp_draw_abort_is_successful_with_and_without_explicit_limits() {
    use bin_rs::reader::BytesReader;
    use wml2::limits::DecodeLimits;

    for height in [1u32, 2, 8] {
        // Independent 24-bit, bottom-up BMP: two red pixels per padded row.
        let mut bytes = b"BM".to_vec();
        bytes.extend_from_slice(&(54 + 8 * height).to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        for value in [54u32, 40, 2, height] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[1, 0, 24, 0]);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&(8 * height).to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        for _ in 0..height {
            bytes.extend_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0]);
        }
        let image = image_from(&bytes).unwrap();
        assert_eq!(
            image.buffer.unwrap(),
            [255, 0, 0, 255].repeat(2 * height as usize)
        );
        for explicit_limits in [false, true] {
            for debug_flag in [0, 1] {
                for term_error in [false, true] {
                    let mut probe = Probe {
                        target: "draw",
                        aborted: false,
                        draws: 0,
                        nexts: 0,
                        terms: 0,
                        // BMP historically delivers metadata after its pixel path,
                        // including cancellation. Retain that legacy behavior.
                        allow_stopped_metadata: true,
                        term_error,
                    };
                    let mut options = DecodeOptions {
                        debug_flag,
                        drawer: &mut probe,
                    };
                    let result = if explicit_limits {
                        image_decoder_with_limits(
                            &mut BytesReader::new(&bytes),
                            &mut options,
                            DecodeLimits::default(),
                        )
                    } else {
                        image_decoder(&mut BytesReader::new(&bytes), &mut options)
                    };
                    if term_error {
                        assert_eq!(
                            result.unwrap_err().to_string(),
                            "deliberate termination failure"
                        );
                    } else {
                        assert!(
                            result.is_ok(),
                            "height {height}, explicit limits {explicit_limits}: {result:?}"
                        );
                    }
                    assert_eq!((probe.draws, probe.nexts, probe.terms), (1, 0, 1));
                }
            }
        }
    }
}
