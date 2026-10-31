//! Resident-buffer decode timing and requested-live-heap measurement. Build in
//! release mode; source loading, hashing and optional PPM export are not timed.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;
use wml2::draw::{ImageBuffer, image_from};
use wml2::jpeg::{JpegScale, image_from_scaled};
struct Count;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn add(n: usize) {
    let live = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(live, Relaxed);
}
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
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
        let new = unsafe { System.realloc(p, l, n) };
        if !new.is_null() {
            if n >= l.size() {
                add(n - l.size());
            } else {
                LIVE.fetch_sub(l.size() - n, Relaxed);
            }
        }
        new
    }
}
#[global_allocator]
static ALLOC: Count = Count;
fn decode(bytes: &[u8], mode: &str) -> Result<ImageBuffer, Box<dyn std::error::Error>> {
    if mode == "full" {
        image_from(bytes)
    } else {
        let scale = match mode {
            "2" => JpegScale::Half,
            "4" => JpegScale::Quarter,
            "8" => JpegScale::Eighth,
            _ => return Err("scale must be full, 2, 4, or 8".into()),
        };
        Ok(image_from_scaled(bytes, scale)?.image)
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err("usage: jpeg_scaled INPUT full|2|4|8 [iterations=20] [output.ppm]".into());
    }
    let bytes = std::fs::read(&args[1])?;
    let n = args
        .get(3)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(20usize);
    if n == 0 {
        return Err("iterations must be positive".into());
    }
    for _ in 0..3 {
        std::hint::black_box(decode(&bytes, &args[2])?);
    }
    let mut times = Vec::with_capacity(n);
    let mut peak = 0;
    let mut hash = 0;
    let mut dims = (0, 0);
    let mut size = 0;
    for _ in 0..n {
        let base = LIVE.load(Relaxed);
        PEAK.store(base, Relaxed);
        let start = Instant::now();
        let image = decode(&bytes, &args[2])?;
        times.push(start.elapsed().as_secs_f64() * 1e6);
        peak = peak.max(PEAK.load(Relaxed) - base);
        dims = (image.width, image.height);
        let raw = image.buffer.as_ref().unwrap();
        size = raw.len();
        hash = raw.iter().fold(0xcbf29ce484222325u64, |v, &b| {
            (v ^ b as u64).wrapping_mul(0x100000001b3)
        });
        std::hint::black_box(&image);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "mode={} {}x{} input={} output={} median_us={:.3} peak_additional_requested_heap={} fnv={hash:016x}",
        args[2],
        dims.0,
        dims.1,
        bytes.len(),
        size,
        times[n / 2],
        peak
    );
    // Linux's process high-water RSS includes runtime, source, allocator
    // retention and stack; it is deliberately separate from requested heap.
    #[cfg(target_os = "linux")]
    if let Ok(status) = std::fs::read_to_string("/proc/self/status")
        && let Some(line) = status.lines().find(|line| line.starts_with("VmHWM:"))
    {
        println!("process_peak_rss {line}");
    }
    if let Some(path) = args.get(4) {
        use std::io::Write;
        let image = decode(&bytes, &args[2])?;
        let mut out = std::fs::File::create(path)?;
        write!(out, "P6\n{} {}\n255\n", image.width, image.height)?;
        for pixel in image.buffer.unwrap().chunks_exact(4) {
            out.write_all(&pixel[..3])?;
        }
    }
    Ok(())
}
