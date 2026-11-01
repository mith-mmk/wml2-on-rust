#![cfg(feature = "jpeg-xl")]
use wml2::{jpegxl::*, limits::DecodeLimits};
fn decode(input: &[u8]) -> JxlAnimationImage {
    animation_rgba16_from_with_resource_limits_and_cancel(
        input,
        DecodeLimits::default(),
        JxlResourceLimits::default(),
        &|| false,
    )
    .unwrap()
}
#[test]
fn every_coalesced_canvas_and_raw_tick_matches_independent_c() {
    for (input, expected, ticks, num, den, loops, o, bits) in include!("jpeg_xl/animation_cases.rs")
    {
        let im = decode(input);
        let size = im.dimensions.output.unwrap();
        assert_eq!(
            (size.width, size.height),
            if o >= 5 { (5, 7) } else { (7, 5) }
        );
        assert_eq!(im.source_orientation, o);
        assert_eq!(im.bits_per_sample, bits);
        assert_eq!(
            (
                im.ticks_per_second_numerator,
                im.ticks_per_second_denominator,
                im.loop_count
            ),
            (num, den, loops)
        );
        assert_eq!(im.frames.len(), expected.len());
        assert_eq!(im.frames.len(), ticks.len());
        for ((frame, expected), ticks) in im.frames.iter().zip(expected).zip(ticks) {
            assert_eq!(frame.duration_ticks, *ticks);
            let gold: Vec<_> = expected
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
                .collect();
            assert_eq!(frame.rgba16.len(), gold.len());
            for (a, b) in frame.rgba16.iter().zip(gold) {
                assert!(a.abs_diff(b) <= 1, "{bits}/{o}: {a}/{b}")
            }
        }
        assert!(wml2::draw::image_from(input).is_err());
    }
}
#[path = "jpeg_xl/allocation_tracking.rs"]
mod allocation_tracking;
#[global_allocator]
static ALLOCATOR: allocation_tracking::Allocator = allocation_tracking::Allocator;
#[test]
fn all_output_layers_and_metadata_have_joint_preallocation_limits() {
    let input = include_bytes!("fixtures/review/jxl-animation-main5-internal.jxl");
    let run = |limits, policy| {
        animation_rgba16_from_with_resource_limits_and_cancel(input, limits, policy, &|| false)
    };
    for limits in [
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            pixels: 34,
            ..Default::default()
        },
        DecodeLimits {
            frames: 2,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 839,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(run(limits, JxlResourceLimits::default()).is_err());
    }
    assert!(
        run(
            DecodeLimits {
                frames: 3,
                animation_bytes: 840,
                ..Default::default()
            },
            JxlResourceLimits::default()
        )
        .is_ok()
    );
    for (cap, ok) in [(3, false), (4, true)] {
        assert_eq!(
            run(
                DecodeLimits::default(),
                JxlResourceLimits {
                    internal_layers: cap,
                    ..Default::default()
                }
            )
            .is_ok(),
            ok
        );
    }
    let (mut lo, mut hi) = (0, 64 << 20);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if run(
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: mid,
                ..Default::default()
            },
        )
        .is_ok()
        {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    for (cap, ok) in [(lo - 1, false), (lo, true)] {
        assert_eq!(
            run(
                DecodeLimits::default(),
                JxlResourceLimits {
                    workspace_bytes: cap,
                    ..Default::default()
                }
            )
            .is_ok(),
            ok
        );
    }
    let (mut lo, mut hi) = (0, 1 << 20);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if run(
            DecodeLimits {
                metadata_bytes: mid,
                ..Default::default()
            },
            JxlResourceLimits::default(),
        )
        .is_ok()
        {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    for (cap, ok) in [(lo - 1, false), (lo, true)] {
        assert_eq!(
            run(
                DecodeLimits {
                    metadata_bytes: cap,
                    ..Default::default()
                },
                JxlResourceLimits::default()
            )
            .is_ok(),
            ok
        );
    }
    assert_eq!(allocation_tracking::measure(|| drop(decode(input))), 0);
}
#[test]
fn unsupported_static_preview_smpte_rate_and_layer_count_refuse() {
    for input in [
        include_bytes!("fixtures/review/jxl-rgba8.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-preview.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-timecodes.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-internal.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-icc16.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-icc16-variable.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-main5-layers65.jxl").as_slice(),
    ] {
        assert!(
            animation_rgba16_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
    for input in [
        include_bytes!("fixtures/review/jxl-animation-variable.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-main5-icc16.jxl").as_slice(),
    ] {
        for n in 0..input.len() {
            assert!(
                animation_rgba16_from_with_resource_limits_and_cancel(
                    &input[..n],
                    DecodeLimits::default(),
                    JxlResourceLimits::default(),
                    &|| false
                )
                .is_err()
            );
        }
        let mut trailing = input.to_vec();
        trailing.push(0);
        assert!(
            animation_rgba16_from_with_resource_limits_and_cancel(
                &trailing,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
}
#[test]
fn profiles_and_late_entropy_errors_never_publish_an_animation() {
    let input = include_bytes!("fixtures/review/jxl-animation-main5-icc16.jxl");
    let im = decode(input);
    assert_eq!(
        im.source_icc,
        include_bytes!("fixtures/review/jxl-gamma2.icc")
    );
    assert_eq!(im.output_icc, im.source_icc);
    let mut corrupted = input.to_vec();
    let n = corrupted.len();
    for x in &mut corrupted[n - 12..] {
        *x = 255;
    }
    let probes = std::cell::Cell::new(0);
    assert!(
        animation_rgba16_from_with_resource_limits_and_cancel(
            &corrupted,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| {
                probes.set(probes.get() + 1);
                false
            }
        )
        .is_err()
    );
    eprintln!("Late entropy failure after {} checkpoints", probes.get());
    assert!(probes.get() > 15);
}
#[test]
fn every_real_multiframe_cancellation_checkpoint_drops_private_frames() {
    use std::cell::Cell;
    let input = include_bytes!("fixtures/review/jxl-animation-main5-icc16-variable.jxl");
    let probes = Cell::new(0);
    let run = |cancel: &dyn Fn() -> bool| {
        animation_rgba16_from_with_resource_limits_and_cancel(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            cancel,
        )
    };
    drop(
        run(&|| {
            probes.set(probes.get() + 1);
            false
        })
        .unwrap(),
    );
    let n = probes.get();
    assert!(n > 20);
    eprintln!("Buffered animation cancellation checkpoints: {n}");
    for stop in 1..=n {
        let count = Cell::new(0);
        assert_eq!(
            allocation_tracking::measure(|| {
                let e = run(&|| {
                    count.set(count.get() + 1);
                    count.get() == stop
                })
                .unwrap_err();
                assert!(e.is::<DecodeCancelled>());
                drop(e);
            }),
            0
        );
    }
    assert!(
        animation_rgba16_from_with_resource_limits_and_cancel(
            &[],
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: usize::MAX,
                ..Default::default()
            },
            &|| true
        )
        .unwrap_err()
        .is::<DecodeCancelled>()
    );
    drop(run(&|| false).unwrap());
}
