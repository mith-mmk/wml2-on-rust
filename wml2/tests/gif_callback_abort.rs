#![cfg(feature = "gif")]

use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;

// Three independent 1x1 frames: red, blue, red. Each LZW stream is clear,
// palette index, end; no encoder or optional external fixture is needed.
fn animation() -> Vec<u8> {
    let mut bytes = b"GIF89a\x01\0\x01\0\x80\0\0\xff\0\0\0\0\xff".to_vec();
    for index in [0, 1, 0] {
        bytes.extend_from_slice(b"!\xf9\x04\0\x02\0\0\0,\0\0\0\0\x01\0\x01\0\0\x02\x02");
        bytes.extend_from_slice(&[0x44 | (index << 3), 1, 0]);
    }
    bytes.push(b';');
    bytes
}

#[derive(Default)]
struct Probe {
    abort_at: usize,
    nexts: usize,
    pixels: Vec<Vec<u8>>,
    terms: usize,
    aborted: bool,
    fail: &'static str,
}
impl Probe {
    fn active(&self) {
        assert!(!self.aborted, "callback forwarded after Abort");
    }
    fn error(&self, event: &str) -> Response {
        if self.fail == event {
            Err(std::io::Error::other(format!("deliberate {event} failure")).into())
        } else {
            Ok(None)
        }
    }
}
impl DrawCallback for Probe {
    fn init(&mut self, w: usize, h: usize, _: Option<InitOptions>) -> Response {
        self.active();
        assert_eq!((w, h), (1, 1));
        self.error("init")
    }
    fn draw(
        &mut self,
        _: usize,
        _: usize,
        w: usize,
        h: usize,
        data: &[u8],
        _: Option<DrawOptions>,
    ) -> Response {
        self.active();
        assert_eq!((w, h), (1, 1));
        self.pixels.push(data.to_vec());
        self.error("draw")
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        self.active();
        self.nexts += 1;
        if self.nexts == self.abort_at {
            self.aborted = true;
            Ok(Some(CallbackResponse::abort()))
        } else {
            self.error("next")
        }
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.terms += 1;
        self.error("terminate")
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        self.active();
        self.error("verbose")
    }
    fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
        self.active();
        self.error("metadata")
    }
}

#[test]
fn public_gif_next_abort_completes_without_forwarding_more_callbacks() {
    let bytes = animation();
    for explicit_limits in [false, true] {
        for debug_flag in [0, 1] {
            for abort_at in [1, 2, 3] {
                let mut probe = Probe {
                    abort_at,
                    ..Default::default()
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
                assert!(
                    result.is_ok(),
                    "abort at {abort_at}, explicit limits {explicit_limits}: {result:?}"
                );
                assert_eq!(probe.nexts, abort_at);
                let expected = if abort_at == 3 { 2 } else { 1 };
                assert_eq!(probe.pixels.len(), expected);
                assert_eq!(probe.pixels[0], [255, 0, 0, 255]);
                if expected == 2 {
                    assert_eq!(probe.pixels[1], [0, 0, 255, 255]);
                }
                assert_eq!(probe.terms, 1);
            }
        }
    }
}

#[test]
fn gif_abort_does_not_read_the_next_frame_or_require_a_trailer() {
    let bytes = animation();
    // Only the complete header and first frame; cancellation must not parse
    // a second frame, an intentionally malformed suffix, or the trailer.
    for suffix in [b"".as_slice(), b"not a GIF block".as_slice()] {
        let mut prefix = bytes[..42].to_vec();
        prefix.extend_from_slice(suffix);
        let mut probe = Probe {
            abort_at: 1,
            ..Default::default()
        };
        image_decoder(
            &mut BytesReader::new(&prefix),
            &mut DecodeOptions {
                debug_flag: 1,
                drawer: &mut probe,
            },
        )
        .unwrap();
        assert_eq!((probe.nexts, probe.pixels.len(), probe.terms), (1, 1, 1));
    }
}

#[test]
fn gif_real_callback_and_input_errors_remain_errors() {
    for fail in ["init", "draw", "next", "verbose", "metadata", "terminate"] {
        let mut probe = Probe {
            abort_at: if fail == "terminate" { 1 } else { 0 },
            fail,
            ..Default::default()
        };
        let error = image_decoder(
            &mut BytesReader::new(&animation()),
            &mut DecodeOptions {
                debug_flag: 1,
                drawer: &mut probe,
            },
        )
        .unwrap_err();
        assert_eq!(error.to_string(), format!("deliberate {fail} failure"));
    }
    for length in [19, 39, 40] {
        let bytes = animation();
        let mut probe = Probe {
            abort_at: 1,
            ..Default::default()
        };
        assert!(
            image_decoder(
                &mut BytesReader::new(&bytes[..length]),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut probe
                }
            )
            .is_err()
        );
        assert!(!probe.aborted);
    }
}

#[test]
fn public_gif_without_abort_retains_all_frame_pixels() {
    let mut probe = Probe::default();
    image_decoder(
        &mut BytesReader::new(&animation()),
        &mut DecodeOptions {
            debug_flag: 1,
            drawer: &mut probe,
        },
    )
    .unwrap();
    assert_eq!(probe.nexts, 3);
    assert_eq!(
        probe.pixels,
        [[255, 0, 0, 255], [0, 0, 255, 255], [255, 0, 0, 255]]
    );
}

#[test]
fn gif87a_first_frame_abort_also_completes() {
    let bytes = animation();
    let mut gif87a = bytes[..19].to_vec();
    gif87a[..6].copy_from_slice(b"GIF87a");
    gif87a.extend_from_slice(&bytes[27..42]); // No GIF89a graphic control extension.
    gif87a.push(b';');
    let mut probe = Probe {
        abort_at: 1,
        ..Default::default()
    };
    image_decoder(
        &mut BytesReader::new(&gif87a),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut probe,
        },
    )
    .unwrap();
    assert_eq!((probe.nexts, probe.pixels.len(), probe.terms), (1, 1, 1));
}
