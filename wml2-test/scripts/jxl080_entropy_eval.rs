//! Evaluation only:0.8 map payloads and reusable LZ77 scratch, not decoder-wide accounting.
use jxl::entropy_budget_evaluation::{
    BitReader, BudgetedHistograms, EntropyBudget, EntropyScratch, Error, SymbolReader,
    decode_context_map_with_budget as map,
};
const WINDOW: usize = 4 * 1024 * 1024;
fn bits(parts: &[(u64, usize)]) -> Vec<u8> {
    let mut out = vec![0; parts.iter().map(|v| v.1).sum::<usize>().div_ceil(8)];
    let mut pos = 0;
    for &(v, n) in parts {
        for b in 0..n {
            out[pos / 8] |= (((v >> b) & 1) as u8) << (pos % 8);
            pos += 1;
        }
    }
    out
}
fn hist_bits(lz: bool) -> Vec<u8> {
    bits(&if lz {
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
    })
}
fn maps() {
    let b = EntropyBudget::new(64);
    let handle = b.clone();
    let mut br = BitReader::new(&[1]);
    let mut cloned = br.clone();
    let a = map(32, &mut br, &mut Vec::new(), Some(&b)).unwrap();
    let c = map(32, &mut cloned, &mut Vec::new(), Some(&handle)).unwrap();
    assert_eq!(b.live_bytes(), 64);
    drop(a);
    assert_eq!(b.live_bytes(), 32);
    drop(c);
    assert_eq!(b.live_bytes(), 0);
    let mut a = map(16, &mut BitReader::new(&[1]), &mut Vec::new(), Some(&b)).unwrap();
    a.try_resize(48, 7).unwrap();
    assert_eq!(b.peak_bytes(), 64);
    assert_eq!(b.live_bytes(), 48);
    assert!(a.try_resize(49, 0).is_err());
    assert_eq!(b.live_bytes(), 48);
    drop(a);
    assert_eq!(b.live_bytes(), 0);
    for n in [0, usize::MAX] {
        assert!(map(n, &mut BitReader::new(&[1]), &mut Vec::new(), Some(&b)).is_err());
    }
    assert_eq!(b.live_bytes(), 0);
    let holes = bits(&[(1, 1), (2, 2), (0, 2), (2, 2)]);
    assert!(map(2, &mut BitReader::new(&holes), &mut Vec::new(), Some(&b)).is_err());
    assert_eq!(b.live_bytes(), 0);
}
fn cache_lifetime() {
    let raw = hist_bits(true);
    let b = EntropyBudget::new(2 + WINDOW);
    let mut br = BitReader::new(&raw);
    let h = BudgetedHistograms::decode(1, &mut br, true, &mut Vec::new(), &b).unwrap();
    let mut cache = EntropyScratch::new(&b);
    let mut reader =
        SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut cache).unwrap();
    assert_eq!(b.live_bytes(), 2 + WINDOW);
    for _ in 0..16 {
        assert_eq!(reader.read_unsigned(&h, &mut br, 0), 0);
    }
    let cp = reader.checkpoint::<4>();
    for _ in 0..4 {
        reader.read_unsigned(&h, &mut br, 0);
    }
    reader.restore(cp);
    reader
        .finish_with_budgeted_scratch(&h, &mut br, &mut cache)
        .unwrap();
    assert_eq!(cache.retained_bytes(), WINDOW);
    assert_eq!(b.live_bytes(), 2 + WINDOW);
    let reader = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut cache).unwrap();
    assert_eq!(cache.retained_bytes(), 0);
    assert_eq!(b.live_bytes(), 2 + WINDOW);
    reader
        .finish_with_budgeted_scratch(&h, &mut br, &mut cache)
        .unwrap();
    let mut empty = EntropyScratch::new(&b);
    assert!(matches!(
        SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut empty),
        Err(Error::EntropyBudgetExceeded)
    ));
    assert_eq!(empty.retained_bytes(), 0);
    assert_eq!(cache.retained_bytes(), WINDOW);
    drop(h);
    assert_eq!(b.live_bytes(), WINDOW);
    let moved = cache;
    drop(moved);
    assert_eq!(b.live_bytes(), 0);
}
fn simultaneous_and_mismatch() {
    let raw = hist_bits(true);
    let b = EntropyBudget::new(2 + 2 * WINDOW);
    let other = EntropyBudget::new(2 + 2 * WINDOW);
    let mut br = BitReader::new(&raw);
    let h = BudgetedHistograms::decode(1, &mut br, true, &mut Vec::new(), &b).unwrap();
    let (mut a, mut c) = (EntropyScratch::new(&b), EntropyScratch::new(&b));
    let first = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut a).unwrap();
    let second = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut c).unwrap();
    assert_eq!(b.live_bytes(), 2 + 2 * WINDOW);
    assert!(SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut c).is_err());
    let mut wrong = EntropyScratch::new(&other);
    assert!(matches!(
        SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut wrong),
        Err(Error::EntropyBudgetMismatch)
    ));
    assert_eq!(other.live_bytes(), 0);
    first
        .finish_with_budgeted_scratch(&h, &mut br, &mut a)
        .unwrap();
    assert_eq!(b.live_bytes(), 2 + 2 * WINDOW);
    second
        .finish_with_budgeted_scratch(&h, &mut br, &mut a)
        .unwrap();
    assert_eq!(b.live_bytes(), 2 + WINDOW);
    a.clear();
    assert_eq!(b.live_bytes(), 2);
    drop(h);
    assert_eq!(b.live_bytes(), 0);
}
fn no_raw_escape_and_failure() {
    let raw = hist_bits(true);
    let b = EntropyBudget::new(2 + WINDOW);
    let mut br = BitReader::new(&raw);
    let h = BudgetedHistograms::decode(1, &mut br, true, &mut Vec::new(), &b).unwrap();
    let mut raw_scratch = Vec::new();
    let mut reader = SymbolReader::new(&h, &mut br, None, &mut raw_scratch).unwrap();
    reader.reclaim_lz77_scratch(&mut raw_scratch);
    assert_eq!(raw_scratch.capacity(), 0);
    assert_eq!(b.live_bytes(), 2 + WINDOW);
    assert_eq!(reader.read_unsigned(&h, &mut br, 0), 0);
    reader.check_final_state(&h, &mut br).unwrap();
    assert_eq!(b.live_bytes(), 2);
    let mut cache = EntropyScratch::new(&b);
    let reader = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut cache).unwrap();
    br.read_optimistic(32);
    assert!(br.check_for_error().is_err());
    assert!(
        reader
            .finish_with_budgeted_scratch(&h, &mut br, &mut cache)
            .is_err()
    );
    assert_eq!(cache.retained_bytes(), 0);
    assert_eq!(b.live_bytes(), 2);
    drop(h);
    assert_eq!(b.live_bytes(), 0);
}
fn nested_maps() {
    for lz in [false, true] {
        for mtf in [false, true] {
            let h = if lz {
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
            };
            let mut p = vec![(0, 1), (mtf as u64, 1)];
            p.extend(h);
            let raw = bits(&p);
            let overlap = if lz { WINDOW + 2 } else { 1 };
            let b = EntropyBudget::new(overlap + 8);
            let value = map(8, &mut BitReader::new(&raw), &mut Vec::new(), Some(&b)).unwrap();
            assert_eq!(&*value, &[0; 8]);
            assert_eq!(b.live_bytes(), 8);
            assert_eq!(b.peak_bytes(), overlap + 8);
            drop(value);
            assert_eq!(b.live_bytes(), 0);
            let b = EntropyBudget::new(overlap + 7);
            assert!(map(8, &mut BitReader::new(&raw), &mut Vec::new(), Some(&b)).is_err());
            assert_eq!(b.live_bytes(), 0);
        }
    }
}
fn padding_and_non_lz_cache() {
    let raw = hist_bits(false);
    let b = EntropyBudget::new(WINDOW + 64);
    let lz = hist_bits(true);
    let mut lz_br = BitReader::new(&lz);
    let previous = BudgetedHistograms::decode(1, &mut lz_br, true, &mut Vec::new(), &b).unwrap();
    let mut cached = EntropyScratch::new(&b);
    let reader =
        SymbolReader::new_with_budgeted_scratch(&previous, &mut lz_br, None, &mut cached).unwrap();
    reader
        .finish_with_budgeted_scratch(&previous, &mut lz_br, &mut cached)
        .unwrap();
    drop(previous);
    assert_eq!(b.live_bytes(), WINDOW);
    let mut br = BitReader::new(&raw);
    let mut h = BudgetedHistograms::decode(1, &mut br, true, &mut Vec::new(), &b).unwrap();
    h.try_resize(17).unwrap();
    assert_eq!(b.live_bytes(), WINDOW + 17);
    let mut c = cached;
    let r = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut c).unwrap();
    r.finish_with_budgeted_scratch(&h, &mut br, &mut c).unwrap();
    assert_eq!(c.retained_bytes(), WINDOW);
    assert_eq!(b.live_bytes(), WINDOW + 17);
    drop(h);
    assert_eq!(b.live_bytes(), WINDOW);
    c.clear();
    assert_eq!(b.live_bytes(), 0);
}
#[unsafe(no_mangle)]
pub extern "C" fn evaluate_entropy080() -> u32 {
    maps();
    cache_lifetime();
    simultaneous_and_mismatch();
    no_raw_escape_and_failure();
    nested_maps();
    padding_and_non_lz_cache();
    6
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::{Cell, RefCell},
    };
    thread_local! {static FAIL:Cell<usize>=const{Cell::new(0)};static HIT:Cell<usize>=const{Cell::new(0)};static WATCH:RefCell<Option<EntropyBudget>>=const{RefCell::new(None)};static WATCH_SIZE:Cell<usize>=const{Cell::new(0)};static LIVE_AT_FREE:Cell<usize>=const{Cell::new(0)};}
    fn refused(n: usize) -> bool {
        FAIL.try_with(|f| {
            if n > 0 && f.get() == n {
                f.set(0);
                HIT.with(|v| v.set(n));
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
    }
    struct A;
    #[global_allocator]
    static ALLOC: A = A;
    unsafe impl GlobalAlloc for A {
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
            if WATCH_SIZE.try_with(Cell::get).unwrap_or(0) == l.size() {
                let _ = WATCH.try_with(|v| {
                    if let Some(b) = v.borrow().as_ref() {
                        LIVE_AT_FREE.with(|v| v.set(b.live_bytes()));
                    }
                });
            }
            unsafe { System.dealloc(p, l) }
        }
    }
    #[test]
    fn map_payload() {
        maps()
    }
    #[test]
    fn retained_scratch() {
        cache_lifetime()
    }
    #[test]
    fn concurrent_windows_and_roots() {
        simultaneous_and_mismatch()
    }
    #[test]
    fn raw_reclaim_cannot_export_budgeted_window() {
        no_raw_escape_and_failure()
    }
    #[test]
    fn nested_entropy() {
        nested_maps()
    }
    #[test]
    fn non_lz_padding() {
        padding_and_non_lz_cache()
    }
    #[test]
    fn allocator_refusal_rolls_back_and_cache_reuse_does_not_allocate() {
        let b = EntropyBudget::new(128);
        FAIL.with(|v| v.set(37));
        assert!(matches!(
            map(37, &mut BitReader::new(&[1]), &mut Vec::new(), Some(&b)),
            Err(Error::OutOfMemory(_))
        ));
        assert_eq!(HIT.with(Cell::get), 37);
        assert_eq!(b.live_bytes(), 0);
        let mut value = map(16, &mut BitReader::new(&[1]), &mut Vec::new(), Some(&b)).unwrap();
        FAIL.with(|v| v.set(47));
        assert!(value.try_resize(47, 0).is_err());
        assert_eq!(b.live_bytes(), 16);
        assert_eq!(value.len(), 16);
        drop(value);
        assert_eq!(b.live_bytes(), 0);
        let raw = hist_bits(true);
        let b = EntropyBudget::new(2 + WINDOW);
        let mut br = BitReader::new(&raw);
        let h = BudgetedHistograms::decode(1, &mut br, true, &mut Vec::new(), &b).unwrap();
        let mut c = EntropyScratch::new(&b);
        FAIL.with(|v| v.set(WINDOW));
        assert!(matches!(
            SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut c),
            Err(Error::OutOfMemory(_))
        ));
        assert_eq!(b.live_bytes(), 2);
        assert_eq!(c.retained_bytes(), 0);
        assert_eq!(FAIL.with(Cell::get), 0);
        let r = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut c).unwrap();
        r.finish_with_budgeted_scratch(&h, &mut br, &mut c).unwrap();
        FAIL.with(|v| v.set(WINDOW));
        let mut empty = EntropyScratch::new(&b);
        assert!(matches!(
            SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut empty),
            Err(Error::EntropyBudgetExceeded)
        ));
        assert_eq!(FAIL.with(Cell::get), WINDOW); // Quota denial happened before allocator request.
        assert_eq!(b.live_bytes(), 2 + WINDOW);
        let r = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut c).unwrap();
        assert_eq!(FAIL.with(Cell::get), WINDOW);
        FAIL.with(|v| v.set(0));
        drop(r);
        assert_eq!(b.live_bytes(), 2);
    }
    #[test]
    fn storage_is_freed_before_old_cache_lease_is_released() {
        let raw = hist_bits(true);
        let b = EntropyBudget::new(2 + 2 * WINDOW);
        let mut br = BitReader::new(&raw);
        let h = BudgetedHistograms::decode(1, &mut br, true, &mut Vec::new(), &b).unwrap();
        let (mut a, mut c) = (EntropyScratch::new(&b), EntropyScratch::new(&b));
        let first = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut a).unwrap();
        first
            .finish_with_budgeted_scratch(&h, &mut br, &mut a)
            .unwrap();
        let second = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut c).unwrap();
        WATCH.with(|v| *v.borrow_mut() = Some(b.clone()));
        WATCH_SIZE.with(|v| v.set(WINDOW));
        second
            .finish_with_budgeted_scratch(&h, &mut br, &mut a)
            .unwrap();
        assert_eq!(LIVE_AT_FREE.with(Cell::get), 2 + 2 * WINDOW);
        assert_eq!(b.live_bytes(), 2 + WINDOW);
        WATCH.with(|v| *v.borrow_mut() = None);
        WATCH_SIZE.with(|v| v.set(0));
    }
    #[test]
    fn eight_threads_share_one_budget() {
        let b = EntropyBudget::new(256);
        let gate = std::sync::Barrier::new(8);
        std::thread::scope(|s| {
            for _ in 0..8 {
                let (b, g) = (&b, &gate);
                s.spawn(move || {
                    let v = map(32, &mut BitReader::new(&[1]), &mut Vec::new(), Some(b)).unwrap();
                    g.wait();
                    assert!(b.live_bytes() <= 256);
                    drop(v);
                });
            }
        });
        assert_eq!(b.live_bytes(), 0);
        assert_eq!(b.peak_bytes(), 256);
    }
    #[test]
    fn fill_and_wrap_preallocated_ring() {
        let raw = hist_bits(true);
        let b = EntropyBudget::new(2 + WINDOW);
        let mut br = BitReader::new(&raw);
        let h = BudgetedHistograms::decode(1, &mut br, true, &mut Vec::new(), &b).unwrap();
        let mut c = EntropyScratch::new(&b);
        let mut r = SymbolReader::new_with_budgeted_scratch(&h, &mut br, None, &mut c).unwrap();
        for _ in 0..(1 << 20) + 17 {
            assert_eq!(r.read_unsigned(&h, &mut br, 0), 0);
        }
        r.finish_with_budgeted_scratch(&h, &mut br, &mut c).unwrap();
        assert_eq!(b.live_bytes(), 2 + WINDOW);
    }
}
