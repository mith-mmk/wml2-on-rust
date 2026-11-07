//! Isolated candidate evaluation, never linked into WML2.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;
use zune_core::bit_depth::BitDepth;
use zune_core::bytestream::{ZByteIoError, ZByteWriterTrait};
use zune_core::colorspace::ColorSpace;
use zune_core::options::EncoderOptions;
use zune_jpegxl::JxlSimpleEncoder;

struct Meter;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static REQUESTED: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicUsize = AtomicUsize::new(0);
fn added(n: usize) {
    let live = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(live, Relaxed);
    REQUESTED.fetch_add(n, Relaxed);
    CALLS.fetch_add(1, Relaxed);
}
// Single-process evaluation instrumentation; requested Rust heap bytes, not RSS.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            added(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            added(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) };
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            LIVE.fetch_sub(l.size(), Relaxed);
            added(n);
        }
        q
    }
}
#[global_allocator]
static ALLOCATOR: Meter = Meter;

struct Sink {
    bytes: Vec<u8>,
    cap: usize,
    reserve_cap: usize,
    reserve_requested: usize,
    peak_before_reserve: usize,
}
impl ZByteWriterTrait for &mut Sink {
    fn write_bytes(&mut self, b: &[u8]) -> Result<usize, ZByteIoError> {
        self.write_all_bytes(b)?;
        Ok(b.len())
    }
    fn write_all_bytes(&mut self, b: &[u8]) -> Result<(), ZByteIoError> {
        if b.len() > self.cap.saturating_sub(self.bytes.len()) {
            return Err(ZByteIoError::Generic("evaluation output cap"));
        }
        self.bytes.extend_from_slice(b);
        Ok(())
    }
    fn write_const_bytes<const N: usize>(&mut self, b: &[u8; N]) -> Result<(), ZByteIoError> {
        self.write_all_bytes(b)
    }
    fn flush_bytes(&mut self) -> Result<(), ZByteIoError> {
        Ok(())
    }
    fn reserve_capacity(&mut self, n: usize) -> Result<(), ZByteIoError> {
        self.reserve_requested = n;
        self.peak_before_reserve = PEAK.load(Relaxed);
        if n > self.reserve_cap {
            return Err(ZByteIoError::Generic("evaluation reservation cap"));
        }
        self.bytes.reserve(n);
        Ok(())
    }
}

fn main() {
    // width height channels bits raw-input output cap reserve-cap
    let a: Vec<_> = std::env::args().collect();
    let width: usize = a[1].parse().unwrap();
    let height: usize = a[2].parse().unwrap();
    let channels: usize = a[3].parse().unwrap();
    let bits: usize = a[4].parse().unwrap();
    let data = std::fs::read(&a[5]).unwrap();
    let color = match channels {
        1 => ColorSpace::Luma,
        2 => ColorSpace::LumaA,
        3 => ColorSpace::RGB,
        4 => ColorSpace::RGBA,
        _ => ColorSpace::CMYK,
    };
    let depth = if bits == 8 {
        BitDepth::Eight
    } else {
        BitDepth::Sixteen
    };
    let options = EncoderOptions::new(width, height, color, depth).set_num_threads(1);
    let mut sink = Sink {
        bytes: Vec::new(),
        cap: a[7].parse().unwrap(),
        reserve_cap: a[8].parse().unwrap(),
        reserve_requested: 0,
        peak_before_reserve: 0,
    };
    let base = LIVE.load(Relaxed);
    PEAK.store(base, Relaxed);
    REQUESTED.store(0, Relaxed);
    CALLS.store(0, Relaxed);
    let start = Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        JxlSimpleEncoder::new(&data, options).encode(&mut sink)
    }));
    let micros = start.elapsed().as_micros();
    let peak = PEAK.load(Relaxed).saturating_sub(base);
    let requested = REQUESTED.load(Relaxed);
    let calls = CALLS.load(Relaxed);
    let status = match &result {
        Ok(Ok(_)) => "ok",
        Ok(Err(_)) => "error",
        Err(_) => "panic",
    };
    let error = match &result {
        Ok(Err(e)) => format!("{e:?}"),
        _ => String::new(),
    };
    // Error text is emitted separately to keep machine output JSON simple.
    if !error.is_empty() {
        eprintln!("{error}");
    }
    if status == "ok" {
        std::fs::write(&a[6], &sink.bytes).unwrap();
    }
    println!(
        "{{\"status\":\"{status}\",\"input_bytes\":{},\"output_bytes\":{},\"output_capacity\":{},\"peak_extra_requested_bytes\":{peak},\"total_requested_bytes\":{requested},\"allocation_calls\":{calls},\"reserve_requested\":{},\"peak_extra_before_reserve\":{},\"encode_us\":{micros}}}",
        data.len(),
        sink.bytes.len(),
        sink.bytes.capacity(),
        sink.reserve_requested,
        sink.peak_before_reserve.saturating_sub(base)
    );
}
