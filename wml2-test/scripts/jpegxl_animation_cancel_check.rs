//! Public-API composition check, compiled only against isolated PR32+PR62.
//! No animation support or mid-animation interruption is claimed.
use std::cell::Cell;
use wml2::jpegxl::{DecodeCancelled, image_from_srgb_with_limits_and_cancel as decode};
use wml2::limits::DecodeLimits;
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let mut checkpoints = 0;
    let mut cases = 0;
    for name in [
        "variable",
        "fractional",
        "reference",
        "internal",
        "timecodes",
        "long",
        "icc16",
        "icc16-variable",
        "preview",
    ] {
        let data = std::fs::read(dir.join(format!("jxl-animation-{name}.jxl"))).unwrap();
        let calls = Cell::new(0);
        let error = decode(&data, DecodeLimits::default(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .err()
        .unwrap();
        assert!(error.downcast_ref::<DecodeCancelled>().is_none());
        assert!(error.to_string().contains("animation"));
        let total = calls.get();
        for target in 1..=total {
            let calls = Cell::new(0);
            let error = decode(&data, DecodeLimits::default(), &|| {
                calls.set(calls.get() + 1);
                calls.get() >= target
            })
            .err()
            .unwrap();
            assert!(error.downcast_ref::<DecodeCancelled>().is_some());
            checkpoints += 1;
        }
        let limits = DecodeLimits {
            input_bytes: 0,
            pixels: 0,
            expanded_bytes: 0,
            metadata_bytes: 0,
            frames: 0,
            animation_bytes: 0,
        };
        assert!(
            decode(&data, limits, &|| true)
                .err()
                .unwrap()
                .downcast_ref::<DecodeCancelled>()
                .is_some()
        );
        assert!(
            decode(&data, limits, &|| false)
                .err()
                .unwrap()
                .downcast_ref::<DecodeCancelled>()
                .is_none()
        );
        cases += 1;
    }
    println!(
        "animation_refusals={cases} deterministic_precancel_checkpoints={checkpoints} limit_precedence_cases={cases}; no animation frame decoded/published"
    );
}
