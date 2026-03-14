use std::fs;
use std::path::{Path, PathBuf};

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "webp", "jpg", "jpeg", "bmp", "gif", "png", "tif", "tiff", "mag", "maki", "pi", "pic",
];

#[derive(Clone, Debug)]
pub struct FileNavigator {
    files: Vec<PathBuf>,
    current: usize,
}

impl FileNavigator {
    pub fn from_path(path: &Path) -> Self {
        let canonical_target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let files = match path.parent() {
            Some(parent) => collect_supported_files(parent),
            None => vec![canonical_target.clone()],
        };

        if files.is_empty() {
            return Self {
                files: vec![canonical_target],
                current: 0,
            };
        }

        let current = files
            .iter()
            .position(|candidate| candidate == &canonical_target)
            .unwrap_or(0);

        Self { files, current }
    }

    pub fn current(&self) -> &Path {
        &self.files[self.current]
    }

    pub fn next(&mut self) -> Option<PathBuf> {
        if self.current + 1 >= self.files.len() {
            return None;
        }
        self.current += 1;
        Some(self.files[self.current].clone())
    }

    pub fn prev(&mut self) -> Option<PathBuf> {
        if self.current == 0 {
            return None;
        }
        self.current -= 1;
        Some(self.files[self.current].clone())
    }

    pub fn first(&mut self) -> Option<PathBuf> {
        if self.files.is_empty() {
            return None;
        }
        self.current = 0;
        Some(self.files[self.current].clone())
    }

    pub fn last(&mut self) -> Option<PathBuf> {
        if self.files.is_empty() {
            return None;
        }
        self.current = self.files.len() - 1;
        Some(self.files[self.current].clone())
    }
}

fn collect_supported_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_supported_image(path))
        .map(|path| path.canonicalize().unwrap_or(path))
        .collect();

    files.sort_by_cached_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    files
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|supported| *supported == ext)
        })
        .unwrap_or(false)
}
