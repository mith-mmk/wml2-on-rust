use std::panic;

use wml2::draw::image_load;

fn malformed_pi_stream() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"Pi");
    bytes.push(0x1a);
    bytes.push(0x00);
    bytes.push(0x80);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x04);
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&3u16.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    bytes
}

fn malformed_pic_stream() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PIC");
    bytes.push(0x1a);
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.push(0x08);
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&3u16.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    bytes
}

#[test]
fn malformed_pi_stream_does_not_panic() {
    let bytes = malformed_pi_stream();
    let result = panic::catch_unwind(|| image_load(&bytes));
    assert!(result.is_ok(), "malformed PI stream caused a panic");
}

#[test]
fn malformed_pic_stream_does_not_panic() {
    let bytes = malformed_pic_stream();
    let result = panic::catch_unwind(|| image_load(&bytes));
    assert!(result.is_ok(), "malformed PIC stream caused a panic");
}
