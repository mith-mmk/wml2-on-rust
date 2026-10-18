#![cfg(all(feature = "jpeg", feature = "png"))]

use std::collections::HashMap;

use wml2::draw::{ImageBuffer, image_from};
use wml2::metadata::DataMap;

// Profile retention is independent of a CMS or ICC semantic validation. Opaque
// bytes deliberately verify that routing uses the raster model, not a new ICC
// parser. The native RGB cases also use the existing real RGB profile fixture.
const SOURCE_PROFILE: &[u8] = b"opaque source ICC bytes\0\xff\x01";
const RGB_PROFILE: &[u8] = include_bytes!("fixtures/review/linear-rgb.icc");
const RGB_JPEG: &[u8] = include_bytes!("fixtures/review/jpeg-icc-base.jpg");

fn profile<'a>(metadata: &'a HashMap<String, DataMap>, key: &str) -> Option<&'a [u8]> {
    match metadata.get(key) {
        Some(DataMap::ICCProfile(bytes)) => Some(bytes),
        _ => None,
    }
}

fn assert_profile(metadata: &HashMap<String, DataMap>, key: &str, expected: &[u8]) {
    assert_eq!(profile(metadata, key), Some(expected));
    let other = if key == "ICC Profile" {
        "Source ICC Profile"
    } else {
        "ICC Profile"
    };
    assert!(!metadata.contains_key(other));
    assert_eq!(
        metadata
            .values()
            .filter(|value| matches!(value, DataMap::ICCProfile(_)))
            .count(),
        1,
        "publish each profile once, under one unambiguous key"
    );
}

fn tagged_jpeg(jpeg: &[u8], profile: &[u8]) -> Vec<u8> {
    let mut output = jpeg[..2].to_vec();
    output.extend_from_slice(&[0xff, 0xe2]);
    output.extend_from_slice(&u16::try_from(profile.len() + 16).unwrap().to_be_bytes());
    output.extend_from_slice(b"ICC_PROFILE\0\x01\x01");
    output.extend_from_slice(profile);
    output.extend_from_slice(&jpeg[2..]);
    output
}

fn unchanged_pixels(tagged: &ImageBuffer, untagged: &ImageBuffer) {
    assert_eq!(
        (tagged.width, tagged.height),
        (untagged.width, untagged.height)
    );
    assert_eq!(tagged.buffer, untagged.buffer);
}

#[test]
fn jpeg_gray_and_four_component_profiles_are_source_metadata_without_changing_pixels() {
    let cases: &[(&str, &[u8], [u8; 4])] = &[
        (
            "baseline Gray",
            include_bytes!("fixtures/review/jpeg-icc-gray.jpg"),
            [128, 128, 128, 255],
        ),
        (
            "progressive Gray",
            include_bytes!("fixtures/review/jpeg-icc-gray-progressive.jpg"),
            [128, 128, 128, 255],
        ),
        (
            "baseline CMYK",
            include_bytes!("fixtures/review/jpeg-icc-cmyk.jpg"),
            [159, 127, 95, 255],
        ),
        (
            "YCCK",
            include_bytes!("fixtures/review/jpeg-icc-ycck.jpg"),
            [158, 128, 95, 255],
        ),
    ];
    for (name, jpeg, pixel) in cases {
        let untagged = image_from(jpeg).unwrap_or_else(|error| panic!("{name}: {error}"));
        let tagged = image_from(&tagged_jpeg(jpeg, SOURCE_PROFILE)).unwrap();
        unchanged_pixels(&tagged, &untagged);
        assert!(
            tagged
                .buffer
                .as_ref()
                .unwrap()
                .chunks_exact(4)
                .all(|p| p == pixel)
        );
        assert_profile(
            tagged.metadata.as_ref().unwrap(),
            "Source ICC Profile",
            SOURCE_PROFILE,
        );
    }
}

#[test]
fn jpeg_progressive_cmyk_metadata_uses_the_same_source_profile_rule() {
    use bin_rs::reader::BytesReader;
    use wml2::jpeg::{header::JpegHaeder, util::make_metadata};

    // The existing progressive decoder rejects this independent CMYK raster.
    // Exercise the shared metadata path without asserting new codec support.
    let data = tagged_jpeg(
        include_bytes!("fixtures/review/jpeg-icc-cmyk-progressive.jpg"),
        SOURCE_PROFILE,
    );
    let header = JpegHaeder::new(&mut BytesReader::new(&data), 0).unwrap();
    let frame = header.frame_header.as_ref().unwrap();
    assert!(frame.is_progressive);
    assert_eq!(frame.plane, 4);
    assert_profile(
        &make_metadata(&header),
        "Source ICC Profile",
        SOURCE_PROFILE,
    );
}

#[test]
fn jpeg_native_rgb_profile_stays_applicable_without_a_color_management_feature() {
    let untagged = image_from(RGB_JPEG).unwrap();
    let tagged = image_from(&tagged_jpeg(RGB_JPEG, RGB_PROFILE)).unwrap();
    unchanged_pixels(&tagged, &untagged);
    assert_profile(
        tagged.metadata.as_ref().unwrap(),
        "ICC Profile",
        RGB_PROFILE,
    );
}

fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    let start = output.len();
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    let crc = wml2::png::utils::CRC32::new().crc32(&output[start..]);
    output.extend_from_slice(&crc.to_be_bytes());
}

fn png(color_type: u8, depth: u8, pixel: &[u8], profile: Option<&[u8]>) -> Vec<u8> {
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = 1u32.to_be_bytes().repeat(2);
    header.extend_from_slice(&[depth, color_type, 0, 0, 0]);
    chunk(&mut output, b"IHDR", &header);
    if let Some(profile) = profile {
        let mut payload = b"test-profile\0\0".to_vec();
        payload.extend(miniz_oxide::deflate::compress_to_vec_zlib(profile, 6));
        chunk(&mut output, b"iCCP", &payload);
    }
    if color_type == 3 {
        chunk(&mut output, b"PLTE", &[51, 102, 153]);
        chunk(&mut output, b"tRNS", &[204]);
    }
    let mut scanline = vec![0];
    scanline.extend_from_slice(pixel);
    chunk(
        &mut output,
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&scanline, 6),
    );
    chunk(&mut output, b"IEND", &[]);
    output
}

#[test]
fn png_gray_and_gray_alpha_profiles_are_source_metadata_at_every_supported_depth() {
    let cases: &[(u8, u8, &[u8], [u8; 4])] = &[
        (0, 1, &[0x80], [255, 255, 255, 255]),
        (0, 2, &[0x80], [170, 170, 170, 255]),
        (0, 4, &[0x80], [136, 136, 136, 255]),
        (0, 8, &[128], [128, 128, 128, 255]),
        (0, 16, &[0x12, 0x34], [0x12, 0x12, 0x12, 255]),
        (4, 8, &[128, 204], [128, 128, 128, 204]),
        (4, 16, &[0x12, 0x34, 0xcd, 0xef], [0x12, 0x12, 0x12, 0xcd]),
    ];
    for &(color_type, depth, pixel, expected) in cases {
        let untagged = image_from(&png(color_type, depth, pixel, None)).unwrap();
        let tagged = image_from(&png(color_type, depth, pixel, Some(SOURCE_PROFILE))).unwrap();
        unchanged_pixels(&tagged, &untagged);
        assert_eq!(tagged.buffer.as_deref(), Some(expected.as_slice()));
        assert_profile(
            tagged.metadata.as_ref().unwrap(),
            "Source ICC Profile",
            SOURCE_PROFILE,
        );
    }
}

#[test]
fn png_native_rgb_rgba_and_palette_profiles_stay_applicable() {
    for (color_type, depth, pixel) in [
        (2, 8, vec![51, 102, 153]),
        (2, 16, vec![0x12, 0x34, 0x45, 0x67, 0x89, 0xab]),
        (6, 8, vec![51, 102, 153, 204]),
        (6, 16, vec![0x12, 0x34, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
        (3, 8, vec![0]),
    ] {
        let untagged = image_from(&png(color_type, depth, &pixel, None)).unwrap();
        let tagged = image_from(&png(color_type, depth, &pixel, Some(RGB_PROFILE))).unwrap();
        unchanged_pixels(&tagged, &untagged);
        assert_profile(
            tagged.metadata.as_ref().unwrap(),
            "ICC Profile",
            RGB_PROFILE,
        );
    }
}

#[cfg(feature = "image-buffer-ex")]
#[test]
fn png_ex_keeps_its_single_source_profile_key_for_gray_and_rgb_storage() {
    use wml2::draw::{PixelFormatEx, PrecisionConversion, image_from_ex};

    for (color_type, depth, pixel, profile) in [
        (0, 8, vec![128], SOURCE_PROFILE),
        (0, 16, vec![0x12, 0x34], SOURCE_PROFILE),
        (4, 8, vec![128, 204], SOURCE_PROFILE),
        (4, 16, vec![0x12, 0x34, 0xcd, 0xef], SOURCE_PROFILE),
        (2, 8, vec![51, 102, 153], RGB_PROFILE),
        (
            6,
            16,
            vec![0x12, 0x34, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
            RGB_PROFILE,
        ),
        (3, 8, vec![0], RGB_PROFILE),
    ] {
        let data = png(color_type, depth, &pixel, Some(profile));
        let untagged = png(color_type, depth, &pixel, None);
        let mut formats = vec![PixelFormatEx::Rgba16];
        if color_type == 0 {
            formats.push(PixelFormatEx::Gray16);
        }
        for storage in formats {
            let tagged = image_from_ex(&data, storage, PrecisionConversion::Exact).unwrap();
            let untagged = image_from_ex(&untagged, storage, PrecisionConversion::Exact).unwrap();
            assert_eq!(tagged.bytes(), untagged.bytes());
            assert_profile(
                tagged.metadata.as_ref().unwrap(),
                "Source ICC Profile",
                profile,
            );
        }
    }
}
