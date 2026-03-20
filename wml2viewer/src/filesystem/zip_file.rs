use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
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

fn open_zip_archive(path: &Path) -> std::io::Result<ZipArchive<ZipCacheReader>> {
    let file = File::open(path)?;
    let reader = ZipCacheReader::new(file)?;
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

struct ZipCacheReader {
    inner: File,
    pos: u64,
    len: u64,
    chunk_size: u64,
    max_chunks: usize,
    cache: HashMap<u64, Vec<u8>>,
    order: VecDeque<u64>,
}

impl ZipCacheReader {
    fn new(inner: File) -> std::io::Result<Self> {
        let len = inner.metadata()?.len();
        Ok(Self {
            inner,
            pos: 0,
            len,
            chunk_size: 1024 * 1024,
            max_chunks: 8,
            cache: HashMap::new(),
            order: VecDeque::new(),
        })
    }

    fn read_chunk(&mut self, chunk_index: u64) -> std::io::Result<&[u8]> {
        if !self.cache.contains_key(&chunk_index) {
            let offset = chunk_index.saturating_mul(self.chunk_size);
            self.inner.seek(SeekFrom::Start(offset))?;
            let remaining = self.len.saturating_sub(offset);
            let size = remaining.min(self.chunk_size) as usize;
            let mut buffer = vec![0u8; size];
            if size > 0 {
                self.inner.read_exact(&mut buffer)?;
            }
            self.cache.insert(chunk_index, buffer);
            self.order.push_back(chunk_index);
            while self.order.len() > self.max_chunks {
                if let Some(oldest) = self.order.pop_front() {
                    self.cache.remove(&oldest);
                }
            }
        }
        self.touch_chunk(chunk_index);
        Ok(self
            .cache
            .get(&chunk_index)
            .map(Vec::as_slice)
            .unwrap_or(&[]))
    }

    fn touch_chunk(&mut self, chunk_index: u64) {
        if let Some(index) = self.order.iter().position(|entry| *entry == chunk_index) {
            self.order.remove(index);
        }
        self.order.push_back(chunk_index);
    }
}

impl Read for ZipCacheReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() || self.pos >= self.len {
            return Ok(0);
        }

        let mut total = 0usize;
        while total < buf.len() && self.pos < self.len {
            let chunk_index = self.pos / self.chunk_size;
            let chunk_offset = (self.pos % self.chunk_size) as usize;
            let chunk = self.read_chunk(chunk_index)?;
            if chunk_offset >= chunk.len() {
                break;
            }
            let available = &chunk[chunk_offset..];
            let copy_len = available.len().min(buf.len() - total);
            buf[total..total + copy_len].copy_from_slice(&available[..copy_len]);
            total += copy_len;
            self.pos = self.pos.saturating_add(copy_len as u64);
        }
        Ok(total)
    }
}

impl Seek for ZipCacheReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let next = match pos {
            SeekFrom::Start(offset) => offset as i128,
            SeekFrom::End(offset) => self.len as i128 + offset as i128,
            SeekFrom::Current(offset) => self.pos as i128 + offset as i128,
        };
        self.pos = next.clamp(0, self.len as i128) as u64;
        Ok(self.pos)
    }
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

#[cfg(test)]
mod tests {
    use super::ZipCacheReader;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("wml2viewer-{name}-{unique}.bin"))
    }

    #[test]
    fn zip_cache_reader_supports_seek_and_read() {
        let path = temp_path("zip-cache");
        let mut file = File::create(&path).unwrap();
        for index in 0..(1024 * 32) {
            let value = (index % 251) as u8;
            file.write_all(&[value]).unwrap();
        }
        drop(file);

        let file = File::open(&path).unwrap();
        let mut reader = ZipCacheReader::new(file).unwrap();
        let mut buf = [0u8; 128];

        reader.seek(SeekFrom::Start(4093)).unwrap();
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], (4093 % 251) as u8);
        assert_eq!(buf[127], ((4093 + 127) % 251) as u8);

        reader.seek(SeekFrom::Start(32)).unwrap();
        reader.read_exact(&mut buf[..8]).unwrap();
        assert_eq!(&buf[..8], &[32, 33, 34, 35, 36, 37, 38, 39]);

        let _ = std::fs::remove_file(path);
    }
}
