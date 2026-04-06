use crate::options::NavigationSortOption;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::SystemTime;

use super::{
    browser_entry_path_from_dir_entry, compare_natural_str, compare_os_str, is_browser_container,
    list_browser_entries,
};

const PREVIEW_CHUNK_SIZE: usize = 64;

#[derive(Clone, Debug, Default)]
pub struct BrowserMetadata {
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
}

#[derive(Clone, Debug)]
pub struct BrowserEntry {
    pub path: PathBuf,
    pub label: String,
    pub is_container: bool,
    pub sort_as_container: bool,
    pub metadata: BrowserMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserSortField {
    Name,
    Modified,
    Size,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserNameSortMode {
    Os,
    CaseSensitive,
    CaseInsensitive,
}

#[derive(Clone, Debug)]
pub struct BrowserScanOptions {
    pub navigation_sort: NavigationSortOption,
    pub sort_field: BrowserSortField,
    pub ascending: bool,
    pub separate_dirs: bool,
    pub archive_as_container_in_sort: bool,
    pub filter_text: String,
    pub extension_filter: String,
    pub name_sort_mode: BrowserNameSortMode,
}

#[derive(Clone)]
pub enum BrowserQuery {
    OpenDirectory {
        request_id: u64,
        dir: PathBuf,
        selected: Option<PathBuf>,
        options: BrowserScanOptions,
    },
}

pub enum BrowserQueryResult {
    Reset {
        request_id: u64,
        directory: PathBuf,
        selected: Option<PathBuf>,
    },
    Append {
        request_id: u64,
        entries: Vec<BrowserEntry>,
    },
    Snapshot {
        request_id: u64,
        directory: PathBuf,
        entries: Vec<BrowserEntry>,
        selected: Option<PathBuf>,
    },
}

pub fn spawn_browser_query_worker() -> (Sender<BrowserQuery>, Receiver<BrowserQueryResult>) {
    let (command_tx, command_rx) = mpsc::channel::<BrowserQuery>();
    let (result_tx, result_rx) = mpsc::channel::<BrowserQueryResult>();

    thread::spawn(move || {
        while let Ok(command) = command_rx.recv() {
            let mut latest = command;
            while let Ok(next) = command_rx.try_recv() {
                latest = next;
            }
            match latest {
                BrowserQuery::OpenDirectory {
                    request_id,
                    dir,
                    selected,
                    options,
                } => {
                    let result_tx = result_tx.clone();
                    thread::spawn(move || {
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            scan_query_request(
                                &result_tx,
                                request_id,
                                dir.clone(),
                                selected.clone(),
                                options,
                            )
                        }));
                        let entries = match result {
                            Ok(entries) => entries,
                            Err(_) => Vec::new(),
                        };
                        let _ = result_tx.send(BrowserQueryResult::Snapshot {
                            request_id,
                            directory: dir,
                            entries,
                            selected,
                        });
                    });
                }
            }
        }
    });

    (command_tx, result_rx)
}

pub fn scan_browser_directory_with_preview(
    dir: &Path,
    options: &BrowserScanOptions,
    mut on_preview_chunk: impl FnMut(Vec<BrowserEntry>),
) -> Vec<BrowserEntry> {
    let collected = collect_browser_entry_paths(dir, options, &mut on_preview_chunk);
    let mut entries = collected
        .into_iter()
        .map(|path| build_browser_entry(path, options.archive_as_container_in_sort))
        .collect::<Vec<_>>();
    sort_browser_entries(
        &mut entries,
        options.sort_field,
        options.ascending,
        options.separate_dirs,
        options.name_sort_mode,
    );
    entries
}

fn scan_query_request(
    result_tx: &Sender<BrowserQueryResult>,
    request_id: u64,
    dir: PathBuf,
    selected: Option<PathBuf>,
    options: BrowserScanOptions,
) -> Vec<BrowserEntry> {
    let _ = result_tx.send(BrowserQueryResult::Reset {
        request_id,
        directory: dir.clone(),
        selected: selected.clone(),
    });

    scan_browser_directory_with_preview(&dir, &options, |entries| {
        let _ = result_tx.send(BrowserQueryResult::Append {
            request_id,
            entries,
        });
    })
}

pub fn sort_browser_entries(
    entries: &mut [BrowserEntry],
    sort_field: BrowserSortField,
    ascending: bool,
    separate_dirs: bool,
    name_sort_mode: BrowserNameSortMode,
) {
    let compare = |left: &BrowserEntry, right: &BrowserEntry| {
        let primary = match sort_field {
            BrowserSortField::Name => {
                compare_browser_name(&left.label, &right.label, name_sort_mode)
            }
            BrowserSortField::Modified => left.metadata.modified.cmp(&right.metadata.modified),
            BrowserSortField::Size => left.metadata.size.cmp(&right.metadata.size),
        };
        let order = if primary == std::cmp::Ordering::Equal {
            compare_browser_name(&left.label, &right.label, name_sort_mode)
        } else {
            primary
        };
        if ascending { order } else { order.reverse() }
    };

    if !separate_dirs {
        entries.sort_by(compare);
        return;
    }

    let mut containers = entries
        .iter()
        .filter(|entry| entry.sort_as_container)
        .cloned()
        .collect::<Vec<_>>();
    let mut files = entries
        .iter()
        .filter(|entry| !entry.sort_as_container)
        .cloned()
        .collect::<Vec<_>>();
    containers.sort_by(compare);
    files.sort_by(compare);

    for (index, entry) in containers.into_iter().chain(files.into_iter()).enumerate() {
        entries[index] = entry;
    }
}

pub fn compare_browser_name(
    left: &str,
    right: &str,
    mode: BrowserNameSortMode,
) -> std::cmp::Ordering {
    match mode {
        BrowserNameSortMode::Os => compare_os_str(left, right),
        BrowserNameSortMode::CaseSensitive => compare_natural_str(left, right, true),
        BrowserNameSortMode::CaseInsensitive => compare_natural_str(left, right, false),
    }
}

fn collect_browser_entry_paths(
    dir: &Path,
    options: &BrowserScanOptions,
    on_preview_chunk: &mut impl FnMut(Vec<BrowserEntry>),
) -> Vec<PathBuf> {
    if !dir.is_dir() {
        let mut collected = Vec::new();
        let mut preview_chunk = Vec::new();
        for path in list_browser_entries(dir, options.navigation_sort) {
            let preview_entry =
                build_preview_entry(path.clone(), options.archive_as_container_in_sort);
            if !matches_filters(
                &preview_entry,
                &options.filter_text,
                &options.extension_filter,
            ) {
                continue;
            }
            collected.push(path);
            preview_chunk.push(preview_entry);
            flush_preview_chunk(on_preview_chunk, &mut preview_chunk);
        }
        if !preview_chunk.is_empty() {
            on_preview_chunk(preview_chunk);
        }
        return collected;
    }

    let mut collected = Vec::new();
    let Ok(read_dir) = fs::read_dir(dir) else {
        return collected;
    };

    let mut preview_chunk = Vec::new();
    for entry in read_dir.filter_map(Result::ok) {
        let Some(path) = browser_entry_path_from_dir_entry(&entry) else {
            continue;
        };
        let preview_entry = build_preview_entry(path.clone(), options.archive_as_container_in_sort);
        if !matches_filters(
            &preview_entry,
            &options.filter_text,
            &options.extension_filter,
        ) {
            continue;
        }
        collected.push(path);
        preview_chunk.push(preview_entry);
        flush_preview_chunk(on_preview_chunk, &mut preview_chunk);
    }
    if !preview_chunk.is_empty() {
        on_preview_chunk(preview_chunk);
    }
    collected
}

fn flush_preview_chunk(
    on_preview_chunk: &mut impl FnMut(Vec<BrowserEntry>),
    preview_chunk: &mut Vec<BrowserEntry>,
) {
    if preview_chunk.len() >= PREVIEW_CHUNK_SIZE {
        on_preview_chunk(std::mem::take(preview_chunk));
    }
}

fn build_browser_entry(path: PathBuf, archive_as_container_in_sort: bool) -> BrowserEntry {
    let metadata = fs::metadata(&path)
        .ok()
        .map(|metadata| BrowserMetadata {
            size: metadata.is_file().then_some(metadata.len()),
            modified: metadata.modified().ok(),
        })
        .unwrap_or_default();
    let is_container = is_browser_container(&path);
    let sort_as_container = sort_group_is_container(&path, archive_as_container_in_sort);
    let label = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(entry)".to_string());
    BrowserEntry {
        path,
        label,
        is_container,
        sort_as_container,
        metadata,
    }
}

fn build_preview_entry(path: PathBuf, archive_as_container_in_sort: bool) -> BrowserEntry {
    let is_container = is_browser_container(&path);
    let sort_as_container = sort_group_is_container(&path, archive_as_container_in_sort);
    let label = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(entry)".to_string());
    BrowserEntry {
        path,
        label,
        is_container,
        sort_as_container,
        metadata: BrowserMetadata::default(),
    }
}

fn sort_group_is_container(path: &Path, archive_as_container_in_sort: bool) -> bool {
    if path.is_dir() {
        return true;
    }
    if archive_as_container_in_sort {
        return is_browser_container(path);
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("wmltxt"))
        .unwrap_or(false)
}

fn matches_filters(entry: &BrowserEntry, filter_text: &str, extension_filter: &str) -> bool {
    let text_ok = if filter_text.trim().is_empty() {
        true
    } else {
        entry
            .label
            .to_ascii_lowercase()
            .contains(&filter_text.to_ascii_lowercase())
    };
    let ext_ok = if extension_filter.trim().is_empty() {
        true
    } else {
        entry
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case(extension_filter.trim().trim_start_matches('.')))
            .unwrap_or(false)
    };

    text_ok && ext_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_sort_orders_numeric_suffixes() {
        assert_eq!(
            compare_browser_name("テスト10.jpg", "テスト2.jpg", BrowserNameSortMode::Os),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn natural_sort_orders_parenthesized_numbers() {
        assert_eq!(
            compare_browser_name("テスト(5).jpg", "テスト(43).jpg", BrowserNameSortMode::Os),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn separate_dirs_places_containers_before_files() {
        let mut entries = vec![
            BrowserEntry {
                path: PathBuf::from("b.png"),
                label: "b.png".to_string(),
                is_container: false,
                sort_as_container: false,
                metadata: BrowserMetadata::default(),
            },
            BrowserEntry {
                path: PathBuf::from("a"),
                label: "a".to_string(),
                is_container: true,
                sort_as_container: true,
                metadata: BrowserMetadata::default(),
            },
        ];

        sort_browser_entries(
            &mut entries,
            BrowserSortField::Name,
            true,
            true,
            BrowserNameSortMode::Os,
        );

        assert!(entries[0].is_container);
        assert!(!entries[1].is_container);
    }

    #[test]
    fn descending_sort_reverses_container_names() {
        let mut entries = vec![
            BrowserEntry {
                path: PathBuf::from("a"),
                label: "a".to_string(),
                is_container: true,
                sort_as_container: true,
                metadata: BrowserMetadata::default(),
            },
            BrowserEntry {
                path: PathBuf::from("b"),
                label: "b".to_string(),
                is_container: true,
                sort_as_container: true,
                metadata: BrowserMetadata::default(),
            },
        ];

        sort_browser_entries(
            &mut entries,
            BrowserSortField::Name,
            false,
            true,
            BrowserNameSortMode::Os,
        );

        assert_eq!(entries[0].label, "b");
        assert_eq!(entries[1].label, "a");
    }
}
