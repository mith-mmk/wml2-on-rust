use wml2::draw::image_load;
use wml2::util::{ImageFormat, format_check};

fn put_le16(buffer: &mut [u8], offset: usize, value: u16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_le32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn tga_2x1() -> Vec<u8> {
    let mut data = vec![0u8; 18];
    data[2] = 2;
    put_le16(&mut data, 12, 2);
    put_le16(&mut data, 14, 1);
    data[16] = 24;
    data.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
    data
}

fn pcx_2x1() -> Vec<u8> {
    let mut data = vec![0u8; 128];
    data[0] = 0x0a;
    data[1] = 5;
    data[2] = 1;
    data[3] = 8;
    data[65] = 1;
    put_le16(&mut data, 8, 1);
    put_le16(&mut data, 10, 0);
    put_le16(&mut data, 66, 2);
    data.extend_from_slice(&[0, 1]);
    data.push(12);
    let mut palette = vec![0u8; 768];
    palette[0..3].copy_from_slice(&[255, 0, 0]);
    palette[3..6].copy_from_slice(&[0, 255, 0]);
    data.extend_from_slice(&palette);
    data
}

fn dds_2x1() -> Vec<u8> {
    let mut data = vec![0u8; 128];
    data[0..4].copy_from_slice(b"DDS ");
    put_le32(&mut data, 4, 124);
    put_le32(&mut data, 8, 0x100f);
    put_le32(&mut data, 12, 1);
    put_le32(&mut data, 16, 2);
    put_le32(&mut data, 20, 8);
    put_le32(&mut data, 76, 32);
    put_le32(&mut data, 80, 0x41);
    put_le32(&mut data, 88, 32);
    put_le32(&mut data, 92, 0x00ff0000);
    put_le32(&mut data, 96, 0x0000ff00);
    put_le32(&mut data, 100, 0x000000ff);
    put_le32(&mut data, 104, 0xff000000);
    put_le32(&mut data, 108, 0x1000);
    data.extend_from_slice(&[0, 0, 255, 255, 0, 255, 0, 255]);
    data
}

#[test]
fn detects_tga_pcx_dds_and_pic2_signatures() {
    assert!(matches!(format_check(&tga_2x1()), ImageFormat::Tga));
    assert!(matches!(format_check(&pcx_2x1()), ImageFormat::Pcx));
    assert!(matches!(format_check(&dds_2x1()), ImageFormat::Dds));
    assert!(matches!(format_check(b"P2DT"), ImageFormat::Pic2));
}

#[test]
fn decodes_minimal_tga() {
    let image = image_load(&tga_2x1()).expect("TGA should decode");
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.buffer.unwrap(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
}

#[test]
fn decodes_minimal_pcx() {
    let image = image_load(&pcx_2x1()).expect("PCX should decode");
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.buffer.unwrap(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
}

#[test]
fn decodes_minimal_dds() {
    let image = image_load(&dds_2x1()).expect("DDS should decode");
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.buffer.unwrap(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
}

#[test]
fn rejects_truncated_new_format_inputs() {
    for input in [
        b"TGA".as_slice(),
        b"PCX".as_slice(),
        b"DDS".as_slice(),
        b"P2D".as_slice(),
    ] {
        assert!(image_load(input).is_err());
    }
}
