#![cfg(feature = "tiff")]

const MANIFEST: &str = include_str!("fixtures/tiff_interop/manifest.json");

fn external_sample(name: &str) -> Option<Vec<u8>> {
    let root = std::env::var_os("WML2_TIFF_CORPUS")?;
    std::fs::read(std::path::Path::new(&root).join(name)).ok()
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
fn external_group4_reserved_extension_is_reported_when_configured() {
    let Some(bytes) = external_sample("fax4.tiff") else {
        eprintln!(
            "skipping external TIFF sample; set WML2_TIFF_CORPUS to the corpus valid directory"
        );
        return;
    };
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
fn external_non_jpeg_ycbcr_samples_decode_when_configured() {
    for (name, dimensions) in [("dscf0013.tif", (640, 480)), ("ycbcr-cat.tif", (250, 325))] {
        let Some(bytes) = external_sample(name) else {
            eprintln!(
                "skipping external TIFF samples; set WML2_TIFF_CORPUS to the corpus valid directory"
            );
            return;
        };
        let image = wml2::draw::image_load(&bytes)
            .unwrap_or_else(|error| panic!("{name} should decode as non-JPEG YCbCr: {error}"));
        assert_eq!((image.width, image.height), dimensions);
        assert_eq!(
            image.buffer.as_ref().map(Vec::len),
            Some(dimensions.0 * dimensions.1 * 4)
        );
    }
}
