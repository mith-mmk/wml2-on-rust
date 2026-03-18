use std::path::PathBuf;

pub fn default_config_dir() -> Option<PathBuf> {
    std::env::current_dir().ok().map(|dir| dir.join(".wml2"))
}
