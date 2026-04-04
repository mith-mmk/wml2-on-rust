#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sample_override_config_path() -> PathBuf {
    package_root().join("tests").join("test_samples.txt")
}

pub fn test_image_root() -> PathBuf {
    repo_root().join("test").join("images")
}

pub fn bundled_test_image_path(name: &str) -> PathBuf {
    test_image_root().join("bundled").join(name)
}

fn legacy_sample_path(name: &str) -> PathBuf {
    repo_root().join("test").join("samples").join(name)
}

fn tracked_sample_paths(name: &str) -> [PathBuf; 4] {
    [
        bundled_test_image_path(name),
        legacy_sample_path(name),
        repo_root().join("_test").join(name),
        repo_root().join("_test").join("animation_webp").join(name),
    ]
}

fn search_dir_recursive(root: &Path, name: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() {
            let matches = path
                .file_name()
                .and_then(|file_name| file_name.to_str())
                .is_some_and(|file_name| file_name.eq_ignore_ascii_case(name));
            if matches {
                return Some(path);
            }
        } else if path.is_dir() {
            if let Some(found) = search_dir_recursive(&path, name) {
                return Some(found);
            }
        }
    }
    None
}

fn resolve_config_path(config_dir: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        config_dir.join(path)
    }
}

fn parse_sample_config() -> HashMap<String, PathBuf> {
    let path = sample_override_config_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return HashMap::new();
    };

    let config_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut map = HashMap::new();
    for (line_no, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            eprintln!(
                "ignoring malformed sample config line {} in {}",
                line_no + 1,
                path.display()
            );
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }

        map.insert(key.to_string(), resolve_config_path(config_dir, value));
    }
    map
}

fn sample_config() -> &'static HashMap<String, PathBuf> {
    static CONFIG: OnceLock<HashMap<String, PathBuf>> = OnceLock::new();
    CONFIG.get_or_init(parse_sample_config)
}

pub fn sample_path(name: &str) -> Option<PathBuf> {
    for tracked in tracked_sample_paths(name) {
        if tracked.is_file() {
            return Some(tracked);
        }
    }

    if let Some(configured) = sample_config().get(name).cloned() {
        if configured.is_file() {
            return Some(configured);
        }
    }

    search_dir_recursive(&test_image_root().join("external"), name)
}

pub fn sample_bytes(name: &str) -> Option<Vec<u8>> {
    let path = sample_path(name)?;
    fs::read(path).ok()
}

pub fn sample_config_hint() -> PathBuf {
    test_image_root().join("README.md")
}
