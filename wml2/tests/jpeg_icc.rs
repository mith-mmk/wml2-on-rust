#![cfg(feature = "jpeg")]

use bin_rs::reader::BytesReader;
use wml2::draw::{ImageBuffer, image_from, image_from_with_limits};
use wml2::jpeg::header::{JpegAppHeaders, JpegHaeder};
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;

// Independently encoded by Pillow. ICC parsing must not alter its raster.
const JPEG: &[u8] = include_bytes!("fixtures/review/jpeg-icc-base.jpg");
const PROFILE: &[u8] = include_bytes!("fixtures/review/linear-rgb.icc");

fn chunk(number: u8, total: u8, data: &[u8]) -> Vec<u8> {
    let mut payload = b"ICC_PROFILE\0".to_vec();
    payload.extend_from_slice(&[number, total]);
    payload.extend_from_slice(data);
    payload
}

fn jpeg(payloads: &[Vec<u8>]) -> Vec<u8> {
    let mut data = JPEG[..2].to_vec();
    for payload in payloads {
        data.extend_from_slice(&[0xff, 0xe2]);
        data.extend_from_slice(&u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
        data.extend_from_slice(payload);
    }
    data.extend_from_slice(&JPEG[2..]);
    data
}

fn profile(image: &ImageBuffer) -> Option<&[u8]> {
    match image.metadata.as_ref().and_then(|m| m.get("ICC Profile")) {
        Some(DataMap::ICCProfile(data)) => Some(data),
        _ => None,
    }
}

fn unchanged_raster(image: &ImageBuffer) {
    let base = image_from(JPEG).unwrap();
    assert_eq!((image.width, image.height), (base.width, base.height));
    assert_eq!(image.buffer, base.buffer);
}

#[test]
fn truncated_icc_envelopes_do_not_panic_or_publish_a_profile() {
    let envelope = chunk(1, 1, &[]);
    for length in 0..envelope.len() {
        let data = jpeg(&[envelope[..length].to_vec()]);
        let image = image_from(&data).unwrap();
        unchanged_raster(&image);
        assert!(profile(&image).is_none(), "payload length {length}");
    }
}

#[test]
fn invalid_outer_app_lengths_return_errors() {
    for length in [0u16, 1] {
        let mut data = JPEG[..2].to_vec();
        data.extend_from_slice(&[0xff, 0xe2]);
        data.extend_from_slice(&length.to_be_bytes());
        data.extend_from_slice(&JPEG[2..]);
        assert!(image_from(&data).is_err());
    }
}

#[test]
fn valid_single_and_reordered_split_profiles_preserve_exact_bytes_and_pixels() {
    for payloads in [
        vec![chunk(1, 1, PROFILE)],
        vec![chunk(1, 2, &PROFILE[..100]), chunk(2, 2, &PROFILE[100..])],
        vec![chunk(2, 2, &PROFILE[100..]), chunk(1, 2, &PROFILE[..100])],
    ] {
        let data = jpeg(&payloads);
        let image = image_from(&data).unwrap();
        unchanged_raster(&image);
        assert_eq!(profile(&image), Some(PROFILE));
        let header = JpegHaeder::new(&mut BytesReader::new(&data), 0).unwrap();
        assert_eq!(header.icc_profile.as_deref(), Some(PROFILE));
        assert_eq!(
            header
                .jpeg_app_headers
                .unwrap()
                .iter()
                .filter(|h| matches!(h, JpegAppHeaders::ICCProfile(_)))
                .count(),
            payloads.len()
        );
    }
}

#[test]
fn invalid_chunk_sets_discard_only_the_profile_even_after_a_complete_set() {
    let first = chunk(1, 2, &PROFILE[..100]);
    let second = chunk(2, 2, &PROFILE[100..]);
    for payloads in [
        vec![first.clone()],
        vec![second.clone()],
        vec![first.clone(), first.clone()],
        vec![first.clone(), chunk(2, 3, &PROFILE[100..])],
        vec![chunk(0, 1, PROFILE)],
        vec![chunk(1, 0, PROFILE)],
        vec![chunk(2, 1, PROFILE)],
        vec![first.clone(), second.clone(), first.clone()],
        vec![first.clone(), second.clone(), b"ICC_PROFILE\0".to_vec()],
        vec![b"ICC_PROFILE\0".to_vec(), first.clone(), second.clone()],
    ] {
        let image = image_from(&jpeg(&payloads)).unwrap();
        unchanged_raster(&image);
        assert!(profile(&image).is_none());
    }
}

#[test]
fn unrelated_app2_does_not_poison_a_valid_profile() {
    let image = image_from(&jpeg(&[
        chunk(2, 2, &PROFILE[100..]),
        b"OTHER\0opaque payload".to_vec(),
        chunk(1, 2, &PROFILE[..100]),
    ]))
    .unwrap();
    unchanged_raster(&image);
    assert_eq!(profile(&image), Some(PROFILE));
}

#[test]
fn all_255_sequence_numbers_are_supported_for_opaque_profile_bytes() {
    let opaque: Vec<u8> = (0..255).collect();
    let payloads: Vec<_> = opaque
        .iter()
        .enumerate()
        .rev()
        .map(|(i, b)| chunk(u8::try_from(i + 1).unwrap(), 255, &[*b]))
        .collect();
    let image = image_from(&jpeg(&payloads)).unwrap();
    unchanged_raster(&image);
    assert_eq!(profile(&image), Some(opaque.as_slice()));
}

#[test]
fn metadata_budget_includes_retained_chunks_and_assembled_profile() {
    let data = jpeg(&[chunk(2, 2, &PROFILE[100..]), chunk(1, 2, &PROFILE[..100])]);
    let required = 2 * PROFILE.len() + 2 * 14;
    let limits = DecodeLimits {
        metadata_bytes: required,
        ..DecodeLimits::unlimited()
    };
    let image = image_from_with_limits(&data, limits).unwrap();
    assert_eq!(profile(&image), Some(PROFILE));
    let error = image_from_with_limits(
        &data,
        DecodeLimits {
            metadata_bytes: required - 1,
            ..limits
        },
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("metadata exceeds decode limit"));
}

#[test]
fn empty_duplicate_envelopes_are_not_free_metadata() {
    let data = jpeg(&vec![chunk(1, 1, &[]); 3]);
    let error = image_from_with_limits(
        &data,
        DecodeLimits {
            metadata_bytes: 2 * 14,
            ..DecodeLimits::unlimited()
        },
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("metadata exceeds decode limit"));
}
