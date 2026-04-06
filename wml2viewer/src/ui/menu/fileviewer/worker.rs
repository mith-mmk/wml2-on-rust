use crate::filesystem::{BrowserScanOptions, scan_browser_directory_with_preview};
use crate::options::NavigationSortOption;
use crate::ui::menu::fileviewer::state::{FilerEntry, FilerMetadata, FilerSortField, NameSortMode};
use std::panic::{AssertUnwindSafe, catch_unwind};
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
        archive_as_container_in_sort: bool,
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
            let mut latest = command;
            while let Ok(next) = command_rx.try_recv() {
                latest = next;
            }
            match latest {
                FilerCommand::OpenDirectory {
                    request_id,
                    dir,
                    sort,
                    selected,
                    sort_field,
                    ascending,
                    separate_dirs,
                    archive_as_container_in_sort,
                    filter_text,
                    extension_filter,
                    name_sort_mode,
                } => {
                    let result_tx = result_tx.clone();
                    thread::spawn(move || {
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            scan_directory_request(
                                &result_tx,
                                request_id,
                                dir.clone(),
                                sort,
                                selected.clone(),
                                sort_field,
                                ascending,
                                separate_dirs,
                                archive_as_container_in_sort,
                                filter_text,
                                extension_filter,
                                name_sort_mode,
                            )
                        }));
                        let entries = match result {
                            Ok(entries) => entries,
                            Err(_) => Vec::new(),
                        };
                        let _ = result_tx.send(FilerResult::Snapshot {
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

fn scan_directory_request(
    result_tx: &Sender<FilerResult>,
    request_id: u64,
    dir: PathBuf,
    sort: NavigationSortOption,
    selected: Option<PathBuf>,
    sort_field: FilerSortField,
    ascending: bool,
    separate_dirs: bool,
    archive_as_container_in_sort: bool,
    filter_text: String,
    extension_filter: String,
    name_sort_mode: NameSortMode,
) -> Vec<FilerEntry> {
    let _ = result_tx.send(FilerResult::Reset {
        request_id,
        directory: dir.clone(),
        selected: selected.clone(),
    });

    let options = BrowserScanOptions {
        navigation_sort: sort,
        sort_field,
        ascending,
        separate_dirs,
        archive_as_container_in_sort,
        filter_text,
        extension_filter,
        name_sort_mode,
    };

    scan_browser_directory_with_preview(&dir, &options, |entries| {
        let _ = result_tx.send(FilerResult::Append {
            request_id,
            entries,
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::{
        BrowserEntry, BrowserNameSortMode, compare_browser_name, sort_browser_entries,
    };

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
                metadata: FilerMetadata::default(),
            },
            BrowserEntry {
                path: PathBuf::from("a"),
                label: "a".to_string(),
                is_container: true,
                sort_as_container: true,
                metadata: FilerMetadata::default(),
            },
        ];

        sort_browser_entries(
            &mut entries,
            FilerSortField::Name,
            true,
            true,
            NameSortMode::Os,
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
                metadata: FilerMetadata::default(),
            },
            BrowserEntry {
                path: PathBuf::from("b"),
                label: "b".to_string(),
                is_container: true,
                sort_as_container: true,
                metadata: FilerMetadata::default(),
            },
        ];

        sort_browser_entries(
            &mut entries,
            FilerSortField::Name,
            false,
            true,
            NameSortMode::Os,
        );

        assert_eq!(entries[0].label, "b");
        assert_eq!(entries[1].label, "a");
    }
}
