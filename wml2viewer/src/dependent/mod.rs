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
