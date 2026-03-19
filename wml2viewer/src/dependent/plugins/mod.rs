mod ffmpeg;
mod susie64;
mod system;

use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    pub susie64: PluginProviderConfig,
    pub system: PluginProviderConfig,
    pub ffmpeg: PluginProviderConfig,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            susie64: susie64::default_provider(),
            system: system::default_provider(),
            ffmpeg: ffmpeg::default_provider(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginProviderConfig {
    pub enable: bool,
    pub search_path: Vec<PathBuf>,
    pub modules: Vec<PluginModuleConfig>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginModuleConfig {
    pub enable: bool,
    pub path: Option<PathBuf>,
    pub plugin_name: String,
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub ext: Vec<PluginExtensionConfig>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginExtensionConfig {
    pub enable: bool,
    pub mime: Vec<String>,
    pub modules: Vec<PluginCapabilityConfig>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginCapabilityConfig {
    #[serde(rename = "type")]
    pub capability_type: String,
    pub priority: String,
}

#[allow(dead_code)]
pub fn discover_plugin_paths(config: &PluginProviderConfig) -> Vec<PathBuf> {
    config
        .search_path
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect()
}

pub fn discover_plugin_modules(
    provider_name: &str,
    config: &PluginProviderConfig,
) -> Vec<PluginModuleConfig> {
    let mut modules = Vec::new();
    for root in discover_plugin_paths(config) {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || !matches_provider(provider_name, &path) {
                continue;
            }
            modules.push(PluginModuleConfig {
                enable: true,
                path: Some(path.clone()),
                plugin_name: path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .unwrap_or("plugin")
                    .to_string(),
                plugin_type: provider_default_type(provider_name).to_string(),
                ext: Vec::new(),
            });
        }
    }
    modules.sort_by(|left, right| left.plugin_name.cmp(&right.plugin_name));
    modules
}

fn matches_provider(provider_name: &str, path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    match provider_name {
        "susie64" => matches!(ext.as_str(), "spi" | "sph" | "dll"),
        "ffmpeg" => {
            matches!(ext.as_str(), "dll" | "so" | "dylib")
                && (name.contains("ffmpeg")
                    || name.contains("avcodec")
                    || name.contains("avformat")
                    || name.contains("avutil"))
        }
        "system" => false,
        _ => false,
    }
}

fn provider_default_type(provider_name: &str) -> &'static str {
    match provider_name {
        "susie64" => "image",
        "ffmpeg" => "image",
        "system" => "image",
        _ => "image",
    }
}
