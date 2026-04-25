mod common;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use wml2::draw::image_from_file;

const ERROR_SAMPLES_DIR_ENV: &str = "WML2_ERROR_SAMPLES_DIR";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn parse_dotenv_var(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (left, right) = line.split_once('=')?;
        if left.trim() != key {
            continue;
        }
        return Some(right.trim().trim_matches('"').trim_matches('\'').to_string());
    }
    None
}

fn resolve_error_samples_root() -> PathBuf {
    let repo = repo_root();
    let configured = env::var(ERROR_SAMPLES_DIR_ENV)
        .ok()
        .or_else(|| parse_dotenv_var(&repo.join(".env"), ERROR_SAMPLES_DIR_ENV));

    match configured {
        Some(path) => {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                repo.join(path)
            }
        }
        None => repo.join(".test").join("errors"),
    }
}

fn collect_files_recursive(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn decode_error_sample(path: &Path) {
    let image = image_from_file(path.to_string_lossy().into_owned()).unwrap_or_else(|err| {
        panic!("failed to decode {}: {}", path.display(), err);
    });
    assert!(image.width > 0);
    assert!(image.height > 0);
}

#[test]
fn decode_error_samples_from_configured_directory() {
    let root = resolve_error_samples_root();
    if !root.is_dir() {
        eprintln!(
            "skipping error samples: directory does not exist: {} (set {} in env or .env)",
            root.display(),
            ERROR_SAMPLES_DIR_ENV
        );
        eprintln!("hint: {}", common::sample_config_hint().display());
        return;
    }

    let files = collect_files_recursive(&root);
    assert!(
        !files.is_empty(),
        "error sample directory is empty: {} (set {} in env or .env)",
        root.display(),
        ERROR_SAMPLES_DIR_ENV
    );

    for path in files {
        print!("Testing error sample: {} ...\n", path.display());
        decode_error_sample(&path);
    }
}
