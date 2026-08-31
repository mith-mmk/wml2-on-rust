use std::panic::{AssertUnwindSafe, catch_unwind};

use wml2::highres::*;

#[test]
fn huge_interleaved_channel_count_fails_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        PlaneLayout::interleaved(1, 1, usize::MAX, Subsampling::FULL)
    }));
    assert!(result.is_ok(), "huge channel count must be fail-closed");
    assert!(result.unwrap().is_err());
    assert!(PlaneLayout::interleaved(1, 1, 5, Subsampling::FULL).is_err());
}
