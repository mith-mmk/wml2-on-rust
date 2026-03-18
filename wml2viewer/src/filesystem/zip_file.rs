use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use zip::ZipArchive;

use super::is_supported_image;

#[derive(Clone, Debug)]
pub(crate) struct ZipEntryRecord {
    pub index: usize,
    pub name: String,
}

pub(crate) fn load_zip_entries(path: &Path) -> Option<Vec<ZipEntryRecord>> {
    static ZIP_INDEX_CACHE: OnceLock<Mutex<std::collections::HashMap<PathBuf, Vec<ZipEntryRecord>>>> =
        OnceLock::new();
    let cache = ZIP_INDEX_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(entries) = cache.lock().ok()?.get(path).cloned() {
        return Some(entries);
    }

    let file = File::open(path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let file = archive.by_index(index).ok()?;
        if file.is_dir() {
            continue;
        }

        let entry_path = PathBuf::from(file.name().replace('\\', "/"));
        if !is_supported_image(&entry_path) {
            continue;
        }

        entries.push(ZipEntryRecord {
            index,
            name: file.name().replace('\\', "/"),
        });
    }

    entries.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    if let Ok(mut cache) = cache.lock() {
        cache.insert(path.to_path_buf(), entries.clone());
    }
    Some(entries)
}

pub(crate) fn load_zip_entry_bytes(path: &Path, entry_index: usize) -> Option<Vec<u8>> {
    let file = File::open(path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    let mut entry = archive.by_index(entry_index).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}
