//! Evaluation only: explicit low-level map/histogram budgets, no decoder option.
use jxl::bit_reader::BitReader;
use jxl::entropy_coding::context_map::decode_context_map_with_budget as map;
use jxl::entropy_coding::decode::{Histograms, SymbolReader};
use jxl::error::Error;
use jxl::util::entropy_budget::EntropyBudget;
const WINDOW: usize = 4 * 1024 * 1024;
fn bits(parts: &[(u64, usize)]) -> Vec<u8> {
    let mut out = vec![0; parts.iter().map(|p| p.1).sum::<usize>().div_ceil(8)];
    let mut pos = 0;
    for &(v, n) in parts {
        for b in 0..n {
            out[pos / 8] |= (((v >> b) & 1) as u8) << (pos % 8);
            pos += 1;
        }
    }
    out
}
// Hand-coded spec fields: LZ77 params, simple zero context map, prefix coding,
// hybrid uint split zero, one-symbol Huffman alphabet. Every symbol is zero.
fn histogram_bits(lz: bool) -> Vec<(u64, usize)> {
    if lz {
        vec![
            (1, 1),
            (0, 2),
            (0, 2),
            (0, 4),
            (1, 1),
            (0, 2),
            (1, 1),
            (0, 4),
            (0, 1),
        ]
    } else {
        vec![(0, 1), (1, 1), (0, 4), (0, 1)]
    }
}
fn simple() {
    let b = EntropyBudget::new(64);
    let clone = b.clone();
    let mut br = BitReader::new(&[1]);
    let mut reader_clone = br.clone();
    let value = map(32, &mut reader_clone, Some(&clone)).unwrap();
    let second = map(32, &mut br, Some(&b)).unwrap();
    assert_eq!(b.live_bytes(), 64);
    assert!(value.iter().all(|&v| v == 0));
    let moved = value;
    drop(clone);
    drop(moved);
    assert_eq!(b.live_bytes(), 32);
    drop(second);
    assert_eq!(b.live_bytes(), 0);
    assert_eq!(b.peak_bytes(), 64);
}
fn resize() {
    let b = EntropyBudget::new(48);
    let mut m = map(16, &mut BitReader::new(&[1]), Some(&b)).unwrap();
    m.try_resize(32, 7).unwrap();
    assert_eq!(b.peak_bytes(), 48);
    assert_eq!(b.live_bytes(), 32);
    assert!(m[..16].iter().all(|&v| v == 0));
    assert!(m[16..].iter().all(|&v| v == 7));
    m.try_resize(4, 0).unwrap();
    assert_eq!(b.live_bytes(), 32);
    drop(m);
    assert_eq!(b.live_bytes(), 0);
    let b = EntropyBudget::new(47);
    let mut m = map(16, &mut BitReader::new(&[1]), Some(&b)).unwrap();
    assert!(matches!(
        m.try_resize(32, 0),
        Err(Error::EntropyBudgetExceeded)
    ));
    assert_eq!(m.len(), 16);
    assert_eq!(b.live_bytes(), 16);
    drop(m);
    assert_eq!(b.live_bytes(), 0);
}
fn boundaries() {
    let b = EntropyBudget::new(usize::MAX);
    assert!(matches!(
        map(usize::MAX, &mut BitReader::new(&[1]), Some(&b)),
        Err(Error::SizeOverflow)
    ));
    assert!(map(0, &mut BitReader::new(&[1]), Some(&b)).is_err());
    assert_eq!(b.live_bytes(), 0);
    let raw = bits(&[(1, 1), (2, 2), (0, 2), (1, 2), (2, 2), (3, 2)]);
    let m = map(4, &mut BitReader::new(&raw), Some(&b)).unwrap();
    assert_eq!(&*m, &[0, 1, 2, 3]);
    drop(m);
    let holes = bits(&[(1, 1), (2, 2), (0, 2), (2, 2)]);
    assert!(matches!(
        map(2, &mut BitReader::new(&holes), Some(&b)),
        Err(Error::InvalidContextMapHole(..))
    ));
    let truncated = bits(&[(1, 1), (2, 2)]);
    assert!(map(64, &mut BitReader::new(&truncated), Some(&b)).is_err());
    assert_eq!(b.live_bytes(), 0);
}
fn nested() {
    for lz in [false, true] {
        for mtf in [false, true] {
            let mut parts = vec![(0, 1), (mtf as u64, 1)];
            parts.extend(histogram_bits(lz));
            let raw = bits(&parts);
            let overlap = if lz { WINDOW + 2 } else { 1 };
            let b = EntropyBudget::new(overlap + 8);
            let m = map(8, &mut BitReader::new(&raw), Some(&b)).unwrap();
            assert_eq!(&*m, &[0; 8]);
            assert_eq!(b.live_bytes(), 8);
            assert_eq!(b.peak_bytes(), overlap + 8);
            drop(m);
            assert_eq!(b.live_bytes(), 0);
            let b = EntropyBudget::new(overlap + 7);
            assert!(matches!(
                map(8, &mut BitReader::new(&raw), Some(&b)),
                Err(Error::EntropyBudgetExceeded)
            ));
            assert_eq!(b.live_bytes(), 0);
        }
    }
}
fn windows() {
    let raw = bits(&histogram_bits(true));
    let b = EntropyBudget::new(2 + 2 * WINDOW);
    let mut br = BitReader::new(&raw);
    let hist = Histograms::decode_with_budget(1, &mut br, true, Some(&b)).unwrap();
    assert_eq!(b.live_bytes(), 2);
    let mut first = SymbolReader::new(&hist, &mut br, None).unwrap();
    let second = SymbolReader::new(&hist, &mut br, None).unwrap();
    assert_eq!(b.live_bytes(), 2 + 2 * WINDOW);
    assert!(matches!(
        SymbolReader::new(&hist, &mut br, None),
        Err(Error::EntropyBudgetExceeded)
    ));
    for _ in 0..8 {
        assert_eq!(first.read_unsigned(&hist, &mut br, 0), 0);
    }
    let point = first.checkpoint::<4>();
    for _ in 0..4 {
        first.read_unsigned(&hist, &mut br, 0);
    }
    first.restore(point);
    assert_eq!(b.live_bytes(), 2 + 2 * WINDOW);
    first.check_final_state(&hist, &mut br).unwrap();
    assert_eq!(b.live_bytes(), 2 + WINDOW);
    drop(hist);
    assert_eq!(b.live_bytes(), WINDOW);
    drop(second);
    assert_eq!(b.live_bytes(), 0);
    let b = EntropyBudget::new(2 + 2 * WINDOW - 1);
    let mut br = BitReader::new(&raw);
    let hist = Histograms::decode_with_budget(1, &mut br, true, Some(&b)).unwrap();
    let first = SymbolReader::new(&hist, &mut br, None).unwrap();
    assert!(matches!(
        SymbolReader::new(&hist, &mut br, None),
        Err(Error::EntropyBudgetExceeded)
    ));
    assert_eq!(b.live_bytes(), 2 + WINDOW);
    drop(first);
    drop(hist);
    assert_eq!(b.live_bytes(), 0);
}
fn histogram_resize() {
    let raw = bits(&histogram_bits(false));
    let b = EntropyBudget::new(18);
    let mut br = BitReader::new(&raw);
    let mut hist = Histograms::decode_with_budget(1, &mut br, true, Some(&b)).unwrap();
    hist.resize(17).unwrap();
    assert_eq!(b.live_bytes(), 17);
    assert_eq!(b.peak_bytes(), 18);
    assert!(hist.resize(18).is_err());
    assert_eq!(b.live_bytes(), 17);
    drop(hist);
    assert_eq!(b.live_bytes(), 0);
}
/// Actual WASM executes these same six independent low-level case groups.
#[unsafe(no_mangle)]
pub extern "C" fn evaluate_entropy_budget() -> u32 {
    simple();
    resize();
    boundaries();
    nested();
    windows();
    histogram_resize();
    6
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::{Cell, RefCell};
    thread_local! { static FAIL:Cell<usize>=const{Cell::new(0)}; static HIT:Cell<usize>=const{Cell::new(0)}; static WATCH:RefCell<Option<EntropyBudget>>=const{RefCell::new(None)}; static WATCH_BYTES:Cell<usize>=const{Cell::new(0)}; static LIVE_AT_FREE:Cell<usize>=const{Cell::new(0)}; }
    struct Alloc;
    fn refused(n: usize) -> bool {
        FAIL.try_with(|f| {
            if n != 0 && f.get() == n {
                f.set(0);
                HIT.with(|h| h.set(n));
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
    }
    unsafe impl GlobalAlloc for Alloc {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            if refused(l.size()) {
                std::ptr::null_mut()
            } else {
                unsafe { System.alloc(l) }
            }
        }
        unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
            if refused(l.size()) {
                std::ptr::null_mut()
            } else {
                unsafe { System.alloc_zeroed(l) }
            }
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            if refused(n) {
                std::ptr::null_mut()
            } else {
                unsafe { System.realloc(p, l, n) }
            }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            if WATCH_BYTES
                .try_with(|n| n.get() == l.size())
                .unwrap_or(false)
            {
                let _ = WATCH.try_with(|w| {
                    if let Ok(value) = w.try_borrow() {
                        if let Some(b) = value.as_ref() {
                            LIVE_AT_FREE.with(|v| v.set(b.live_bytes()));
                        }
                    }
                });
            }
            unsafe { System.dealloc(p, l) }
        }
    }
    #[global_allocator]
    static ALLOC: Alloc = Alloc;
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            FAIL.with(|f| f.set(0));
            WATCH_BYTES.with(|n| n.set(0));
            WATCH.with(|w| *w.borrow_mut() = None);
        }
    }
    fn arm(n: usize) {
        HIT.with(|h| h.set(0));
        FAIL.with(|f| f.set(n));
    }
    #[test]
    fn backing_storage_is_freed_before_lease_release() {
        let _reset = Reset;
        let b = EntropyBudget::new(48);
        let mut m = map(16, &mut BitReader::new(&[1]), Some(&b)).unwrap();
        WATCH.with(|w| *w.borrow_mut() = Some(b.clone()));
        WATCH_BYTES.with(|n| n.set(16));
        m.try_resize(32, 0).unwrap();
        assert_eq!(LIVE_AT_FREE.with(Cell::get), 48);
        WATCH_BYTES.with(|n| n.set(32));
        drop(m);
        assert_eq!(LIVE_AT_FREE.with(Cell::get), 32);
        assert_eq!(b.live_bytes(), 0);
    }
    #[test]
    fn histogram_raw_storage_growth_failure_and_release_order() {
        let _reset = Reset;
        let raw = bits(&histogram_bits(false));
        let b = EntropyBudget::new(48);
        let mut br = BitReader::new(&raw);
        let mut hist = Histograms::decode_with_budget(1, &mut br, true, Some(&b)).unwrap();
        hist.resize(16).unwrap();
        WATCH.with(|w| *w.borrow_mut() = Some(b.clone()));
        WATCH_BYTES.with(|n| n.set(16));
        arm(32);
        assert!(matches!(hist.resize(32), Err(Error::OutOfMemory(_))));
        assert_eq!(HIT.with(Cell::get), 32);
        assert_eq!(b.live_bytes(), 16);
        assert_eq!(hist.map_context_to_cluster(15), 0);
        hist.resize(32).unwrap();
        assert_eq!(LIVE_AT_FREE.with(Cell::get), 48);
        assert_eq!(b.live_bytes(), 32);
        WATCH_BYTES.with(|n| n.set(32));
        drop(hist);
        assert_eq!(LIVE_AT_FREE.with(Cell::get), 32);
        assert_eq!(b.live_bytes(), 0);
    }
    #[test]
    fn map_clone_and_move() {
        simple()
    }
    #[test]
    fn resize_overlap() {
        resize()
    }
    #[test]
    fn invalid_and_overflow() {
        boundaries()
    }
    #[test]
    fn complex_maps_nested_window_and_mtf() {
        nested()
    }
    #[test]
    fn simultaneous_windows_and_checkpoints() {
        windows()
    }
    #[test]
    fn histogram_padding() {
        histogram_resize()
    }
    #[test]
    fn quota_denial_precedes_allocator_request() {
        let _r = Reset;
        let b = EntropyBudget::new(63);
        arm(64);
        let e = map(64, &mut BitReader::new(&[1]), Some(&b));
        FAIL.with(|f| f.set(0));
        assert!(matches!(e, Err(Error::EntropyBudgetExceeded)));
        assert_eq!(HIT.with(Cell::get), 0);
        assert_eq!(b.live_bytes(), 0);
    }
    #[test]
    fn actual_map_allocation_refusal() {
        let _r = Reset;
        let b = EntropyBudget::new(64);
        arm(64);
        assert!(matches!(
            map(64, &mut BitReader::new(&[1]), Some(&b)),
            Err(Error::OutOfMemory(_))
        ));
        assert_eq!(HIT.with(Cell::get), 64);
        assert_eq!(b.live_bytes(), 0);
        drop(map(64, &mut BitReader::new(&[1]), Some(&b)).unwrap());
        assert_eq!(b.live_bytes(), 0);
    }
    #[test]
    fn actual_resize_allocation_refusal() {
        let _r = Reset;
        let b = EntropyBudget::new(48);
        let mut m = map(16, &mut BitReader::new(&[1]), Some(&b)).unwrap();
        arm(32);
        assert!(matches!(m.try_resize(32, 0), Err(Error::OutOfMemory(_))));
        assert_eq!(HIT.with(Cell::get), 32);
        assert_eq!(b.live_bytes(), 16);
        assert_eq!(m.len(), 16);
        m.try_resize(32, 0).unwrap();
        drop(m);
        assert_eq!(b.live_bytes(), 0);
    }
    #[test]
    fn actual_window_allocation_refusal() {
        let _r = Reset;
        let raw = bits(&histogram_bits(true));
        let b = EntropyBudget::new(WINDOW + 2);
        let mut br = BitReader::new(&raw);
        let hist = Histograms::decode_with_budget(1, &mut br, true, Some(&b)).unwrap();
        arm(WINDOW);
        assert!(matches!(
            SymbolReader::new(&hist, &mut br, None),
            Err(Error::OutOfMemory(_))
        ));
        assert_eq!(HIT.with(Cell::get), WINDOW);
        assert_eq!(b.live_bytes(), 2);
        drop(SymbolReader::new(&hist, &mut br, None).unwrap());
        drop(hist);
        assert_eq!(b.live_bytes(), 0);
    }
    #[test]
    fn cloned_handles_share_atomic_limit_across_threads() {
        let b = EntropyBudget::new(256);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let b = b.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let m = map(32, &mut BitReader::new(&[1]), Some(&b)).unwrap();
                    barrier.wait();
                    assert!(map(1, &mut BitReader::new(&[1]), Some(&b)).is_err());
                    barrier.wait();
                    drop(m);
                })
            })
            .collect();
        for w in workers {
            w.join().unwrap();
        }
        assert_eq!(b.live_bytes(), 0);
        assert_eq!(b.peak_bytes(), 256);
    }
    #[test]
    fn real_icc_preflight_direct_calls_use_explicit_budget() {
        use jxl::headers::encodings::{Empty, UnconditionalCoder};
        use jxl::headers::{FileHeader, JxlHeader};
        let bytes = std::fs::read(
            std::path::Path::new(env!("WML2_ROOT"))
                .join("wml2/tests/fixtures/review/jxl-icc-lossless.jxl"),
        )
        .unwrap();
        let mut pos = 0;
        let mut stream = None;
        while pos + 8 <= bytes.len() {
            let n = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
            assert!(n >= 8 && pos + n <= bytes.len());
            if &bytes[pos + 4..pos + 8] == b"jxlc" {
                stream = Some(&bytes[pos + 8..pos + n]);
                break;
            }
            pos += n;
        }
        let stream = stream.unwrap();
        let mut br = BitReader::new(stream);
        let header = FileHeader::read(&mut br).unwrap();
        assert!(header.image_metadata.color_encoding.want_icc);
        u64::read_unconditional(&(), &mut br, &Empty {}).unwrap();
        let mut clone = br.clone();
        let b = EntropyBudget::new(16 * 1024 * 1024);
        let h = Histograms::decode_with_budget(41, &mut br, true, Some(&b)).unwrap();
        let mut r = SymbolReader::new(&h, &mut br, None).unwrap();
        let first = b.live_bytes();
        assert!(first >= 41);
        let h2 = Histograms::decode_with_budget(41, &mut clone, true, Some(&b)).unwrap();
        let r2 = SymbolReader::new(&h2, &mut clone, None).unwrap();
        assert_eq!(b.live_bytes(), 2 * first);
        for _ in 0..20 {
            let _ = r.read_unsigned(&h, &mut br, 0);
            br.check_for_error().unwrap();
        }
        drop(r);
        drop(h);
        assert_eq!(b.live_bytes(), first);
        drop(r2);
        drop(h2);
        assert_eq!(b.live_bytes(), 0);
        let zero = EntropyBudget::new(0);
        let mut br = BitReader::new(stream);
        FileHeader::read(&mut br).unwrap();
        u64::read_unconditional(&(), &mut br, &Empty {}).unwrap();
        assert!(matches!(
            Histograms::decode_with_budget(41, &mut br, true, Some(&zero)),
            Err(Error::EntropyBudgetExceeded)
        ));
        assert_eq!(zero.live_bytes(), 0);
    }
}
