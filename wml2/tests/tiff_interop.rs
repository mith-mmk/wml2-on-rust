#![cfg(feature = "tiff")]

const MANIFEST: &str = include_str!("fixtures/tiff_interop/manifest.json");

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
