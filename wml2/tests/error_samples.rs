mod common;

use std::path::PathBuf;

use wml2::draw::image_from_file;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn error_sample_path(name: &str) -> PathBuf {
    repo_root().join(".test").join("errors").join(name)
}

fn decode_error_sample_if_available(name: &str) {
    let path = error_sample_path(name);
    if !path.is_file() {
        eprintln!(
            "skipping missing error sample: {name} (configure {})",
            common::sample_config_hint().display()
        );
        return;
    }

    let image = image_from_file(path.to_string_lossy().into_owned()).unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
}

#[test]
fn decode_56_byte_bmp_header_samples() {
    decode_error_sample_if_available("lena-01.bmp");
    decode_error_sample_if_available("lena-02.bmp");
    decode_error_sample_if_available("lena-03.bmp");
}

#[test]
fn decode_cmyk_jpeg_error_sample() {
    decode_error_sample_if_available("cmyk.jpg");
}
