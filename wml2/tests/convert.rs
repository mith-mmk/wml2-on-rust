use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use wml2::draw::{
    AnimationLayer, EncodeOptions, ImageBuffer, ImageRect, NextBlend, NextDispose, NextOption,
    NextOptions, convert, image_encoder, image_load,
};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

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

fn solid_rgba(width: usize, height: usize, rgba: [u8; 4]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(width * height * 4);
    for _ in 0..(width * height) {
        buffer.extend_from_slice(&rgba);
    }
    buffer
}

fn frame_control(width: usize, height: usize, delay_ms: u64) -> NextOptions {
    NextOptions {
        flag: NextOption::Continue,
        await_time: delay_ms,
        image_rect: Some(ImageRect {
            start_x: 0,
            start_y: 0,
            width,
            height,
        }),
        dispose_option: Some(NextDispose::None),
        blend: Some(NextBlend::Override),
    }
}

fn animated_png_bytes() -> Vec<u8> {
    let first = solid_rgba(2, 2, [255, 0, 0, 255]);
    let second = solid_rgba(2, 2, [0, 0, 255, 255]);

    let mut image = ImageBuffer::from_buffer(2, 2, first.clone());
    image.loop_count = Some(2);
    image.animation = Some(vec![
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 0,
            start_y: 0,
            buffer: first,
            control: frame_control(2, 2, 120),
        },
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 0,
            start_y: 0,
            buffer: second,
            control: frame_control(2, 2, 240),
        },
    ]);

    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    image_encoder(&mut encode, ImageFormat::Png).unwrap()
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
fn convert_animated_png_file_to_apng_via_public_api() {
    let input_path = temp_path("convert-animation-input", "png");
    let output_path = temp_path("convert-animation-output", "png");
    fs::write(&input_path, animated_png_bytes()).unwrap();

    convert(
        input_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
        None,
    )
    .unwrap();

    let png = fs::read(&output_path).unwrap();
    assert!(png.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
    assert!(png.windows(4).any(|window| window == b"acTL"));

    let decoded = image_load(&png).unwrap();
    assert!(
        decoded
            .animation
            .as_ref()
            .map(|frames| frames.len())
            .unwrap_or(0)
            > 1
    );

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}
