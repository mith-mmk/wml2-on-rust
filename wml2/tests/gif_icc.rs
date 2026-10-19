#![cfg(feature = "gif")]

use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;

const PROFILE: &[u8] = include_bytes!("fixtures/review/linear-rgb.icc");
const IDENTIFIER: &[u8] = b"ICCRGBG1012";

// Independent clear/index/end LZW streams, not produced by WML2's encoder.
fn gif(animated: bool) -> Vec<u8> {
    let mut bytes = b"GIF89a\x01\0\x01\0\x80\0\0\x80\x60\x40\x10\x20\x30".to_vec();
    bytes.extend_from_slice(b"!\xff\x0bNETSCAPE2.0\x03\x01\x03\0\0");
    let indices: &[u8] = if animated { &[0, 1, 0] } else { &[0] };
    for (i, index) in indices.iter().enumerate() {
        bytes.extend_from_slice(&[b'!', 0xf9, 4, 1, i as u8 + 2, 0, 1, 0]);
        bytes.extend_from_slice(b",\0\0\0\0\x01\0\x01\0\0\x02\x02");
        bytes.extend_from_slice(&[0x44 | (index << 3), 1, 0]);
    }
    bytes.push(b';');
    bytes
}

fn extension(identifier: &[u8], profile: &[u8], block_size: usize) -> Vec<u8> {
    let mut data = vec![b'!', 0xff, identifier.len() as u8];
    data.extend_from_slice(identifier);
    for part in profile.chunks(block_size) {
        data.push(part.len() as u8);
        data.extend_from_slice(part);
    }
    data.push(0);
    data
}

fn tagged(animated: bool, profile: &[u8], size: usize, trailing: bool) -> Vec<u8> {
    let mut bytes = gif(animated);
    let at = if trailing { bytes.len() - 1 } else { 19 };
    bytes.splice(at..at, extension(IDENTIFIER, profile, size));
    bytes
}

fn source(image: &ImageBuffer) -> Option<&[u8]> {
    match image
        .metadata
        .as_ref()
        .and_then(|m| m.get("Source ICC Profile"))
    {
        Some(DataMap::ICCProfile(bytes)) => Some(bytes),
        _ => None,
    }
}

fn unchanged(image: &ImageBuffer, plain: &ImageBuffer) {
    assert_eq!((image.width, image.height), (plain.width, plain.height));
    assert_eq!(image.buffer, plain.buffer);
    assert_eq!(image.loop_count, plain.loop_count);
    assert_eq!(image.first_wait_time, plain.first_wait_time);
    let frames = |image: &ImageBuffer| {
        image
            .animation
            .as_ref()
            .unwrap()
            .iter()
            .map(|frame| {
                (
                    frame.width,
                    frame.height,
                    frame.start_x,
                    frame.start_y,
                    frame.buffer.clone(),
                    frame.control.await_time,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(frames(image), frames(plain));
    assert!(!image.metadata.as_ref().unwrap().contains_key("ICC Profile"));
}

#[test]
fn single_icc_reassembles_exact_source_bytes_without_changing_pixels_or_animation() {
    for animated in [false, true] {
        let plain = image_from(&gif(animated)).unwrap();
        for block_size in [1, 7, 127, 255] {
            for trailing in [false, true] {
                let image = image_from(&tagged(animated, PROFILE, block_size, trailing)).unwrap();
                unchanged(&image, &plain);
                assert_eq!(source(&image), Some(PROFILE));
                assert_eq!(
                    image.metadata.as_ref().unwrap()["Animation GIF"],
                    DataMap::Ascii("NETSCAPE2.0".into())
                );
            }
        }
    }
}

#[test]
fn opaque_and_empty_profile_bytes_are_retained_without_a_cms_validation_gate() {
    for profile in [b"opaque unsupported ICC\0\xff".as_slice(), &[]] {
        let image = image_from(&tagged(false, profile, 255, false)).unwrap();
        assert_eq!(source(&image), Some(profile));
        unchanged(&image, &image_from(&gif(false)).unwrap());
    }
}

#[test]
fn exact_identifier_and_authentication_are_required_without_affecting_other_apps() {
    for identifier in [
        b"ICCRGBG1013".as_slice(),
        b"ICCRGBG1012x",
        b"ICCRGBG1",
        b"UNRELATED00",
    ] {
        let mut bytes = gif(true);
        bytes.splice(19..19, extension(identifier, PROFILE, 255));
        let image = image_from(&bytes).unwrap();
        assert!(source(&image).is_none());
        unchanged(&image, &image_from(&gif(true)).unwrap());
    }
}

#[test]
fn multiple_carriers_are_distinct_source_records_not_one_concatenated_profile() {
    let mut bytes = gif(true);
    bytes.splice(19..19, extension(IDENTIFIER, PROFILE, 255));
    let at = bytes.len() - 1;
    bytes.splice(at..at, extension(IDENTIFIER, b"second source", 3));
    let image = image_from(&bytes).unwrap();
    assert!(source(&image).is_none());
    let metadata = image.metadata.as_ref().unwrap();
    assert_eq!(
        metadata["GIF Source ICC Profile 1"],
        DataMap::ICCProfile(PROFILE.to_vec())
    );
    assert_eq!(
        metadata["GIF Source ICC Profile 2"],
        DataMap::ICCProfile(b"second source".to_vec())
    );
    unchanged(&image, &image_from(&gif(true)).unwrap());
}

#[test]
fn carrier_records_and_source_bytes_share_one_aggregate_metadata_budget() {
    // The contract charges the 11-byte carrier, retained Vec record, payload,
    // and published metadata key. Sub-block boundaries do not change that cost.
    let exact = 11 + std::mem::size_of::<Vec<u8>>() + PROFILE.len() + "Source ICC Profile".len();
    for block_size in [1, 255] {
        let bytes = tagged(false, PROFILE, block_size, false);
        let image = image_from_with_limits(
            &bytes,
            DecodeLimits {
                metadata_bytes: exact,
                ..DecodeLimits::default()
            },
        )
        .unwrap();
        assert_eq!(source(&image), Some(PROFILE));
        assert!(
            image_from_with_limits(
                &bytes,
                DecodeLimits {
                    metadata_bytes: exact - 1,
                    ..DecodeLimits::default()
                }
            )
            .err()
            .unwrap()
            .to_string()
            .contains("metadata")
        );
    }
    let mut bytes = gif(false);
    for _ in 0..32 {
        bytes.splice(19..19, extension(IDENTIFIER, &[], 255));
    }
    assert!(
        image_from_with_limits(
            &bytes,
            DecodeLimits {
                metadata_bytes: 31 * (11 + std::mem::size_of::<Vec<u8>>()),
                ..DecodeLimits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn truncated_icc_subblocks_fail_without_a_partial_source_declaration() {
    let extension = extension(IDENTIFIER, PROFILE, 255);
    for length in 3..extension.len() {
        let mut bytes = gif(false)[..19].to_vec();
        bytes.extend_from_slice(&extension[..length]);
        let mut image = ImageBuffer::new();
        let result = image_decoder(
            &mut BytesReader::new(&bytes),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
        );
        assert!(result.is_err(), "length={length}");
        assert!(source(&image).is_none());
    }
}

#[test]
fn reuse_does_not_carry_profile_to_an_unprofiled_gif() {
    let mut image = ImageBuffer::new();
    for bytes in [tagged(false, PROFILE, 255, false), gif(false)] {
        image_decoder(
            &mut BytesReader::new(&bytes),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
        )
        .unwrap();
    }
    assert!(source(&image).is_none());
}

#[derive(Default)]
struct SourceProbe {
    fail: bool,
    aborted: bool,
    sources: usize,
    draws: usize,
    terms: usize,
}
impl DrawCallback for SourceProbe {
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
        assert!(!self.aborted);
        self.draws += 1;
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        assert!(!self.aborted);
        Ok(None)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.terms += 1;
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, key: &str, value: DataMap) -> Response {
        assert!(!self.aborted, "metadata callback after Abort");
        if key.starts_with("GIF Source ICC Profile") || key == "Source ICC Profile" {
            assert_eq!(self.draws, 3, "source delivery follows complete animation");
            assert_eq!(value, DataMap::ICCProfile(PROFILE.to_vec()));
            self.sources += 1;
            if self.fail {
                return Err(std::io::Error::other("source metadata failure").into());
            }
            self.aborted = true;
            return Ok(Some(CallbackResponse::abort()));
        }
        Ok(None)
    }
}

#[test]
fn source_metadata_abort_stops_multiple_carrier_delivery_and_errors_propagate() {
    for multiple in [false, true] {
        let mut bytes = tagged(true, PROFILE, 255, false);
        if multiple {
            bytes.splice(19..19, extension(IDENTIFIER, PROFILE, 7));
        }
        for direct in [false, true] {
            for fail in [false, true] {
                let mut probe = SourceProbe {
                    fail,
                    ..Default::default()
                };
                let mut reader = BytesReader::new(&bytes);
                let mut options = DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut probe,
                };
                let result = if direct {
                    wml2::gif::decoder::decode(&mut reader, &mut options)
                } else {
                    image_decoder(&mut reader, &mut options)
                };
                assert_eq!(probe.sources, 1);
                if fail {
                    assert_eq!(result.unwrap_err().to_string(), "source metadata failure");
                } else {
                    assert!(result.is_ok());
                    assert_eq!(probe.terms, 1);
                }
            }
        }
    }
}
