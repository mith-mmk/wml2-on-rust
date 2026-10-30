//! Compile only against isolated PR32 + PR62. Checks BUFFERED cancellation,
//! never an incremental session or interruption of serial parser sections.
use std::cell::Cell;
use wml2::jpegxl::{DecodeCancelled, image_from_srgb_with_limits_and_cancel as decode};
use wml2::limits::DecodeLimits;
fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let mut checks = 0;
    let mut cases = 0;
    for source in [
        "ac",
        "ac-jxlc",
        "ac-jxlp",
        "ac-trailer",
        "dc",
        "dc-jxlc",
        "dc-jxlp",
        "dc-trailer",
    ] {
        let data = std::fs::read(root.join(format!("jxl-progressive-{source}.jxl"))).unwrap();
        let calls = Cell::new(0);
        let image = decode(&data, DecodeLimits::default(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
        let actual = image.buffer.unwrap();
        let oracle = std::fs::read(root.join(format!(
            "jxl-progressive-{}.rgba16le",
            source.split('-').next().unwrap()
        )))
        .unwrap();
        assert_eq!(actual.len() * 2, oracle.len());
        for (i, (a, b)) in actual.iter().zip(oracle.chunks_exact(2)).enumerate() {
            let expected =
                ((u32::from(u16::from_le_bytes([b[0], b[1]])) * 255 + 32767) / 65535) as u8;
            if i % 4 == 3 {
                assert_eq!(*a, expected);
            } else {
                assert!(a.abs_diff(expected) <= 2);
            }
        }
        for target in 1..=calls.get() {
            let observed = Cell::new(0);
            let error = decode(&data, DecodeLimits::default(), &|| {
                observed.set(observed.get() + 1);
                observed.get() >= target
            })
            .err()
            .unwrap();
            assert!(error.downcast_ref::<DecodeCancelled>().is_some());
            checks += 1;
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
        "buffered_progressive_cases={cases} deterministic_cancel_checkpoints={checks} limit_precedence_cases={cases}; no cancelled result returned; no streaming/call-latency claim"
    );
}
