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
