use std::collections::HashMap;

use bin_rs::Endian;
use wml2::draw::{EncodeOptions, ImageBuffer, image_encoder, image_load};
use wml2::metadata::DataMap;
use wml2::tiff::header::{DataPack, TiffHeader, TiffHeaders, exif_to_bytes};
use wml2::util::ImageFormat;

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
fn encode_jpeg_via_public_api() {
    let rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
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

#[test]
fn encode_jpeg_via_public_api_with_exif_option() {
    let rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let mut image = ImageBuffer::from_buffer(2, 2, rgba);
    let mut options = HashMap::new();
    options.insert("quality".to_string(), DataMap::UInt(90));
    options.insert("exif".to_string(), DataMap::Raw(exif_bytes()));
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: Some(options),
    };

    let data = image_encoder(&mut encode, ImageFormat::Jpeg).unwrap();

    assert!(data.windows(6).any(|window| window == b"Exif\0\0"));
    let decoded = image_load(&data).unwrap();
    let metadata = decoded.metadata.as_ref().unwrap();
    assert!(matches!(metadata.get("EXIF"), Some(DataMap::Exif(_))));
}
