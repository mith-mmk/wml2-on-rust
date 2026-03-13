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
use wml2::webp::decoder::{WebpFormat, get_features};

fn solid_rgba(width: usize, height: usize, rgba: [u8; 4]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(width * height * 4);
    for _ in 0..(width * height) {
        buffer.extend_from_slice(&rgba);
    }
    buffer
}

fn gradient_rgba(width: usize, height: usize, alpha: bool) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            buffer.push((x * 17 + y * 5) as u8);
            buffer.push((x * 9 + y * 13) as u8);
            buffer.push((x * 3 + y * 19) as u8);
            buffer.push(if alpha {
                ((x * 11 + y * 7) & 0xff) as u8
            } else {
                255
            });
        }
    }
    buffer
}

fn frame_control(
    x: i32,
    y: i32,
    width: usize,
    height: usize,
    delay_ms: u64,
    dispose: NextDispose,
    blend: NextBlend,
) -> NextOptions {
    NextOptions {
        flag: NextOption::Continue,
        await_time: delay_ms,
        image_rect: Some(ImageRect {
            start_x: x,
            start_y: y,
            width,
            height,
        }),
        dispose_option: Some(dispose),
        blend: Some(blend),
    }
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

fn blend_source_over(dst: &mut [u8], src: &[u8]) {
    let src_alpha = src[3] as u32;
    if src_alpha == 0 {
        return;
    }
    if src_alpha == 255 {
        dst.copy_from_slice(src);
        return;
    }

    let dst_alpha = dst[3] as u32;
    let out_alpha = src_alpha + ((dst_alpha * (255 - src_alpha) + 127) / 255);
    if out_alpha == 0 {
        dst.copy_from_slice(&[0, 0, 0, 0]);
        return;
    }

    for channel in 0..3 {
        let src_premul = src[channel] as u32 * src_alpha;
        let dst_premul = dst[channel] as u32 * dst_alpha;
        let out_premul = src_premul + ((dst_premul * (255 - src_alpha) + 127) / 255);
        dst[channel] = ((out_premul * 255 + (out_alpha / 2)) / out_alpha) as u8;
    }
    dst[3] = out_alpha as u8;
}

fn apply_frame(
    canvas: &mut [u8],
    canvas_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    buffer: &[u8],
    blend: bool,
) {
    for row in 0..height {
        let src_row = row * width * 4;
        let dst_row = (y + row) * canvas_width * 4;
        for col in 0..width {
            let src_offset = src_row + col * 4;
            let dst_offset = dst_row + (x + col) * 4;
            if blend {
                blend_source_over(
                    &mut canvas[dst_offset..dst_offset + 4],
                    &buffer[src_offset..src_offset + 4],
                );
            } else {
                canvas[dst_offset..dst_offset + 4]
                    .copy_from_slice(&buffer[src_offset..src_offset + 4]);
            }
        }
    }
}

fn clear_rect(
    canvas: &mut [u8],
    canvas_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) {
    for row in 0..height {
        let dst_row = (y + row) * canvas_width * 4;
        for col in 0..width {
            let dst_offset = dst_row + (x + col) * 4;
            canvas[dst_offset..dst_offset + 4].copy_from_slice(&[0, 0, 0, 0]);
        }
    }
}

fn animated_image() -> ImageBuffer {
    let frame0 = solid_rgba(2, 2, [255, 0, 0, 255]);
    let frame1 = solid_rgba(2, 2, [0, 0, 255, 128]);
    let frame2 = solid_rgba(2, 2, [0, 255, 0, 255]);

    let mut image = ImageBuffer::from_buffer(4, 4, vec![0; 4 * 4 * 4]);
    image.loop_count = Some(3);
    image.animation = Some(vec![
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 0,
            start_y: 0,
            buffer: frame0,
            control: frame_control(0, 0, 2, 2, 80, NextDispose::None, NextBlend::Override),
        },
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 1,
            start_y: 1,
            buffer: frame1,
            control: frame_control(1, 1, 2, 2, 120, NextDispose::Background, NextBlend::Source),
        },
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 2,
            start_y: 0,
            buffer: frame2,
            control: frame_control(2, 0, 2, 2, 40, NextDispose::None, NextBlend::Override),
        },
    ]);
    image
}

fn expected_animation_canvases() -> Vec<Vec<u8>> {
    let frame0 = solid_rgba(2, 2, [255, 0, 0, 255]);
    let frame1 = solid_rgba(2, 2, [0, 0, 255, 128]);
    let frame2 = solid_rgba(2, 2, [0, 255, 0, 255]);
    let mut canvas = vec![0; 4 * 4 * 4];

    apply_frame(&mut canvas, 4, 0, 0, 2, 2, &frame0, false);
    let expected0 = canvas.clone();

    apply_frame(&mut canvas, 4, 1, 1, 2, 2, &frame1, true);
    let expected1 = canvas.clone();
    clear_rect(&mut canvas, 4, 1, 1, 2, 2);

    apply_frame(&mut canvas, 4, 2, 0, 2, 2, &frame2, false);
    let expected2 = canvas;

    vec![expected0, expected1, expected2]
}

#[test]
fn encode_lossless_webp_via_public_api() {
    let rgba = gradient_rgba(32, 32, true);
    let mut image = ImageBuffer::from_buffer(32, 32, rgba.clone());
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };

    let data = image_encoder(&mut encode, ImageFormat::Webp).unwrap();

    assert!(data.starts_with(b"RIFF"));
    assert_eq!(&data[8..12], b"WEBP");
    let features = get_features(&data).unwrap();
    assert_eq!(features.format, WebpFormat::Lossless);
    assert!(features.has_alpha);
    assert!(!features.has_animation);

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 32);
    assert_eq!(decoded.height, 32);
    assert_eq!(decoded.buffer.as_ref().unwrap(), &rgba);
}

#[test]
fn encode_lossy_webp_via_public_api() {
    let rgba = gradient_rgba(17, 13, false);
    let mut image = ImageBuffer::from_buffer(17, 13, rgba);
    let mut options = HashMap::new();
    options.insert("quality".to_string(), DataMap::UInt(75));
    options.insert("optimize".to_string(), DataMap::UInt(2));
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: Some(options),
    };

    let data = image_encoder(&mut encode, ImageFormat::Webp).unwrap();

    assert!(data.starts_with(b"RIFF"));
    assert_eq!(&data[8..12], b"WEBP");
    let features = get_features(&data).unwrap();
    assert_eq!(features.format, WebpFormat::Lossy);
    assert!(!features.has_alpha);
    assert!(!features.has_animation);

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 17);
    assert_eq!(decoded.height, 13);
}

#[test]
fn encode_animated_webp_via_public_api() {
    let expected = expected_animation_canvases();
    let mut image = animated_image();
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };

    let data = image_encoder(&mut encode, ImageFormat::Webp).unwrap();

    assert!(data.windows(4).any(|window| window == b"ANIM"));
    assert!(data.windows(4).any(|window| window == b"ANMF"));
    let features = get_features(&data).unwrap();
    assert!(features.has_animation);
    assert!(features.has_alpha);

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
    assert_eq!(decoded.loop_count, Some(3));
    assert_eq!(decoded.first_wait_time, Some(80));
    assert_eq!(
        decoded.animation.as_ref().map(|frames| frames.len()),
        Some(3)
    );

    let frames = decoded.animation.as_ref().unwrap();
    assert_eq!(frames[0].buffer, expected[0]);
    assert_eq!(frames[1].buffer, expected[1]);
    assert_eq!(frames[2].buffer, expected[2]);
    assert_eq!(frames[1].control.await_time, 120);
    assert_eq!(frames[2].control.await_time, 40);
}

#[test]
fn convert_apng_file_to_webp_via_public_api() {
    let mut image = animated_image();
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    let png = image_encoder(&mut encode, ImageFormat::Png).unwrap();

    let input_path = temp_path("convert-animation-input", "png");
    let output_path = temp_path("convert-animation-output", "webp");
    fs::write(&input_path, png).unwrap();

    let mut options = HashMap::new();
    options.insert("optimize".to_string(), DataMap::UInt(4));
    convert(
        input_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
        Some(options),
    )
    .unwrap();

    let webp = fs::read(&output_path).unwrap();
    let features = get_features(&webp).unwrap();
    assert!(features.has_animation);
    assert_eq!(features.width, 4);
    assert_eq!(features.height, 4);

    let decoded = image_load(&webp).unwrap();
    assert_eq!(
        decoded.animation.as_ref().map(|frames| frames.len()),
        Some(3)
    );

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}
