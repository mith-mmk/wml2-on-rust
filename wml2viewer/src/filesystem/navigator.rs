use std::path::{Path, PathBuf};

use crate::options::{ArchiveBrowseOption, EndOfFolderOption, NavigationSortOption};

use super::cache::FilesystemCache;
use super::path::{
    is_listed_file_path, is_virtual_listed_child, is_virtual_zip_child, is_zip_file_path,
    listed_virtual_identity_from_virtual_path, listed_virtual_name_from_virtual_path,
    listed_virtual_root, resolve_start_path, zip_virtual_root,
};

#[derive(Clone, Debug)]
pub(crate) struct FileNavigator {
    current_path: PathBuf,
    files: Option<Vec<PathBuf>>,
    current: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct NavigationTarget {
    pub(crate) navigation_path: PathBuf,
    pub(crate) load_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) enum NavigationOutcome {
    Resolved(NavigationTarget),
    NoPath,
}

#[derive(Clone, Copy)]
pub(crate) enum PendingDirection {
    Next,
    Prev,
}

impl FileNavigator {
    pub(crate) fn from_current_path(path: PathBuf, _cache: &mut FilesystemCache) -> Self {
        Self {
            current_path: path,
            files: None,
            current: 0,
        }
    }

    pub(crate) fn current(&self) -> &Path {
        &self.current_path
    }

    pub(crate) fn set_current_input(&mut self, path: PathBuf, cache: &mut FilesystemCache) {
        let Some(navigation_path) = resolve_navigation_path(&path, cache) else {
            return;
        };

        self.current_path = navigation_path;
        self.files = None;
        self.current = 0;
    }

    pub(crate) fn first(&mut self, cache: &mut FilesystemCache) -> Option<PathBuf> {
        let files = edge_entries(self.current(), cache)?;
        if files.is_empty() {
            return None;
        }

        self.current = 0;
        self.files = Some(files.clone());
        let path = files.first()?.clone();
        self.current_path = path.clone();
        Some(path)
    }

    pub(crate) fn last(&mut self, cache: &mut FilesystemCache) -> Option<PathBuf> {
        let files = edge_entries(self.current(), cache)?;
        let len = files.len();
        if len == 0 {
            return None;
        }

        self.current = len - 1;
        self.files = Some(files.clone());
        let path = files.get(self.current)?.clone();
        self.current_path = path.clone();
        Some(path)
    }

    pub(crate) fn current_target(&self) -> NavigationOutcome {
        let Some(load_path) = resolve_start_path(&self.current_path) else {
            return NavigationOutcome::NoPath;
        };

        NavigationOutcome::Resolved(NavigationTarget {
            navigation_path: self.current_path.clone(),
            load_path,
        })
    }

    pub(crate) fn next_with_policy(
        &mut self,
        policy: EndOfFolderOption,
        cache: &mut FilesystemCache,
    ) -> NavigationOutcome {
        if self.next(cache).is_some() {
            return self.current_target();
        }

        match policy {
            EndOfFolderOption::Stop => NavigationOutcome::NoPath,
            EndOfFolderOption::Loop => self
                .first(cache)
                .map(|_| self.current_target())
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Next => self
                .jump_to_adjacent_directory(true, cache)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Recursive => find_recursive_next_path(cache, self.current())
                .map(|path| {
                    self.current_path = path;
                    self.files = None;
                    self.current = 0;
                    self.current_target()
                })
                .unwrap_or(NavigationOutcome::NoPath),
        }
    }

    pub(crate) fn prev_with_policy(
        &mut self,
        policy: EndOfFolderOption,
        cache: &mut FilesystemCache,
    ) -> NavigationOutcome {
        if self.prev(cache).is_some() {
            return self.current_target();
        }

        match policy {
            EndOfFolderOption::Stop => NavigationOutcome::NoPath,
            EndOfFolderOption::Loop => self
                .last(cache)
                .map(|_| self.current_target())
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Next => self
                .jump_to_adjacent_directory(false, cache)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Recursive => find_recursive_prev_path(cache, self.current())
                .map(|path| {
                    self.current_path = path;
                    self.files = None;
                    self.current = 0;
                    self.current_target()
                })
                .unwrap_or(NavigationOutcome::NoPath),
        }
    }

    fn normalize_current_path(&mut self, cache: &mut FilesystemCache) {
        if let Some(navigation_path) = resolve_navigation_path(&self.current_path, cache) {
            if navigation_path != self.current_path {
                self.current_path = navigation_path;
                self.files = None;
                self.current = 0;
            }
        }
    }

    fn ensure_files<'a>(&'a mut self, cache: &mut FilesystemCache) -> &'a [PathBuf] {
        self.normalize_current_path(cache);
        if self.files.is_none() {
            let files = flat_container_entries(&self.current_path, cache)
                .unwrap_or_else(|| vec![self.current_path.clone()]);
            self.current = files
                .iter()
                .position(|candidate| candidate == &self.current_path)
                .unwrap_or(0);
            self.files = Some(files);
        }

        self.files.as_deref().unwrap_or(&[])
    }

    fn next(&mut self, cache: &mut FilesystemCache) -> Option<PathBuf> {
        let len = self.ensure_files(cache).len();
        if self.current + 1 >= len {
            return None;
        }

        self.current += 1;
        let path = self.files.as_ref()?.get(self.current)?.clone();
        self.current_path = path.clone();
        Some(path)
    }

    fn prev(&mut self, cache: &mut FilesystemCache) -> Option<PathBuf> {
        let _ = self.ensure_files(cache);
        if self.current == 0 {
            return None;
        }

        self.current -= 1;
        let path = self.files.as_ref()?.get(self.current)?.clone();
        self.current_path = path.clone();
        Some(path)
    }

    fn jump_to_adjacent_directory(
        &mut self,
        forward: bool,
        cache: &mut FilesystemCache,
    ) -> Option<NavigationOutcome> {
        let current_dir = next_policy_directory(self.current())?;
        let parent_dir = current_dir.parent()?;
        let directories = cache.child_directories(parent_dir);
        let current_index = directories.iter().position(|dir| dir == &current_dir)?;

        let target = if forward {
            directories.iter().skip(current_index + 1).find_map(|dir| {
                cache
                    .first_supported_file(dir)
                    .map(|path| (dir.clone(), path))
            })
        } else {
            directories[..current_index].iter().rev().find_map(|dir| {
                cache
                    .last_supported_file(dir)
                    .map(|path| (dir.clone(), path))
            })
        }?;

        let _ = target.0;
        self.current_path = target.1;
        self.files = None;
        self.current = 0;
        Some(self.current_target())
    }
}

pub fn adjacent_entry(
    path: &Path,
    sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
    step: isize,
) -> Option<PathBuf> {
    let mut cache = FilesystemCache::new(sort, archive_mode);
    let start_path = resolve_navigation_path(path, &mut cache)?;
    let mut navigator = FileNavigator::from_current_path(start_path, &mut cache);

    if step == 0 {
        return Some(navigator.current().to_path_buf());
    }

    let count = step.unsigned_abs();
    let mut result = None;
    for _ in 0..count {
        result = if step > 0 {
            navigator.next(&mut cache)
        } else {
            navigator.prev(&mut cache)
        };
        result.as_ref()?;
    }
    result
}

pub fn navigation_branch_path(path: &Path) -> Option<PathBuf> {
    recursive_branch_dir(path)
}

pub fn resolve_navigation_entry_path(path: &Path) -> Option<PathBuf> {
    let mut cache = FilesystemCache::default();
    resolve_navigation_path(path, &mut cache)
}

pub(crate) fn resolve_navigation_path(path: &Path, cache: &mut FilesystemCache) -> Option<PathBuf> {
    if is_virtual_zip_child(path) {
        return resolve_start_path(path).map(|_| path.to_path_buf());
    }

    if is_virtual_listed_child(path) {
        return rebase_virtual_listed_child_path(path, cache)
            .or_else(|| resolve_start_path(path).map(|_| path.to_path_buf()));
    }

    if is_listed_file_path(path) || is_zip_file_path(path) || path.is_dir() {
        return cache
            .probe_first_supported_file(path)
            .or_else(|| cache.first_supported_file(path))
            .or_else(|| Some(path.to_path_buf()));
    }

    resolve_start_path(path).map(|_| path.to_path_buf())
}

fn rebase_virtual_listed_child_path(path: &Path, cache: &mut FilesystemCache) -> Option<PathBuf> {
    let listed_root = listed_virtual_root(path)?;
    let expected_identity = listed_virtual_identity_from_virtual_path(path);
    cache
        .supported_entries(&listed_root)
        .into_iter()
        .find(|entry| {
            listed_virtual_identity_from_virtual_path(entry)
                .zip(expected_identity)
                .map(|(left, right)| left == right)
                .unwrap_or(false)
        })
        .or_else(|| {
            let expected_name = listed_virtual_name_from_virtual_path(path)?;
            cache
                .supported_entries(&listed_root)
                .into_iter()
                .find(|entry| {
                    listed_virtual_name_from_virtual_path(entry)
                        .map(|name| name.eq_ignore_ascii_case(&expected_name))
                        .unwrap_or(false)
                })
        })
}

fn flat_container_entries(path: &Path, cache: &mut FilesystemCache) -> Option<Vec<PathBuf>> {
    if path.is_dir() || is_zip_file_path(path) || is_listed_file_path(path) {
        return Some(cache.supported_entries(path));
    }
    let dir = flat_container_dir(path)?;
    Some(cache.supported_entries(&dir))
}

fn edge_entries(path: &Path, cache: &mut FilesystemCache) -> Option<Vec<PathBuf>> {
    if let Some(zip_root) = zip_virtual_root(path) {
        return Some(cache.supported_entries(&zip_root));
    }

    if let Some(listed_root) = listed_virtual_root(path) {
        return Some(cache.supported_entries(&listed_root));
    }

    flat_container_entries(path, cache)
}

fn flat_container_dir(path: &Path) -> Option<PathBuf> {
    if let Some(zip_root) = zip_virtual_root(path) {
        return zip_root.parent().map(Path::to_path_buf);
    }

    if let Some(listed_root) = listed_virtual_root(path) {
        return listed_root.parent().map(Path::to_path_buf);
    }

    path.parent().map(Path::to_path_buf)
}

fn next_policy_directory(path: &Path) -> Option<PathBuf> {
    if path.is_dir() || is_zip_file_path(path) || is_listed_file_path(path) {
        return Some(path.to_path_buf());
    }

    if let Some(zip_root) = zip_virtual_root(path) {
        return zip_root.parent().map(Path::to_path_buf);
    }

    if let Some(listed_root) = listed_virtual_root(path) {
        return listed_root.parent().map(Path::to_path_buf);
    }

    path.parent().map(Path::to_path_buf)
}

fn recursive_branch_dir(path: &Path) -> Option<PathBuf> {
    if path.is_dir() || is_zip_file_path(path) || is_listed_file_path(path) {
        return Some(path.to_path_buf());
    }

    if let Some(zip_root) = zip_virtual_root(path) {
        return Some(zip_root);
    }

    if let Some(listed_root) = listed_virtual_root(path) {
        return Some(listed_root);
    }

    path.parent().map(Path::to_path_buf)
}

fn find_recursive_next_path(cache: &mut FilesystemCache, current_path: &Path) -> Option<PathBuf> {
    let mut branch_dir = recursive_branch_dir(current_path)?;

    loop {
        let parent_dir = branch_dir.parent()?.to_path_buf();
        let directories = cache.child_directories(&parent_dir);
        let current_index = directories.iter().position(|dir| dir == &branch_dir)?;

        for sibling_dir in directories.iter().skip(current_index + 1) {
            if let Some(path) = first_path_in_subtree(cache, sibling_dir) {
                return Some(path);
            }
        }

        branch_dir = parent_dir;
    }
}

fn find_recursive_prev_path(cache: &mut FilesystemCache, current_path: &Path) -> Option<PathBuf> {
    let mut branch_dir = recursive_branch_dir(current_path)?;

    loop {
        let parent_dir = branch_dir.parent()?.to_path_buf();
        let directories = cache.child_directories(&parent_dir);
        let current_index = directories.iter().position(|dir| dir == &branch_dir)?;

        for sibling_dir in directories[..current_index].iter().rev() {
            if let Some(path) = last_path_in_subtree(cache, sibling_dir) {
                return Some(path);
            }
        }

        branch_dir = parent_dir;
    }
}

fn first_path_in_subtree(cache: &mut FilesystemCache, dir: &Path) -> Option<PathBuf> {
    if let Some(path) = cache.first_supported_file(dir) {
        return Some(path);
    }

    for child_dir in cache.child_directories(dir) {
        if let Some(path) = first_path_in_subtree(cache, &child_dir) {
            return Some(path);
        }
    }

    None
}

fn last_path_in_subtree(cache: &mut FilesystemCache, dir: &Path) -> Option<PathBuf> {
    let child_dirs = cache.child_directories(dir);
    for child_dir in child_dirs.iter().rev() {
        if let Some(path) = last_path_in_subtree(cache, &child_dir) {
            return Some(path);
        }
    }

    cache.last_supported_file(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{ArchiveBrowseOption, NavigationSortOption};

    #[test]
    fn navigator_from_current_path_does_not_eagerly_expand_file_list() {
        let mut cache =
            FilesystemCache::new(NavigationSortOption::OsName, ArchiveBrowseOption::Folder);
        let path = PathBuf::from("sample.png");

        let navigator = FileNavigator::from_current_path(path.clone(), &mut cache);

        assert_eq!(navigator.current_path, path);
        assert!(navigator.files.is_none());
        assert_eq!(navigator.current, 0);
    }
}
