#![cfg(feature = "c2pa")]

use std::collections::HashMap;

use wml2::draw::{EncodeOptions, ImageBuffer, image_encoder, image_load};
use wml2::metadata::DataMap;
use wml2::metadata::c2pa::{C2PA_JSON_KEY, C2PA_RAW_KEY, PNG_CHUNK_TYPE};
use wml2::util::ImageFormat;

const C2PA_MANIFEST_STORE_UUID: [u8; 16] = [
    0x63, 0x32, 0x70, 0x61, 0x00, 0x11, 0x00, 0x10, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

fn box_bytes(box_type: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + payload.len());
    data.extend_from_slice(&((8 + payload.len()) as u32).to_be_bytes());
    data.extend_from_slice(box_type);
    data.extend_from_slice(payload);
    data
}

fn c2pa_jumbf() -> Vec<u8> {
    let mut jumd = Vec::new();
    jumd.extend_from_slice(&C2PA_MANIFEST_STORE_UUID);
    jumd.push(0x03);
    jumd.extend_from_slice(b"c2pa\0");
    let jumd = box_bytes(b"jumd", &jumd);
    let json = box_bytes(b"json", br#"{"claim":"test"}"#);

    let mut payload = Vec::new();
    payload.extend_from_slice(&jumd);
    payload.extend_from_slice(&json);
    box_bytes(b"jumb", &payload)
}

fn solid_rgba(width: usize, height: usize, rgba: [u8; 4]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(width * height * 4);
    for _ in 0..(width * height) {
        buffer.extend_from_slice(&rgba);
    }
    buffer
}

fn encode_test_image(format: ImageFormat) -> Vec<u8> {
    let rgba = solid_rgba(2, 2, [32, 64, 96, 255]);
    let mut image = ImageBuffer::from_buffer(2, 2, rgba);
    let mut options = HashMap::new();
    if matches!(&format, ImageFormat::Jpeg) {
        options.insert("quality".to_string(), DataMap::UInt(90));
    }
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: Some(options),
    };
    image_encoder(&mut encode, format).unwrap()
}

fn insert_png_chunk_after_ihdr(png: &[u8], chunk_type: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let insert_at = 8 + 4 + 4 + 13 + 4;
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    chunk.extend_from_slice(chunk_type);
    chunk.extend_from_slice(payload);
    chunk.extend_from_slice(&0u32.to_be_bytes());

    let mut out = Vec::new();
    out.extend_from_slice(&png[..insert_at]);
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&png[insert_at..]);
    out
}

fn insert_jpeg_app11_after_soi(jpeg: &[u8], payload: &[u8]) -> Vec<u8> {
    let segment_len = u16::try_from(payload.len() + 2).unwrap();
    let mut segment = Vec::new();
    segment.extend_from_slice(&[0xff, 0xeb]);
    segment.extend_from_slice(&segment_len.to_be_bytes());
    segment.extend_from_slice(payload);

    let mut out = Vec::new();
    out.extend_from_slice(&jpeg[..2]);
    out.extend_from_slice(&segment);
    out.extend_from_slice(&jpeg[2..]);
    out
}

#[test]
fn png_cabx_metadata_is_reported_as_json_and_raw_payload() {
    let payload = c2pa_jumbf();
    let png = encode_test_image(ImageFormat::Png);
    let png = insert_png_chunk_after_ihdr(&png, &PNG_CHUNK_TYPE, &payload);

    let decoded = image_load(&png).unwrap();
    let metadata = decoded.metadata.as_ref().unwrap();

    match metadata.get(C2PA_JSON_KEY).unwrap() {
        DataMap::JSON(json) => {
            assert!(json.contains("\"source\":\"png-caBX\""));
            assert!(json.contains("\"label\":\"c2pa\""));
            assert!(json.contains("\"manifest_store_base64\""));
        }
        other => panic!("unexpected C2PA metadata: {other:?}"),
    }
    assert!(matches!(
        metadata.get(C2PA_RAW_KEY),
        Some(DataMap::Raw(raw)) if raw == &payload
    ));
}

#[test]
fn jpeg_app11_c2pa_metadata_is_reported_as_json_and_raw_payload() {
    let payload = c2pa_jumbf();
    let jpeg = encode_test_image(ImageFormat::Jpeg);
    let jpeg = insert_jpeg_app11_after_soi(&jpeg, &payload);

    let decoded = image_load(&jpeg).unwrap();
    let metadata = decoded.metadata.as_ref().unwrap();

    match metadata.get(C2PA_JSON_KEY).unwrap() {
        DataMap::JSON(json) => {
            assert!(json.contains("\"source\":\"jpeg-app11\""));
            assert!(json.contains("\"label\":\"c2pa\""));
            assert!(json.contains("63327061-0011-0010-8000-00aa00389b71"));
        }
        other => panic!("unexpected C2PA metadata: {other:?}"),
    }
    assert!(matches!(
        metadata.get(C2PA_RAW_KEY),
        Some(DataMap::Raw(raw)) if raw == &payload
    ));
}
