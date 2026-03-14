use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use crate::options::EndOfFolderOption;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "webp", "jpg", "jpeg", "bmp", "gif", "png", "tif", "tiff", "mag", "maki", "pi", "pic",
];

#[derive(Clone, Debug)]
pub struct FileNavigator {
    files: Vec<PathBuf>,
    current: usize,
}

struct RecursiveIndex {
    files: Vec<PathBuf>,
    pending_dirs: VecDeque<PathBuf>,
    complete: bool,
    batch_size: usize,
}

#[derive(Default)]
struct FilesystemCache {
    files_by_dir: HashMap<PathBuf, Vec<PathBuf>>,
    child_dirs_by_parent: HashMap<PathBuf, Vec<PathBuf>>,
    first_file_by_dir: HashMap<PathBuf, Option<PathBuf>>,
    last_file_by_dir: HashMap<PathBuf, Option<PathBuf>>,
}

enum SearchState {
    Found(PathBuf),
    Pending,
    Exhausted,
}

#[derive(Clone, Copy)]
enum PendingDirection {
    Next,
    Prev,
}

#[derive(Clone, Copy)]
struct PendingNavigation {
    request_id: u64,
    direction: PendingDirection,
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
                files: vec![target],
                current: 0,
            };
        }

        let current = files
            .iter()
            .position(|candidate| candidate == &target)
            .unwrap_or(0);

        Self { files, current }
    }

    fn from_directory(path: &Path, cache: &mut FilesystemCache) -> Option<(Self, PathBuf)> {
        let first = cache.first_supported_file(path)?;
        let files = cache.supported_files(path);
        Some((Self { files, current: 0 }, first))
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

    fn next_with_policy(
        &mut self,
        policy: EndOfFolderOption,
        recursive_index: Option<&RecursiveIndex>,
        cache: &mut FilesystemCache,
    ) -> NavigationOutcome {
        if let Some(path) = self.next() {
            return NavigationOutcome::Resolved(path);
        }

        match policy {
            EndOfFolderOption::Stop => NavigationOutcome::NoPath,
            EndOfFolderOption::Loop => self
                .first()
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Next => self
                .jump_to_adjacent_directory(true, cache)
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Recursive => {
                let current = self.current().to_path_buf();
                let Some(index) = recursive_index else {
                    return NavigationOutcome::NoPath;
                };
                match index.find_next_after(&current) {
                    SearchState::Found(path) => {
                        self.reset_to_path(path.clone(), cache);
                        NavigationOutcome::Resolved(path)
                    }
                    SearchState::Pending => NavigationOutcome::Pending,
                    SearchState::Exhausted => NavigationOutcome::NoPath,
                }
            }
        }
    }

    fn prev_with_policy(
        &mut self,
        policy: EndOfFolderOption,
        recursive_index: Option<&RecursiveIndex>,
        cache: &mut FilesystemCache,
    ) -> NavigationOutcome {
        if let Some(path) = self.prev() {
            return NavigationOutcome::Resolved(path);
        }

        match policy {
            EndOfFolderOption::Stop => NavigationOutcome::NoPath,
            EndOfFolderOption::Loop => self
                .last()
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Next => self
                .jump_to_adjacent_directory(false, cache)
                .map(NavigationOutcome::Resolved)
                .unwrap_or(NavigationOutcome::NoPath),
            EndOfFolderOption::Recursive => {
                let current = self.current().to_path_buf();
                let Some(index) = recursive_index else {
                    return NavigationOutcome::NoPath;
                };
                match index.find_prev_before(&current) {
                    SearchState::Found(path) => {
                        self.reset_to_path(path.clone(), cache);
                        NavigationOutcome::Resolved(path)
                    }
                    SearchState::Pending => NavigationOutcome::Pending,
                    SearchState::Exhausted => NavigationOutcome::NoPath,
                }
            }
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
                let Some(path) = cache.first_supported_file(target_dir) else {
                    continue;
                };
                self.reset_to_path(path.clone(), cache);
                return Some(path);
            }
        } else {
            for target_dir in directories[..current_index].iter().rev() {
                let Some(path) = cache.last_supported_file(target_dir) else {
                    continue;
                };
                self.reset_to_path(path.clone(), cache);
                return Some(path);
            }
        }

        None
    }

    fn reset_to_path(&mut self, path: PathBuf, cache: &mut FilesystemCache) {
        let dir = path.parent().unwrap_or(Path::new("."));
        self.files = cache.supported_files(dir);
        self.current = self
            .files
            .iter()
            .position(|candidate| candidate == &path)
            .unwrap_or(0);
    }
}

enum NavigationOutcome {
    Resolved(PathBuf),
    Pending,
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

impl RecursiveIndex {
    fn new(path: &Path, cache: &mut FilesystemCache) -> Self {
        let current_dir = path.parent().unwrap_or(Path::new("."));
        let root = current_dir.parent().unwrap_or(current_dir).to_path_buf();

        let mut pending_dirs = VecDeque::new();
        pending_dirs.push_back(root.clone());

        let mut files = cache.supported_files(current_dir);
        files.sort_by_cached_key(|path| path_sort_key(path));

        Self {
            files,
            pending_dirs,
            complete: false,
            batch_size: 10,
        }
    }

    fn advance(&mut self, cache: &mut FilesystemCache) {
        if self.complete {
            return;
        }

        let mut processed = 0;
        while processed < self.batch_size {
            let Some(dir) = self.pending_dirs.pop_front() else {
                self.complete = true;
                break;
            };

            let mut subdirs = cache.child_directories(&dir);
            subdirs.sort_by_cached_key(|path| path_sort_key(path));
            for subdir in subdirs {
                self.pending_dirs.push_back(subdir);
            }

            let mut files = cache.supported_files(&dir);
            self.files.append(&mut files);
            self.files.sort_by_cached_key(|path| path_sort_key(path));
            self.files.dedup();
            processed += 1;
        }

        self.batch_size = (self.batch_size.saturating_mul(2)).min(4096);
    }

    fn find_next_after(&self, current: &Path) -> SearchState {
        if let Some(index) = self.files.iter().position(|path| path == current) {
            if let Some(path) = self.files.get(index + 1) {
                return SearchState::Found(path.clone());
            }
        }

        if self.complete {
            SearchState::Exhausted
        } else {
            SearchState::Pending
        }
    }

    fn find_prev_before(&self, current: &Path) -> SearchState {
        if let Some(index) = self.files.iter().position(|path| path == current) {
            if index > 0 {
                return SearchState::Found(self.files[index - 1].clone());
            }
        }

        if self.complete {
            SearchState::Exhausted
        } else {
            SearchState::Pending
        }
    }
}

pub fn spawn_filesystem_worker() -> (Sender<FilesystemCommand>, Receiver<FilesystemResult>) {
    let (command_tx, command_rx) = mpsc::channel::<FilesystemCommand>();
    let (result_tx, result_rx) = mpsc::channel::<FilesystemResult>();

    thread::spawn(move || {
        let mut navigator: Option<FileNavigator> = None;
        let mut recursive_index: Option<RecursiveIndex> = None;
        let mut pending_navigation: Option<PendingNavigation> = None;
        let mut cache = FilesystemCache::default();

        loop {
            match command_rx.recv_timeout(Duration::from_millis(10)) {
                Ok(command) => match command {
                    FilesystemCommand::Init { request_id, path } => {
                        pending_navigation = None;
                        let start_path = resolve_start_path(&path).unwrap_or(path.clone());
                        navigator = Some(if path.is_dir() {
                            FileNavigator::from_directory(&path, &mut cache)
                                .map(|(nav, _)| nav)
                                .unwrap_or_else(|| FileNavigator::from_path(&start_path, &mut cache))
                        } else {
                            FileNavigator::from_path(&start_path, &mut cache)
                        });
                        recursive_index = None;
                        let _ = result_tx.send(FilesystemResult::NavigatorReady { request_id });
                    }
                    FilesystemCommand::SetCurrent { request_id, path } => {
                        pending_navigation = None;
                        let start_path = resolve_start_path(&path).unwrap_or(path.clone());
                        navigator = Some(FileNavigator::from_path(&start_path, &mut cache));
                        recursive_index = None;
                        let _ = request_id;
                        let _ = result_tx.send(FilesystemResult::CurrentSet);
                    }
                    FilesystemCommand::Next { request_id, policy } => {
                        pending_navigation = None;
                        handle_navigation_request(
                            &result_tx,
                            &mut pending_navigation,
                            navigator.as_mut(),
                            &mut recursive_index,
                            &mut cache,
                            request_id,
                            policy,
                            PendingDirection::Next,
                        );
                    }
                    FilesystemCommand::Prev { request_id, policy } => {
                        pending_navigation = None;
                        handle_navigation_request(
                            &result_tx,
                            &mut pending_navigation,
                            navigator.as_mut(),
                            &mut recursive_index,
                            &mut cache,
                            request_id,
                            policy,
                            PendingDirection::Prev,
                        );
                    }
                    FilesystemCommand::First { request_id } => {
                        pending_navigation = None;
                        let _ = send_nav_result(
                            &result_tx,
                            request_id,
                            navigator.as_mut().and_then(FileNavigator::first),
                        );
                    }
                    FilesystemCommand::Last { request_id } => {
                        pending_navigation = None;
                        let _ = send_nav_result(
                            &result_tx,
                            request_id,
                            navigator.as_mut().and_then(FileNavigator::last),
                        );
                    }
                },
                Err(RecvTimeoutError::Timeout) => {
                    if let Some(pending) = pending_navigation {
                        if let Some(index) = &mut recursive_index {
                            index.advance(&mut cache);
                        }
                        let outcome = match (navigator.as_mut(), recursive_index.as_ref()) {
                            (Some(nav), Some(index)) => match pending.direction {
                                PendingDirection::Next => {
                                    nav.next_with_policy(
                                        EndOfFolderOption::Recursive,
                                        Some(index),
                                        &mut cache,
                                    )
                                }
                                PendingDirection::Prev => {
                                    nav.prev_with_policy(
                                        EndOfFolderOption::Recursive,
                                        Some(index),
                                        &mut cache,
                                    )
                                }
                            },
                            _ => NavigationOutcome::NoPath,
                        };
                        match outcome {
                            NavigationOutcome::Resolved(path) => {
                                pending_navigation = None;
                                let _ = send_nav_result(&result_tx, pending.request_id, Some(path));
                            }
                            NavigationOutcome::NoPath => {
                                pending_navigation = None;
                                let _ = send_nav_result(&result_tx, pending.request_id, None);
                            }
                            NavigationOutcome::Pending => {}
                        }
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
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
    pending_navigation: &mut Option<PendingNavigation>,
    navigator: Option<&mut FileNavigator>,
    recursive_index: &mut Option<RecursiveIndex>,
    cache: &mut FilesystemCache,
    request_id: u64,
    policy: EndOfFolderOption,
    direction: PendingDirection,
) {
    let outcome = match navigator {
        Some(nav) => {
            if matches!(policy, EndOfFolderOption::Recursive) && recursive_index.is_none() {
                *recursive_index = Some(RecursiveIndex::new(nav.current(), cache));
            }
            match direction {
                PendingDirection::Next => {
                    nav.next_with_policy(policy, recursive_index.as_ref(), cache)
                }
                PendingDirection::Prev => {
                    nav.prev_with_policy(policy, recursive_index.as_ref(), cache)
                }
            }
        }
        None => NavigationOutcome::NoPath,
    };

    match outcome {
        NavigationOutcome::Resolved(path) => {
            let _ = send_nav_result(tx, request_id, Some(path));
        }
        NavigationOutcome::NoPath => {
            let _ = send_nav_result(tx, request_id, None);
        }
        NavigationOutcome::Pending => {
            *pending_navigation = Some(PendingNavigation {
                request_id,
                direction,
            });
        }
    }
}

impl FilesystemCache {
    fn supported_files(&mut self, dir: &Path) -> Vec<PathBuf> {
        if let Some(files) = self.files_by_dir.get(dir) {
            return files.clone();
        }

        let files = scan_supported_files(dir);
        self.files_by_dir.insert(dir.to_path_buf(), files.clone());
        files
    }

    fn child_directories(&mut self, dir: &Path) -> Vec<PathBuf> {
        if let Some(directories) = self.child_dirs_by_parent.get(dir) {
            return directories.clone();
        }

        let directories = scan_child_directories(dir);
        self.child_dirs_by_parent
            .insert(dir.to_path_buf(), directories.clone());
        directories
    }

    fn first_supported_file(&mut self, dir: &Path) -> Option<PathBuf> {
        if let Some(path) = self.first_file_by_dir.get(dir) {
            return path.clone();
        }

        let path = scan_edge_supported_file(dir, false);
        self.first_file_by_dir.insert(dir.to_path_buf(), path.clone());
        path
    }

    fn last_supported_file(&mut self, dir: &Path) -> Option<PathBuf> {
        if let Some(path) = self.last_file_by_dir.get(dir) {
            return path.clone();
        }

        let path = scan_edge_supported_file(dir, true);
        self.last_file_by_dir.insert(dir.to_path_buf(), path.clone());
        path
    }
}

fn scan_supported_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Some(entries) = fs::read_dir(dir).ok() else {
        return files;
    };

    for entry in entries.filter_map(Result::ok) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        if is_supported_image(&path) {
            files.push(path);
        }
    }

    files.sort_by_cached_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    files
}

fn scan_child_directories(dir: &Path) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    let Some(entries) = fs::read_dir(dir).ok() else {
        return directories;
    };

    for entry in entries.filter_map(Result::ok) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            directories.push(entry.path());
        }
    }

    directories.sort_by_cached_key(|path| path_sort_key(path));
    directories
}

fn scan_edge_supported_file(dir: &Path, reverse: bool) -> Option<PathBuf> {
    let mut files = scan_supported_files(dir);
    if reverse {
        files.pop()
    } else {
        files.into_iter().next()
    }
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

fn path_sort_key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}
