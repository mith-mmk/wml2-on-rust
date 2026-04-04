mod common;

use std::panic;

use common::{bundled_test_image_path, sample_config_hint, sample_path};
use wml2::draw::image_from_file;

#[test]
fn bundled_error_regressions_do_not_panic() {
    let path = bundled_test_image_path("WML2Viewer_error.webp");
    assert!(
        path.is_file(),
        "missing bundled regression image: {}",
        path.display()
    );

    let result = panic::catch_unwind(|| image_from_file(path.to_string_lossy().into_owned()));
    assert!(result.is_ok(), "WML2Viewer_error.webp caused a panic");
    assert!(
        result.unwrap().is_ok(),
        "WML2Viewer_error.webp failed to decode"
    );
}

#[test]
fn external_error_samples_do_not_panic_when_available() {
    let mut found = 0usize;
    for name in ["CCITT_8.tiff", "earthlab.tif"] {
        let Some(path) = sample_path(name) else {
            continue;
        };

        found += 1;
        let result = panic::catch_unwind(|| image_from_file(path.to_string_lossy().into_owned()));
        assert!(result.is_ok(), "{name} caused a panic");
    }

    if found == 0 {
        eprintln!(
            "skipping external error samples (see {})",
            sample_config_hint().display()
        );
    }
}

#[test]
fn local_g3_regressions_decode_when_samples_are_available() {
    let mut found = 0usize;
    for name in ["G31D.tiff", "G32DS.tiff"] {
        let Some(path) = sample_path(name) else {
            continue;
        };

        found += 1;
        let path = path.to_string_lossy().into_owned();
        let result = panic::catch_unwind(|| image_from_file(path));
        assert!(result.is_ok(), "{name} caused a panic");
        assert!(result.unwrap().is_ok(), "{name} failed to decode");
    }

    if found == 0 {
        eprintln!(
            "skipping local G3 regression samples (configure {})",
            sample_config_hint().display()
        );
    }
}
