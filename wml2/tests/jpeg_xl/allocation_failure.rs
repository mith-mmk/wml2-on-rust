use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {static FAIL:Cell<(usize,usize)>=const{Cell::new((0,0))};}
fn refuse(bytes: usize) -> bool {
    FAIL.with(|f| {
        let (size, n) = f.get();
        if size == bytes && n > 0 {
            f.set((size, n - 1));
            n == 1
        } else {
            false
        }
    })
}
pub fn fail_nth_size<T>(size: usize, n: usize, f: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            FAIL.with(|v| v.set((0, 0)));
        }
    }
    let _reset = Reset;
    FAIL.with(|v| v.set((size, n)));
    f()
}
pub struct Allocator;
// SAFETY: forward unchanged System layouts/pointers except the single armed null return.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if refuse(l.size()) {
            std::ptr::null_mut()
        } else {
            unsafe { System.alloc(l) }
        }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if refuse(l.size()) {
            std::ptr::null_mut()
        } else {
            unsafe { System.alloc_zeroed(l) }
        }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if refuse(n) {
            std::ptr::null_mut()
        } else {
            unsafe { System.realloc(p, l, n) }
        }
    }
}
