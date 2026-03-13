#![cfg(not(feature = "noretoro"))]

mod common;

use common::{sample_bytes, sample_config_hint, sample_path};
use wml2::draw::{image_from_file, image_load};

fn decode_from_file_if_available(path: &str, require_animation: bool) {
    let Some(path) = sample_path(path) else {
        eprintln!(
            "skipping missing sample: {path} (configure {})",
            sample_config_hint().display()
        );
        return;
    };

    let image = image_from_file(path.to_string_lossy().into_owned()).unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
    if require_animation {
        assert!(
            image
                .animation
                .as_ref()
                .map(|frames| !frames.is_empty())
                .unwrap_or(false)
        );
    }
}

fn decode_from_bytes_if_available(path: &str, require_animation: bool) {
    let Some(bytes) = sample_bytes(path) else {
        eprintln!(
            "skipping missing sample: {path} (configure {})",
            sample_config_hint().display()
        );
        return;
    };

    let image = image_load(&bytes).unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
    if require_animation {
        assert!(
            image
                .animation
                .as_ref()
                .map(|frames| !frames.is_empty())
                .unwrap_or(false)
        );
    }
}

#[test]
fn decode_maki_sample() {
    decode_from_file_if_available("sample.mki", false);
}

#[test]
fn decode_maki_sample_from_bytes() {
    decode_from_bytes_if_available("sample.mki", false);
}

#[test]
fn decode_pi_sample() {
    decode_from_file_if_available("sample.pi", false);
}

#[test]
fn decode_pi_sample_from_bytes() {
    decode_from_bytes_if_available("sample.pi", false);
}

#[test]
fn decode_pi_second_sample() {
    decode_from_file_if_available("sample2.pi", false);
}

#[test]
fn decode_pi_second_sample_from_bytes() {
    decode_from_bytes_if_available("sample2.pi", false);
}

#[test]
fn decode_pic_sample() {
    decode_from_file_if_available("sample.pic", false);
}

#[test]
fn decode_pic_sample_from_bytes() {
    decode_from_bytes_if_available("sample.pic", false);
}

#[test]
fn decode_vsp_dat_sample() {
    decode_from_file_if_available("sample.dat", true);
}

#[test]
fn decode_vsp_dat_sample_from_bytes() {
    decode_from_bytes_if_available("sample.dat", true);
}
