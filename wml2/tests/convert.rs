use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use wml2::draw::{convert, image_encoder, image_load, EncodeOptions, ImageBuffer};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

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

fn temp_path(name: &str, extension: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "wml2-{name}-{}-{unique}.{extension}",
        std::process::id()
    ))
}

#[test]
fn convert_png_file_to_jpeg_via_public_api() {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            rgba.push((x * 7 + y * 3) as u8);
            rgba.push((x * 11 + y * 5) as u8);
            rgba.push((x * 13 + y * 17) as u8);
            rgba.push(255);
        }
    }
    let mut image = ImageBuffer::from_buffer(32, 32, rgba);
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    let png = image_encoder(&mut encode, ImageFormat::Png).unwrap();

    let input_path = temp_path("convert-input", "png");
    let output_path = temp_path("convert-output", "jpg");
    fs::write(&input_path, png).unwrap();

    let mut options = HashMap::new();
    options.insert("quality".to_string(), DataMap::UInt(90));
    convert(
        input_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
        Some(options),
    )
    .unwrap();

    let jpeg = fs::read(&output_path).unwrap();
    assert!(jpeg.starts_with(&[0xff, 0xd8]));
    let decoded = image_load(&jpeg).unwrap();
    assert_eq!(decoded.width, 32);
    assert_eq!(decoded.height, 32);

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}

#[test]
fn convert_gif_file_to_apng_via_public_api() {
    let output_path = temp_path("convert-animation", "png");

    convert(
        sample_path("bird-wings-flying-feature.gif"),
        output_path.to_string_lossy().into_owned(),
        None,
    )
    .unwrap();

    let png = fs::read(&output_path).unwrap();
    assert!(png.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
    assert!(png.windows(4).any(|window| window == b"acTL"));

    let decoded = image_load(&png).unwrap();
    assert!(decoded.animation.as_ref().map(|frames| frames.len()).unwrap_or(0) > 1);

    let _ = fs::remove_file(output_path);
}
