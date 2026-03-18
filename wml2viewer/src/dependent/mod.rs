use directories::{BaseDirs,ProjectDirs};
use std::path::PathBuf;

#[cfg(any(target_os = "windows", target_os = "macos",  unix))]
pub fn default_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("io.github", "mith-mmk", "wml2")
        .map(|proj| proj.config_dir().to_path_buf())
}

#[cfg(any(target_os = "windows", target_os = "macos",  unix))]
pub fn available_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/")];

    if let Some(base) = BaseDirs::new() {
        roots.push(base.home_dir().to_path_buf());
    }
    roots
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(unix)]
mod linux;
#[cfg(target_os = "macos")]
mod darwin;
#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
)))]
mod other;

//use eframe::egui::Direction;
#[cfg(target_os = "windows")]
//pub use windows::*;
#[cfg(unix)]
//pub use linux::*;
#[cfg(target_os = "macos")]
//pub use darwin::*;
#[cfg(target_os = "android")]
pub use android::*;
#[cfg(target_os = "ios")]
pub use ios::*;
#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
)))]
pub use other::*;
