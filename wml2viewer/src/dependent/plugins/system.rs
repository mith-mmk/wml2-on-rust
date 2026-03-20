use crate::dependent::plugins::{PluginModuleConfig, PluginProviderConfig};
use crate::drawers::image::LoadedImage;
use std::path::Path;

pub(super) fn default_provider() -> PluginProviderConfig {
    PluginProviderConfig {
        enable: false,
        search_path: Vec::new(),
        modules: Vec::new(),
    }
}

pub(super) fn decode_from_file(
    _path: &Path,
    _module: Option<&PluginModuleConfig>,
) -> Option<LoadedImage> {
    None
}

pub(super) fn decode_from_bytes(
    _data: &[u8],
    _path_hint: Option<&Path>,
    _module: Option<&PluginModuleConfig>,
) -> Option<LoadedImage> {
    None
}
