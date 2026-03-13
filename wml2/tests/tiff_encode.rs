use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bin_rs::Endian;
use bin_rs::reader::BytesReader;
use wml2::draw::{
    AnimationLayer, EncodeOptions, ImageBuffer, ImageRect, NextBlend, NextDispose, NextOption,
    NextOptions, convert, image_encoder, image_load,
};
use wml2::metadata::{DataMap, get_exif};
use wml2::tiff::header::{DataPack, Rational, TiffHeader, TiffHeaders, exif_to_bytes, read_tags};
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

fn animated_image() -> ImageBuffer {
    let frame0 = solid_rgba(2, 2, [255, 0, 0, 255]);
    let frame1 = solid_rgba(2, 2, [0, 0, 255, 128]);
    let frame2 = solid_rgba(2, 2, [0, 255, 0, 255]);

    let mut image = ImageBuffer::from_buffer(4, 4, vec![0; 4 * 4 * 4]);
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

fn expected_animation_pages() -> Vec<Vec<u8>> {
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

fn first_ifd_tags(tags: &[TiffHeader]) -> &[TiffHeader] {
    let mut split_index = tags.len();
    for index in 1..tags.len() {
        if tags[index].tagid < tags[index - 1].tagid {
            split_index = index;
            break;
        }
    }
    &tags[..split_index]
}

fn exif_fixture() -> TiffHeaders {
    let mut headers = TiffHeaders::empty(Endian::LittleEndian);
    headers.headers.push(TiffHeader {
        tagid: 0x010f,
        data: DataPack::Ascii("wml2".to_string()),
        length: 4,
    });
    headers.headers.push(TiffHeader {
        tagid: 0x0110,
        data: DataPack::Ascii("test-model".to_string()),
        length: 10,
    });
    headers.exif = Some(vec![
        TiffHeader {
            tagid: 0x829a,
            data: DataPack::Rational(vec![Rational { n: 1, d: 30 }]),
            length: 1,
        },
        TiffHeader {
            tagid: 0x9000,
            data: DataPack::Undef(b"0231".to_vec()),
            length: 4,
        },
    ]);
    headers.gps = Some(vec![
        TiffHeader {
            tagid: 0x0000,
            data: DataPack::Bytes(vec![2, 3, 0, 0]),
            length: 4,
        },
        TiffHeader {
            tagid: 0x0001,
            data: DataPack::Ascii("N".to_string()),
            length: 1,
        },
    ]);
    headers
}

fn animated_png_bytes() -> Vec<u8> {
    let mut image = animated_image();
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    image_encoder(&mut encode, ImageFormat::Png).unwrap()
}

#[test]
fn exif_writer_roundtrips_tiff_headers() {
    let headers = exif_fixture();
    let bytes = exif_to_bytes(&headers).unwrap();
    assert!(bytes.starts_with(b"II*\0"));

    let mut metadata = HashMap::new();
    metadata.insert("EXIF".to_string(), DataMap::Exif(headers.clone()));
    assert_eq!(get_exif(Some(&metadata)).unwrap(), Some(bytes.clone()));

    let mut reader = BytesReader::new(&bytes);
    let parsed = read_tags(&mut reader).unwrap();

    let make = first_ifd_tags(&parsed.headers)
        .iter()
        .find(|tag| tag.tagid == 0x010f)
        .unwrap();
    match &make.data {
        DataPack::Ascii(value) => assert_eq!(value.trim_end_matches('\0'), "wml2"),
        other => panic!("unexpected Make tag: {other:?}"),
    }

    let exif_version = parsed
        .exif
        .as_ref()
        .unwrap()
        .iter()
        .find(|tag| tag.tagid == 0x9000)
        .unwrap();
    match &exif_version.data {
        DataPack::Undef(value) => assert_eq!(value, b"0231"),
        other => panic!("unexpected ExifVersion tag: {other:?}"),
    }

    let gps_version = parsed
        .gps
        .as_ref()
        .unwrap()
        .iter()
        .find(|tag| tag.tagid == 0x0000)
        .unwrap();
    match &gps_version.data {
        DataPack::Bytes(value) => assert_eq!(value, &vec![2, 3, 0, 0]),
        other => panic!("unexpected GPSVersionID tag: {other:?}"),
    }
}

#[test]
fn encode_tiff_via_public_api_roundtrips_pixels_and_metadata() {
    let mut rgba = Vec::with_capacity(7 * 5 * 4);
    for y in 0..5 {
        for x in 0..7 {
            rgba.push((x * 31 + y * 7) as u8);
            rgba.push((x * 13 + y * 17) as u8);
            rgba.push((x * 19 + y * 11) as u8);
            rgba.push(((x * 29 + y * 37) & 0xff) as u8);
        }
    }

    let mut image = ImageBuffer::from_buffer(7, 5, rgba.clone());
    let mut metadata = HashMap::new();
    metadata.insert("EXIF".to_string(), DataMap::Exif(exif_fixture()));
    metadata.insert(
        "ICC Profile".to_string(),
        DataMap::ICCProfile(vec![0x12, 0x34, 0x56, 0x78]),
    );
    image.metadata = Some(metadata);

    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };
    let data = image_encoder(&mut encode, ImageFormat::Tiff).unwrap();

    assert!(data.starts_with(b"II*\0"));

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 7);
    assert_eq!(decoded.height, 5);
    assert_eq!(decoded.buffer.as_ref().unwrap(), &rgba);

    let metadata = decoded.metadata.as_ref().unwrap();
    match metadata.get("ICC Profile").unwrap() {
        DataMap::ICCProfile(profile) => assert_eq!(profile, &vec![0x12, 0x34, 0x56, 0x78]),
        other => panic!("unexpected ICC profile metadata: {other:?}"),
    }

    let headers = match metadata.get("Tiff headers").unwrap() {
        DataMap::Exif(headers) => headers,
        other => panic!("unexpected TIFF metadata: {other:?}"),
    };
    let make = first_ifd_tags(&headers.headers)
        .iter()
        .find(|tag| tag.tagid == 0x010f)
        .unwrap();
    match &make.data {
        DataPack::Ascii(value) => assert_eq!(value.trim_end_matches('\0'), "wml2"),
        other => panic!("unexpected Make tag: {other:?}"),
    }
    assert!(headers.exif.as_ref().unwrap().iter().any(|tag| tag.tagid == 0x9000));
    assert!(headers.gps.as_ref().unwrap().iter().any(|tag| tag.tagid == 0x0000));
}

#[test]
fn encode_lzw_tiff_via_public_api_roundtrips_pixels() {
    let mut rgba = Vec::with_capacity(13 * 9 * 4);
    for y in 0..9 {
        for x in 0..13 {
            rgba.push((x * 19 + y * 3) as u8);
            rgba.push((x * 7 + y * 23) as u8);
            rgba.push((x * 29 + y * 11) as u8);
            rgba.push(255);
        }
    }

    let mut image = ImageBuffer::from_buffer(13, 9, rgba.clone());
    let mut options = HashMap::new();
    options.insert("compression".to_string(), DataMap::Ascii("lzw".to_string()));
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: Some(options),
    };
    let data = image_encoder(&mut encode, ImageFormat::Tiff).unwrap();

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 13);
    assert_eq!(decoded.height, 9);
    assert_eq!(decoded.buffer.as_ref().unwrap(), &rgba);

    let metadata = decoded.metadata.as_ref().unwrap();
    match metadata.get("compression").unwrap() {
        DataMap::Ascii(value) => assert_eq!(value, "LZW(Tiff)"),
        other => panic!("unexpected compression metadata: {other:?}"),
    }
}

#[test]
fn encode_animated_tiff_via_public_api() {
    let expected = expected_animation_pages();
    let mut image = animated_image();
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: None,
    };

    let data = image_encoder(&mut encode, ImageFormat::Tiff).unwrap();
    assert!(data.starts_with(b"II*\0"));

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
    assert_eq!(decoded.buffer.as_ref().unwrap(), &expected[0]);
    assert_eq!(
        decoded.animation.as_ref().map(|frames| frames.len()),
        Some(2)
    );
    assert_eq!(decoded.animation.as_ref().unwrap()[0].buffer, expected[1]);
    assert_eq!(decoded.animation.as_ref().unwrap()[1].buffer, expected[2]);
}

#[test]
fn convert_animated_png_file_to_tiff_via_public_api() {
    let input_path = temp_path("convert-animation-input", "png");
    let output_path = temp_path("convert-animation-output", "tiff");
    fs::write(&input_path, animated_png_bytes()).unwrap();

    convert(
        input_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
        None,
    )
    .unwrap();

    let data = fs::read(&output_path).unwrap();
    assert!(data.starts_with(b"II*\0"));

    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 4);
    assert_eq!(decoded.height, 4);
    assert_eq!(
        decoded.animation.as_ref().map(|frames| frames.len()),
        Some(2)
    );

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}
