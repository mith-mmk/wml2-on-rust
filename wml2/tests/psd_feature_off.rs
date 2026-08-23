#[cfg(not(feature = "psd"))]
#[test]
fn psd_is_not_advertised_without_feature() {
    let mut header = Vec::new();
    header.extend_from_slice(b"8BPS");
    header.extend_from_slice(&1u16.to_be_bytes());
    header.extend_from_slice(&[0; 6]);
    header.extend_from_slice(&3u16.to_be_bytes());
    header.extend_from_slice(&1u32.to_be_bytes());
    header.extend_from_slice(&1u32.to_be_bytes());
    header.extend_from_slice(&8u16.to_be_bytes());
    header.extend_from_slice(&3u16.to_be_bytes());

    assert!(!wml2::get_can_decode(&header).unwrap());
    assert!(
        !wml2::get_decoder_extentions()
            .iter()
            .any(|ext| ext == "psd")
    );
    assert!(wml2::draw::image_load(&header).is_err());
}
