use std::fs;
use std::panic;
use std::path::PathBuf;

use wml2::draw::image_from_file;

fn error_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test")
        .join("errors")
}

fn local_g3_path(name: &str) -> PathBuf {
    PathBuf::from(r"D:\data\samples\images\tiff\G3").join(name)
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
        let path = local_g3_path(name);
        if !path.is_file() {
            continue;
        }

        found += 1;
        let path = path.to_string_lossy().into_owned();
        let result = panic::catch_unwind(|| image_from_file(path));
        assert!(result.is_ok(), "{name} caused a panic");
        assert!(result.unwrap().is_ok(), "{name} failed to decode");
    }

    if found == 0 {
        eprintln!("skipping local G3 regression samples");
    }
}
