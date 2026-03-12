use std::panic;
use std::path::PathBuf;

use wml2::draw::image_from_file;

fn error_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test")
        .join("errors")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn critical_tiff_samples_do_not_panic() {
    for name in ["CCITT_8.tiff", "earthlab.tif"] {
        let path = error_path(name);
        let result = panic::catch_unwind(|| image_from_file(path));
        assert!(result.is_ok(), "{name} caused a panic");
    }
}
