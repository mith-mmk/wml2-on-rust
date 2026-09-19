use std::env;
use std::fs;
use std::path::Path;

use wml2::draw::image_load;
use wml2::util::{ImageFormat, format_check};

fn q4_header() -> Vec<u8> {
    let mut header = vec![0u8; 22];
    header[0..2].copy_from_slice(&0x001au16.to_le_bytes());
    header[2] = 0x12;
    header[3] = 1;
    header[4] = 1;
    header[11..16].copy_from_slice(b"MAJYO");
    header
}

#[test]
fn detects_q4_header() {
    assert!(matches!(format_check(&q4_header()), ImageFormat::Q4));
    assert!(
        wml2::get_decoder_extentions()
            .iter()
            .any(|extension| extension == "q4")
    );
}

#[test]
fn rejects_truncated_q4() {
    assert!(image_load(&q4_header()).is_err());
    assert!(image_load(b"Q4").is_err());
}

#[test]
fn decodes_collected_q4_samples() {
    let Some(root) = env::var_os("WML2_Q4_SAMPLE_ROOT") else {
        return;
    };
    let root = Path::new(&root).join("samples");
    let mut count = 0;
    for entry in fs::read_dir(&root).expect("Q4 sample directory should be readable") {
        let path = entry.expect("Q4 sample entry should be readable").path();
        if !path.is_file()
            || !path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("q4"))
        {
            continue;
        }
        let data = fs::read(&path).expect("Q4 sample should be readable");
        let image = image_load(&data)
            .unwrap_or_else(|error| panic!("{} should decode: {error}", path.display()));
        assert_eq!((image.width, image.height), (640, 400));
        let buffer = image.buffer.as_ref().expect("Q4 image should have pixels");
        assert_eq!(buffer.len(), 640 * 400 * 4);
        if path.file_name().is_some_and(|name| name == "30000.Q4") {
            for (pixel, rgb) in [
                (0usize, [255, 255, 255]),
                (639, [221, 255, 238]),
                (640, [255, 255, 255]),
                (1000, [221, 255, 238]),
            ] {
                let offset = pixel * 4;
                assert_eq!(&buffer[offset..offset + 4], &[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        count += 1;
    }
    assert!(count >= 1, "at least one Q4 sample is required");
}
