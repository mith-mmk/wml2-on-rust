//! Fixed oversized-input admission checks. The caller Vec is allocated before
//! metering. Build as a wasm32 cdylib against the indicated WML2 feature set.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use wml2::draw::*;
static TRACK: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static LARGEST: AtomicUsize = AtomicUsize::new(0);
static BEFORE: AtomicUsize = AtomicUsize::new(0);
static AFTER: AtomicUsize = AtomicUsize::new(0);
struct Meter;
fn add(n: usize) {
    if TRACK.load(Relaxed) {
        COUNT.fetch_add(1, Relaxed);
        TOTAL.fetch_add(n, Relaxed);
        LARGEST.fetch_max(n, Relaxed);
    }
}
// SAFETY: unchanged System layouts/pointers; counters allocate nothing.
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
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, l, n) };
        if !next.is_null() {
            add(n);
        }
        next
    }
}
#[global_allocator]
static ALLOC: Meter = Meter;
#[unsafe(no_mangle)]
pub extern "C" fn allocation_count() -> usize {
    COUNT.load(Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn requested_bytes() -> usize {
    TOTAL.load(Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn largest_allocation() -> usize {
    LARGEST.load(Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn capacity_before_decode() -> usize {
    BEFORE.load(Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn capacity_after_decode() -> usize {
    AFTER.load(Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn oversized_refusal(mode: u32) -> u32 {
    let mut input = vec![0u8; 64 * 1024 * 1024 + 1];
    if mode != 3 {
        input[..2].copy_from_slice(&[255, 10]);
    }
    COUNT.store(0, Relaxed);
    TOTAL.store(0, Relaxed);
    LARGEST.store(0, Relaxed);
    BEFORE.store(core::arch::wasm32::memory_size(0) * 65536, Relaxed);
    TRACK.store(true, Relaxed);
    let result = match mode {
        0 => image_from(&input).map(|_| ()),
        1 => image_from_ex(&input, PixelFormatEx::Rgba16, PrecisionConversion::Round).map(|_| ()),
        2 | 3 => wml2::jpegxl::image_from_srgb(&input).map(|_| ()),
        _ => {
            TRACK.store(false, Relaxed);
            return 0;
        }
    };
    TRACK.store(false, Relaxed);
    AFTER.store(core::arch::wasm32::memory_size(0) * 65536, Relaxed);
    let refused = result
        .err()
        .is_some_and(|e| e.to_string() == "Out of memory: JPEG XL input exceeds decode limit");
    u32::from(
        refused
            && LARGEST.load(Relaxed) < 4096
            && TOTAL.load(Relaxed) < 8192
            && COUNT.load(Relaxed) < 32,
    )
}
