//! Link unchanged baseline/current release rlibs as wml_before/wml_after.
//! Arguments: plain.avif profile.avif iterations. Input generation is independent.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;
struct Meter;
static COUNT: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Relaxed);
        BYTES.fetch_add(l.size(), Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Relaxed);
        BYTES.fetch_add(l.size(), Relaxed);
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        COUNT.fetch_add(1, Relaxed);
        BYTES.fetch_add(n, Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: Meter = Meter;
fn pixels(data: &[u8], after: bool) -> Vec<u8> {
    if after {
        wml_after::draw::image_from(data).unwrap().buffer.unwrap()
    } else {
        wml_before::draw::image_from(data).unwrap().buffer.unwrap()
    }
}
fn measure(data: &[u8], after: bool) -> u64 {
    let bytes = pixels(data, after);
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let inputs = [
        std::fs::read(&args[1]).unwrap(),
        std::fs::read(&args[2]).unwrap(),
    ];
    let iterations: usize = args[3].parse().unwrap();
    let plain = pixels(&inputs[0], false);
    let profiled = pixels(&inputs[1], false);
    assert_eq!(plain, pixels(&inputs[0], true));
    assert_eq!(profiled, pixels(&inputs[1], true));
    assert_eq!(plain.len(), profiled.len());
    eprintln!(
        "ICC/plain max RGBA difference: {}",
        plain
            .iter()
            .zip(&profiled)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap()
    );
    println!("iteration,variant,ns,allocations,allocated_bytes,hash");
    let names = ["before_plain", "after_plain", "before_icc", "after_icc"];
    for i in 0..iterations {
        for j in 0..4 {
            let variant = if i % 2 == 0 {
                (j + i) % 4
            } else {
                (3 - j + i) % 4
            };
            COUNT.store(0, Relaxed);
            BYTES.store(0, Relaxed);
            let start = Instant::now();
            let hash = measure(&inputs[variant / 2], variant % 2 == 1);
            let ns = start.elapsed().as_nanos();
            let count = COUNT.load(Relaxed);
            let bytes = BYTES.load(Relaxed);
            println!("{i},{},{ns},{count},{bytes},{hash}", names[variant]);
        }
    }
}
