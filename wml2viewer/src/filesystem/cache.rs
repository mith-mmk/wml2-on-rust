use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::options::NavigationSortOption;

use super::listed_file::load_listed_file_entries;
use super::path::{
    is_listed_file_name, is_listed_file_path, is_supported_image_name, is_zip_file_name,
    is_zip_file_path, listed_virtual_child_path, resolve_start_path, zip_virtual_child_path,
};
use super::sort_paths;
use super::zip_file::load_zip_entries;

pub(crate) struct FilesystemCache {
    listings_by_dir: HashMap<PathBuf, DirectoryListing>,
    sort: NavigationSortOption,
}

impl Default for FilesystemCache {
    fn default() -> Self {
        Self::new(NavigationSortOption::OsName)
    }
}

#[derive(Clone, Default)]
pub(crate) struct DirectoryListing {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
    first_file: Option<PathBuf>,
    last_file: Option<PathBuf>,
}

impl FilesystemCache {
    pub(crate) fn new(sort: NavigationSortOption) -> Self {
        Self {
            listings_by_dir: HashMap::new(),
            sort,
        }
    }

    pub(crate) fn listing(&mut self, dir: &Path) -> &DirectoryListing {
        if is_listed_file_path(dir) {
            let listing = scan_directory_listing(dir, self.sort);
            self.listings_by_dir.insert(dir.to_path_buf(), listing);
            return self
                .listings_by_dir
                .get(dir)
                .expect("listed file listing inserted");
        }
        let sort = self.sort;
        self.listings_by_dir
            .entry(dir.to_path_buf())
            .or_insert_with(|| scan_directory_listing(dir, sort))
    }

    pub(crate) fn supported_entries(&mut self, dir: &Path) -> Vec<PathBuf> {
        self.listing(dir).files.clone()
    }

    pub(crate) fn child_directories(&mut self, dir: &Path) -> Vec<PathBuf> {
        self.listing(dir).dirs.clone()
    }

    pub(crate) fn first_supported_file(&mut self, dir: &Path) -> Option<PathBuf> {
        self.listing(dir).first_file.clone()
    }

    pub(crate) fn last_supported_file(&mut self, dir: &Path) -> Option<PathBuf> {
        self.listing(dir).last_file.clone()
    }
}

#[allow(dead_code)]
pub fn list_openable_entries(dir: &Path, sort: NavigationSortOption) -> Vec<PathBuf> {
    let mut cache = FilesystemCache::new(sort);
    cache.supported_entries(dir)
}

pub fn list_browser_entries(dir: &Path, sort: NavigationSortOption) -> Vec<PathBuf> {
    if is_zip_file_path(dir) {
        return build_zip_virtual_children(dir);
    }

    if is_listed_file_path(dir) {
        return build_listed_virtual_children(dir);
    }

    let mut entries = Vec::new();
    let Ok(read_dir) = fs::read_dir(dir) else {
        return entries;
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in read_dir.filter_map(Result::ok) {
        let Some(path) = browser_entry_path_from_dir_entry(&entry) else {
            continue;
        };
        if dir_entry_is_browser_file(&entry, &path) {
            files.push(path.clone());
        }
        if dir_entry_is_browser_container(&entry, &path) {
            dirs.push(path);
        }
    }

    sort_paths(&mut dirs, sort);
    sort_paths(&mut files, sort);
    entries.extend(dirs);
    entries.extend(files);
    entries
}

pub fn is_browser_container(path: &Path) -> bool {
    path.is_dir() || is_zip_file_path(path) || is_listed_file_path(path)
}

pub(crate) fn scan_directory_listing(dir: &Path, sort: NavigationSortOption) -> DirectoryListing {
    if is_zip_file_path(dir) {
        return scan_zip_virtual_directory(dir);
    }

    if is_listed_file_path(dir) {
        return scan_listed_virtual_directory(dir);
    }

    scan_real_directory_listing(dir, sort)
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

    DirectoryListing {
        first_file: files.first().cloned(),
        last_file: files.last().cloned(),
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

    DirectoryListing {
        first_file: files.first().cloned(),
        last_file: files.last().cloned(),
        files,
        dirs: Vec::new(),
    }
}

fn scan_real_directory_listing(dir: &Path, sort: NavigationSortOption) -> DirectoryListing {
    let Some(entries) = fs::read_dir(dir).ok() else {
        return DirectoryListing::default();
    };

    let mut raw_files = Vec::new();
    let mut raw_dirs = Vec::new();

    for entry in entries.filter_map(Result::ok) {
        let Some(path) = browser_entry_path_from_dir_entry(&entry) else {
            continue;
        };
        if dir_entry_is_browser_file(&entry, &path) {
            raw_files.push(path.clone());
        }
        if dir_entry_is_browser_container(&entry, &path) {
            raw_dirs.push(path);
        }
    }

    sort_paths(&mut raw_files, sort);
    sort_paths(&mut raw_dirs, sort);

    let mut files = Vec::new();
    for path in raw_files {
        if is_listed_file_path(&path) {
            files.extend(build_listed_virtual_children(&path));
        } else if is_zip_file_path(&path) {
            files.extend(build_zip_virtual_children(&path));
        } else {
            files.push(path);
        }
    }

    DirectoryListing {
        first_file: files.first().cloned(),
        last_file: files.last().cloned(),
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

fn dir_entry_is_browser_file(entry: &fs::DirEntry, path: &Path) -> bool {
    let file_name = entry.file_name();
    is_supported_image_name(&file_name) || is_listed_file_path(path) || is_zip_file_path(path)
}

fn dir_entry_is_browser_container(entry: &fs::DirEntry, path: &Path) -> bool {
    is_listed_file_path(path) || is_zip_file_path(path) || dir_entry_is_directory(entry)
}
