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
    files: Vec<PathBuf>,
    current: usize,
}

pub enum FilesystemCommand {
    Init {
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

    pub fn next_with_policy(&mut self, policy: EndOfFolderOption) -> Option<PathBuf> {
        if let Some(path) = self.next() {
            return Some(path);
        }

        match policy {
            EndOfFolderOption::Stop => None,
            EndOfFolderOption::Loop => self.first(),
            EndOfFolderOption::Next => self.jump_to_adjacent_directory(true),
            EndOfFolderOption::Recursive => self.jump_recursive(true),
        }
    }

    pub fn prev_with_policy(&mut self, policy: EndOfFolderOption) -> Option<PathBuf> {
        if let Some(path) = self.prev() {
            return Some(path);
        }

        match policy {
            EndOfFolderOption::Stop => None,
            EndOfFolderOption::Loop => self.last(),
            EndOfFolderOption::Next => self.jump_to_adjacent_directory(false),
            EndOfFolderOption::Recursive => self.jump_recursive(false),
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

    fn jump_recursive(&mut self, forward: bool) -> Option<PathBuf> {
        let current = self.current().to_path_buf();
        let current_dir = current.parent()?;
        let root = current_dir.parent().unwrap_or(current_dir);
        let files = collect_supported_files_recursive(root);
        let index = files.iter().position(|path| path == &current)?;

        let next_index = if forward {
            index.checked_add(1)?
        } else {
            index.checked_sub(1)?
        };

        let target = files.get(next_index)?.clone();
        self.reset_to_path(target.clone());
        Some(target)
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

pub fn spawn_filesystem_worker() -> (Sender<FilesystemCommand>, Receiver<FilesystemResult>) {
    let (command_tx, command_rx) = mpsc::channel::<FilesystemCommand>();
    let (result_tx, result_rx) = mpsc::channel::<FilesystemResult>();

    thread::spawn(move || {
        let mut navigator: Option<FileNavigator> = None;

        while let Ok(command) = command_rx.recv() {
            match command {
                FilesystemCommand::Init { request_id, path } => {
                    navigator = Some(FileNavigator::from_path(&path));
                    let _ = result_tx.send(FilesystemResult::NavigatorReady { request_id });
                }
                FilesystemCommand::Next { request_id, policy } => {
                    let _ = send_nav_result(
                        &result_tx,
                        request_id,
                        navigator
                            .as_mut()
                            .and_then(|nav| nav.next_with_policy(policy)),
                    );
                }
                FilesystemCommand::Prev { request_id, policy } => {
                    let _ = send_nav_result(
                        &result_tx,
                        request_id,
                        navigator
                            .as_mut()
                            .and_then(|nav| nav.prev_with_policy(policy)),
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

fn collect_supported_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_directories(dir, &mut |path| {
        if path.is_file() && is_supported_image(path) {
            files.push(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
        }
    });
    files.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());
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

fn visit_directories(dir: &Path, visitor: &mut dyn FnMut(&Path)) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                visit_directories(&path, visitor);
            } else {
                visitor(&path);
            }
        }
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
