use wml2::draw::{image_from_with_limits, image_load};
use wml2::limits::DecodeLimits;
use wml2::util::{ImageFormat, format_check};

fn put_le16(buffer: &mut [u8], offset: usize, value: u16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_le32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_be16(buffer: &mut [u8], offset: usize, value: u16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn put_be32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
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

fn dds_dx10_1x1(dxgi: u32, pixel: &[u8]) -> Vec<u8> {
    let mut data = vec![0u8; 148];
    data[0..4].copy_from_slice(b"DDS ");
    put_le32(&mut data, 4, 124);
    put_le32(&mut data, 8, 0x100f);
    put_le32(&mut data, 12, 1);
    put_le32(&mut data, 16, 1);
    put_le32(&mut data, 20, pixel.len() as u32);
    put_le32(&mut data, 76, 32);
    put_le32(&mut data, 80, 0x4);
    data[84..88].copy_from_slice(b"DX10");
    put_le32(&mut data, 128, dxgi);
    put_le32(&mut data, 132, 3);
    put_le32(&mut data, 136, 0);
    put_le32(&mut data, 140, 1);
    put_le32(&mut data, 144, 0);
    data.extend_from_slice(pixel);
    data
}

fn dds_fourcc_1x1(fourcc: &[u8; 4], pixel: &[u8]) -> Vec<u8> {
    let mut data = vec![0u8; 128];
    data[0..4].copy_from_slice(b"DDS ");
    put_le32(&mut data, 4, 124);
    put_le32(&mut data, 8, 0x100f);
    put_le32(&mut data, 12, 1);
    put_le32(&mut data, 16, 1);
    put_le32(&mut data, 20, pixel.len() as u32);
    put_le32(&mut data, 76, 32);
    put_le32(&mut data, 80, 0x4);
    data[84..88].copy_from_slice(fourcc);
    data.extend_from_slice(pixel);
    data
}

fn pic2_1x1(block_x: u16) -> Vec<u8> {
    let mut data = vec![0u8; 153];
    data[0..4].copy_from_slice(b"P2DT");
    put_be32(&mut data, 106, 124);
    put_be16(&mut data, 110, 24);
    put_be16(&mut data, 112, 1);
    put_be16(&mut data, 114, 1);
    put_be16(&mut data, 116, 1);
    put_be16(&mut data, 118, 1);

    let block = 124;
    data[block..block + 4].copy_from_slice(b"P2BM");
    put_be32(&mut data, block + 4, 29);
    put_be16(&mut data, block + 10, 1);
    put_be16(&mut data, block + 12, 1);
    put_be16(&mut data, block + 14, block_x);
    put_be16(&mut data, block + 16, 0);
    data[block + 26..block + 29].copy_from_slice(&[0, 0, 255]);
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
fn tga_checks_decode_limits_before_allocating_pixels() {
    let mut data = vec![0u8; 18];
    data[2] = 2;
    put_le16(&mut data, 12, 1024);
    put_le16(&mut data, 14, 1024);
    data[16] = 24;

    let result = image_from_with_limits(
        &data,
        DecodeLimits {
            pixels: 1,
            expanded_bytes: 4,
            ..DecodeLimits::default()
        },
    );
    let error = match result {
        Ok(_) => panic!("the TGA pixel budget must be enforced"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("pixels exceeds decode limit"));
}

#[test]
fn rejects_pic2_blocks_outside_the_declared_canvas() {
    assert!(image_load(&pic2_1x1(0)).is_ok());
    let error = match image_load(&pic2_1x1(1)) {
        Ok(_) => panic!("outside-canvas PIC2 block must fail"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("outside canvas"));
}

#[cfg(feature = "vsp")]
#[test]
fn vsp_detection_precedes_tga_detection() {
    let mut data = tga_2x1();
    data[4] = 8;
    data[6] = 3;
    data[8] = 0;
    data.resize(58, 0);
    assert!(matches!(format_check(&data), ImageFormat::Vsp));
}

#[test]
fn decodes_signed_bc4_and_bc5() {
    let mut bc4 = [0u8; 8];
    bc4[0] = 0;
    bc4[1] = 127;
    assert_eq!(
        image_load(&dds_fourcc_1x1(b"BC4S", &bc4))
            .unwrap()
            .buffer
            .unwrap(),
        [128, 128, 128, 255]
    );

    let mut bc5 = [0u8; 16];
    bc5[0] = 0;
    bc5[1] = 127;
    bc5[8] = 0x80;
    bc5[9] = 127;
    assert_eq!(
        image_load(&dds_fourcc_1x1(b"BC5S", &bc5))
            .unwrap()
            .buffer
            .unwrap(),
        [128, 0, 0, 255]
    );
}

#[test]
fn decodes_dx10_rgba8_variants() {
    assert_eq!(
        image_load(&dds_dx10_1x1(28, &[1, 2, 3, 4]))
            .unwrap()
            .buffer
            .unwrap(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        image_load(&dds_dx10_1x1(91, &[3, 2, 1, 4]))
            .unwrap()
            .buffer
            .unwrap(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        image_load(&dds_dx10_1x1(88, &[3, 2, 1, 0]))
            .unwrap()
            .buffer
            .unwrap(),
        [1, 2, 3, 255]
    );
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
