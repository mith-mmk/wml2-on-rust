use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::dependent::default_temp_dir;
use crate::options::{ArchiveBrowseOption, NavigationSortOption};
use serde::{Deserialize, Serialize};

use super::browser::BrowserMetadata;
use super::listed_file::load_listed_file_entries;
use super::path::{
    is_listed_file_name, is_listed_file_path, is_supported_image_name, is_zip_file_name,
    is_zip_file_path, listed_virtual_child_path, resolve_start_path, zip_virtual_child_path,
};
use super::sort_paths;
use super::zip_file::load_zip_entries;

pub(crate) struct FilesystemCache {
    listings_by_dir: HashMap<PathBuf, DirectoryListing>,
    metadata_by_path: HashMap<PathBuf, BrowserMetadata>,
    sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
}

pub(crate) type SharedFilesystemCache = Arc<Mutex<FilesystemCache>>;

impl Default for FilesystemCache {
    fn default() -> Self {
        Self::new(NavigationSortOption::OsName, ArchiveBrowseOption::Folder)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct DirectoryListing {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
    browser_entries: Vec<PathBuf>,
    first_file: Option<PathBuf>,
    last_file: Option<PathBuf>,
}

impl FilesystemCache {
    pub(crate) fn new(sort: NavigationSortOption, archive_mode: ArchiveBrowseOption) -> Self {
        let mut cache = load_persistent_cache().unwrap_or(Self {
            listings_by_dir: HashMap::new(),
            metadata_by_path: HashMap::new(),
            sort,
            archive_mode,
        });
        cache.ensure_settings(sort, archive_mode);
        cache
    }

    pub(crate) fn listing(&mut self, dir: &Path) -> &DirectoryListing {
        if is_listed_file_path(dir) {
            let listing = scan_directory_listing(dir, self.sort, self.archive_mode);
            self.listings_by_dir.insert(dir.to_path_buf(), listing);
            persist_cache(self);
            return self
                .listings_by_dir
                .get(dir)
                .expect("listed file listing inserted");
        }
        let sort = self.sort;
        let archive_mode = self.archive_mode;
        if !self.listings_by_dir.contains_key(dir) {
            let listing = scan_directory_listing(dir, sort, archive_mode);
            self.listings_by_dir.insert(dir.to_path_buf(), listing);
            persist_cache(self);
        }
        self.listings_by_dir
            .get(dir)
            .expect("directory listing inserted")
    }

    pub(crate) fn ensure_settings(
        &mut self,
        sort: NavigationSortOption,
        archive_mode: ArchiveBrowseOption,
    ) {
        if self.sort != sort || self.archive_mode != archive_mode {
            self.sort = sort;
            self.archive_mode = archive_mode;
            self.listings_by_dir.clear();
        }
    }

    pub(crate) fn browser_metadata_batch(
        &mut self,
        paths: &[PathBuf],
    ) -> HashMap<PathBuf, BrowserMetadata> {
        let mut changed = false;
        let mut result = HashMap::with_capacity(paths.len());
        for path in paths {
            let metadata = match self.metadata_by_path.get(path) {
                Some(metadata) => metadata.clone(),
                None => {
                    let metadata = load_browser_metadata(path);
                    self.metadata_by_path.insert(path.clone(), metadata.clone());
                    changed = true;
                    metadata
                }
            };
            result.insert(path.clone(), metadata);
        }
        if changed {
            persist_cache(self);
        }
        result
    }

    pub(crate) fn supported_entries(&mut self, dir: &Path) -> Vec<PathBuf> {
        self.listing(dir).files.clone()
    }

    pub(crate) fn child_directories(&mut self, dir: &Path) -> Vec<PathBuf> {
        self.listing(dir).dirs.clone()
    }

    pub(crate) fn browser_entries(&mut self, dir: &Path) -> Vec<PathBuf> {
        self.listing(dir).browser_entries.clone()
    }

    pub(crate) fn first_supported_file(&mut self, dir: &Path) -> Option<PathBuf> {
        self.listing(dir).first_file.clone()
    }

    pub(crate) fn last_supported_file(&mut self, dir: &Path) -> Option<PathBuf> {
        self.listing(dir).last_file.clone()
    }
}

pub(crate) fn new_shared_filesystem_cache(
    sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
) -> SharedFilesystemCache {
    Arc::new(Mutex::new(FilesystemCache::new(sort, archive_mode)))
}

#[allow(dead_code)]
pub fn list_openable_entries(dir: &Path, sort: NavigationSortOption) -> Vec<PathBuf> {
    let mut cache = FilesystemCache::new(sort, ArchiveBrowseOption::Folder);
    cache.supported_entries(dir)
}

pub fn list_browser_entries(dir: &Path, sort: NavigationSortOption) -> Vec<PathBuf> {
    let mut cache = FilesystemCache::new(sort, ArchiveBrowseOption::Folder);
    cache.browser_entries(dir)
}

pub fn is_browser_container(path: &Path) -> bool {
    path.is_dir() || is_zip_file_path(path) || is_listed_file_path(path)
}

pub(crate) fn scan_directory_listing(
    dir: &Path,
    sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
) -> DirectoryListing {
    if archive_mode == ArchiveBrowseOption::Folder && is_zip_file_path(dir) {
        return scan_zip_virtual_directory(dir);
    }

    if archive_mode == ArchiveBrowseOption::Folder && is_listed_file_path(dir) {
        return scan_listed_virtual_directory(dir);
    }

    scan_real_directory_listing(dir, sort, archive_mode)
}

pub(crate) fn browser_entry_path_from_dir_entry(entry: &fs::DirEntry) -> Option<PathBuf> {
    let file_name = entry.file_name();
    let path = entry.path();
    if is_supported_image_name(&file_name)
        || is_listed_file_name(&file_name)
        || is_zip_file_name(&file_name)
    {
        return Some(path);
    }

    dir_entry_is_directory(entry).then_some(path)
}

pub(crate) fn build_listed_virtual_children(listed_file: &Path) -> Vec<PathBuf> {
    load_listed_file_entries(listed_file)
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter_map(|(index, entry_path)| {
            resolve_start_path(&entry_path)
                .map(|_| listed_virtual_child_path(listed_file, index, &entry_path))
        })
        .collect()
}

pub(crate) fn build_zip_virtual_children(zip_file: &Path) -> Vec<PathBuf> {
    load_zip_entries(zip_file)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| zip_virtual_child_path(zip_file, entry.index, &entry.name))
        .collect()
}

fn scan_listed_virtual_directory(listed_file: &Path) -> DirectoryListing {
    let files = build_listed_virtual_children(listed_file);
    let browser_entries = files.clone();

    DirectoryListing {
        first_file: files.first().cloned(),
        last_file: files.last().cloned(),
        browser_entries,
        files,
        dirs: Vec::new(),
    }
}

fn scan_zip_virtual_directory(zip_file: &Path) -> DirectoryListing {
    let entries = load_zip_entries(zip_file).unwrap_or_default();
    let files = entries
        .iter()
        .map(|entry| zip_virtual_child_path(zip_file, entry.index, &entry.name))
        .collect::<Vec<_>>();
    let browser_entries = files.clone();

    DirectoryListing {
        first_file: files.first().cloned(),
        last_file: files.last().cloned(),
        browser_entries,
        files,
        dirs: Vec::new(),
    }
}

fn scan_real_directory_listing(
    dir: &Path,
    sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
) -> DirectoryListing {
    let Some(entries) = fs::read_dir(dir).ok() else {
        return DirectoryListing::default();
    };

    let mut raw_files = Vec::new();
    let mut raw_dirs = Vec::new();

    for entry in entries.filter_map(Result::ok) {
        let Some(path) = browser_entry_path_from_dir_entry(&entry) else {
            continue;
        };
        if dir_entry_is_browser_file(&entry, &path, archive_mode) {
            raw_files.push(path.clone());
        }
        if dir_entry_is_browser_container(&entry, &path, archive_mode) {
            raw_dirs.push(path);
        }
    }

    sort_paths(&mut raw_files, sort);
    sort_paths(&mut raw_dirs, sort);
    let mut browser_entries = raw_dirs.clone();
    browser_entries.extend(raw_files.clone());

    let mut files = Vec::new();
    for path in raw_files {
        match archive_mode {
            ArchiveBrowseOption::Folder => {
                if is_listed_file_path(&path) {
                    files.extend(build_listed_virtual_children(&path));
                } else if is_zip_file_path(&path) {
                    files.extend(build_zip_virtual_children(&path));
                } else {
                    files.push(path);
                }
            }
            ArchiveBrowseOption::Skip => {
                if !is_listed_file_path(&path) && !is_zip_file_path(&path) {
                    files.push(path);
                }
            }
            ArchiveBrowseOption::Archiver => {
                files.push(path);
            }
        }
    }

    DirectoryListing {
        first_file: files.first().cloned(),
        last_file: files.last().cloned(),
        browser_entries,
        files,
        dirs: raw_dirs,
    }
}

fn dir_entry_is_directory(entry: &fs::DirEntry) -> bool {
    entry
        .file_type()
        .map(|file_type| file_type.is_dir())
        .or_else(|_| entry.metadata().map(|metadata| metadata.is_dir()))
        .unwrap_or(false)
}

fn dir_entry_is_browser_file(
    entry: &fs::DirEntry,
    path: &Path,
    archive_mode: ArchiveBrowseOption,
) -> bool {
    let file_name = entry.file_name();
    is_supported_image_name(&file_name)
        || match archive_mode {
            ArchiveBrowseOption::Folder | ArchiveBrowseOption::Archiver => {
                is_listed_file_path(path) || is_zip_file_path(path)
            }
            ArchiveBrowseOption::Skip => false,
        }
}

fn dir_entry_is_browser_container(
    entry: &fs::DirEntry,
    path: &Path,
    archive_mode: ArchiveBrowseOption,
) -> bool {
    match archive_mode {
        ArchiveBrowseOption::Folder => {
            is_listed_file_path(path) || is_zip_file_path(path) || dir_entry_is_directory(entry)
        }
        ArchiveBrowseOption::Skip | ArchiveBrowseOption::Archiver => dir_entry_is_directory(entry),
    }
}

fn load_browser_metadata(path: &Path) -> BrowserMetadata {
    fs::metadata(path)
        .ok()
        .map(|metadata| BrowserMetadata {
            size: metadata.is_file().then_some(metadata.len()),
            modified: metadata.modified().ok(),
        })
        .unwrap_or_default()
}

#[derive(Serialize, Deserialize)]
struct PersistentFilesystemCache {
    sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
    listings_by_dir: HashMap<PathBuf, DirectoryListing>,
    metadata_by_path: HashMap<PathBuf, BrowserMetadata>,
}

fn persistent_cache_path() -> Option<PathBuf> {
    Some(default_temp_dir()?.join("filesystem-cache.json"))
}

fn load_persistent_cache() -> Option<FilesystemCache> {
    let text = fs::read_to_string(persistent_cache_path()?).ok()?;
    let snapshot = serde_json::from_str::<PersistentFilesystemCache>(&text).ok()?;
    Some(FilesystemCache {
        listings_by_dir: snapshot.listings_by_dir,
        metadata_by_path: snapshot.metadata_by_path,
        sort: snapshot.sort,
        archive_mode: snapshot.archive_mode,
    })
}

fn persist_cache(cache: &FilesystemCache) {
    let Some(path) = persistent_cache_path() else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let snapshot = PersistentFilesystemCache {
        sort: cache.sort,
        archive_mode: cache.archive_mode,
        listings_by_dir: cache.listings_by_dir.clone(),
        metadata_by_path: cache.metadata_by_path.clone(),
    };
    let Ok(text) = serde_json::to_string(&snapshot) else {
        return;
    };
    let _ = fs::write(path, text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wml2viewer_cache_{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn browser_metadata_batch_reuses_cached_values() {
        let dir = make_temp_dir();
        let file = dir.join("page.png");
        fs::write(&file, [1u8]).unwrap();

        let mut cache = FilesystemCache::default();
        let first = cache.browser_metadata_batch(std::slice::from_ref(&file));
        fs::write(&file, [1u8, 2, 3, 4]).unwrap();
        let second = cache.browser_metadata_batch(std::slice::from_ref(&file));

        assert_eq!(first.get(&file).and_then(|meta| meta.size), Some(1));
        assert_eq!(second.get(&file).and_then(|meta| meta.size), Some(1));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scan_real_directory_listing_respects_archive_mode() {
        let dir = make_temp_dir();
        let image = dir.join("001.png");
        let archive = dir.join("images.zip");
        fs::write(&image, []).unwrap();
        fs::write(&archive, []).unwrap();

        let folder_listing = scan_real_directory_listing(
            &dir,
            NavigationSortOption::OsName,
            ArchiveBrowseOption::Folder,
        );
        let skip_listing = scan_real_directory_listing(
            &dir,
            NavigationSortOption::OsName,
            ArchiveBrowseOption::Skip,
        );
        let archiver_listing = scan_real_directory_listing(
            &dir,
            NavigationSortOption::OsName,
            ArchiveBrowseOption::Archiver,
        );

        assert!(folder_listing.browser_entries.contains(&archive));
        assert!(!skip_listing.browser_entries.contains(&archive));
        assert!(archiver_listing.browser_entries.contains(&archive));
        assert!(archiver_listing.files.contains(&archive));

        let _ = fs::remove_dir_all(dir);
    }
}
