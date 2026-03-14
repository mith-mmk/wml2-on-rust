use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::options::EndOfFolderOption;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "webp", "jpg", "jpeg", "bmp", "gif", "png", "tif", "tiff", "mag", "maki", "pi", "pic",
];

#[derive(Clone, Debug)]
pub struct FileNavigator {
    current_path: PathBuf,
    files: Option<Vec<PathBuf>>,
    current: usize,
}

#[derive(Default)]
struct FilesystemCache {
    listings_by_dir: HashMap<PathBuf, DirectoryListing>,
}

#[derive(Clone, Default)]
struct DirectoryListing {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
    first_file: Option<PathBuf>,
    last_file: Option<PathBuf>,
}

#[derive(Clone, Copy)]
enum PendingDirection {
    Next,
    Prev,
}

pub enum FilesystemCommand {
    Init {
        request_id: u64,
        path: PathBuf,
    },
    SetCurrent {
        request_id: u64,
        path: PathBuf,
    },
    Next {
        request_id: u64,
        policy: EndOfFolderOption,
    },
    Prev {
        request_id: u64,
        policy: EndOfFolderOption,
    },
    First {
        request_id: u64,
    },
    Last {
        request_id: u64,
    },
}

pub enum FilesystemResult {
    NavigatorReady { request_id: u64 },
    CurrentSet,
    PathResolved { request_id: u64, path: PathBuf },
    NoPath { request_id: u64 },
}

impl FileNavigator {
    fn from_path(path: &Path, cache: &mut FilesystemCache) -> Self {
        let target = path.to_path_buf();
        let files = match path.parent() {
            Some(parent) => cache.supported_files(parent),
            None => vec![target.clone()],
        };

        if files.is_empty() {
            return Self {
                current_path: target,
                files: None,
                current: 0,
            };
        }

        let current = files
            .iter()
            .position(|candidate| candidate == &target)
            .unwrap_or(0);

        Self {
            current_path: target,
            files: Some(files),
            current,
        }
    }

    fn from_directory(path: &Path, cache: &mut FilesystemCache) -> Option<(Self, PathBuf)> {
        let first = cache.first_supported_file(path)?;
        let files = cache.supported_files(path);
        Some((
            Self {
                current_path: first.clone(),
                files: Some(files),
                current: 0,
            },
            first,
        ))
    }

    pub fn current(&self) -> &Path {
        &self.current_path
    }

    fn ensure_files<'a>(&'a mut self, cache: &mut FilesystemCache) -> &'a [PathBuf] {
        if self.files.is_none() {
            let files = match self.current_path.parent() {
                Some(parent) => cache.supported_files(parent),
                None => vec![self.current_path.clone()],
            };
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

    fn first(&mut self, cache: &mut FilesystemCache) -> Option<PathBuf> {
        let files = self.ensure_files(cache);
        if files.is_empty() {
            return None;
        }
        self.current = 0;
        let path = self.files.as_ref()?.first()?.clone();
        self.current_path = path.clone();
        Some(path)
    }

    fn last(&mut self, cache: &mut FilesystemCache) -> Option<PathBuf> {
        let len = self.ensure_files(cache).len();
        if len == 0 {
            return None;
        }
        self.current = len - 1;
        let path = self.files.as_ref()?.get(self.current)?.clone();
        self.current_path = path.clone();
        Some(path)
    }

    fn set_current_path(&mut self, path: PathBuf) {
        self.current_path = path;
        self.files = None;
        self.current = 0;
    }

    fn next_with_policy(
        &mut self,
        policy: EndOfFolderOption,
        cache: &mut FilesystemCache,
    ) -> NavigationOutcome {
        if let Some(path) = self.next(cache) {
            return NavigationOutcome::Resolved(path);
        }

        match policy {
            EndOfFolderOption::Stop => NavigationOutcome::NoPath,
            EndOfFolderOption::Loop => self
                .first(cache)
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Next => self
                .jump_to_adjacent_directory(true, cache)
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Recursive => find_recursive_next_path(cache, self.current())
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
        }
    }

    fn prev_with_policy(
        &mut self,
        policy: EndOfFolderOption,
        cache: &mut FilesystemCache,
    ) -> NavigationOutcome {
        if let Some(path) = self.prev(cache) {
            return NavigationOutcome::Resolved(path);
        }

        match policy {
            EndOfFolderOption::Stop => NavigationOutcome::NoPath,
            EndOfFolderOption::Loop => self
                .last(cache)
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Next => self
                .jump_to_adjacent_directory(false, cache)
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Recursive => find_recursive_prev_path(cache, self.current())
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
        }
    }

    fn jump_to_adjacent_directory(
        &mut self,
        forward: bool,
        cache: &mut FilesystemCache,
    ) -> Option<PathBuf> {
        let current_dir = self.current().parent()?;
        let parent_dir = current_dir.parent()?;
        let directories = cache.child_directories(parent_dir);
        let current_index = directories.iter().position(|dir| dir == current_dir)?;

        if forward {
            for target_dir in directories.iter().skip(current_index + 1) {
                if let Some(path) = cache.first_supported_file(target_dir) {
                    return Some(path);
                }
            }
        } else {
            for target_dir in directories[..current_index].iter().rev() {
                if let Some(path) = cache.last_supported_file(target_dir) {
                    return Some(path);
                }
            }
        }

        None
    }

}

enum NavigationOutcome {
    Resolved(PathBuf),
    NoPath,
}

pub fn resolve_start_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }

    if path.is_dir() {
        let mut cache = FilesystemCache::default();
        return FileNavigator::from_directory(path, &mut cache).map(|(_, first)| first);
    }

    None
}

pub fn spawn_filesystem_worker() -> (Sender<FilesystemCommand>, Receiver<FilesystemResult>) {
    let (command_tx, command_rx) = mpsc::channel::<FilesystemCommand>();
    let (result_tx, result_rx) = mpsc::channel::<FilesystemResult>();

    thread::spawn(move || {
        let mut navigator: Option<FileNavigator> = None;
        let mut cache = FilesystemCache::default();

        while let Ok(command) = command_rx.recv() {
            match command {
                FilesystemCommand::Init { request_id, path } => {
                    let start_path = resolve_start_path(&path).unwrap_or(path.clone());
                    navigator = Some(if path.is_dir() {
                        FileNavigator::from_directory(&path, &mut cache)
                            .map(|(nav, _)| nav)
                            .unwrap_or_else(|| FileNavigator::from_path(&start_path, &mut cache))
                    } else {
                        FileNavigator::from_path(&start_path, &mut cache)
                    });
                    let _ = result_tx.send(FilesystemResult::NavigatorReady { request_id });
                }
                FilesystemCommand::SetCurrent { request_id, path } => {
                    let start_path = resolve_start_path(&path).unwrap_or(path.clone());
                    match navigator.as_mut() {
                        Some(nav) => nav.set_current_path(start_path),
                        None => {
                            navigator = Some(FileNavigator::from_path(&path, &mut cache));
                        }
                    }
                    let _ = request_id;
                    let _ = result_tx.send(FilesystemResult::CurrentSet);
                }
                FilesystemCommand::Next { request_id, policy } => {
                    handle_navigation_request(
                        &result_tx,
                        navigator.as_mut(),
                        &mut cache,
                        request_id,
                        policy,
                        PendingDirection::Next,
                    );
                }
                FilesystemCommand::Prev { request_id, policy } => {
                    handle_navigation_request(
                        &result_tx,
                        navigator.as_mut(),
                        &mut cache,
                        request_id,
                        policy,
                        PendingDirection::Prev,
                    );
                }
                FilesystemCommand::First { request_id } => {
                    let _ = send_nav_result(
                        &result_tx,
                        request_id,
                        navigator.as_mut().and_then(|nav| nav.first(&mut cache)),
                    );
                }
                FilesystemCommand::Last { request_id } => {
                    let _ = send_nav_result(
                        &result_tx,
                        request_id,
                        navigator.as_mut().and_then(|nav| nav.last(&mut cache)),
                    );
                }
            }
        }
    });

    (command_tx, result_rx)
}

fn send_nav_result(
    tx: &Sender<FilesystemResult>,
    request_id: u64,
    path: Option<PathBuf>,
) -> Result<(), mpsc::SendError<FilesystemResult>> {
    match path {
        Some(path) => tx.send(FilesystemResult::PathResolved { request_id, path }),
        None => tx.send(FilesystemResult::NoPath { request_id }),
    }
}

fn handle_navigation_request(
    tx: &Sender<FilesystemResult>,
    navigator: Option<&mut FileNavigator>,
    cache: &mut FilesystemCache,
    request_id: u64,
    policy: EndOfFolderOption,
    direction: PendingDirection,
) {
    let outcome = match navigator {
        Some(nav) => match direction {
            PendingDirection::Next => nav.next_with_policy(policy, cache),
            PendingDirection::Prev => nav.prev_with_policy(policy, cache),
        },
        None => NavigationOutcome::NoPath,
    };

    match outcome {
        NavigationOutcome::Resolved(path) => {
            let _ = send_nav_result(tx, request_id, Some(path));
        }
        NavigationOutcome::NoPath => {
            let _ = send_nav_result(tx, request_id, None);
        }
    }
}

fn find_recursive_next_path(cache: &mut FilesystemCache, current_path: &Path) -> Option<PathBuf> {
    let mut branch_dir = current_path.parent()?.to_path_buf();

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
    let mut branch_dir = current_path.parent()?.to_path_buf();

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
        if let Some(path) = last_path_in_subtree(cache, child_dir) {
            return Some(path);
        }
    }

    cache.last_supported_file(dir)
}

impl FilesystemCache {
    fn listing(&mut self, dir: &Path) -> &DirectoryListing {
        self.listings_by_dir
            .entry(dir.to_path_buf())
            .or_insert_with(|| scan_directory_listing(dir))
    }

    fn supported_files(&mut self, dir: &Path) -> Vec<PathBuf> {
        self.listing(dir).files.clone()
    }

    fn child_directories(&mut self, dir: &Path) -> Vec<PathBuf> {
        self.listing(dir).dirs.clone()
    }

    fn first_supported_file(&mut self, dir: &Path) -> Option<PathBuf> {
        self.listing(dir).first_file.clone()
    }

    fn last_supported_file(&mut self, dir: &Path) -> Option<PathBuf> {
        self.listing(dir).last_file.clone()
    }
}

fn scan_directory_listing(dir: &Path) -> DirectoryListing {
    let mut listing = DirectoryListing::default();
    let Some(entries) = fs::read_dir(dir).ok() else {
        return listing;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if is_supported_image(&path) {
            listing.files.push(path);
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            listing.dirs.push(path);
        }
    }

    listing
        .files
        .sort_by_cached_key(|path| file_name_sort_key(path));
    listing.dirs.sort_by_cached_key(|path| file_name_sort_key(path));
    listing.first_file = listing.files.first().cloned();
    listing.last_file = listing.files.last().cloned();
    listing
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

fn file_name_sort_key(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}
