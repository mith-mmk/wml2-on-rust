#![cfg(not(feature = "noretoro"))]

use std::path::PathBuf;

use wml2::draw::image_from_file;

fn sample_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test")
        .join("samples")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn decode_maki_sample() {
    let image = image_from_file(sample_path("sample.mki")).unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
}

#[test]
fn decode_pi_sample() {
    let image = image_from_file(sample_path("sample.pi")).unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
}

#[test]
fn decode_pi_second_sample() {
    let image = image_from_file(sample_path("sample2.pi")).unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
}

#[test]
fn decode_pic_sample() {
    let image = image_from_file(sample_path("sample.pic")).unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
}

#[test]
fn decode_vsp_dat_sample() {
    let image = image_from_file(sample_path("sample.dat")).unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
    assert!(image.animation.as_ref().map(|frames| !frames.is_empty()).unwrap_or(false));
}
