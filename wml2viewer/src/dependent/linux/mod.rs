use std::path::PathBuf;

pub fn default_config_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|base| base.join(".wml2"))
}
