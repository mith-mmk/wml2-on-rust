use std::fs;
use std::path::PathBuf;

use wml2::draw::{image_from_file, image_load};
use wml2::metadata::DataMap;

fn sample_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test")
        .join("samples")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn sample_bytes(name: &str) -> Vec<u8> {
    fs::read(sample_path(name)).unwrap()
}

fn animated_sample_bytes() -> Vec<u8> {
    vec![
        82, 73, 70, 70, 192, 0, 0, 0, 87, 69, 66, 80, 86, 80, 56, 88, 10, 0, 0, 0, 2, 0, 0, 0, 3,
        0, 0, 3, 0, 0, 65, 78, 73, 77, 6, 0, 0, 0, 255, 255, 255, 255, 1, 0, 65, 78, 77, 70, 72, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 3, 0, 0, 100, 0, 0, 2, 86, 80, 56, 32, 48, 0, 0, 0, 208,
        1, 0, 157, 1, 42, 4, 0, 4, 0, 2, 0, 52, 37, 160, 2, 116, 186, 1, 248, 0, 3, 176, 0, 254,
        240, 232, 247, 255, 32, 185, 97, 117, 200, 215, 255, 32, 63, 227, 42, 124, 101, 79, 248,
        242, 0, 0, 0, 65, 78, 77, 70, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 3, 0, 0, 100, 0, 0,
        0, 86, 80, 56, 32, 44, 0, 0, 0, 148, 1, 0, 157, 1, 42, 4, 0, 4, 0, 0, 0, 52, 37, 160, 2,
        116, 186, 0, 3, 152, 0, 254, 249, 147, 111, 255, 144, 31, 255, 144, 31, 255, 144, 31, 255,
        32, 63, 226, 23, 123, 32, 48, 0,
    ]
}

fn assert_webp_metadata(image: &wml2::draw::ImageBuffer, width: usize, height: usize, codec: &str) {
    let metadata = image.metadata.as_ref().unwrap();
    assert!(matches!(
        metadata.get("Format"),
        Some(DataMap::Ascii(format)) if format == "WEBP"
    ));
    assert!(matches!(
        metadata.get("width"),
        Some(DataMap::UInt(actual)) if *actual == width as u64
    ));
    assert!(matches!(
        metadata.get("height"),
        Some(DataMap::UInt(actual)) if *actual == height as u64
    ));
    assert!(matches!(
        metadata.get("WebP codec"),
        Some(DataMap::Ascii(actual)) if actual == codec
    ));
}

#[test]
fn decode_webp_still_samples_from_file() {
    let cases = [
        ("sample.webp", 1920, 1080, "Lossy"),
        ("sample_lossy.webp", 1152, 896, "Lossy"),
        ("sample_lossless.webp", 1152, 896, "Lossless"),
    ];

    for (name, width, height, codec) in cases {
        let image = image_from_file(sample_path(name)).unwrap();
        assert_eq!(image.width, width);
        assert_eq!(image.height, height);
        assert!(
            image
                .buffer
                .as_ref()
                .map(|buffer| !buffer.is_empty())
                .unwrap_or(false)
        );
        assert_webp_metadata(&image, width, height, codec);
    }
}

#[test]
fn decode_webp_still_samples_from_bytes() {
    let cases = [
        ("sample.webp", 1920, 1080, "Lossy"),
        ("sample_lossy.webp", 1152, 896, "Lossy"),
        ("sample_lossless.webp", 1152, 896, "Lossless"),
    ];

    for (name, width, height, codec) in cases {
        let bytes = sample_bytes(name);
        let image = image_load(&bytes).unwrap();
        assert_eq!(image.width, width);
        assert_eq!(image.height, height);
        assert!(
            image
                .buffer
                .as_ref()
                .map(|buffer| !buffer.is_empty())
                .unwrap_or(false)
        );
        assert_webp_metadata(&image, width, height, codec);
    }
}

#[test]
fn decode_animated_webp_and_collect_frames() {
    let bytes = animated_sample_bytes();
    let image = image_load(&bytes).unwrap();

    assert_eq!(image.width, 4);
    assert_eq!(image.height, 4);
    assert_eq!(image.first_wait_time, Some(100));
    assert_eq!(image.animation.as_ref().map(|frames| frames.len()), Some(2));

    let metadata = image.metadata.as_ref().unwrap();
    assert!(matches!(
        metadata.get("WebP animated"),
        Some(DataMap::Ascii(flag)) if flag == "true"
    ));
    assert!(matches!(
        metadata.get("Animation frames"),
        Some(DataMap::UInt(count)) if *count == 2
    ));
    assert!(matches!(
        metadata.get("Animation frame durations"),
        Some(DataMap::UIntAllay(durations)) if durations == &vec![100, 100]
    ));
}
