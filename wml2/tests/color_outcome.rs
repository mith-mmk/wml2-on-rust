#[cfg(feature = "jpeg-xl")]
#[test]
fn jxl_reconstruction_and_managed_output_are_distinct() {
    use wml2::color_outcome::*;
    let input = include_bytes!("fixtures/review/jxl-icc-lossless.jxl");
    let image = wml2::draw::image_from(input).unwrap();
    let metadata = image.metadata.as_ref().unwrap();
    assert_eq!(
        ColorOutcome::from_metadata(metadata.get(&metadata_key(0))),
        ColorOutcome {
            conversion: Conversion::NotApplied(NotAppliedReason::ReconstructionPreserved),
            output: OutputColorSpace::Reconstruction
        }
    );
    assert!(metadata.contains_key("JPEG XL Source ICC Profile"));
    assert!(metadata.contains_key("ICC Profile"));
    #[cfg(feature = "color-management")]
    {
        let image = wml2::jpegxl::image_from_srgb(input).unwrap();
        let metadata = image.metadata.as_ref().unwrap();
        assert_eq!(
            ColorOutcome::from_metadata(metadata.get(&metadata_key(0))),
            ColorOutcome {
                conversion: Conversion::AppliedToSrgb,
                output: OutputColorSpace::Srgb
            }
        );
        for key in [
            "JPEG XL Source ICC Profile",
            "JPEG XL Reconstruction ICC Profile",
            "ICC Profile",
        ] {
            assert!(metadata.contains_key(key));
        }
        assert!(
            wml2::jpegxl::image_from_srgb_with_limits_and_cancel(
                input,
                Default::default(),
                &|| true
            )
            .is_err()
        );
        assert!(wml2::jpegxl::image_from_srgb(&input[..input.len() / 2]).is_err());
    }
}

#[cfg(feature = "jpeg-xl")]
#[test]
fn jxl_animation_records_match_poster_and_playback_frames() {
    use wml2::color_outcome::*;
    let input = include_bytes!("fixtures/review/jxl-animation-xyb-icc16.jxl");
    let ordinary = wml2::draw::image_from(input).unwrap();
    let metadata = ordinary.metadata.as_ref().unwrap();
    let count = ordinary.animation.as_ref().unwrap().len();
    assert_eq!(count, 3);
    for i in 0..=count {
        assert_eq!(
            ColorOutcome::from_metadata(metadata.get(&metadata_key(i))).conversion,
            Conversion::NotApplied(NotAppliedReason::ReconstructionPreserved)
        );
    }
    assert_eq!(
        ColorOutcome::from_metadata(metadata.get(&metadata_key(count + 1))),
        ColorOutcome::default()
    );
    #[cfg(feature = "color-management")]
    {
        let managed = wml2::jpegxl::image_from_srgb(input).unwrap();
        let metadata = managed.metadata.as_ref().unwrap();
        for i in 0..=count {
            assert_eq!(
                ColorOutcome::from_metadata(metadata.get(&metadata_key(i))).conversion,
                Conversion::AppliedToSrgb
            );
        }
    }
}

#[cfg(all(feature = "jpeg-xl", feature = "color-management"))]
#[test]
fn cancelled_owned_decode_never_returns_success_at_any_probe() {
    use std::cell::Cell;
    let input = include_bytes!("fixtures/review/jxl-icc-lossless.jxl");
    let calls = Cell::new(0);
    wml2::jpegxl::image_from_srgb_with_limits_and_cancel(input, Default::default(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        let result = wml2::jpegxl::image_from_srgb_with_limits_and_cancel(
            input,
            Default::default(),
            &|| {
                count.set(count.get() + 1);
                count.get() >= stop
            },
        );
        assert!(
            result.is_err(),
            "cancel probe {stop} returned pixels/outcome"
        );
    }
}

#[cfg(feature = "jpeg-xl")]
#[test]
fn outcome_abort_stops_jxl_static_and_animation_callbacks() {
    use wml2::draw::*;
    #[derive(Default)]
    struct Probe {
        stopped: bool,
        ends: usize,
    }
    impl DrawCallback for Probe {
        fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
            assert!(!self.stopped);
            Ok(None)
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
            assert!(!self.stopped);
            Ok(None)
        }
        fn next(&mut self, _: Option<NextOptions>) -> Response {
            assert!(!self.stopped);
            Ok(None)
        }
        fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
            assert!(!self.stopped);
            Ok(None)
        }
        fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
            self.ends += 1;
            Ok(None)
        }
        fn set_metadata(&mut self, key: &str, _: wml2::metadata::DataMap) -> Response {
            assert!(!self.stopped, "metadata after Abort");
            if key == wml2::color_outcome::metadata_key(0) {
                self.stopped = true;
                Ok(Some(CallbackResponse::abort()))
            } else {
                Ok(None)
            }
        }
    }
    for input in [
        include_bytes!("fixtures/review/jxl-icc-lossless.jxl").as_slice(),
        include_bytes!("fixtures/review/jxl-animation-xyb-icc16.jxl").as_slice(),
    ] {
        let mut probe = Probe::default();
        image_loader(
            input,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut probe,
            },
        )
        .unwrap();
        assert!(probe.stopped);
        assert_eq!(probe.ends, 1);
    }
}
