use std::collections::VecDeque;
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

    pub fn from_directory(path: &Path) -> Option<(Self, PathBuf)> {
        let files = collect_supported_files(path);
        let first = files.first()?.clone();
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
        recursive_index: Option<&mut RecursiveIndex>,
    ) -> Option<PathBuf> {
        if let Some(path) = self.next() {
            return Some(path);
        }

        match policy {
            EndOfFolderOption::Stop => None,
            EndOfFolderOption::Loop => self.first(),
            EndOfFolderOption::Next => self.jump_to_adjacent_directory(true),
            EndOfFolderOption::Recursive => {
                let current = self.current().to_path_buf();
                let index = recursive_index?;
                index
                    .find_next_after(&current)
                    .inspect(|path| self.reset_to_path(path.clone()))
            }
        }
    }

    fn prev_with_policy(
        &mut self,
        policy: EndOfFolderOption,
        recursive_index: Option<&mut RecursiveIndex>,
    ) -> Option<PathBuf> {
        if let Some(path) = self.prev() {
            return Some(path);
        }

        match policy {
            EndOfFolderOption::Stop => None,
            EndOfFolderOption::Loop => self.last(),
            EndOfFolderOption::Next => self.jump_to_adjacent_directory(false),
            EndOfFolderOption::Recursive => {
                let current = self.current().to_path_buf();
                let index = recursive_index?;
                index
                    .find_prev_before(&current)
                    .inspect(|path| self.reset_to_path(path.clone()))
            }
        }
    }

    fn jump_to_adjacent_directory(&mut self, forward: bool) -> Option<PathBuf> {
        let current_dir = self.current().parent()?;
        let parent_dir = current_dir.parent()?;
        let mut directories = collect_child_directories(parent_dir);
        directories.retain(|dir| dir != current_dir);

        let target_dir = if forward {
            directories.into_iter().find(|dir| dir > current_dir)
        } else {
            directories.into_iter().rev().find(|dir| dir < current_dir)
        }?;

        let files = collect_supported_files(&target_dir);
        if files.is_empty() {
            return None;
        }

        self.files = files;
        self.current = if forward { 0 } else { self.files.len() - 1 };
        Some(self.files[self.current].clone())
    }

    fn reset_to_path(&mut self, path: PathBuf) {
        let dir = path.parent().unwrap_or(Path::new("."));
        self.files = collect_supported_files(dir);
        self.current = self
            .files
            .iter()
            .position(|candidate| candidate == &path)
            .unwrap_or(0);
    }
}

pub fn resolve_start_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return path
            .canonicalize()
            .ok()
            .or_else(|| Some(path.to_path_buf()));
    }

    if path.is_dir() {
        return FileNavigator::from_directory(path).map(|(_, first)| first);
    }

    None
}

impl RecursiveIndex {
    fn new(path: &Path) -> Self {
        let current = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let current_dir = current.parent().unwrap_or(Path::new("."));
        let root = current_dir.parent().unwrap_or(current_dir).to_path_buf();

        let mut pending_dirs = VecDeque::new();
        pending_dirs.push_back(root.clone());

        let mut files = collect_supported_files(current_dir);
        files.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());

        Self {
            files,
            pending_dirs,
            complete: false,
            batch_size: 10,
        }
    }

    fn advance(&mut self) {
        if self.complete {
            return;
        }

        let mut processed = 0;
        while processed < self.batch_size {
            let Some(dir) = self.pending_dirs.pop_front() else {
                self.complete = true;
                break;
            };

            let mut subdirs = collect_child_directories(&dir);
            subdirs.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
            for subdir in subdirs {
                self.pending_dirs.push_back(subdir);
            }

            let mut files = collect_supported_files(&dir);
            self.files.append(&mut files);
            self.files
                .sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
            self.files.dedup();
            processed += 1;
        }

        self.batch_size = (self.batch_size.saturating_mul(2)).min(4096);
    }

    fn find_next_after(&mut self, current: &Path) -> Option<PathBuf> {
        loop {
            if let Some(index) = self.files.iter().position(|path| path == current) {
                if let Some(path) = self.files.get(index + 1) {
                    return Some(path.clone());
                }
            }
            if self.complete {
                return None;
            }
            self.advance();
        }
    }

    fn find_prev_before(&mut self, current: &Path) -> Option<PathBuf> {
        loop {
            if let Some(index) = self.files.iter().position(|path| path == current) {
                if index > 0 {
                    return Some(self.files[index - 1].clone());
                }
            }
            if self.complete {
                return None;
            }
            self.advance();
        }
    }
}

pub fn spawn_filesystem_worker() -> (Sender<FilesystemCommand>, Receiver<FilesystemResult>) {
    let (command_tx, command_rx) = mpsc::channel::<FilesystemCommand>();
    let (result_tx, result_rx) = mpsc::channel::<FilesystemResult>();

    thread::spawn(move || {
        let mut navigator: Option<FileNavigator> = None;
        let mut recursive_index: Option<RecursiveIndex> = None;

        loop {
            match command_rx.recv_timeout(Duration::from_millis(10)) {
                Ok(command) => match command {
                    FilesystemCommand::Init { request_id, path } => {
                        let start_path = resolve_start_path(&path).unwrap_or(path.clone());
                        navigator = Some(if path.is_dir() {
                            FileNavigator::from_directory(&path)
                                .map(|(nav, _)| nav)
                                .unwrap_or_else(|| FileNavigator::from_path(&start_path))
                        } else {
                            FileNavigator::from_path(&start_path)
                        });
                        recursive_index = Some(RecursiveIndex::new(&start_path));
                        let _ = result_tx.send(FilesystemResult::NavigatorReady { request_id });
                    }
                    FilesystemCommand::SetCurrent { request_id, path } => {
                        let start_path = resolve_start_path(&path).unwrap_or(path.clone());
                        navigator = Some(FileNavigator::from_path(&start_path));
                        recursive_index = Some(RecursiveIndex::new(&start_path));
                        let _ = request_id;
                        let _ = result_tx.send(FilesystemResult::CurrentSet);
                    }
                    FilesystemCommand::Next { request_id, policy } => {
                        let _ = send_nav_result(
                            &result_tx,
                            request_id,
                            navigator.as_mut().and_then(|nav| {
                                nav.next_with_policy(policy, recursive_index.as_mut())
                            }),
                        );
                    }
                    FilesystemCommand::Prev { request_id, policy } => {
                        let _ = send_nav_result(
                            &result_tx,
                            request_id,
                            navigator.as_mut().and_then(|nav| {
                                nav.prev_with_policy(policy, recursive_index.as_mut())
                            }),
                        );
                    }
                    FilesystemCommand::First { request_id } => {
                        let _ = send_nav_result(
                            &result_tx,
                            request_id,
                            navigator.as_mut().and_then(FileNavigator::first),
                        );
                    }
                    FilesystemCommand::Last { request_id } => {
                        let _ = send_nav_result(
                            &result_tx,
                            request_id,
                            navigator.as_mut().and_then(FileNavigator::last),
                        );
                    }
                },
                Err(RecvTimeoutError::Timeout) => {
                    if let Some(index) = &mut recursive_index {
                        index.advance();
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

fn collect_child_directories(dir: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| path.canonicalize().unwrap_or(path))
        .collect();

    directories.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
    directories
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
