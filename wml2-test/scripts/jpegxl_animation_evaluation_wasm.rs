//! Actual wasm32 execution of the trusted-fixture decoder-core experiment.
#[path = "jpegxl_animation_core.rs"]
mod core;
#[unsafe(no_mangle)]
pub extern "C" fn verify_jxl_animation_evaluation() -> u32 {
    let mut checks = 0;
    let cases = include!("jpegxl_animation_evaluation_cases.rs");
    for (input, expected, ticks, num, den, loops, size) in cases {
        let decoded = core::decode(input).unwrap();
        assert_eq!(decoded.size, size);
        assert_eq!(
            (decoded.numerator, decoded.denominator, decoded.loops),
            (num, den, loops)
        );
        assert_eq!(decoded.frames.len(), 3);
        assert!(decoded.profile.len() >= 128);
        checks += 1;
        for (i, (frame, oracle)) in decoded.frames.iter().zip(expected).enumerate() {
            assert_eq!(frame.ticks, ticks[i]);
            assert_eq!(frame.last, i == 2);
            let expected_ms =
                (u128::from(ticks[i]) * 1000 * u128::from(den)) as f64 / f64::from(num);
            assert!((frame.milliseconds - expected_ms).abs() < 1e-9);
            assert_eq!(frame.pixels.len(), oracle.len());
            for (a, b) in frame.pixels.chunks_exact(2).zip(oracle.chunks_exact(2)) {
                assert!(
                    u16::from_le_bytes([a[0], a[1]]).abs_diff(u16::from_le_bytes([b[0], b[1]]))
                        <= 1
                );
            }
            checks += 1;
        }
    }
    checks
}
