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

fn tga_16bit_scaled() -> Vec<u8> {
    let mut data = vec![0u8; 18];
    data[2] = 2;
    put_le16(&mut data, 12, 1);
    put_le16(&mut data, 14, 1);
    data[16] = 16;
    let value = (5u16 << 10) | (27u16 << 5) | 24;
    data.extend_from_slice(&value.to_le_bytes());
    data
}

fn tga_transparent_32bit() -> Vec<u8> {
    let mut data = vec![0u8; 18];
    data[2] = 2;
    put_le16(&mut data, 12, 1);
    put_le16(&mut data, 14, 1);
    data[16] = 32;
    data[17] = 8;
    data.extend_from_slice(&[115, 156, 24, 0]);
    data
}

fn tga_truecolor_with_unused_cmap() -> Vec<u8> {
    let mut data = vec![0u8; 18];
    data[1] = 1;
    data[2] = 2;
    put_le16(&mut data, 5, 1);
    data[7] = 24;
    put_le16(&mut data, 12, 1);
    put_le16(&mut data, 14, 1);
    data[16] = 24;
    data.extend_from_slice(&[0, 255, 0]);
    data.extend_from_slice(&[0, 0, 255]);
    data
}

fn tga_indexed_16bit_palette() -> Vec<u8> {
    let mut data = vec![0u8; 18];
    data[1] = 1;
    data[2] = 1;
    put_le16(&mut data, 5, 1);
    data[7] = 16;
    put_le16(&mut data, 12, 1);
    put_le16(&mut data, 14, 1);
    data[16] = 8;
    data.extend_from_slice(&[0x00, 0x7c]);
    data.push(0);
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

fn pcx_1bit_colored_palette() -> Vec<u8> {
    let mut data = vec![0u8; 128];
    data[0] = 0x0a;
    data[1] = 5;
    data[2] = 1;
    data[3] = 1;
    data[65] = 1;
    put_le16(&mut data, 8, 1);
    put_le16(&mut data, 10, 0);
    put_le16(&mut data, 66, 1);
    data[16..19].copy_from_slice(&[255, 0, 0]);
    data[19..22].copy_from_slice(&[0, 255, 0]);
    data.push(0x40);
    data
}

fn pcx_large_header() -> Vec<u8> {
    let mut data = vec![0u8; 128];
    data[0] = 0x0a;
    data[1] = 5;
    data[2] = 1;
    data[3] = 8;
    data[65] = 1;
    put_le16(&mut data, 8, 1023);
    put_le16(&mut data, 10, 1023);
    put_le16(&mut data, 66, 1024);
    data.push(12);
    data.extend_from_slice(&[0u8; 768]);
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

fn dds_alpha_only_1x1(alpha: u8) -> Vec<u8> {
    let mut data = vec![0u8; 128];
    data[0..4].copy_from_slice(b"DDS ");
    put_le32(&mut data, 4, 124);
    put_le32(&mut data, 8, 0x100f);
    put_le32(&mut data, 12, 1);
    put_le32(&mut data, 16, 1);
    put_le32(&mut data, 20, 1);
    put_le32(&mut data, 76, 32);
    put_le32(&mut data, 80, 0x2);
    put_le32(&mut data, 88, 8);
    put_le32(&mut data, 104, 0xff);
    put_le32(&mut data, 108, 0x1000);
    data.push(alpha);
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

fn pic2_image(width: u16, height: u16, block_x: u16, block_y: u16) -> Vec<u8> {
    let mut data = vec![0u8; 153];
    data[0..4].copy_from_slice(b"P2DT");
    put_be32(&mut data, 106, 124);
    put_be16(&mut data, 110, 24);
    put_be16(&mut data, 112, 1);
    put_be16(&mut data, 114, 1);
    put_be16(&mut data, 116, width);
    put_be16(&mut data, 118, height);

    let block = 124;
    data[block..block + 4].copy_from_slice(b"P2BM");
    put_be32(&mut data, block + 4, 29);
    put_be16(&mut data, block + 10, 1);
    put_be16(&mut data, block + 12, 1);
    put_be16(&mut data, block + 14, block_x);
    put_be16(&mut data, block + 16, block_y);
    data[block + 26..block + 29].copy_from_slice(&[0, 0, 255]);
    data
}

fn pic2_1x1(block_x: u16) -> Vec<u8> {
    pic2_image(1, 1, block_x, 0)
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
fn pcx_single_bit_plane_matches_imagemagick_binary_output() {
    let image = image_load(&pcx_1bit_colored_palette()).expect("1-bit PCX should decode");
    assert_eq!(
        image.buffer.unwrap(),
        vec![255, 255, 255, 255, 0, 0, 0, 255]
    );
}

#[test]
fn tga_16bit_colors_use_imagemagick_8bit_quantization() {
    let image = image_load(&tga_16bit_scaled()).expect("16-bit TGA should decode");
    assert_eq!(image.buffer.unwrap(), vec![41, 222, 197, 255]);
}

#[test]
fn tga_zero_alpha_clears_rgb() {
    let image = image_load(&tga_transparent_32bit()).expect("transparent TGA should decode");
    assert_eq!(image.buffer.unwrap(), vec![0, 0, 0, 0]);
}

#[test]
fn tga_truecolor_accepts_an_unused_color_map() {
    let image = image_load(&tga_truecolor_with_unused_cmap())
        .expect("true-color TGA may carry an unused color map");
    assert_eq!(image.buffer.unwrap(), vec![255, 0, 0, 255]);
}

#[test]
fn tga_indexed_16bit_palette_without_alpha_is_opaque() {
    let image = image_load(&tga_indexed_16bit_palette()).expect("indexed TGA should decode");
    assert_eq!(image.buffer.unwrap(), vec![255, 0, 0, 255]);
}

#[test]
fn decodes_minimal_dds() {
    let image = image_load(&dds_2x1()).expect("DDS should decode");
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.buffer.unwrap(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
}

#[test]
fn dds_uses_ddsd_pitch_from_header_flags() {
    let mut data = dds_2x1();
    put_le32(&mut data, 8, 0x1007);
    put_le32(&mut data, 20, 4);
    put_le32(&mut data, 80, 0x48);

    let image = image_load(&data).expect("DDSD_PITCH is a header flag");
    assert_eq!(image.buffer.unwrap(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
}

#[test]
fn dds_alpha_only_uses_the_alpha_mask() {
    let image = image_load(&dds_alpha_only_1x1(0x80)).expect("alpha-only DDS should decode");
    assert_eq!(image.buffer.as_deref(), Some(&[0, 0, 0, 0x80][..]));
}

#[test]
fn pcx_rejects_eight_bit_two_plane_layouts() {
    let mut data = pcx_2x1();
    data[65] = 2;
    let error = match image_load(&data) {
        Ok(_) => panic!("8-bit two-plane PCX must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("Unsupported PCX pixel layout"));
}

#[test]
fn pcx_rejects_eight_bit_single_plane_without_palette() {
    let mut data = pcx_2x1();
    data.truncate(130);
    let error = match image_load(&data) {
        Ok(_) => panic!("8-bit PCX without a palette must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("256-color palette is missing"));
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

#[test]
fn pic2_checks_decode_limits_before_allocating_output() {
    let result = image_from_with_limits(
        &pic2_image(1024, 1024, 0, 0),
        DecodeLimits {
            pixels: usize::MAX,
            expanded_bytes: 4,
            ..DecodeLimits::default()
        },
    );
    let error = match result {
        Ok(_) => panic!("the PIC2 pixel budget must be enforced"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("RGBA image exceeds decode limit")
    );
}

#[test]
fn pcx_checks_decode_limits_before_allocating_output() {
    let result = image_from_with_limits(
        &pcx_large_header(),
        DecodeLimits {
            pixels: 1,
            expanded_bytes: 4,
            ..DecodeLimits::default()
        },
    );
    let error = match result {
        Ok(_) => panic!("the PCX pixel budget must be enforced"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("pixels exceeds decode limit"));
}

#[test]
fn dds_checks_decode_limits_before_allocating_output() {
    let mut uncompressed = dds_2x1();
    put_le32(&mut uncompressed, 12, 1024);
    put_le32(&mut uncompressed, 16, 1024);
    put_le32(&mut uncompressed, 20, 4096);

    let mut compressed = dds_fourcc_1x1(b"DXT1", &[0u8; 8]);
    put_le32(&mut compressed, 12, 1024);
    put_le32(&mut compressed, 16, 1024);

    for data in [uncompressed, compressed] {
        let result = image_from_with_limits(
            &data,
            DecodeLimits {
                pixels: 1,
                expanded_bytes: 4,
                ..DecodeLimits::default()
            },
        );
        let error = match result {
            Ok(_) => panic!("the DDS pixel budget must be enforced"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("pixels exceeds decode limit"),
            "unexpected DDS error: {error}"
        );
    }
}

#[cfg(feature = "vsp")]
#[test]
fn vsp_detection_precedes_tga_detection() {
    let mut data = vec![0u8; 58];
    put_le16(&mut data, 0, 0);
    put_le16(&mut data, 2, 0);
    put_le16(&mut data, 4, 8);
    put_le16(&mut data, 6, 1);
    data[8] = 0;
    assert!(matches!(format_check(&data), ImageFormat::Vsp));
}

#[test]
fn tga_detection_wins_for_an_overlapping_vsp_signature() {
    let mut data = vec![0u8; 58];
    put_le16(&mut data, 0, 0);
    put_le16(&mut data, 2, 2);
    put_le16(&mut data, 4, 8);
    put_le16(&mut data, 6, 3);
    data[8] = 0;
    data[2] = 2;
    put_le16(&mut data, 12, 2);
    put_le16(&mut data, 14, 1);
    data[16] = 24;
    assert!(matches!(format_check(&data), ImageFormat::Tga));
}

#[test]
fn dds_rejects_invalid_pixel_format_size() {
    let mut data = dds_2x1();
    put_le32(&mut data, 76, 0);
    let error = match image_load(&data) {
        Ok(_) => panic!("invalid DDS pixel format size must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("pixel format size"));
}

#[test]
fn dds_rejects_dx10_non_2d_surfaces() {
    let mut volume = dds_dx10_1x1(27, &[0, 0, 255, 255]);
    put_le32(&mut volume, 132, 4);
    let error = match image_load(&volume) {
        Ok(_) => panic!("DX10 3D textures must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("resource dimension"));

    let mut array = dds_dx10_1x1(27, &[0, 0, 255, 255]);
    put_le32(&mut array, 140, 2);
    let error = match image_load(&array) {
        Ok(_) => panic!("DX10 arrays must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("array size"));

    let mut cube = dds_dx10_1x1(27, &[0, 0, 255, 255]);
    put_le32(&mut cube, 136, 0x4);
    let error = match image_load(&cube) {
        Ok(_) => panic!("DX10 cubemaps must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("cubemap"));
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
fn decodes_premultiplied_dxt2_and_dxt4() {
    let mut dxt2 = [0u8; 16];
    dxt2[..8].fill(0x88);
    dxt2[8..10].copy_from_slice(&0x4000u16.to_le_bytes());
    assert_eq!(
        image_load(&dds_fourcc_1x1(b"DXT2", &dxt2))
            .unwrap()
            .buffer
            .unwrap(),
        [124, 0, 0, 136]
    );

    let mut dxt4 = [0u8; 16];
    dxt4[0] = 136;
    dxt4[1] = 0;
    dxt4[8..10].copy_from_slice(&0x4000u16.to_le_bytes());
    assert_eq!(
        image_load(&dds_fourcc_1x1(b"DXT4", &dxt4))
            .unwrap()
            .buffer
            .unwrap(),
        [124, 0, 0, 136]
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
