use std::collections::HashMap;

use wml2::draw::{image_encoder, image_load, EncodeOptions, ImageBuffer};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

#[test]
fn encode_jpeg_via_public_api() {
    let rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255,
        0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let mut image = ImageBuffer::from_buffer(2, 2, rgba);
    let mut options = HashMap::new();
    options.insert("quality".to_string(), DataMap::UInt(90));
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: Some(options),
    };

    let data = image_encoder(&mut encode, ImageFormat::Jpeg).unwrap();

    assert!(data.starts_with(&[0xff, 0xd8]));
    let decoded = image_load(&data).unwrap();
    assert_eq!(decoded.width, 2);
    assert_eq!(decoded.height, 2);
}
