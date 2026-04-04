use std::collections::HashMap;
use std::path::PathBuf;

use bin_rs::Endian;
use wml2::draw::{
    AnimationLayer, EncodeOptions, ImageBuffer, ImageRect, NextBlend, NextDispose, NextOption,
    NextOptions, image_encoder, image_from_file, image_load,
};
use wml2::metadata::DataMap;
use wml2::tiff::header::{DataPack, TiffHeader, TiffHeaders, exif_to_bytes};
use wml2::util::ImageFormat;

fn solid_rgba(width: usize, height: usize, rgba: [u8; 4]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(width * height * 4);
    for _ in 0..(width * height) {
        buffer.extend_from_slice(&rgba);
    }
    buffer
}

fn gradient_rgba(width: usize, height: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            buffer.push((x * 13 + y * 7) as u8);
            buffer.push((x * 5 + y * 17) as u8);
            buffer.push((x * 19 + y * 3) as u8);
            buffer.push(255);
        }
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

fn apng_blend_ops(data: &[u8]) -> Vec<u8> {
    let mut cursor = 8;
    let mut blend_ops = Vec::new();

    while cursor + 12 <= data.len() {
        let length = u32::from_be_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
        let chunk_type = &data[cursor + 4..cursor + 8];
        let data_start = cursor + 8;
        let data_end = data_start + length;
        if data_end + 4 > data.len() {
            break;
        }
        if chunk_type == b"fcTL" && length == 26 {
            blend_ops.push(data[data_end - 1]);
        }
        cursor = data_end + 4;
    }

    blend_ops
}

fn first_fctl_rect(data: &[u8]) -> Option<(u32, u32, u32, u32)> {
    let mut cursor = 8;

    while cursor + 12 <= data.len() {
        let length = u32::from_be_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
        let chunk_type = &data[cursor + 4..cursor + 8];
        let data_start = cursor + 8;
        let data_end = data_start + length;
        if data_end + 4 > data.len() {
            break;
        }
        if chunk_type == b"fcTL" && length == 26 {
            let width =
                u32::from_be_bytes(data[data_start + 4..data_start + 8].try_into().unwrap());
            let height =
                u32::from_be_bytes(data[data_start + 8..data_start + 12].try_into().unwrap());
            let x_offset =
                u32::from_be_bytes(data[data_start + 12..data_start + 16].try_into().unwrap());
            let y_offset =
                u32::from_be_bytes(data[data_start + 16..data_start + 20].try_into().unwrap());
            return Some((width, height, x_offset, y_offset));
        }
        cursor = data_end + 4;
    }

    None
}

fn exif_bytes() -> Vec<u8> {
    let mut headers = TiffHeaders::empty(Endian::LittleEndian);
    headers.headers.push(TiffHeader {
        tagid: 0x010f,
        data: DataPack::Ascii("wml2".to_string()),
        length: 4,
    });
    exif_to_bytes(&headers).unwrap()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn bundled_test_image_path(name: &str) -> PathBuf {
    repo_root()
        .join("test")
        .join("images")
        .join("bundled")
        .join(name)
}

#[test]
fn encode_png_via_public_api() {
    let rgba = gradient_rgba(32, 32);
    let mut image = ImageBuffer::from_buffer(32, 32, rgba);
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };

    let data = image_encoder(&mut encode, ImageFormat::Png).unwrap();

    assert!(data.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
    assert!(!data.windows(4).any(|window| window == b"acTL"));
    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 32);
    assert_eq!(decoded.height, 32);
}

#[test]
fn encode_png_via_public_api_with_exif_option() {
    let rgba = gradient_rgba(8, 8);
    let mut image = ImageBuffer::from_buffer(8, 8, rgba);
    let mut options = HashMap::new();
    options.insert("exif".to_string(), DataMap::Raw(exif_bytes()));
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: Some(options),
    };

    let data = image_encoder(&mut encode, ImageFormat::Png).unwrap();

    assert!(data.windows(4).any(|window| window == b"eXIf"));
    let decoded = image_load(&data).unwrap();
    let metadata = decoded.metadata.as_ref().unwrap();
    assert!(matches!(metadata.get("EXIF"), Some(DataMap::Exif(_))));
    assert!(matches!(metadata.get("EXIF Raw"), Some(DataMap::Raw(_))));
}

#[test]
fn encode_apng_via_public_api() {
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
            buffer: first.clone(),
            control: frame_control(2, 2, 120),
        },
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 0,
            start_y: 0,
            buffer: second.clone(),
            control: frame_control(2, 2, 240),
        },
    ]);

    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };

    let data = image_encoder(&mut encode, ImageFormat::Png).unwrap();

    assert!(data.windows(4).any(|window| window == b"acTL"));
    assert!(data.windows(4).any(|window| window == b"fcTL"));
    assert!(data.windows(4).any(|window| window == b"fdAT"));

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 2);
    assert_eq!(decoded.height, 2);
    assert_eq!(decoded.loop_count, Some(2));
    assert_eq!(decoded.first_wait_time, Some(120));
    assert_eq!(
        decoded.animation.as_ref().map(|frames| frames.len()),
        Some(2)
    );
    assert_eq!(
        decoded.animation.as_ref().unwrap()[1].control.await_time,
        240
    );
    assert_eq!(decoded.animation.as_ref().unwrap()[1].buffer, second);
}

#[test]
fn encode_apng_preserves_over_blend_for_transparent_frames() {
    let first = solid_rgba(2, 2, [255, 0, 0, 255]);
    let second = solid_rgba(2, 2, [0, 0, 255, 128]);
    let third = solid_rgba(2, 2, [0, 255, 0, 64]);

    let mut image = ImageBuffer::from_buffer(2, 2, first.clone());
    image.loop_count = Some(1);
    image.animation = Some(vec![
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 0,
            start_y: 0,
            buffer: first,
            control: NextOptions {
                blend: Some(NextBlend::Source),
                ..frame_control(2, 2, 60)
            },
        },
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 0,
            start_y: 0,
            buffer: second,
            control: NextOptions {
                blend: Some(NextBlend::Source),
                ..frame_control(2, 2, 90)
            },
        },
        AnimationLayer {
            width: 2,
            height: 2,
            start_x: 0,
            start_y: 0,
            buffer: third,
            control: NextOptions {
                blend: Some(NextBlend::Source),
                ..frame_control(2, 2, 120)
            },
        },
    ]);

    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    let data = image_encoder(&mut encode, ImageFormat::Png).unwrap();

    assert!(data.windows(4).any(|window| window == b"acTL"));
    assert!(data.windows(4).any(|window| window == b"fdAT"));
    assert_eq!(apng_blend_ops(&data), vec![1, 1, 1]);

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.first_wait_time, Some(60));
    assert_eq!(
        decoded.animation.as_ref().map(|frames| frames.len()),
        Some(3)
    );
}

#[test]
fn encode_apng_normalizes_offset_first_frame_to_full_canvas() {
    let mut expected = solid_rgba(4, 4, [0, 0, 0, 0]);
    for y in 0..2 {
        for x in 0..2 {
            let offset = ((y + 1) * 4 + (x + 1)) * 4;
            expected[offset..offset + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
    }

    let mut image = ImageBuffer::from_buffer(4, 4, expected.clone());
    image.loop_count = Some(1);
    image.animation = Some(vec![AnimationLayer {
        width: 2,
        height: 2,
        start_x: 1,
        start_y: 1,
        buffer: solid_rgba(2, 2, [255, 0, 0, 255]),
        control: NextOptions {
            image_rect: Some(ImageRect {
                start_x: 1,
                start_y: 1,
                width: 2,
                height: 2,
            }),
            ..frame_control(2, 2, 90)
        },
    }]);

    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    let data = image_encoder(&mut encode, ImageFormat::Png).unwrap();

    assert!(data.windows(4).any(|window| window == b"acTL"));
    assert_eq!(first_fctl_rect(&data), Some((4, 4, 0, 0)));

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
    assert_eq!(decoded.buffer.as_ref(), Some(&expected));
}

#[test]
fn encode_viewer_error_sample_to_png_uses_full_canvas_first_frame() {
    let path = bundled_test_image_path("WML2Viewer_error.webp");
    let mut image = image_from_file(path.to_string_lossy().into_owned()).unwrap();

    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    let data = image_encoder(&mut encode, ImageFormat::Png).unwrap();

    assert_eq!(first_fctl_rect(&data), Some((900, 900, 0, 0)));

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 900);
    assert_eq!(decoded.height, 900);
    assert!(
        decoded
            .buffer
            .as_ref()
            .is_some_and(|buffer| !buffer.is_empty())
    );
}
