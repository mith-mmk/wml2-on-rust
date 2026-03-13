use wml2::draw::{
    AnimationLayer, EncodeOptions, ImageBuffer, ImageRect, NextBlend, NextDispose, NextOption,
    NextOptions, image_encoder, image_load,
};
use wml2::util::ImageFormat;

fn solid_rgba(width: usize, height: usize, rgba: [u8; 4]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(width * height * 4);
    for _ in 0..(width * height) {
        buffer.extend_from_slice(&rgba);
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

fn apply_source_over(dst: &mut [u8], src: &[u8]) {
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
                apply_source_over(
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

fn expected_animation_frames() -> Vec<Vec<u8>> {
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

fn flatten_partial_alpha_for_gif(rgba: &[u8]) -> Vec<u8> {
    let mut flattened = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        match pixel[3] {
            0 => flattened.extend_from_slice(pixel),
            255 => flattened.extend_from_slice(pixel),
            alpha => {
                flattened.push(((pixel[0] as u32 * alpha as u32 + 127) / 255) as u8);
                flattened.push(((pixel[1] as u32 * alpha as u32 + 127) / 255) as u8);
                flattened.push(((pixel[2] as u32 * alpha as u32 + 127) / 255) as u8);
                flattened.push(255);
            }
        }
    }
    flattened
}

#[test]
fn encode_gif_preserves_exact_palette_without_dithering() {
    let palette = [
        [0, 0, 0, 255],
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
        [255, 0, 255, 255],
        [0, 255, 255, 255],
        [255, 255, 255, 255],
    ];
    let mut rgba = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16 {
        for x in 0..16 {
            rgba.extend_from_slice(&palette[(x + y) % palette.len()]);
        }
    }

    let mut image = ImageBuffer::from_buffer(16, 16, rgba.clone());
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    let data = image_encoder(&mut encode, ImageFormat::Gif).unwrap();

    assert!(data.starts_with(b"GIF89a"));
    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 16);
    assert_eq!(decoded.height, 16);
    assert_eq!(decoded.buffer.as_ref().unwrap(), &rgba);
}

#[test]
fn encode_gif_reserves_transparent_index() {
    let mut rgba = vec![0; 4 * 4 * 4];
    for y in 0..4usize {
        for x in 0..4usize {
            let offset = (y * 4 + x) * 4;
            if (x + y) % 2 == 0 {
                rgba[offset..offset + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }
    }

    let mut image = ImageBuffer::from_buffer(4, 4, rgba.clone());
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    let data = image_encoder(&mut encode, ImageFormat::Gif).unwrap();

    assert!(data.starts_with(b"GIF89a"));
    assert!(data.windows(2).any(|window| window == [0x21, 0xf9]));

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.buffer.as_ref().unwrap(), &rgba);
}

#[test]
fn encode_animated_gif_via_public_api() {
    let expected = expected_animation_frames();
    let expected_gif_frame1 = flatten_partial_alpha_for_gif(&expected[1]);
    let mut image = animated_image();
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };

    let data = image_encoder(&mut encode, ImageFormat::Gif).unwrap();
    assert!(data.starts_with(b"GIF89a"));

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
    assert_eq!(decoded.loop_count, Some(3));
    assert_eq!(decoded.first_wait_time, Some(80));
    assert_eq!(decoded.buffer.as_ref().unwrap(), &expected[0]);
    assert_eq!(
        decoded.animation.as_ref().map(|frames| frames.len()),
        Some(3)
    );
    let frames = decoded.animation.as_ref().unwrap();
    assert_eq!(frames[0].buffer, expected[0]);
    assert_eq!(frames[1].buffer, expected_gif_frame1);
    assert_eq!(frames[2].buffer, expected[2]);
}

#[test]
fn encode_gif_semitransparent_canvas_via_public_api() {
    let expected = expected_animation_frames();
    let mut image = ImageBuffer::from_buffer(4, 4, expected[1].clone());
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };

    let data = image_encoder(&mut encode, ImageFormat::Gif).unwrap();
    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
}

#[test]
fn encode_gif_third_animation_canvas_via_public_api() {
    let expected = expected_animation_frames();
    let mut image = ImageBuffer::from_buffer(4, 4, expected[2].clone());
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };

    let data = image_encoder(&mut encode, ImageFormat::Gif).unwrap();
    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
}
