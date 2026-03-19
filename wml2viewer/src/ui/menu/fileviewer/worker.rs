use crate::filesystem::list_browser_entries;
use crate::options::NavigationSortOption;
use crate::ui::menu::fileviewer::state::{FilerEntry, FilerMetadata};
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
    },
}

pub(crate) enum FilerResult {
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
                } => {
                    let entries = list_browser_entries(&dir, sort)
                        .into_iter()
                        .map(|path| {
                            let metadata = fs::metadata(&path)
                                .ok()
                                .map(|metadata| FilerMetadata {
                                    size: metadata.is_file().then_some(metadata.len()),
                                    modified: metadata.modified().ok(),
                                })
                                .unwrap_or_default();
                            let is_dir = path.is_dir();
                            let label = path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "(entry)".to_string());
                            FilerEntry {
                                path,
                                label,
                                is_dir,
                                metadata,
                            }
                        })
                        .collect::<Vec<_>>();
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
