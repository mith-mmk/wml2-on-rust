use crate::dependent::plugins::PluginProviderConfig;
use std::path::PathBuf;

pub(super) fn default_provider() -> PluginProviderConfig {
    let search_path = if cfg!(target_os = "windows") {
        vec![PathBuf::from("./")]
    } else {
        Vec::new()
    };

    PluginProviderConfig {
        enable: false,
        search_path,
        modules: Vec::new(),
    }
}
