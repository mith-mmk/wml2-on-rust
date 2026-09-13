#![cfg(feature = "tiff")]

use wml2::decoder::lzw::Lzwdecode;
use wml2::encoder::lzw::{encode_tiff_libtiff_compat, encode_tiff_standard, encode_tiff_wml2_lsb};

fn patterned_bytes() -> Vec<u8> {
    (0..4096usize)
        .map(|index| ((index * 37 + index / 127) % 256) as u8)
        .collect()
}

#[test]
fn standard_tiff_lzw_is_msb_early_change() {
    let source = patterned_bytes();
    let encoded = encode_tiff_standard(&source).unwrap();
    assert_eq!(encoded.first(), Some(&0x80));
    assert_eq!(
        Lzwdecode::tiff_standard()
            .decode_with_limit(&encoded, source.len())
            .unwrap(),
        source
    );
}

#[test]
fn libtiff_compat_lzw_is_lsb_late_change() {
    let source = patterned_bytes();
    let encoded = encode_tiff_libtiff_compat(&source).unwrap();
    assert_eq!(encoded.first(), Some(&0));
    assert!(encoded.get(1).is_some_and(|byte| byte & 1 != 0));
    assert_eq!(
        Lzwdecode::tiff_libtiff_compat()
            .decode_with_limit(&encoded, source.len())
            .unwrap(),
        source
    );
}

#[test]
fn wml2_lsb_lzw_remains_explicit_and_distinct() {
    let source = patterned_bytes();
    let encoded = encode_tiff_wml2_lsb(&source).unwrap();
    assert_eq!(
        Lzwdecode::tiff_wml2_lsb()
            .decode_with_limit(&encoded, source.len())
            .unwrap(),
        source
    );
    assert!(
        Lzwdecode::tiff_standard()
            .decode_with_limit(&encoded, source.len())
            .is_err()
    );
}
