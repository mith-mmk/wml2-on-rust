use std::path::PathBuf;

pub fn default_config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|base| base.join("wml2"))
}

pub fn available_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for drive in b'A'..=b'Z' {
        let path = PathBuf::from(format!("{}:\\", drive as char));
        if path.exists() {
            roots.push(path);
        }
    }
    roots
}
