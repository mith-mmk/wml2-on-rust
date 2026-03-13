mod common;

use std::fs;
use std::panic;
use std::path::PathBuf;

use common::{sample_config_hint, sample_path};
use wml2::draw::image_from_file;

fn error_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test")
        .join("errors")
}

#[test]
fn critical_tiff_samples_do_not_panic() {
    let mut paths: Vec<PathBuf> = fs::read_dir(error_dir())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "test/errors has no recovery samples");

    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let path = path.to_string_lossy().into_owned();
        let result = panic::catch_unwind(|| image_from_file(path));
        assert!(result.is_ok(), "{name} caused a panic");
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
