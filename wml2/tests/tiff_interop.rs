#![cfg(feature = "tiff")]

const MANIFEST: &str = include_str!("fixtures/tiff_interop/manifest.json");

fn external_sample(name: &str) -> Vec<u8> {
    let root = std::env::var_os("WML2_TIFF_CORPUS")
        .unwrap_or_else(|| panic!("set WML2_TIFF_CORPUS to run external sample {name}"));
    std::fs::read(std::path::Path::new(&root).join(name))
        .unwrap_or_else(|error| panic!("cannot read external TIFF sample {name}: {error}"))
}

#[test]
fn external_tiff_manifest_is_pinned_and_complete() {
    assert!(MANIFEST.contains("8e10d4d765667c1c49d74413878fc4bfb46dcf8d"));
    for sample in [
        "fax4.tiff",
        "dscf0013.tif",
        "ycbcr-cat.tif",
        "ojpeg_chewey_subsamp21_multi_strip.tiff",
        "ojpeg_single_strip_no_rowsperstrip.tiff",
        "ojpeg_zackthecat_subsamp22_single_strip.tiff",
    ] {
        assert!(MANIFEST.contains(sample), "manifest is missing {sample}");
    }
    assert!(MANIFEST.contains("distribution\": \"external-only\""));
}

#[test]
#[ignore = "requires the external TIFF corpus; run with --ignored and WML2_TIFF_CORPUS"]
fn external_group4_reserved_extension_is_reported_when_configured() {
    let bytes = external_sample("fax4.tiff");
    let error = match wml2::draw::image_load(&bytes) {
        Ok(_) => panic!("reserved Group 4 extensions must be reported"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("unsupported CCITT Group 4 extension")
    );
}

#[test]
#[ignore = "requires the external TIFF corpus; run with --ignored and WML2_TIFF_CORPUS"]
fn external_non_jpeg_ycbcr_samples_decode_when_configured() {
    for (name, dimensions) in [("dscf0013.tif", (640, 480)), ("ycbcr-cat.tif", (250, 325))] {
        let bytes = external_sample(name);
        let image = wml2::draw::image_load(&bytes)
            .unwrap_or_else(|error| panic!("{name} should decode as non-JPEG YCbCr: {error}"));
        assert_eq!((image.width, image.height), dimensions);
        assert_eq!(
            image.buffer.as_ref().map(Vec::len),
            Some(dimensions.0 * dimensions.1 * 4)
        );
    }
}

#[cfg(feature = "tiff-jpeg")]
#[test]
#[ignore = "requires the external TIFF corpus; run with --ignored and WML2_TIFF_CORPUS"]
fn external_old_style_jpeg_samples_decode_when_configured() {
    for (name, dimensions) in [
        ("ojpeg_chewey_subsamp21_multi_strip.tiff", (392, 575)),
        ("ojpeg_single_strip_no_rowsperstrip.tiff", (234, 213)),
        ("ojpeg_zackthecat_subsamp22_single_strip.tiff", (234, 213)),
    ] {
        let bytes = external_sample(name);
        let image = wml2::draw::image_load(&bytes)
            .unwrap_or_else(|error| panic!("{name} should decode as old-style JPEG: {error}"));
        assert_eq!((image.width, image.height), dimensions);
        assert_eq!(
            image.buffer.as_ref().map(Vec::len),
            Some(dimensions.0 * dimensions.1 * 4)
        );
    }
}
