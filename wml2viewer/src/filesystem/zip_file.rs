use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use encoding_rs::SHIFT_JIS;
use zip::ZipArchive;

use super::{compare_natural_str, is_supported_image};

#[derive(Clone, Debug)]
pub(crate) struct ZipEntryRecord {
    pub index: usize,
    pub name: String,
    pub size: u64,
}

pub(crate) fn load_zip_entries(path: &Path) -> Option<Vec<ZipEntryRecord>> {
    static ZIP_INDEX_CACHE: OnceLock<
        Mutex<std::collections::HashMap<PathBuf, Vec<ZipEntryRecord>>>,
    > = OnceLock::new();
    let cache = ZIP_INDEX_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(entries) = cache.lock().ok()?.get(path).cloned() {
        return Some(entries);
    }

    let mut archive = open_zip_archive(path).ok()?;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            continue;
        };
        if file.is_dir() {
            continue;
        }

        let name = decode_zip_name(&file);
        let entry_path = PathBuf::from(name.replace('\\', "/"));
        if !is_supported_image(&entry_path) {
            continue;
        }

        entries.push(ZipEntryRecord {
            index,
            name: name.replace('\\', "/"),
            size: file.size(),
        });
    }

    entries.sort_by(|left, right| compare_natural_str(&left.name, &right.name, false));
    if let Ok(mut cache) = cache.lock() {
        cache.insert(path.to_path_buf(), entries.clone());
    }
    Some(entries)
}

pub(crate) fn load_zip_entry_bytes(path: &Path, entry_index: usize) -> Option<Vec<u8>> {
    let fallback_name = load_zip_entries(path)?
        .into_iter()
        .find(|entry| entry.index == entry_index)
        .map(|entry| entry.name);
    let mut archive = open_zip_archive(path).ok()?;
    let mut entry = if let Ok(entry) = archive.by_index(entry_index) {
        entry
    } else {
        archive.by_name(fallback_name.as_deref()?).ok()?
    };
    let mut buf = Vec::new();
    let expected_size = entry.size().min(512 * 1024 * 1024) as usize;
    if expected_size > 0 {
        buf.reserve(expected_size);
    }
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}

pub(crate) fn zip_entry_record(path: &Path, entry_index: usize) -> Option<ZipEntryRecord> {
    load_zip_entries(path)?
        .into_iter()
        .find(|entry| entry.index == entry_index)
}

fn open_zip_archive(path: &Path) -> std::io::Result<ZipArchive<BufReader<File>>> {
    let file = File::open(path)?;
    let reader = BufReader::with_capacity(1024 * 256, file);
    ZipArchive::new(reader).map_err(std::io::Error::other)
}

fn decode_zip_name(file: &zip::read::ZipFile<'_>) -> String {
    let raw = file.name_raw();
    if let Ok(utf8) = std::str::from_utf8(raw) {
        return utf8.to_string();
    }
    let (decoded, _, had_errors) = SHIFT_JIS.decode(raw);
    if !had_errors {
        return decoded.into_owned();
    }
    String::from_utf8_lossy(raw).into_owned()
}
