use crate::filesystem::{
    compare_natural_str, is_browser_container, is_browser_entry, list_browser_entries,
};
use crate::options::NavigationSortOption;
use crate::ui::menu::fileviewer::state::{FilerEntry, FilerMetadata, FilerSortField, NameSortMode};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub(crate) enum FilerCommand {
    OpenDirectory {
        request_id: u64,
        dir: PathBuf,
        sort: NavigationSortOption,
        selected: Option<PathBuf>,
        sort_field: FilerSortField,
        ascending: bool,
        separate_dirs: bool,
        filter_text: String,
        extension_filter: String,
        name_sort_mode: NameSortMode,
    },
}

pub(crate) enum FilerResult {
    Reset {
        request_id: u64,
        directory: PathBuf,
        selected: Option<PathBuf>,
    },
    Append {
        request_id: u64,
        entries: Vec<FilerEntry>,
    },
    Snapshot {
        request_id: u64,
        directory: PathBuf,
        entries: Vec<FilerEntry>,
        selected: Option<PathBuf>,
    },
}

pub(crate) fn spawn_filer_worker() -> (Sender<FilerCommand>, Receiver<FilerResult>) {
    let (command_tx, command_rx) = mpsc::channel::<FilerCommand>();
    let (result_tx, result_rx) = mpsc::channel::<FilerResult>();

    thread::spawn(move || {
        while let Ok(command) = command_rx.recv() {
            match command {
                FilerCommand::OpenDirectory {
                    request_id,
                    dir,
                    sort,
                    selected,
                    sort_field,
                    ascending,
                    separate_dirs,
                    filter_text,
                    extension_filter,
                    name_sort_mode,
                } => {
                    let mut entries = if dir.is_dir() {
                        let _ = result_tx.send(FilerResult::Reset {
                            request_id,
                            directory: dir.clone(),
                            selected: selected.clone(),
                        });
                        let mut preview = Vec::new();
                        let mut collected = Vec::new();
                        let Ok(read_dir) = fs::read_dir(&dir) else {
                            continue;
                        };
                        for entry in read_dir.filter_map(Result::ok) {
                            let path = entry.path();
                            if !is_browser_entry(&path) {
                                continue;
                            }
                            let entry = build_filer_entry(path);
                            if !matches_filters(&entry, &filter_text, &extension_filter) {
                                continue;
                            }
                            preview.push(entry.clone());
                            collected.push(entry);
                            if preview.len() >= 128 {
                                let _ = result_tx.send(FilerResult::Append {
                                    request_id,
                                    entries: std::mem::take(&mut preview),
                                });
                            }
                        }
                        if !preview.is_empty() {
                            let _ = result_tx.send(FilerResult::Append {
                                request_id,
                                entries: preview,
                            });
                        }
                        collected
                    } else {
                        list_browser_entries(&dir, sort)
                            .into_iter()
                            .map(build_filer_entry)
                            .filter(|entry| matches_filters(entry, &filter_text, &extension_filter))
                            .collect::<Vec<_>>()
                    };
                    sort_entries(
                        &mut entries,
                        sort_field,
                        ascending,
                        separate_dirs,
                        name_sort_mode,
                    );
                    let _ = result_tx.send(FilerResult::Snapshot {
                        request_id,
                        directory: dir,
                        entries,
                        selected,
                    });
                }
            }
        }
    });

    (command_tx, result_rx)
}

fn build_filer_entry(path: PathBuf) -> FilerEntry {
    let metadata = fs::metadata(&path)
        .ok()
        .map(|metadata| FilerMetadata {
            size: metadata.is_file().then_some(metadata.len()),
            modified: metadata.modified().ok(),
        })
        .unwrap_or_default();
    let is_container = is_browser_container(&path);
    let label = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(entry)".to_string());
    FilerEntry {
        path,
        label,
        is_container,
        metadata,
    }
}

fn matches_filters(entry: &FilerEntry, filter_text: &str, extension_filter: &str) -> bool {
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

fn sort_entries(
    entries: &mut [FilerEntry],
    sort_field: FilerSortField,
    ascending: bool,
    separate_dirs: bool,
    name_sort_mode: NameSortMode,
) {
    entries.sort_by(|left, right| {
        if separate_dirs && left.is_container != right.is_container {
            return right.is_container.cmp(&left.is_container);
        }

        let order = match sort_field {
            FilerSortField::Name => compare_name(&left.label, &right.label, name_sort_mode),
            FilerSortField::Modified => left.metadata.modified.cmp(&right.metadata.modified),
            FilerSortField::Size => left.metadata.size.cmp(&right.metadata.size),
        };

        if ascending { order } else { order.reverse() }
    });
}

fn compare_name(left: &str, right: &str, mode: NameSortMode) -> std::cmp::Ordering {
    match mode {
        NameSortMode::Os => compare_natural_str(left, right, false),
        NameSortMode::CaseSensitive => compare_natural_str(left, right, true),
        NameSortMode::CaseInsensitive => compare_natural_str(left, right, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_sort_orders_numeric_suffixes() {
        assert_eq!(
            compare_name("テスト10.jpg", "テスト2.jpg", NameSortMode::Os),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn natural_sort_orders_parenthesized_numbers() {
        assert_eq!(
            compare_name("テスト(5).jpg", "テスト(43).jpg", NameSortMode::Os),
            std::cmp::Ordering::Less
        );
    }
}
