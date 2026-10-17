//! Standalone Linux reduction measurements; requested live bytes and process RSS.
//! Build the same harness against base/current in separate target directories.
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
enum Raster {
    Legacy(ImageBuffer),
    Native(ImageBufferEx),
}
impl Raster {
    fn shape(&self) -> (usize, usize) {
        match self {
            Self::Legacy(v) => (v.width, v.height),
            Self::Native(v) => (v.width(), v.height()),
        }
    }
    fn bytes(&self) -> Result<&[u8], Error> {
        match self {
            Self::Legacy(v) => v.buffer.as_deref().ok_or_else(|| "missing pixels".into()),
            Self::Native(v) => v.bytes().ok_or_else(|| "missing pixels".into()),
        }
    }
}
fn decode(
    input: &[u8],
    reduce: u8,
    bits: u8,
    nc: usize,
    limits: DecodeLimits,
) -> Result<Raster, Error> {
    if bits == 8 {
        let mut image = ImageBuffer::new();
        let mut reader = bin_rs::reader::BytesReader::new(input);
        let mut options = DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        };
        if reduce == 0 {
            // Same existing API in both builds; isolate full-resolution regression
            // from the negligible generic/codec-specific dispatch difference.
            image_decoder_with_limits(&mut reader, &mut options, limits)?;
        } else {
            #[cfg(feature = "reduced-api")]
            wml2::jpeg2000::decode_with_reduction(&mut reader, &mut options, reduce, limits)?;
            #[cfg(not(feature = "reduced-api"))]
            return Err("baseline has no nonzero reduction API".into());
        }
        Ok(Raster::Legacy(image))
    } else {
        let format = match (bits, nc) {
            (12, 1) => PixelFormatEx::Gray12,
            (16, 1) => PixelFormatEx::Gray16,
            (12, _) => PixelFormatEx::Rgba12,
            (16, _) => PixelFormatEx::Rgba16,
            _ => return Err("depth".into()),
        };
        if reduce == 0 {
            return Ok(Raster::Native(image_from_ex_with_limits(
                input,
                format,
                PrecisionConversion::Exact,
                limits,
            )?));
        }
        #[cfg(feature = "reduced-api")]
        return Ok(Raster::Native(
            wml2::jpeg2000::image_from_ex_with_reduction(
                input,
                format,
                PrecisionConversion::Exact,
                reduce,
                limits,
            )?,
        ));
        #[cfg(not(feature = "reduced-api"))]
        Err("baseline has no nonzero reduction API".into())
    }
}
fn vmhwm() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")
                .and_then(|v| v.split_whitespace().next())
                .and_then(|v| v.parse().ok())
        })
        .unwrap()
}
fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err("INPUT OUTPUT REDUCE 8|12|16 1|3|4 default|large".into());
    }
    let reduce: u8 = args[3].parse()?;
    let bits: u8 = args[4].parse()?;
    let nc: usize = args[5].parse()?;
    if !matches!(bits, 8 | 12 | 16) || !matches!(nc, 1 | 3 | 4) {
        return Err("probe depth/components".into());
    }
    let mut limits = DecodeLimits::default();
    match args[6].as_str() {
        "large" => limits.expanded_bytes = 3 * 1024 * 1024 * 1024,
        "default" => {}
        _ => return Err("probe budget".into()),
    }
    let input = std::fs::read(&args[1])?;
    let mut output = std::fs::File::create(&args[2])?;
    let baseline = LIVE.load(Ordering::Relaxed);
    PEAK.store(baseline, Ordering::Relaxed);
    let start = Instant::now();
    let image = decode(&input, reduce, bits, nc, limits);
    let micros = start.elapsed().as_micros();
    let peak = PEAK.load(Ordering::Relaxed);
    let at_result = LIVE.load(Ordering::Relaxed);
    let (accepted, width, height, error) = match image {
        Ok(image) => {
            let (width, height) = image.shape();
            output.write_all(image.bytes()?)?;
            black_box(&image);
            (true, width, height, None)
        }
        Err(error) => (false, 0, 0, Some(error.to_string())),
    };
    black_box(&input);
    let after_drop = LIVE.load(Ordering::Relaxed);
    let rss = vmhwm();
    println!(
        "{{\"accepted\":{accepted},\"width\":{width},\"height\":{height},\"reduce\":{reduce},\"bits\":{bits},\"components\":{nc},\"input_bytes\":{},\"baseline_live_bytes\":{baseline},\"peak_live_bytes\":{peak},\"peak_extra_bytes\":{},\"live_at_result_bytes\":{at_result},\"live_after_result_drop_bytes\":{after_drop},\"self_vmhwm_kib\":{rss},\"operation_us\":{micros},\"error\":{}}}",
        input.len(),
        peak - baseline,
        error.map(|s| format!("{s:?}")).unwrap_or("null".into())
    );
    Ok(())
}
