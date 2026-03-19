use crate::dependent::plugins::PluginProviderConfig;
use std::path::PathBuf;

pub(super) fn default_provider() -> PluginProviderConfig {
    PluginProviderConfig {
        enable: false,
        search_path: vec![PathBuf::from("./ffmpeg"), PathBuf::from("./")],
        modules: Vec::new(),
    }
}
