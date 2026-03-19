pub mod plugins;
mod thirdparty;
pub use thirdparty::{default_config_dir, normalize_locale_tag, resource_locale_fallbacks};

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "macos")]
mod darwin;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
)))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

//use eframe::egui::Direction;
#[cfg(target_os = "android")]
pub use android::*;
#[cfg(target_os = "macos")]
pub use darwin::*;
#[cfg(target_os = "ios")]
pub use ios::*;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
)))]
pub use other::*;
#[cfg(target_os = "windows")]
pub use windows::*;

pub fn ui_available_roots() -> Vec<std::path::PathBuf> {
    available_roots()
}

pub fn pick_save_directory() -> Option<std::path::PathBuf> {
    pick_directory_dialog()
}

pub fn download_http_url(url: &str) -> Option<std::path::PathBuf> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    download_url_to_temp(url)
}
