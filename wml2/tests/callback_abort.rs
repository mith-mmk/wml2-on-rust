#![cfg(any(feature = "png", feature = "jpeg"))]
use wml2::draw::*;
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;
struct Probe {
    target: &'static str,
    aborted: bool,
    draws: usize,
    nexts: usize,
    terms: usize,
}
impl Probe {
    fn event(&mut self, name: &str) -> Response {
        assert!(!self.aborted, "callback {name} after Abort");
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
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        self.event("verbose")
    }
    fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
        self.event("metadata")
    }
}
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
