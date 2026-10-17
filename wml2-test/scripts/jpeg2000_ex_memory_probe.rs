//! Standalone Linux measurement harness; compiled against immutable base/current
//! path dependencies in separate Cargo target directories. Never a library allocator.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};
use wml2::{draw::*, limits::DecodeLimits};
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
struct Meter;
fn added(n: usize) {
    let live = LIVE.fetch_add(n, Ordering::Relaxed) + n;
    PEAK.fetch_max(live, Ordering::Relaxed);
}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            added(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            added(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, n: usize) -> *mut u8 {
        let out = unsafe { System.realloc(p, layout, n) };
        if !out.is_null() {
            if n >= layout.size() {
                added(n - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - n, Ordering::Relaxed);
            }
        }
        out
    }
}
#[global_allocator]
static ALLOC: Meter = Meter;
type Error = Box<dyn std::error::Error>;
struct Target {
    image: ImageBufferEx,
    file: std::fs::File,
    discard: bool,
    w: usize,
    h: usize,
    before_init: usize,
    after_init: usize,
    after_draw: usize,
}
impl DrawCallback for Target {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        Err("EX declaration required".into())
    }
    fn init_ex(&mut self, w: usize, h: usize, o: Option<InitOptionsEx>) -> Response {
        self.w = w;
        self.h = h;
        self.before_init = LIVE.load(Ordering::Relaxed);
        if !self.discard {
            self.image.init_ex(w, h, o)?;
        }
        self.after_init = LIVE.load(Ordering::Relaxed);
        Ok(None)
    }
    fn draw(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        data: &[u8],
        o: Option<DrawOptions>,
    ) -> Response {
        assert_eq!((x, y, w, h), (0, 0, self.w, self.h));
        if self.discard {
            self.file.write_all(data)?;
        } else {
            self.image.draw(x, y, w, h, data, o)?;
            self.file
                .write_all(self.image.bytes().ok_or("missing pixels")?)?;
        }
        self.after_draw = LIVE.load(Ordering::Relaxed);
        Ok(None)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        Err("static only".into())
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, _: &str, _: wml2::metadata::DataMap) -> Response {
        Err("unexpected metadata callback".into())
    }
}
struct Measurement {
    ok: bool,
    w: usize,
    h: usize,
    peak: usize,
    result_live: usize,
    us: u128,
    error: Option<String>,
    before_init: usize,
    after_init: usize,
    after_draw: usize,
}
fn vmhwm() -> usize {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    status
        .lines()
        .find_map(|l| {
            l.strip_prefix("VmHWM:")
                .and_then(|l| l.split_whitespace().next())
                .and_then(|v| v.parse().ok())
        })
        .unwrap()
}
fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err("INPUT OUTPUT owned|callback|discard|highres gray12|gray16|rgba12|rgba16|rgba8 exact|round default|large".into());
    }
    let input = std::fs::read(&args[1])?;
    let mut file = std::fs::File::create(&args[2])?;
    let storage = match args[4].as_str() {
        "gray12" => PixelFormatEx::Gray12,
        "gray16" => PixelFormatEx::Gray16,
        "rgba12" => PixelFormatEx::Rgba12,
        "rgba16" => PixelFormatEx::Rgba16,
        "rgba8" => PixelFormatEx::Rgba8,
        _ => return Err("format".into()),
    };
    let policy = match args[5].as_str() {
        "exact" => PrecisionConversion::Exact,
        "round" => PrecisionConversion::Round,
        _ => return Err("policy".into()),
    };
    let mut limits = DecodeLimits::default();
    if args[6] == "large" {
        limits.expanded_bytes = 3 * 1024 * 1024 * 1024;
    } else if args[6] != "default" {
        return Err("budget".into());
    }
    let baseline = LIVE.load(Ordering::Relaxed);
    PEAK.store(baseline, Ordering::Relaxed);
    let start = Instant::now();
    let m = if args[3] == "owned" || args[3] == "highres" {
        match image_from_ex_with_limits(&input, storage, policy, limits) {
            Ok(image) => {
                let typed = if args[3] == "highres" {
                    Some(image.to_highres_pixels()?)
                } else {
                    None
                };
                let m = Measurement {
                    ok: true,
                    w: image.width(),
                    h: image.height(),
                    peak: PEAK.load(Ordering::Relaxed),
                    result_live: LIVE.load(Ordering::Relaxed),
                    us: start.elapsed().as_micros(),
                    error: None,
                    before_init: 0,
                    after_init: 0,
                    after_draw: 0,
                };
                file.write_all(image.bytes().ok_or("missing pixels")?)?;
                black_box(&image);
                black_box(&typed);
                m
            }
            Err(error) => {
                let peak = PEAK.load(Ordering::Relaxed);
                let result_live = LIVE.load(Ordering::Relaxed);
                Measurement {
                    ok: false,
                    w: 0,
                    h: 0,
                    peak,
                    result_live,
                    us: start.elapsed().as_micros(),
                    error: Some(error.to_string()),
                    before_init: 0,
                    after_init: 0,
                    after_draw: 0,
                }
            }
        }
    } else {
        let discard = args[3] == "discard";
        if !discard && args[3] != "callback" {
            return Err("mode".into());
        }
        let mut target = Target {
            image: ImageBufferEx::with_storage(storage, policy),
            file,
            discard,
            w: 0,
            h: 0,
            before_init: 0,
            after_init: 0,
            after_draw: 0,
        };
        let result = image_decoder_ex_with_limits(
            &mut bin_rs::reader::BytesReader::new(&input),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut target,
            },
            limits,
        );
        let peak = PEAK.load(Ordering::Relaxed);
        let result_live = LIVE.load(Ordering::Relaxed);
        let m = Measurement {
            ok: result.is_ok(),
            w: target.w,
            h: target.h,
            peak,
            result_live,
            us: start.elapsed().as_micros(),
            error: result.err().map(|e| e.to_string()),
            before_init: target.before_init,
            after_init: target.after_init,
            after_draw: target.after_draw,
        };
        black_box(&target);
        m
    };
    black_box(&input);
    let resident = vmhwm();
    println!(
        "{{\"accepted\":{},\"width\":{},\"height\":{},\"input_bytes\":{},\"baseline_live_bytes\":{},\"peak_live_bytes\":{},\"peak_extra_bytes\":{},\"live_at_result_bytes\":{},\"live_after_result_drop_bytes\":{},\"self_vmhwm_kib\":{},\"operation_us\":{},\"callback_before_init_live\":{},\"callback_after_init_live\":{},\"callback_after_draw_live\":{},\"error\":{}}}",
        m.ok,
        m.w,
        m.h,
        input.len(),
        baseline,
        m.peak,
        m.peak - baseline,
        m.result_live,
        LIVE.load(Ordering::Relaxed),
        resident,
        m.us,
        m.before_init,
        m.after_init,
        m.after_draw,
        m.error.map(|e| format!("{e:?}")).unwrap_or("null".into())
    );
    Ok(())
}
