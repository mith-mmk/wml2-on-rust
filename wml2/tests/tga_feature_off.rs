#[cfg(any(not(feature = "tga"), feature = "noretoro"))]
#[test]
fn tga_encoder_is_not_advertised_when_disabled() {
    assert!(!wml2::get_encode_extentions().iter().any(|ext| ext == "tga"));
}
