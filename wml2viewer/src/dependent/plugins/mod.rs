mod ffmpeg;
mod susie64;
mod system;

use serde::{Deserialize, Serialize};
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
