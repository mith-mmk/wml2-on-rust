use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::options::{EndOfFolderOption, NavigationSortOption};

use super::browser::{SharedBrowserWorkerState, preload_browser_directory_for_path};
use super::cache::{FilesystemCache, SharedFilesystemCache};
use super::navigator::{
    FileNavigator, NavigationOutcome, NavigationTarget, PendingDirection, resolve_navigation_path,
};

#[derive(Clone)]
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
    NavigatorReady {
        request_id: u64,
        navigation_path: Option<PathBuf>,
        load_path: Option<PathBuf>,
    },
    CurrentSet,
    PathResolved {
        request_id: u64,
        navigation_path: PathBuf,
        load_path: PathBuf,
    },
    NoPath {
        request_id: u64,
    },
}

pub(crate) fn spawn_filesystem_worker(
    sort: NavigationSortOption,
    shared_cache: SharedFilesystemCache,
    shared_browser_state: SharedBrowserWorkerState,
) -> (Sender<FilesystemCommand>, Receiver<FilesystemResult>) {
    let (command_tx, command_rx) = mpsc::channel::<FilesystemCommand>();
    let (result_tx, result_rx) = mpsc::channel::<FilesystemResult>();

    thread::spawn(move || {
        let mut navigator: Option<FileNavigator> = None;

        while let Ok(command) = command_rx.recv() {
            let Ok(mut cache) = shared_cache.lock() else {
                break;
            };
            cache.ensure_sort(sort);
            match command {
                FilesystemCommand::Init { request_id, path } => {
                    let Some(start_path) = resolve_navigation_path(&path, &mut cache) else {
                        let _ = result_tx.send(FilesystemResult::NoPath { request_id });
                        continue;
                    };

                    navigator = Some(FileNavigator::from_current_path(start_path, &mut cache));
                    if let Some(nav) = navigator.as_ref() {
                        preload_browser_directory_for_path(
                            &shared_browser_state,
                            nav.current(),
                            sort,
                            &mut cache,
                        );
                    }
                    let initial_target = navigator
                        .as_ref()
                        .and_then(|nav| navigation_outcome_to_target(nav.current_target()));
                    let _ = result_tx.send(FilesystemResult::NavigatorReady {
                        request_id,
                        navigation_path: initial_target
                            .as_ref()
                            .map(|target| target.navigation_path.clone()),
                        load_path: initial_target.map(|target| target.load_path),
                    });
                }
                FilesystemCommand::SetCurrent { request_id, path } => {
                    if let Some(nav) = navigator.as_mut() {
                        nav.set_current_input(path, &mut cache);
                        preload_browser_directory_for_path(
                            &shared_browser_state,
                            nav.current(),
                            sort,
                            &mut cache,
                        );
                    } else if let Some(start_path) = resolve_navigation_path(&path, &mut cache) {
                        navigator = Some(FileNavigator::from_current_path(start_path, &mut cache));
                        if let Some(nav) = navigator.as_ref() {
                            preload_browser_directory_for_path(
                                &shared_browser_state,
                                nav.current(),
                                sort,
                                &mut cache,
                            );
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
                    let outcome = navigator
                        .as_mut()
                        .and_then(|nav| nav.first(&mut cache).map(|_| nav.current_target()))
                        .unwrap_or(NavigationOutcome::NoPath);
                    let _ = send_nav_result(
                        &result_tx,
                        request_id,
                        navigation_outcome_to_target(outcome),
                    );
                }
                FilesystemCommand::Last { request_id } => {
                    let outcome = navigator
                        .as_mut()
                        .and_then(|nav| nav.last(&mut cache).map(|_| nav.current_target()))
                        .unwrap_or(NavigationOutcome::NoPath);
                    let _ = send_nav_result(
                        &result_tx,
                        request_id,
                        navigation_outcome_to_target(outcome),
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
    target: Option<NavigationTarget>,
) -> Result<(), mpsc::SendError<FilesystemResult>> {
    match target {
        Some(target) => tx.send(FilesystemResult::PathResolved {
            request_id,
            navigation_path: target.navigation_path,
            load_path: target.load_path,
        }),
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

    let _ = send_nav_result(tx, request_id, navigation_outcome_to_target(outcome));
}

fn navigation_outcome_to_target(outcome: NavigationOutcome) -> Option<NavigationTarget> {
    match outcome {
        NavigationOutcome::Resolved(target) => Some(target),
        NavigationOutcome::NoPath => None,
    }
}
