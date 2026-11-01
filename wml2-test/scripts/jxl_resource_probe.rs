//! Development-only resource measurements on trusted fixtures. No new runtime
//! limit/profile or arbitrary-input decoding API is provided by this program.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use wml2::draw::*;
use wml2::limits::DecodeLimits;
type Error = Box<dyn std::error::Error>;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
struct Meter;
fn add(n: usize) {
    TOTAL.fetch_add(n, Relaxed);
    let live = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(live, Relaxed);
}
// SAFETY: every request is forwarded unchanged to System; counters allocate
// nothing, and only successful allocations change live/traffic accounting.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, l, n) };
        if !next.is_null() {
            LIVE.fetch_sub(l.size(), Relaxed);
            add(n);
        }
        next
    }
}
#[global_allocator]
static ALLOC: Meter = Meter;
// The generated admission module is the unchanged source prefix through
// preflight(), plus a reporting wrapper. Only the private check shim is local;
// it uses the exact current DecodeLimits/error semantics. No caps are replaced.
mod draw {
    pub use wml2::draw::*;
}
mod error {
    pub use wml2::error::*;
}
mod metadata {
    pub use wml2::metadata::*;
}
mod warning {
    pub use wml2::warning::*;
}
mod limits {
    pub use wml2::limits::DecodeLimits;
    pub fn check(value: usize, limit: usize, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if value > limit {
            Err(Box::new(wml2::error::ImgError::new_const(
                wml2::error::ImgErrorKind::OutOfMemory,
                format!("{name} exceeds decode limit"),
            )))
        } else {
            Ok(())
        }
    }
}
#[allow(unused_imports, dead_code)]
mod admission {
    include!(concat!(env!("JXL_RESOURCE_OUT"), "/admission.rs"));
}
enum Image {
    Ordinary(ImageBuffer),
    Ex(ImageBufferEx),
    Core {
        width: usize,
        height: usize,
        pixels: Vec<u16>,
    },
    Plan([usize; 10]),
}
impl Image {
    fn size(&self) -> (usize, usize) {
        match self {
            Self::Ordinary(i) => (i.width, i.height),
            Self::Ex(i) => (i.width(), i.height()),
            Self::Core { width, height, .. } => (*width, *height),
            Self::Plan(p) => (p[0], p[1]),
        }
    }
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Ordinary(i) => i.buffer.as_deref().unwrap(),
            Self::Ex(i) => i.bytes().unwrap(),
            Self::Plan(_) => &[],
            Self::Core { pixels, .. } => unsafe {
                std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), pixels.len() * 2)
            },
        }
    }
    fn meta_bytes(&self) -> usize {
        let map = match self {
            Self::Ordinary(i) => &i.metadata,
            Self::Ex(i) => &i.metadata,
            _ => return 0,
        };
        map.as_ref().map_or(0, |map| {
            map.iter()
                .map(|(k, v)| {
                    k.len()
                        + match v {
                            wml2::metadata::DataMap::Raw(b)
                            | wml2::metadata::DataMap::ICCProfile(b) => b.len(),
                            wml2::metadata::DataMap::Ascii(s) => s.len(),
                            _ => 8,
                        }
                })
                .sum()
        })
    }
}
fn complete<T, U>(r: jxl_codec::api::ProcessingResult<T, U>) -> Result<T, Error> {
    match r {
        jxl_codec::api::ProcessingResult::Complete { result } => Ok(result),
        _ => Err("truncated trusted core input".into()),
    }
}
fn core(input: &[u8]) -> Result<Image, Error> {
    use jxl_codec::api::*;
    // Only separately registered, trusted <=4Mi canvas fixtures may use this
    // core-only comparison. It intentionally has NO WML2-equivalent preflight.
    // The codec's own complexity and finite sample checks remain enabled.
    let mut options = JxlDecoderOptions::default();
    options.high_precision = true;
    options.render_spot_colors = false;
    options.sample_limit = Some(16 * 1024 * 1024 + 1);
    let mut stream = input;
    let mut d =
        complete(JxlDecoder::<states::Initialized>::new(options).process(&mut stream, None)?)?;
    let (width, height) = d.basic_info().size;
    if width > 32768
        || height > 32768
        || width
            .checked_mul(height)
            .ok_or("core dimensions overflow")?
            > 4 * 1024 * 1024
    {
        return Err("trusted core fixture guard exceeded".into());
    }
    d.set_pixel_format(JxlPixelFormat::rgba16(d.basic_info().extra_channels.len()))?;
    let d = complete(d.process(&mut stream, None)?)?;
    let mut pixels = vec![0u16; width * height * 4];
    let raw = unsafe {
        std::slice::from_raw_parts_mut(pixels.as_mut_ptr().cast::<u8>(), pixels.len() * 2)
    };
    let buffers = &mut [JxlOutputBuffer::new(raw, height, width * 8)];
    let d = complete(d.process(&mut stream, buffers, None)?)?;
    if d.has_more_frames() {
        return Err("core fixture must be a still".into());
    }
    Ok(Image::Core {
        width,
        height,
        pixels,
    })
}
fn decode(input: &[u8], route: &str, limits: DecodeLimits) -> Result<Image, Error> {
    Ok(match route {
        "ordinary" => Image::Ordinary(image_from_with_limits(input, limits)?),
        "cms" => Image::Ordinary(wml2::jpegxl::image_from_srgb_with_limits(input, limits)?),
        "rgba8" => Image::Ex(image_from_ex_with_limits(
            input,
            PixelFormatEx::Rgba8,
            PrecisionConversion::Round,
            limits,
        )?),
        "rgba16" => Image::Ex(image_from_ex_with_limits(
            input,
            PixelFormatEx::Rgba16,
            PrecisionConversion::Round,
            limits,
        )?),
        "gray8" => Image::Ex(image_from_ex_with_limits(
            input,
            PixelFormatEx::Gray8,
            PrecisionConversion::Round,
            limits,
        )?),
        "gray16" => Image::Ex(image_from_ex_with_limits(
            input,
            PixelFormatEx::Gray16,
            PrecisionConversion::Round,
            limits,
        )?),
        "inspect" => Image::Plan(admission::describe(input, limits)?),
        "core-only" => core(input)?,
        _ => return Err("unknown route".into()),
    })
}
#[cfg(not(target_family = "wasm"))]
fn main() -> Result<(), Error> {
    use std::time::Instant;
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: probe INPUT ROUTE BUDGET ITERATIONS".into());
    }
    let bytes = std::fs::read(&args[1])?;
    let budget = budget(&args[3])?;
    let iterations: usize = args[4].parse()?;
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    for _ in 0..3 {
        let _ = std::hint::black_box(decode(&bytes, &args[2], budget));
    }
    let mut times = Vec::with_capacity(iterations);
    let mut max_peak = 0;
    let mut max_total = 0;
    let mut last = None;
    for _ in 0..iterations {
        let base = LIVE.load(Relaxed);
        PEAK.store(base, Relaxed);
        TOTAL.store(0, Relaxed);
        let start = Instant::now();
        let result = decode(&bytes, &args[2], budget);
        times.push(start.elapsed().as_secs_f64() * 1e6);
        let peak = PEAK.load(Relaxed).saturating_sub(base);
        let total = TOTAL.load(Relaxed);
        max_peak = max_peak.max(peak);
        max_total = max_total.max(total);
        last = Some(match result {
            Ok(image) => {
                let size = image.size();
                let raw = image.bytes();
                let hash = raw.iter().fold(0xcbf29ce484222325u64, |v, &b| {
                    (v ^ b as u64).wrapping_mul(0x100000001b3)
                });
                serde_json::json!({"ok":true,"width":size.0,"height":size.1,"output_bytes":raw.len(),"metadata_bytes":image.meta_bytes(),"fnv64":format!("{hash:016x}"),"preflight":if let Image::Plan(p)=&image{Some(p)}else{None}})
            }
            Err(error) => serde_json::json!({"ok":false,"error":error.to_string()}),
        });
    }
    times.sort_by(f64::total_cmp);
    let rss = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .map(str::to_owned)
        });
    let mut result = last.unwrap();
    result["route"] = args[2].clone().into();
    result["budget"] = args[3].clone().into();
    result["input_bytes"] = bytes.len().into();
    result["iterations"] = iterations.into();
    result["median_us"] = times[iterations / 2].into();
    result["peak_additional_requested_heap"] = max_peak.into();
    result["allocated_requested_bytes"] = max_total.into();
    result["process_peak_rss"] = rss.into();
    println!("{result}");
    Ok(())
}
fn budget(name: &str) -> Result<DecodeLimits, Error> {
    let mut b = if name == "unlimited" {
        DecodeLimits::unlimited()
    } else {
        DecodeLimits::default()
    };
    if let Some((kind, n)) = name.split_once('=') {
        let n: usize = n.parse()?;
        match kind {
            "expanded" => b.expanded_bytes = n,
            "metadata" => b.metadata_bytes = n,
            "pixels" => b.pixels = n,
            "input" => b.input_bytes = n,
            "frames" => b.frames = n,
            "animation" => b.animation_bytes = n,
            _ => return Err("unknown budget".into()),
        }
    } else if !matches!(name, "unlimited" | "default") {
        return Err("unknown budget".into());
    }
    Ok(b)
}
