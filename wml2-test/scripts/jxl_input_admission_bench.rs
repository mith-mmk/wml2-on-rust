//! Resident-slice JPEG XL regression benchmark; build against a --locked WML2
//! rlib with jpeg-xl,image-buffer-ex,color-management. See the admission report.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;
use wml2::draw::*;
type Error = Box<dyn std::error::Error>;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
struct Meter;
fn add(n: usize) {
    COUNT.fetch_add(1, Relaxed);
    TOTAL.fetch_add(n, Relaxed);
    let live = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(live, Relaxed);
}
// SAFETY: exact System forwarding; counters allocate nothing and failed
// requests leave live accounting unchanged. Realloc-internal overlap excluded.
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
enum Image {
    Ordinary(ImageBuffer),
    Ex(ImageBufferEx),
}
impl Image {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Ordinary(i) => i.buffer.as_deref().unwrap(),
            Self::Ex(i) => i.bytes().unwrap(),
        }
    }
    fn size(&self) -> (usize, usize) {
        match self {
            Self::Ordinary(i) => (i.width, i.height),
            Self::Ex(i) => (i.width(), i.height()),
        }
    }
}
fn decode(input: &[u8], mode: &str) -> Result<Image, Error> {
    Ok(match mode {
        "ordinary" => Image::Ordinary(image_from(input)?),
        "ex16" => Image::Ex(image_from_ex(
            input,
            PixelFormatEx::Rgba16,
            PrecisionConversion::Round,
        )?),
        "cms" => Image::Ordinary(wml2::jpegxl::image_from_srgb(input)?),
        _ => return Err("mode must be ordinary/ex16/cms".into()),
    })
}
fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        return Err("usage: bench INPUT MODE ITERATIONS [output.ppm]".into());
    }
    let input = std::fs::read(&args[1])?;
    let n: usize = args[3].parse()?;
    if n == 0 {
        return Err("iterations must be positive".into());
    }
    for _ in 0..3 {
        std::hint::black_box(decode(&input, &args[2])?);
    }
    let mut times = Vec::with_capacity(n);
    let mut peak = 0;
    let mut count = 0;
    let mut total = 0;
    let mut hash = 0;
    let mut size = (0, 0);
    let mut len = 0;
    for _ in 0..n {
        let base = LIVE.load(Relaxed);
        PEAK.store(base, Relaxed);
        COUNT.store(0, Relaxed);
        TOTAL.store(0, Relaxed);
        let start = Instant::now();
        let image = decode(&input, &args[2])?;
        times.push(start.elapsed().as_secs_f64() * 1e6);
        peak = peak.max(PEAK.load(Relaxed) - base);
        count = count.max(COUNT.load(Relaxed));
        total = total.max(TOTAL.load(Relaxed));
        size = image.size();
        let raw = image.bytes();
        len = raw.len();
        hash = raw.iter().fold(0xcbf29ce484222325u64, |v, &b| {
            (v ^ b as u64).wrapping_mul(0x100000001b3)
        });
        std::hint::black_box(&image);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "mode={} input={} width={} height={} bytes={} median_us={:.3} allocations={} requested_bytes={} peak_additional_requested_heap={} fnv={hash:016x}",
        args[2],
        input.len(),
        size.0,
        size.1,
        len,
        times[n / 2],
        count,
        total,
        peak
    );
    if let Some(path) = args.get(4) {
        use std::io::Write;
        let image = decode(&input, &args[2])?;
        let mut out = std::fs::File::create(path)?;
        let (w, h) = image.size();
        write!(out, "P6\n{w} {h}\n255\n")?;
        match &image {
            Image::Ordinary(_) => {
                for p in image.bytes().chunks_exact(4) {
                    out.write_all(&p[..3])?;
                }
            }
            Image::Ex(_) => {
                for p in image.bytes().chunks_exact(8) {
                    for c in 0..3 {
                        let v = u16::from_le_bytes([p[c * 2], p[c * 2 + 1]]);
                        out.write_all(&[((u32::from(v) + 128) / 257) as u8])?;
                    }
                }
            }
        }
    }
    Ok(())
}
