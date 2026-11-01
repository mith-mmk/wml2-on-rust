use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, UnsafeCell},
};
#[derive(Clone, Copy)]
struct Entry {
    ptr: usize,
    bytes: usize,
    trace: usize,
}
const EMPTY: Entry = Entry {
    ptr: 0,
    bytes: 0,
    trace: 0,
};
const N: usize = 32768;
#[derive(Clone, Copy)]
struct Trace {
    ip: [usize; 32],
    len: usize,
}
const NO_TRACE: Trace = Trace {
    ip: [0; 32],
    len: 0,
};
#[link(name = "gcc_s")]
unsafe extern "C" {
    fn _Unwind_Backtrace(
        cb: unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> i32,
        data: *mut std::ffi::c_void,
    ) -> i32;
    fn _Unwind_GetIP(ctx: *mut std::ffi::c_void) -> usize;
}
unsafe extern "C" fn unwind(ctx: *mut std::ffi::c_void, data: *mut std::ffi::c_void) -> i32 {
    let t = unsafe { &mut *data.cast::<Trace>() };
    if t.len == 32 {
        return 5;
    }
    t.ip[t.len] = unsafe { _Unwind_GetIP(ctx) };
    t.len += 1;
    0
}
struct State {
    traces: [Trace; 2048],
    count: usize,
    slots: [Entry; N],
    overflow: bool,
    external_frees: usize,
}
thread_local! {static ON:Cell<bool>=const{Cell::new(false)};static STATE:UnsafeCell<State>=const{UnsafeCell::new(State{traces:[NO_TRACE;2048],count:0,slots:[EMPTY;N],overflow:false,external_frees:0})};}
fn record(p: *mut u8, n: usize, remove: bool) {
    if !ON.try_with(Cell::get).unwrap_or(false) || p.is_null() {
        return;
    }
    let _ = STATE.try_with(|s| {
        let st = unsafe { &mut *s.get() };
        let addr = p as usize;
        let mut i = (addr >> 4).wrapping_mul(11400714819323198485u64 as usize) & (N - 1);
        for _ in 0..N {
            let e = &mut st.slots[i];
            if e.ptr == addr {
                if remove {
                    *e = Entry {
                        ptr: 1,
                        bytes: 0,
                        trace: 0,
                    }
                } else {
                    e.bytes = n
                }
                return;
            }
            if e.ptr == 0 || (!remove && e.ptr == 1) {
                if !remove {
                    let trace = if matches!(n, 24 | 768 | 1500 | 1716) && st.count < 2048 {
                        let id = st.count;
                        st.count += 1;
                        unsafe {
                            _Unwind_Backtrace(unwind, (&mut st.traces[id] as *mut Trace).cast());
                        }
                        id + 1
                    } else {
                        0
                    };
                    *e = Entry {
                        ptr: addr,
                        bytes: n,
                        trace,
                    }
                } else {
                    st.external_frees += n
                }
                return;
            }
            i = (i + 1) & (N - 1)
        }
        st.overflow = true;
    });
}
struct A;
#[global_allocator]
static A0: A = A;
unsafe impl GlobalAlloc for A {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        record(p, l.size(), false);
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        record(p, l.size(), false);
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        record(p, l.size(), true);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            record(p, l.size(), true);
            record(q, n, false)
        }
        q
    }
}
const INPUT: &[u8] =
    include_bytes!("../../wml2/tests/fixtures/review/jxl-animation-main5-icc16-variable.jxl");

use wml2::draw::*;
const INPUT8: &[u8] =
    include_bytes!("../../wml2/tests/fixtures/review/jxl-animation-main5-internal.jxl");
struct Probe {
    abort: &'static str,
}
impl DrawCallback for Probe {
    fn begin_image(&mut self) -> Response {
        Ok((self.abort == "begin").then(CallbackResponse::abort))
    }
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        Ok((self.abort == "init").then(CallbackResponse::abort))
    }
    fn draw(
        &mut self,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
        _: &[u8],
        _: Option<DrawOptions>,
    ) -> Response {
        Ok((self.abort == "draw").then(CallbackResponse::abort))
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        Ok(None)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, _: &str, _: wml2::metadata::DataMap) -> Response {
        Ok(None)
    }
}
fn decode(mode: &str) -> &'static str {
    let mut corrupt = [0u8; 4096];
    match mode {
        "success" => {
            drop(
                wml2::jpegxl::image_from_with_resource_limits(
                    INPUT,
                    Default::default(),
                    Default::default(),
                )
                .unwrap(),
            );
            "ok"
        }
        "native" => {
            drop(
                wml2::jpegxl::animation_rgba16_from_with_resource_limits_and_cancel(
                    INPUT,
                    Default::default(),
                    Default::default(),
                    &|| false,
                )
                .unwrap(),
            );
            "ok"
        }
        "eight" => {
            drop(
                wml2::jpegxl::image_from_with_resource_limits(
                    INPUT8,
                    Default::default(),
                    Default::default(),
                )
                .unwrap(),
            );
            "ok"
        }
        "budget" => {
            let cache = 25 * (3 * (143 + 64 + 125) * 4 + 3 * std::mem::size_of::<usize>());
            let policy = wml2::jpegxl::JxlResourceLimits {
                workspace_bytes: (1 << 20) + cache - 1,
                ..Default::default()
            };
            let error =
                wml2::jpegxl::image_from_with_resource_limits(INPUT, Default::default(), policy)
                    .err()
                    .unwrap();
            assert!(error.to_string().contains("header workspace"));
            "error"
        }
        "early_error" => {
            assert!(
                wml2::jpegxl::image_from_with_resource_limits(
                    &INPUT[..INPUT.len() - 1],
                    Default::default(),
                    Default::default()
                )
                .is_err()
            );
            "error"
        }
        "late_error" => {
            let n = INPUT.len();
            assert!(n <= 4096);
            corrupt[..n].copy_from_slice(INPUT);
            corrupt[n - 12..n].fill(255);
            assert!(
                wml2::jpegxl::image_from_with_resource_limits(
                    &corrupt[..n],
                    Default::default(),
                    Default::default()
                )
                .is_err()
            );
            "error"
        }
        "begin" | "init" | "draw" => {
            let mut probe = Probe {
                abort: match mode {
                    "begin" => "begin",
                    "init" => "init",
                    _ => "draw",
                },
            };
            let mut reader = bin_rs::reader::StreamReader::new(std::io::Cursor::new(INPUT));
            image_decoder_with_limits(
                &mut reader,
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut probe,
                },
                Default::default(),
            )
            .unwrap();
            "abort"
        }
        _ => panic!("unknown mode"),
    }
}
fn sample(mode: &str) {
    STATE.with(|s| unsafe {
        *s.get() = State {
            traces: [NO_TRACE; 2048],
            count: 0,
            slots: [EMPTY; N],
            overflow: false,
            external_frees: 0,
        }
    });
    ON.with(|c| c.set(true));
    let outcome = decode(mode);
    ON.with(|c| c.set(false));
    STATE.with(|s| {
        let st = unsafe { &*s.get() };
        let mut sizes: Vec<_> = st
            .slots
            .iter()
            .filter(|e| e.ptr > 1)
            .map(|e| e.bytes)
            .collect();
        sizes.sort();
        for e in st.slots.iter().filter(|e| e.ptr > 1) {
            if e.trace > 0 {
                let t = &st.traces[e.trace - 1];
                println!("trace bytes={} ip={:x?}", e.bytes, &t.ip[..t.len]);
            }
        }
        println!(
            "mode={mode},outcome={outcome},live={sizes:?},sum={},external_frees={},overflow={}",
            sizes.iter().sum::<usize>(),
            st.external_frees,
            st.overflow
        );
        assert!(!st.overflow);
    });
}
fn main() {
    let mode = std::env::args().nth(1).unwrap_or("success".into());
    if mode == "depths" {
        for m in ["success", "eight", "success", "eight"] {
            sample(m)
        }
    } else if mode == "threads" {
        for _ in 0..2 {
            std::thread::Builder::new()
                .stack_size(8 << 20)
                .spawn(|| sample("success"))
                .unwrap()
                .join()
                .unwrap();
        }
    } else {
        for _ in 0..3 {
            sample(&mode);
        }
    }
}
