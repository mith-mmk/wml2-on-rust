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

fn sample_config_path() -> PathBuf {
    package_root().join("tests").join("test_samples.txt")
}

pub fn tracked_sample_path(name: &str) -> PathBuf {
    repo_root().join("test").join("samples").join(name)
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
    let path = sample_config_path();
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
    let tracked = tracked_sample_path(name);
    if tracked.is_file() {
        return Some(tracked);
    }

    let configured = sample_config().get(name)?.clone();
    configured.is_file().then_some(configured)
}

pub fn sample_bytes(name: &str) -> Option<Vec<u8>> {
    let path = sample_path(name)?;
    fs::read(path).ok()
}

pub fn sample_config_hint() -> PathBuf {
    sample_config_path()
}
