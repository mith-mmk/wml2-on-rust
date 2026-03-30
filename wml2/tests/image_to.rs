use std::collections::HashMap;

use bin_rs::Endian;
use wml2::draw::{
    AnimationLayer, ImageBuffer, ImageRect, NextBlend, NextDispose, NextOption, NextOptions,
    image_load, image_to,
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

fn exif_bytes() -> Vec<u8> {
    let mut headers = TiffHeaders::empty(Endian::LittleEndian);
    headers.headers.push(TiffHeader {
        tagid: 0x010f,
        data: DataPack::Ascii("wml2".to_string()),
        length: 4,
    });
    exif_to_bytes(&headers).unwrap()
}

#[test]
fn image_to_encodes_png_from_imagebuffer() {
    let rgba = solid_rgba(4, 3, [32, 64, 96, 255]);
    let mut image = ImageBuffer::from_buffer(4, 3, rgba);

    let png = image_to(&mut image, ImageFormat::Png, None).unwrap();

    assert!(png.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
    let decoded = image_load(&png).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 3);
}

#[test]
fn image_to_encodes_bmp_from_imagebuffer() {
    let rgba = solid_rgba(3, 2, [12, 34, 56, 255]);
    let mut image = ImageBuffer::from_buffer(3, 2, rgba.clone());

    let bmp = image_to(&mut image, ImageFormat::Bmp, None).unwrap();

    assert!(bmp.starts_with(b"BM"));
    let decoded = image_load(&bmp).unwrap();
    assert_eq!(decoded.width, 3);
    assert_eq!(decoded.height, 2);
    assert_eq!(decoded.buffer.unwrap(), rgba);
}

#[test]
fn image_to_encodes_animated_webp_from_imagebuffer() {
    let first = solid_rgba(2, 2, [255, 0, 0, 255]);
    let second = solid_rgba(2, 2, [0, 255, 0, 255]);

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

    let webp = image_to(&mut image, ImageFormat::Webp, None).unwrap();

    assert!(webp.starts_with(b"RIFF"));
    assert!(webp.windows(4).any(|window| window == b"ANIM"));
    let decoded = image_load(&webp).unwrap();
    assert!(
        decoded
            .animation
            .as_ref()
            .map(|frames| frames.len())
            .unwrap_or(0)
            > 1
    );
}

#[test]
fn image_to_forwards_exif_options() {
    let rgba = solid_rgba(4, 4, [10, 20, 30, 255]);
    let mut image = ImageBuffer::from_buffer(4, 4, rgba);
    let mut options = HashMap::new();
    options.insert("exif".to_string(), DataMap::Raw(exif_bytes()));

    let png = image_to(&mut image, ImageFormat::Png, Some(options)).unwrap();

    let decoded = image_load(&png).unwrap();
    let metadata = decoded.metadata.as_ref().unwrap();
    assert!(matches!(metadata.get("EXIF"), Some(DataMap::Exif(_))));
    assert!(matches!(metadata.get("EXIF Raw"), Some(DataMap::Raw(_))));
}
