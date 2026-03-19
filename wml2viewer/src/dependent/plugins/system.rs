use crate::dependent::plugins::PluginProviderConfig;

pub(super) fn default_provider() -> PluginProviderConfig {
    PluginProviderConfig {
        enable: false,
        search_path: Vec::new(),
        modules: Vec::new(),
    }
}
