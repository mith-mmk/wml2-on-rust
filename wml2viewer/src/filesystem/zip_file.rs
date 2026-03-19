use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use crate::dependent::default_temp_dir;
use crate::options::ZipWorkaroundOptions;
use encoding_rs::SHIFT_JIS;
use zip::ZipArchive;

use super::{compare_natural_str, is_supported_image};

#[derive(Clone, Debug)]
pub(crate) struct ZipEntryRecord {
    pub index: usize,
    pub name: String,
    pub size: u64,
}

#[derive(Clone)]
enum ZipArchiveAccess {
    Direct(PathBuf),
    Sequential(PathBuf),
}

pub(crate) fn load_zip_entries(path: &Path) -> Option<Vec<ZipEntryRecord>> {
    let cache = zip_index_cache();
    if let Some(entries) = cache.lock().ok()?.get(path).cloned() {
        return Some(entries);
    }

    let access = resolve_zip_archive_access(path)?;
    let mut archive = open_zip_archive(access.path()).ok()?;
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

    if !matches!(access, ZipArchiveAccess::Sequential(_)) {
        entries.sort_by(|left, right| compare_natural_str(&left.name, &right.name, false));
    }
    if let Ok(mut cache) = cache.lock() {
        cache.insert(path.to_path_buf(), entries.clone());
    }
    Some(entries)
}

pub(crate) fn load_zip_entry_bytes(path: &Path, entry_index: usize) -> Option<Vec<u8>> {
    let access = resolve_zip_archive_access(path)?;
    let archive_path = access.path();
    let mut archive = open_zip_archive(archive_path).ok()?;
    if let Ok(mut entry) = archive.by_index(entry_index) {
        let mut buf = Vec::new();
        let expected_size = entry.size().min(512 * 1024 * 1024) as usize;
        if expected_size > 0 {
            buf.reserve(expected_size);
        }
        entry.read_to_end(&mut buf).ok()?;
        return Some(buf);
    }

    let fallback_name = load_zip_entries(path)?
        .into_iter()
        .find(|entry| entry.index == entry_index)
        .map(|entry| entry.name)?;
    let mut archive = open_zip_archive(archive_path).ok()?;
    let mut entry = archive.by_name(&fallback_name).ok()?;
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

pub(crate) fn set_zip_workaround_options(options: ZipWorkaroundOptions) {
    if let Ok(mut config) = zip_workaround_config().lock() {
        *config = options;
    }
    clear_zip_caches();
}

pub(crate) fn zip_prefers_low_io(path: &Path) -> bool {
    matches!(
        resolve_zip_archive_access(path),
        Some(ZipArchiveAccess::Sequential(_))
    )
}

fn open_zip_archive(path: &Path) -> std::io::Result<ZipArchive<BufReader<File>>> {
    let file = File::open(path)?;
    let reader = BufReader::with_capacity(1024 * 256, file);
    ZipArchive::new(reader).map_err(std::io::Error::other)
}

impl ZipArchiveAccess {
    fn path(&self) -> &Path {
        match self {
            Self::Direct(path) | Self::Sequential(path) => path.as_path(),
        }
    }
}

fn current_zip_workaround_options() -> ZipWorkaroundOptions {
    zip_workaround_config()
        .lock()
        .map(|config| config.clone())
        .unwrap_or_default()
}

fn resolve_zip_archive_access(path: &Path) -> Option<ZipArchiveAccess> {
    let metadata = std::fs::metadata(path).ok()?;
    let options = current_zip_workaround_options();
    let threshold_bytes = options.threshold_mb.saturating_mul(1024 * 1024);
    let needs_workaround = is_probably_network_path(path) || metadata.len() >= threshold_bytes;
    if !needs_workaround {
        return Some(ZipArchiveAccess::Direct(path.to_path_buf()));
    }

    if options.local_cache {
        if let Some(cached) = ensure_local_archive_cache(path, &metadata) {
            return Some(ZipArchiveAccess::Direct(cached));
        }
    }

    Some(ZipArchiveAccess::Sequential(path.to_path_buf()))
}

fn ensure_local_archive_cache(path: &Path, metadata: &std::fs::Metadata) -> Option<PathBuf> {
    let cache = local_archive_cache();
    if let Some(cached) = cache
        .lock()
        .ok()?
        .get(path)
        .cloned()
        .filter(|cached| cached.exists())
    {
        return Some(cached);
    }

    let temp_root = default_temp_dir()?.join("archive-cache");
    std::fs::create_dir_all(&temp_root).ok()?;

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
        .hash(&mut hasher);
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("zip");
    let destination = temp_root.join(format!("{:016x}.{ext}", hasher.finish()));
    if !destination.exists() {
        std::fs::copy(path, &destination).ok()?;
    }
    if let Ok(mut cache) = cache.lock() {
        cache.insert(path.to_path_buf(), destination.clone());
    }
    Some(destination)
}

fn clear_zip_caches() {
    if let Ok(mut cache) = zip_index_cache().lock() {
        cache.clear();
    }
    if let Ok(mut cache) = local_archive_cache().lock() {
        cache.clear();
    }
}

fn zip_index_cache() -> &'static Mutex<HashMap<PathBuf, Vec<ZipEntryRecord>>> {
    static ZIP_INDEX_CACHE: OnceLock<Mutex<HashMap<PathBuf, Vec<ZipEntryRecord>>>> =
        OnceLock::new();
    ZIP_INDEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn local_archive_cache() -> &'static Mutex<HashMap<PathBuf, PathBuf>> {
    static LOCAL_ARCHIVE_CACHE: OnceLock<Mutex<HashMap<PathBuf, PathBuf>>> = OnceLock::new();
    LOCAL_ARCHIVE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn zip_workaround_config() -> &'static Mutex<ZipWorkaroundOptions> {
    static CONFIG: OnceLock<Mutex<ZipWorkaroundOptions>> = OnceLock::new();
    CONFIG.get_or_init(|| Mutex::new(ZipWorkaroundOptions::default()))
}

fn is_probably_network_path(path: &Path) -> bool {
    let text = path.to_string_lossy();
    text.starts_with(r"\\") || text.starts_with(r"//")
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
