#![cfg(all(feature = "pcx", not(feature = "noretoro")))]
use wml2::draw::{EncodeOptions, ImageBuffer, image_load};
use wml2::metadata::DataMap;

fn encode(
    width: usize,
    height: usize,
    pixels: Vec<u8>,
    alpha: Option<&str>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut image = ImageBuffer::from_buffer(width, height, pixels);
    let mut options = EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: alpha.map(|v| {
            [("alpha".into(), DataMap::Ascii(v.into()))]
                .into_iter()
                .collect()
        }),
    };
    wml2::draw::image_encoder(&mut options, wml2::util::ImageFormat::Pcx)
}

#[test]
fn rgb_roundtrip_padding_and_escaped_literals() {
    let pixels: Vec<u8> = (0..129 * 3)
        .flat_map(|i| [(i % 256) as u8, 192, (255 - i % 256) as u8, 255])
        .collect();
    let bytes = encode(129, 3, pixels.clone(), None).unwrap();
    assert_eq!(&bytes[..4], &[10, 5, 1, 8]);
    assert_eq!(bytes[65], 3);
    assert_eq!(u16::from_le_bytes([bytes[66], bytes[67]]), 130);
    assert_eq!(image_load(&bytes).unwrap().buffer.unwrap(), pixels);
}

#[test]
fn alpha_loss_requires_explicit_option() {
    assert!(
        encode(1, 1, vec![10, 20, 30, 128], None)
            .err()
            .unwrap()
            .to_string()
            .contains("alpha")
    );
    let bytes = encode(1, 1, vec![10, 20, 30, 128], Some("discard")).unwrap();
    assert_eq!(
        image_load(&bytes).unwrap().buffer.unwrap(),
        vec![10, 20, 30, 255]
    );
    assert!(encode(1, 1, vec![0, 0, 0, 255], Some("other")).is_err());
}

#[test]
fn rle_runs_split_at_63_and_rows() {
    let bytes = encode(64, 2, vec![255; 64 * 2 * 4], None).unwrap();
    // 192 samples per row: three 63-runs and a 3-run, separately per row.
    assert_eq!(
        &bytes[128..],
        &[
            255, 255, 255, 255, 255, 255, 195, 255, 255, 255, 255, 255, 255, 255, 195, 255
        ]
    );
    assert_eq!(
        image_load(&bytes).unwrap().buffer.unwrap(),
        vec![255; 64 * 2 * 4]
    );
}

#[test]
fn dimensions_and_callback_lengths_are_checked() {
    for (w, h) in [(0, 1), (65535, 1), (1, 0), (1, 65537)] {
        assert!(encode(w, h, vec![], None).is_err());
    }
    assert!(encode(2, 1, vec![255; 4], None).is_err());
}

#[test]
fn oversized_lowbit_palette_layouts_are_rejected() {
    for (bits, planes) in [(2, 3), (2, 4), (4, 2), (4, 3), (4, 4)] {
        let mut data = vec![0u8; 128];
        data[0] = 10;
        data[1] = 5;
        data[2] = 1;
        data[3] = bits;
        data[65] = planes;
        data[66] = 2;
        data.extend_from_slice(&[0; 8]);
        assert!(
            image_load(&data)
                .err()
                .unwrap()
                .to_string()
                .contains("Unsupported PCX pixel layout")
        );
    }
}

#[test]
#[ignore = "manual independent decoder and display/benchmark artifact"]
fn pcx_external_artifact() {
    let pixels: Vec<u8> = (0..257 * 129)
        .flat_map(|i| {
            [
                (i % 257) as u8,
                (i / 257 * 2) as u8,
                (i % 17 * 15) as u8,
                255,
            ]
        })
        .collect();
    let before = std::time::Instant::now();
    let bytes = encode(257, 129, pixels.clone(), None).unwrap();
    let start = std::time::Instant::now();
    for _ in 0..100 {
        std::hint::black_box(encode(257, 129, pixels.clone(), None).unwrap());
    }
    println!(
        "PCX encode first={:?}, 100 encodes={:?}",
        start.duration_since(before),
        start.elapsed()
    );
    let start = std::time::Instant::now();
    for _ in 0..100 {
        std::hint::black_box(image_load(&bytes).unwrap());
    }
    println!("PCX 100 decodes={:?}", start.elapsed());
    if let Ok(path) = std::env::var("WML2_PCX_EXTERNAL") {
        let image = image_load(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!((image.width, image.height), (65, 11));
        assert_eq!(
            image.buffer.unwrap(),
            std::fs::read(format!("{path}.rgba")).unwrap()
        );
    }
    let path = std::env::var("WML2_PCX_ARTIFACT").expect("WML2_PCX_ARTIFACT");
    std::fs::write(&path, bytes).unwrap();
    std::fs::write(format!("{path}.rgba"), pixels).unwrap();
}

#[test]
fn malformed_rle_is_rejected_and_plane_boundary_runs_work() {
    fn header() -> Vec<u8> {
        let mut data = vec![0u8; 128];
        data[..4].copy_from_slice(&[10, 5, 1, 8]);
        data[65] = 3;
        data[66] = 2;
        data
    }
    for stream in [&[0xc0, 1][..], &[0xc7, 1][..], &[0xc6][..], &[1, 2, 3][..]] {
        let mut data = header();
        data.extend_from_slice(stream);
        assert!(image_load(&data).is_err());
    }
    let mut data = header();
    data.extend_from_slice(&[0xc6, 42]);
    assert_eq!(
        image_load(&data).unwrap().buffer.unwrap(),
        vec![42, 42, 42, 255]
    );
}

#[test]
fn palette_modes_up_to_four_total_bits_remain_supported() {
    for (bits, planes) in [(1, 2), (1, 3), (1, 4), (2, 1), (2, 2), (4, 1)] {
        let mut data = vec![0u8; 128];
        data[..4].copy_from_slice(&[10, 5, 1, bits]);
        data[65] = planes;
        data[66] = 2;
        let index = (1usize << (bits * planes)) - 1;
        data[16 + index * 3..19 + index * 3].copy_from_slice(&[12, 34, 56]);
        for _ in 0..planes {
            data.extend_from_slice(&[0xc1, 255, 0]);
        }
        assert_eq!(
            image_load(&data).unwrap().buffer.unwrap(),
            vec![12, 34, 56, 255]
        );
    }
}

fn indexed_encode(
    width: usize,
    height: usize,
    pixels: Vec<u8>,
    alpha: Option<&str>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut image = ImageBuffer::from_buffer(width, height, pixels);
    let mut settings = std::collections::HashMap::new();
    settings.insert("color_type".into(), DataMap::Ascii("indexed".into()));
    if let Some(alpha) = alpha {
        settings.insert("alpha".into(), DataMap::Ascii(alpha.into()));
    }
    wml2::pcx::encode(&mut EncodeOptions {
        debug_flag: 0,
        drawer: &mut image,
        options: Some(settings),
    })
}

#[test]
fn indexed_palette_order_padding_and_roundtrip() {
    let pixels = vec![
        30, 20, 10, 255, 1, 2, 3, 255, 30, 20, 10, 255, 4, 5, 6, 255, 1, 2, 3, 255, 4, 5, 6, 255,
    ];
    let bytes = indexed_encode(3, 2, pixels.clone(), None).unwrap();
    assert_eq!(bytes[65], 1);
    assert_eq!(bytes[66], 4);
    assert_eq!(bytes[bytes.len() - 769], 12);
    assert_eq!(
        &bytes[bytes.len() - 768..bytes.len() - 759],
        &[30, 20, 10, 1, 2, 3, 4, 5, 6]
    );
    assert!(bytes[bytes.len() - 759..].iter().all(|v| *v == 0));
    assert_eq!(image_load(&bytes).unwrap().buffer.unwrap(), pixels);
    assert_eq!(indexed_encode(3, 2, pixels, None).unwrap(), bytes);
}

#[test]
fn indexed_accepts_256_colors_and_rejects_257_without_quantization() {
    let mut pixels: Vec<u8> = (0..256).flat_map(|i| [i as u8, 0, 0, 255]).collect();
    let bytes = indexed_encode(256, 1, pixels.clone(), None).unwrap();
    assert_eq!(image_load(&bytes).unwrap().buffer.unwrap(), pixels);
    pixels.extend_from_slice(&[0, 1, 0, 255]);
    assert!(
        indexed_encode(257, 1, pixels, None)
            .err()
            .unwrap()
            .to_string()
            .contains("256 exact")
    );
}

#[test]
fn indexed_alpha_policy_is_explicit() {
    assert!(indexed_encode(1, 1, vec![1, 2, 3, 0], None).is_err());
    let bytes = indexed_encode(1, 1, vec![1, 2, 3, 0], Some("discard")).unwrap();
    assert_eq!(
        image_load(&bytes).unwrap().buffer.unwrap(),
        vec![1, 2, 3, 255]
    );
}

#[test]
#[ignore = "manual Pillow indexed interoperability artifact"]
fn pcx_indexed_external_artifact() {
    let pixels: Vec<u8> = (0..257 * 11)
        .flat_map(|i| [(i % 256) as u8, 0, 0, 255])
        .collect();
    let bytes = indexed_encode(257, 11, pixels.clone(), None).unwrap();
    let path = std::env::var("WML2_PCX_INDEXED_ARTIFACT").unwrap();
    std::fs::write(&path, bytes).unwrap();
    std::fs::write(format!("{path}.rgba"), pixels).unwrap();
    if let Ok(path) = std::env::var("WML2_PCX_INDEXED_EXTERNAL") {
        let image = image_load(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            image.buffer.unwrap(),
            std::fs::read(format!("{path}.rgba")).unwrap()
        );
    }
}

#[test]
fn indexed_checks_dimensions_and_source_rows() {
    assert!(indexed_encode(65535, 1, vec![], None).is_err());
    assert!(indexed_encode(2, 1, vec![255; 4], None).is_err());
    let mut image = ImageBuffer::from_buffer(1, 1, vec![0, 0, 0, 255]);
    for value in [DataMap::Ascii("gray".into()), DataMap::UInt(1)] {
        let options = Some([("color_type".into(), value)].into_iter().collect());
        assert!(
            wml2::pcx::encode(&mut EncodeOptions {
                debug_flag: 0,
                drawer: &mut image,
                options
            })
            .is_err()
        );
    }
}

#[test]
fn indexed_encoder_honors_pixel_and_expanded_limits() {
    for (pixel_limit, byte_limit) in [(1, usize::MAX), (usize::MAX, 7)] {
        let mut image = ImageBuffer::from_buffer(2, 1, vec![255; 8]);
        let mut limits = wml2::limits::DecodeLimits::unlimited();
        limits.pixels = pixel_limit;
        limits.expanded_bytes = byte_limit;
        let options = Some(
            [("color_type".into(), DataMap::Ascii("indexed".into()))]
                .into_iter()
                .collect(),
        );
        assert!(
            wml2::draw::image_to_with_limits(
                &mut image,
                wml2::util::ImageFormat::Pcx,
                options,
                limits
            )
            .is_err()
        );
    }
}

fn old_pcx(version: u8, bits: u8, planes: u8, width: u16, raster: &[u8]) -> Vec<u8> {
    let mut data = vec![0u8; 128];
    data[..4].copy_from_slice(&[10, version, 1, bits]);
    data[8..10].copy_from_slice(&(width - 1).to_le_bytes());
    data[65] = planes;
    data[66] = 2;
    data.extend_from_slice(raster);
    data
}

#[test]
fn old_ega_versions_ignore_header_palette() {
    let expected: Vec<u8> = [
        [0, 0, 0, 255],
        [0, 0, 170, 255],
        [0, 170, 0, 255],
        [0, 170, 170, 255],
        [170, 0, 0, 255],
        [170, 0, 170, 255],
        [170, 85, 0, 255],
        [170, 170, 170, 255],
        [85, 85, 85, 255],
        [85, 85, 255, 255],
        [85, 255, 85, 255],
        [85, 255, 255, 255],
        [255, 85, 85, 255],
        [255, 85, 255, 255],
        [255, 255, 85, 255],
        [255, 255, 255, 255],
    ]
    .into_iter()
    .flatten()
    .collect();
    for version in [0, 3] {
        // Four planes, 16 pixels: bit0 toggles fastest, bit3 slowest.
        let mut data = old_pcx(
            version,
            1,
            4,
            16,
            &[0x55, 0x55, 0x33, 0x33, 0x0f, 0x0f, 0, 0xc1, 255],
        );
        data[16..64].fill(37);
        assert_eq!(image_load(&data).unwrap().buffer.unwrap(), expected);
    }
}

#[test]
fn old_cga_control_bits_and_background_are_interpreted() {
    for version in [0, 2, 3] {
        for (control, colors) in [
            (0, [[0, 170, 0], [170, 0, 0], [170, 85, 0]]),
            (0x20, [[85, 255, 85], [255, 85, 85], [255, 255, 85]]),
            (0x40, [[0, 170, 170], [170, 0, 170], [170, 170, 170]]),
            (0x60, [[85, 255, 255], [255, 85, 255], [255, 255, 255]]),
        ] {
            let mut data = old_pcx(version, 2, 1, 4, &[0x1b, 0]);
            data[16] = 0x9f;
            data[19] = control | 0x1f;
            let expected: Vec<u8> = std::iter::once([85, 85, 255])
                .chain(colors)
                .flat_map(|c| [c[0], c[1], c[2], 255])
                .collect();
            assert_eq!(image_load(&data).unwrap().buffer.unwrap(), expected);
        }
    }
}

#[test]
fn cga_ambiguous_monochrome_and_unknown_versions_are_rejected() {
    let mut data = old_pcx(2, 2, 1, 4, &[0x1b, 0]);
    data[19] = 0x80;
    assert!(
        image_load(&data)
            .err()
            .unwrap()
            .to_string()
            .contains("CGA monochrome")
    );
    data[1] = 1;
    data[19] = 0;
    assert!(
        image_load(&data)
            .err()
            .unwrap()
            .to_string()
            .contains("PCX version")
    );
    for version in [0, 2, 3, 4, 5] {
        let data = old_pcx(version, 1, 1, 2, &[0x40, 0]);
        assert_eq!(
            image_load(&data).unwrap().buffer.unwrap(),
            vec![255, 255, 255, 255, 0, 0, 0, 255]
        );
    }
}

#[test]
fn newer_packed_cga_depth_uses_rgb_header_palette() {
    for version in [4, 5] {
        let mut data = old_pcx(version, 2, 1, 4, &[0x1b, 0]);
        for i in 0..4 {
            data[16 + i * 3..19 + i * 3].copy_from_slice(&[i as u8, 22, 33]);
        }
        let expected: Vec<u8> = (0..4).flat_map(|i| [i, 22, 33, 255]).collect();
        assert_eq!(image_load(&data).unwrap().buffer.unwrap(), expected);
    }
}

#[test]
fn encoded_budget_includes_header_escaped_rle_and_palette_exactly() {
    fn bounded(
        indexed: bool,
        limit: usize,
        pixels: Vec<u8>,
        width: usize,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut image = ImageBuffer::from_buffer(width, 1, pixels);
        let settings = Some(
            [(
                "color_type".into(),
                DataMap::Ascii(if indexed { "indexed" } else { "rgb" }.into()),
            )]
            .into_iter()
            .collect(),
        );
        let mut limits = wml2::limits::DecodeLimits::unlimited();
        limits.expanded_bytes = limit;
        wml2::draw::image_to_with_limits(&mut image, wml2::util::ImageFormat::Pcx, settings, limits)
    }
    // Three isolated >=192 literals each need an escape; each plane has padding.
    let rgb = vec![192, 193, 194, 255];
    let encoded = encode(1, 1, rgb.clone(), None).unwrap();
    assert_eq!(encoded.len(), 137);
    assert!(bounded(false, 127, rgb.clone(), 1).is_err());
    assert!(bounded(false, 136, rgb.clone(), 1).is_err());
    assert_eq!(bounded(false, 137, rgb, 1).unwrap().len(), 137);
    let pixels = vec![0, 0, 0, 255];
    let encoded = indexed_encode(1, 1, pixels.clone(), None).unwrap();
    assert_eq!(encoded.len(), 899);
    assert!(bounded(true, 898, pixels.clone(), 1).is_err());
    assert_eq!(bounded(true, 899, pixels, 1).unwrap().len(), 899);
    let escaped: Vec<u8> = (0..16)
        .flat_map(|i| [192 + i, 224 + i, 208 + i, 255])
        .collect();
    assert_eq!(encode(16, 1, escaped.clone(), None).unwrap().len(), 224);
    assert!(bounded(false, 223, escaped.clone(), 16).is_err());
    assert!(bounded(false, 224, escaped, 16).is_ok());
    // No worst-case bound rejection: constant image compresses far below it.
    assert!(bounded(false, 255, vec![255; 64 * 4], 64).is_err()); // independentRGBA budget
    assert!(bounded(false, 256, vec![255; 64 * 4], 64).is_ok());
}

#[test]
fn old_pcx_signature_precedes_broad_tga_and_vsp_signatures() {
    for version in [0, 2, 3] {
        let mut data = old_pcx(version, 2, 1, 4, &[0x1b, 0]);
        data[12..16].copy_from_slice(&[72, 0, 72, 0]);
        data[16] = 16;
        assert!(matches!(
            wml2::util::format_check(&data),
            wml2::util::ImageFormat::Pcx
        ));
    }
}

#[test]
#[ignore = "manual old-palette display and independent oracle comparison"]
fn pcx_old_palette_artifacts() {
    let root = std::env::var("WML2_PCX_OLD_ARTIFACT_DIR").unwrap();
    for name in ["ega0", "ega3", "cga0", "cga32", "cga64", "cga96"] {
        let path = format!("{root}/{name}.pcx");
        let image = image_load(&std::fs::read(path).unwrap()).unwrap();
        std::fs::write(format!("{root}/{name}.wml.rgba"), image.buffer.unwrap()).unwrap();
    }
}
