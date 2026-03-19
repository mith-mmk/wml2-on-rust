use directories::{BaseDirs, ProjectDirs};
use std::path::PathBuf;

#[cfg(any(target_os = "windows", target_os = "macos", unix))]
pub fn default_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("io.github", "mith-mmk", "wml2").map(|proj| proj.config_dir().to_path_buf())
}

#[cfg(any(target_os = "windows", target_os = "macos", unix))]
#[allow(dead_code)]
pub fn available_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/")];

    if let Some(base) = BaseDirs::new() {
        roots.push(base.home_dir().to_path_buf());
    }
    roots
}
