#![cfg(feature = "jpeg")]
use std::path::PathBuf;
use wml2::draw::*;
use wml2::limits::DecodeLimits;

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/review")
        .join(name)
}
fn decode(name: &str) -> ImageBuffer {
    image_from(&std::fs::read(path(name)).unwrap())
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn restart_intervals_preserve_every_pixel_in_baseline_and_progressive() {
    for prefix in [
        "minimal", "b444", "b420", "odd420", "p444", "p420", "bgray", "pgray",
    ] {
        let plain = decode(&format!("restart-{prefix}-i0.jpg"));
        for suffix in ["i1", "i2", "i3", "i8", "i100", "row"] {
            let name = format!("restart-{prefix}-{suffix}.jpg");
            let restarted = decode(&name);
            assert_eq!(
                (restarted.width, restarted.height),
                (plain.width, plain.height),
                "{name}"
            );
            assert!(restarted.buffer == plain.buffer, "pixel mismatch: {name}");
        }
    }
}

fn markers(bytes: &[u8]) -> Vec<usize> {
    bytes
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            (pair[0] == 0xff && (0xd0..=0xd7).contains(&pair[1])).then_some(index)
        })
        .collect()
}

#[test]
fn restart_sequence_wrap_missing_marker_and_truncation_are_checked() {
    for prefix in ["b444", "b420", "p444", "p420"] {
        let bytes = std::fs::read(path(&format!("restart-{prefix}-i1.jpg"))).unwrap();
        let positions = markers(&bytes);
        assert!(positions.len() > 8);
        assert_eq!(bytes[positions[0] + 1], 0xd0);
        assert_eq!(bytes[positions[8] + 1], 0xd0);
        for index in [0, 8] {
            let mut changed = bytes.clone();
            changed[positions[index] + 1] = 0xd3;
            let error = image_from(&changed).err().unwrap().to_string();
            assert!(error.contains("Expected RST0"), "{prefix}: {error}");
        }
        for length in [positions[0], positions[0] + 1] {
            assert!(image_from(&bytes[..length]).is_err(), "{prefix}");
        }
        let mut missing = bytes.clone();
        missing.drain(positions[0]..positions[0] + 2);
        assert!(image_from(&missing).is_err(), "{prefix}");
        assert!(
            image_from_with_limits(
                &bytes,
                DecodeLimits {
                    pixels: 128 * 48 - 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            image_from_with_limits(
                &bytes,
                DecodeLimits {
                    expanded_bytes: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}

#[test]
fn progressive_dri_between_scans_requires_a_four_byte_segment() {
    let mut bytes = std::fs::read(path("restart-p420-row.jpg")).unwrap();
    let sos = bytes
        .windows(2)
        .position(|pair| pair == [0xff, 0xda])
        .unwrap();
    let dri = bytes
        .windows(2)
        .enumerate()
        .find(|(index, pair)| *index > sos && *pair == [0xff, 0xdd])
        .unwrap()
        .0;
    bytes[dri + 3] = 5;
    assert!(
        image_from(&bytes)
            .err()
            .unwrap()
            .to_string()
            .contains("Invalid progressive DRI length")
    );
}

#[derive(Default)]
struct Stop {
    fail: bool,
    draws: usize,
    terms: usize,
}
impl DrawCallback for Stop {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
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
        self.draws += 1;
        assert_eq!(self.draws, 1);
        if self.fail {
            Err(std::io::Error::other("draw failed deliberately").into())
        } else {
            Ok(Some(CallbackResponse::abort()))
        }
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.terms += 1;
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, _: &str, _: wml2::metadata::DataMap) -> Response {
        Ok(None)
    }
}

#[test]
fn restart_decode_abort_and_draw_failure_join_the_worker() {
    let bytes = std::fs::read(path("restart-b444-i1.jpg")).unwrap();
    for fail in [false, true] {
        let mut stop = Stop {
            fail,
            ..Default::default()
        };
        let result = image_loader(
            &bytes,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut stop,
            },
        );
        if fail {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("draw failed deliberately")
            );
        } else {
            assert!(result.is_ok());
        }
        // Existing callback contract: Abort terminates once; an ordinary draw
        // error propagates directly, without a synthesized terminate callback.
        assert_eq!((stop.draws, stop.terms), (1, usize::from(!fail)));
    }
}
