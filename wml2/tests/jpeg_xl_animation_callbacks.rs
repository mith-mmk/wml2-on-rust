#![cfg(feature = "jpeg-xl")]
use wml2::{draw::*, jpegxl::*, limits::DecodeLimits};
type Error = Box<dyn std::error::Error>;
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
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
        let image =
            image_from_srgb_with_resource_limits(input, Default::default(), Default::default())
                .unwrap();
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
fn cumulative_callback_output_and_workspace_boundaries() {
    // Initialize shared callback/TLS caches before measuring steady-state ownership.
    drop(image_from_with_resource_limits(INPUT, Default::default(), Default::default()).unwrap());
    assert_eq!(
        allocation_tracking::measure(|| drop(
            image_from_with_resource_limits(INPUT, Default::default(), Default::default()).unwrap()
        )),
        0
    );
    for (cap, ok) in [(1819, false), (1820, true)] {
        assert_eq!(
            image_from_with_resource_limits(
                INPUT,
                DecodeLimits {
                    animation_bytes: cap,
                    ..Default::default()
                },
                Default::default()
            )
            .is_ok(),
            ok
        );
    }
    #[cfg(feature = "image-buffer-ex")]
    for (cap, ok) in [(3079, false), (3080, true)] {
        assert_eq!(
            image_from_ex_with_limits(
                INPUT,
                PixelFormatEx::Rgba16,
                PrecisionConversion::Exact,
                DecodeLimits {
                    animation_bytes: cap,
                    ..Default::default()
                }
            )
            .is_ok(),
            ok
        );
    }
    let (mut lo, mut hi) = (0, 64 << 20);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if image_from_with_resource_limits(
            INPUT,
            Default::default(),
            JxlResourceLimits {
                workspace_bytes: mid,
                ..Default::default()
            },
        )
        .is_ok()
        {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    for (cap, ok) in [(lo - 1, false), (lo, true)] {
        assert_eq!(
            image_from_with_resource_limits(
                INPUT,
                Default::default(),
                JxlResourceLimits {
                    workspace_bytes: cap,
                    ..Default::default()
                }
            )
            .is_ok(),
            ok
        );
    }
    #[cfg(feature = "image-buffer-ex")]
    {
        assert!(image_from_ex(INPUT, PixelFormatEx::Rgba12, PrecisionConversion::Exact).is_err());
        assert!(image_from_ex(INPUT, PixelFormatEx::Rgba12, PrecisionConversion::Round).is_ok());
    }
}
#[cfg(feature = "color-management")]
#[test]
fn every_managed_decode_cms_and_output_cancellation_drops_owned_results() {
    use std::cell::Cell;
    let total = Cell::new(0);
    drop(
        image_from_srgb_with_resource_limits_and_cancel(
            INPUT,
            Default::default(),
            Default::default(),
            &|| {
                total.set(total.get() + 1);
                false
            },
        )
        .unwrap(),
    );
    eprintln!(
        "Animation delivery cancellation checkpoints:{}",
        total.get()
    );
    assert!(total.get() > 45);
    for stop in 1..=total.get() {
        let count = Cell::new(0);
        assert_eq!(
            allocation_tracking::measure(|| {
                let e = image_from_srgb_with_resource_limits_and_cancel(
                    INPUT,
                    Default::default(),
                    Default::default(),
                    &|| {
                        count.set(count.get() + 1);
                        count.get() == stop
                    },
                )
                .err()
                .unwrap();
                assert!(e.is::<DecodeCancelled>());
                drop(e);
            }),
            0
        );
    }
    assert!(
        image_from_srgb_with_resource_limits_and_cancel(
            &[],
            Default::default(),
            JxlResourceLimits {
                workspace_bytes: usize::MAX,
                ..Default::default()
            },
            &|| true
        )
        .err()
        .unwrap()
        .is::<DecodeCancelled>()
    );
}
