#[cfg(not(feature = "avif"))]
#[test]
fn avif_is_not_advertised_or_decodable_without_feature() {
    use std::path::PathBuf;

    use wml2::util::{ImageFormat, format_check};

    let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root should exist")
        .join("samples")
        .join("WML2Viewer.avif");
    let data = std::fs::read(sample_path).expect("sample AVIF should exist");

    assert!(matches!(format_check(&data), ImageFormat::Avif));
    assert!(!wml2::get_can_decode(&data).unwrap());
    assert!(
        !wml2::get_decoder_extentions()
            .iter()
            .any(|extension| extension == "avif")
    );
}
