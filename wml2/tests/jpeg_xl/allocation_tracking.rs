use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {static ENABLED:Cell<bool>=const{Cell::new(false)};static LIVE:Cell<isize>=const{Cell::new(0)};}
pub struct Allocator;
fn charge(n: isize) {
    if ENABLED.try_with(Cell::get).unwrap_or(false) {
        let _ = LIVE.try_with(|v| v.set(v.get() + n));
    }
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            charge(l.size() as isize)
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            charge(l.size() as isize)
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        charge(-(l.size() as isize));
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            charge(n as isize - l.size() as isize)
        }
        q
    }
}
pub fn measure(f: impl FnOnce()) -> isize {
    LIVE.with(|v| v.set(0));
    ENABLED.with(|v| v.set(true));
    f();
    ENABLED.with(|v| v.set(false));
    LIVE.with(Cell::get)
}
