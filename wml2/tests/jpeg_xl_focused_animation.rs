#![cfg(feature = "jpeg-xl")]
use wml2::{draw::*, jpegxl::*, limits::DecodeLimits};
type Error = Box<dyn std::error::Error>;
#[path = "jpeg_xl/allocation_failure.rs"]
mod allocation_failure;
#[global_allocator]
static ALLOCATOR: allocation_failure::Allocator = allocation_failure::Allocator;
const INPUT: &[u8] = include_bytes!("fixtures/review/jxl-animation-main5-icc16-variable.jxl");
#[test]
fn ordinary_and_ex_keep_all_c_canvases_exact_time_and_loop() {
    for (input, gold, ticks, num, den, loops, _, _) in include!("jpeg_xl/animation_cases.rs") {
        let exact = ticks
            .iter()
            .all(|&v| u128::from(v) * u128::from(den) * 1000 % u128::from(num) == 0);
        let result = image_from(input);
        assert_eq!(result.is_ok(), exact);
        if !exact {
            continue;
        }
        let image = result.unwrap();
        assert_eq!(image.loop_count, Some(loops));
        let frames = image.animation.unwrap();
        assert_eq!(frames.len(), gold.len());
        assert_eq!(image.buffer.as_ref().unwrap(), &frames[0].buffer);
        for ((f, g), t) in frames.iter().zip(gold).zip(ticks) {
            assert_eq!(
                f.control.await_time,
                u64::from(*t) * u64::from(den) * 1000 / u64::from(num)
            );
            assert!(matches!(f.control.blend, Some(NextBlend::Override)));
            for (a, b) in f.buffer.iter().zip(g.chunks_exact(2)) {
                let v = u32::from(u16::from_le_bytes(b.try_into().unwrap()));
                assert!(a.abs_diff(((v * 255 + 32767) / 65535) as u8) <= 1);
            }
        }
        #[cfg(feature = "image-buffer-ex")]
        {
            let ex =
                image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert_eq!(ex.loop_count(), Some(loops));
            assert_eq!(ex.source_format(), Some(PixelFormatEx::Rgba16));
            let frames = ex.animation().unwrap();
            assert_eq!(frames.len(), gold.len());
            assert_eq!(ex.bytes().unwrap(), frames[0].pixels.bytes());
            for (f, g) in frames.iter().zip(gold) {
                assert_eq!(f.pixels.bytes().len(), g.len());
                for (a, b) in f.pixels.bytes().chunks_exact(2).zip(g.chunks_exact(2)) {
                    assert!(
                        u16::from_le_bytes(a.try_into().unwrap())
                            .abs_diff(u16::from_le_bytes(b.try_into().unwrap()))
                            <= 1
                    );
                }
            }
            assert!(ex.to_rgba8(PrecisionConversion::Round).is_err());
        }
    }
}
#[cfg(feature = "color-management")]
#[test]
fn managed_all_frames_match_independent_lcms_with_alpha_unchanged() {
    for (input, gold) in include!("jpeg_xl/animation_display_cases.rs") {
        let image = image_from_srgb_with_limits(input, Default::default()).unwrap();
        let frames = image.animation.as_ref().unwrap();
        assert_eq!(frames.len(), gold.len());
        assert_eq!(image.buffer.as_ref().unwrap(), &frames[0].buffer);
        for (f, g) in frames.iter().zip(gold) {
            assert_eq!(f.buffer.len(), g.len());
            for (i, (&a, &b)) in f.buffer.iter().zip(g.iter()).enumerate() {
                assert!(
                    a.abs_diff(b) <= if i % 4 == 3 { 0 } else { 2 },
                    "CMS {i}: {a}/{b}"
                );
            }
        }
    }
}
#[derive(Default)]
struct Probe {
    events: Vec<&'static str>,
    abort: Option<usize>,
    fail: Option<usize>,
}
impl Probe {
    fn event(&mut self, name: &'static str) -> Response {
        self.events.push(name);
        let i = self.events.len() - 1;
        if self.fail == Some(i) {
            return Err(std::io::Error::other("injected callback error").into());
        }
        Ok((self.abort == Some(i)).then(CallbackResponse::abort))
    }
}
impl DrawCallback for Probe {
    fn begin_image(&mut self) -> Response {
        self.event("begin")
    }
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
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        self.event("next")
    }
    fn set_metadata(&mut self, _: &str, _: wml2::metadata::DataMap) -> Response {
        self.event("metadata")
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.event("end")
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        self.event("verbose")
    }
}
fn run(input: &[u8], p: &mut Probe, ex: bool) -> Result<(), Error> {
    let mut r = bin_rs::reader::StreamReader::new(std::io::Cursor::new(input));
    let mut o = DecodeOptions {
        debug_flag: 0,
        drawer: p,
    };
    #[cfg(feature = "image-buffer-ex")]
    if ex {
        return image_decoder_ex_with_limits(&mut r, &mut o, DecodeLimits::default()).map(|_| ());
    }
    let _ = ex;
    image_decoder_with_limits(&mut r, &mut o, DecodeLimits::default()).map(|_| ())
}
#[test]
fn abort_and_callback_errors_stop_at_every_boundary_and_terminate_once() {
    for ex in [false, true] {
        let mut baseline = Probe::default();
        run(INPUT, &mut baseline, ex).unwrap();
        assert_eq!(baseline.events.iter().filter(|&&e| e == "next").count(), 3);
        let len = baseline.events.len();
        for stop in 0..len {
            let mut p = Probe {
                abort: Some(stop),
                ..Default::default()
            };
            run(INPUT, &mut p, ex).unwrap();
            assert_eq!(&p.events[..stop + 1], &baseline.events[..stop + 1]);
            assert_eq!(p.events.iter().filter(|&&e| e == "end").count(), 1);
            assert_eq!(p.events.len(), stop + 1 + usize::from(stop + 1 < len));
        }
        for stop in 1..len {
            let mut p = Probe {
                fail: Some(stop),
                ..Default::default()
            };
            assert!(run(INPUT, &mut p, ex).is_err());
            assert_eq!(p.events.iter().filter(|&&e| e == "end").count(), 1);
            assert_eq!(p.events.len(), stop + 1 + usize::from(stop + 1 < len));
        }
    }
}
#[test]
fn malformed_or_unrepresentable_input_never_initializes_callbacks() {
    for input in [
        include_bytes!("fixtures/review/jxl-animation-fractional.jxl").as_slice(),
        &INPUT[..INPUT.len() - 1],
    ] {
        let mut p = Probe::default();
        assert!(run(input, &mut p, false).is_err());
        assert!(
            p.events == vec!["begin", "end"] || p.events == vec!["begin"],
            "{:?}",
            p.events
        );
    }
    let max = include_bytes!("fixtures/review/jxl-animation-timebase-max.jxl");
    let result = image_from(max).unwrap();
    assert_eq!(result.loop_count, Some(u32::MAX));
    let frames = result.animation.unwrap();
    for (i, f) in frames.iter().enumerate() {
        assert_eq!(
            f.control.await_time,
            (u64::from(u32::MAX) - i as u64) * 1024 * 1000
        );
    }
    let mut bad = include_bytes!("fixtures/review/jxl-animation-main5-icc16.jxl").to_vec();
    let n = bad.len();
    bad[n - 12..].fill(255);
    let mut p = Probe::default();
    assert!(run(&bad, &mut p, false).is_err());
    assert_eq!(p.events, vec!["begin", "end"]);
}

#[test]
fn rational_native_frames_match_c_and_limits_remain_independent() {
    for (input, gold, ticks, num, den, loops, o, bits) in include!("jpeg_xl/animation_cases.rs") {
        let image =
            animation_rgba16_from_with_limits_and_cancel(input, Default::default(), &|| false)
                .unwrap();
        assert_eq!(
            (
                image.ticks_per_second_numerator,
                image.ticks_per_second_denominator,
                image.loop_count
            ),
            (num, den, loops)
        );
        assert_eq!((image.source_orientation, image.bits_per_sample), (o, bits));
        assert_eq!(image.frames.len(), gold.len());
        for ((f, g), t) in image.frames.iter().zip(gold).zip(ticks) {
            assert_eq!(f.duration_ticks, *t);
            assert_eq!(f.rgba16.len() * 2, g.len());
            for (a, b) in f.rgba16.iter().zip(g.chunks_exact(2)) {
                assert!(a.abs_diff(u16::from_le_bytes(b.try_into().unwrap())) <= 1);
            }
        }
    }
    let boundary = include_bytes!("fixtures/review/jxl-animation-main5-layers64.jxl");
    assert_eq!(
        animation_rgba16_from_with_limits_and_cancel(boundary, Default::default(), &|| false)
            .unwrap()
            .frames
            .len(),
        64
    );
    assert_eq!(image_from(boundary).unwrap().animation.unwrap().len(), 64);
    // Three RGBA16 canvases + prepared RGBA8 + collector poster and three frames.
    for (cap, ok) in [(1819, false), (1820, true)] {
        assert_eq!(
            image_from_with_limits(
                INPUT,
                DecodeLimits {
                    animation_bytes: cap,
                    ..Default::default()
                }
            )
            .is_ok(),
            ok
        );
    }
    for limits in [
        DecodeLimits {
            input_bytes: INPUT.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            pixels: 34,
            ..Default::default()
        },
        DecodeLimits {
            frames: 2,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 839,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1024,
            ..Default::default()
        },
    ] {
        assert!(animation_rgba16_from_with_limits_and_cancel(INPUT, limits, &|| false).is_err());
    }
    for input in [
        include_bytes!("fixtures/review/jxl-animation-preview.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-timecodes.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-main5-layers65.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-internal.jxl").as_slice(),
    ] {
        assert!(
            animation_rgba16_from_with_limits_and_cancel(input, DecodeLimits::unlimited(), &|| {
                false
            })
            .is_err()
        );
    }
    for end in 0..INPUT.len() {
        assert!(
            animation_rgba16_from_with_limits_and_cancel(
                &INPUT[..end],
                Default::default(),
                &|| false
            )
            .is_err()
        );
    }
}
#[test]
fn cancellation_never_returns_a_partial_native_or_managed_image() {
    use std::cell::Cell;
    let count = Cell::new(0);
    let run = |stop| {
        animation_rgba16_from_with_limits_and_cancel(INPUT, Default::default(), &|| {
            let i = count.get();
            count.set(i + 1);
            i == stop
        })
    };
    run(usize::MAX).unwrap();
    let total = count.get();
    for stop in 0..total {
        count.set(0);
        let e = run(stop).unwrap_err();
        assert!(e.is::<DecodeCancelled>());
    }
    assert!(
        animation_rgba16_from_with_limits_and_cancel(&[], DecodeLimits::unlimited(), &|| true)
            .unwrap_err()
            .is::<DecodeCancelled>()
    );
    #[cfg(feature = "color-management")]
    {
        count.set(0);
        let run = |stop| {
            image_from_srgb_with_limits_and_cancel(INPUT, Default::default(), &|| {
                let i = count.get();
                count.set(i + 1);
                i == stop
            })
        };
        run(usize::MAX).unwrap();
        let total = count.get();
        for stop in 0..total {
            count.set(0);
            assert!(
                run(stop)
                    .err()
                    .expect("must cancel")
                    .is::<DecodeCancelled>()
            );
        }
    }
}
#[test]
fn buffer_reservation_failure_returns_error_and_can_retry() {
    // Arm one actual fallible full-canvas allocation, after previous frames exist.
    let result = allocation_failure::fail_nth_size(280, 2, || {
        animation_rgba16_from_with_limits_and_cancel(INPUT, Default::default(), &|| false)
    });
    assert!(result.is_err());
    assert!(
        animation_rgba16_from_with_limits_and_cancel(INPUT, Default::default(), &|| false).is_ok()
    );
}

#[test]
fn retained_brob_metadata_survives_nested_decode_limits_failure_and_retry() {
    fn add_box(out: &mut Vec<u8>, kind: &[u8; 4], payload: &[u8]) {
        out.extend_from_slice(&u32::try_from(payload.len() + 8).unwrap().to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
    }
    let mut bytes = INPUT.to_vec();
    assert!(bytes.starts_with(b"\0\0\0\x0cJXL \r\n\x87\n"));
    let mut brob = b"test".to_vec();
    brob.extend_from_slice(include_bytes!("fixtures/review/jxl-brob-repeat.br"));
    for _ in 0..4 {
        add_box(&mut bytes, b"brob", &brob);
    }
    let expected = image_from(INPUT).unwrap();
    let run = |cap| {
        image_from_with_limits(
            &bytes,
            DecodeLimits {
                expanded_bytes: cap,
                ..Default::default()
            },
        )
    };
    let (mut low, mut high) = (0, 32 * 1024 * 1024);
    while low < high {
        let mid = low + (high - low) / 2;
        if run(mid).is_ok() {
            high = mid
        } else {
            low = mid + 1
        }
    }
    assert!(run(low - 1).is_err());
    let image = run(low).unwrap();
    assert_eq!(image.buffer, expected.buffer);
    assert_eq!(image.animation.as_ref().unwrap().len(), 3);
    assert_eq!(
        image
            .metadata
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(key, value)| key.starts_with("JPEG XL Decoded Box ")
                && matches!(value,wml2::metadata::DataMap::Raw(b) if b.len()==65659))
            .count(),
        4
    );
    let mut sink = Probe::default();
    let mut reader = bin_rs::reader::StreamReader::new(std::io::Cursor::new(&bytes));
    assert!(
        image_decoder_with_limits(
            &mut reader,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut sink
            },
            DecodeLimits {
                expanded_bytes: low - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        !sink
            .events
            .iter()
            .any(|&e| matches!(e, "init" | "draw" | "next"))
    );
    let mut truncated = bytes.clone();
    truncated.pop();
    assert!(image_from(&truncated).is_err());
    assert_eq!(
        run(low).unwrap().buffer,
        expected.buffer,
        "failed nested decode must release its reservation"
    );
}
