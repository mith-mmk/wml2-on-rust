use crate::options::{ArchiveBrowseOption, NavigationSortOption};
use serde::{Deserialize, Serialize};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

use super::protocol::{FilesystemCommand, FilesystemResult};
use super::{
    FilesystemCache, SharedFilesystemCache, compare_natural_str, compare_os_str,
    is_browser_container, listed_virtual_root, resolve_navigation_entry_path, zip_virtual_root,
};

const PREVIEW_CHUNK_SIZE: usize = 64;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    pub archive_mode: ArchiveBrowseOption,
    pub sort_field: BrowserSortField,
    pub ascending: bool,
    pub separate_dirs: bool,
    pub archive_as_container_in_sort: bool,
    pub filter_text: String,
    pub extension_filter: String,
    pub name_sort_mode: BrowserNameSortMode,
    pub thumbnail_hint_count: usize,
    pub thumbnail_hint_max_side: u32,
}

impl Default for BrowserScanOptions {
    fn default() -> Self {
        Self {
            navigation_sort: NavigationSortOption::OsName,
            archive_mode: ArchiveBrowseOption::Folder,
            sort_field: BrowserSortField::Name,
            ascending: true,
            separate_dirs: true,
            archive_as_container_in_sort: false,
            filter_text: String::new(),
            extension_filter: String::new(),
            name_sort_mode: BrowserNameSortMode::Os,
            thumbnail_hint_count: 0,
            thumbnail_hint_max_side: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct CachedBrowserEntry {
    path: PathBuf,
    metadata: BrowserMetadata,
}

#[derive(Clone, Debug, Default)]
pub struct BrowserSnapshotState {
    pub entries: Vec<BrowserEntry>,
    pub directory: Option<PathBuf>,
    pub selected: Option<PathBuf>,
    pub pending_request_id: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct BrowserWorkerState {
    options: BrowserScanOptions,
    cached_directory: Option<PathBuf>,
    cached_navigation_sort: Option<NavigationSortOption>,
    cached_archive_mode: Option<ArchiveBrowseOption>,
    cached_entries: Vec<CachedBrowserEntry>,
}

pub(crate) type SharedBrowserWorkerState = Arc<Mutex<BrowserWorkerState>>;

impl BrowserSnapshotState {
    pub fn begin_request(&mut self, request_id: u64) {
        self.pending_request_id = Some(request_id);
    }

    pub fn clear_pending_request(&mut self) {
        self.pending_request_id = None;
    }

    pub fn sync_with_navigation(
        &mut self,
        current_navigation_path: &Path,
        pending_navigation_path: Option<&Path>,
        current_load_path: Option<&Path>,
    ) -> Option<(PathBuf, Option<PathBuf>)> {
        let directory = pending_navigation_path
            .and_then(|path| browser_directory_for_path(path, current_load_path))
            .or_else(|| browser_directory_for_path(current_navigation_path, current_load_path))?;
        let selected = browser_selected_path_for_directory(
            &directory,
            current_navigation_path,
            pending_navigation_path,
            current_load_path,
            self.selected.clone(),
        );

        if self.directory.as_ref() == Some(&directory) {
            self.selected = selected.clone();
            if self.entries.is_empty() && self.pending_request_id.is_none() {
                return Some((directory, selected));
            }
            return None;
        }

        Some((directory, selected))
    }

    pub fn apply_query_result(
        &mut self,
        result: FilesystemResult,
        current_navigation_path: &Path,
        pending_navigation_path: Option<&Path>,
        current_load_path: Option<&Path>,
    ) -> bool {
        match result {
            FilesystemResult::BrowserReset {
                request_id,
                directory,
                selected,
            } => {
                if self.pending_request_id != Some(request_id) {
                    return false;
                }
                self.directory = Some(directory);
                self.entries.clear();
                self.selected = browser_selected_path_for_directory(
                    self.directory.as_deref().unwrap(),
                    current_navigation_path,
                    pending_navigation_path,
                    current_load_path,
                    selected,
                );
                true
            }
            FilesystemResult::BrowserAppend {
                request_id,
                entries,
            } => {
                if self.pending_request_id != Some(request_id) {
                    return false;
                }
                self.entries.extend(entries);
                true
            }
            FilesystemResult::ThumbnailHint { .. } => false,
            FilesystemResult::BrowserFinish {
                request_id,
                directory,
                entries,
                selected,
            } => {
                if self.pending_request_id != Some(request_id) {
                    return false;
                }
                self.pending_request_id = None;
                self.directory = Some(directory);
                self.entries = entries;
                self.selected = browser_selected_path_for_directory(
                    self.directory.as_deref().unwrap(),
                    current_navigation_path,
                    pending_navigation_path,
                    current_load_path,
                    selected,
                );
                true
            }
            FilesystemResult::BrowserFailed { request_id } => {
                if self.pending_request_id != Some(request_id) {
                    return false;
                }
                self.pending_request_id = None;
                true
            }
            FilesystemResult::InputPathResolved { .. }
            | FilesystemResult::InputPathFailed { .. } => false,
            _ => false,
        }
    }
}

pub fn browser_directory_for_path(
    path: &Path,
    current_load_path: Option<&Path>,
) -> Option<PathBuf> {
    if path.is_dir() {
        return Some(path.to_path_buf());
    }

    if let Some(root) = listed_virtual_root(path) {
        return Some(root);
    }

    if let Some(root) = zip_virtual_root(path) {
        return Some(root);
    }

    path.parent()
        .map(Path::to_path_buf)
        .or_else(|| current_load_path.and_then(|current| current.parent().map(Path::to_path_buf)))
}

pub fn browser_selected_path_for_directory(
    directory: &Path,
    current_navigation_path: &Path,
    pending_navigation_path: Option<&Path>,
    current_load_path: Option<&Path>,
    fallback: Option<PathBuf>,
) -> Option<PathBuf> {
    for candidate in [pending_navigation_path, Some(current_navigation_path)] {
        let Some(candidate) = candidate else {
            continue;
        };
        if browser_directory_for_path(candidate, current_load_path).as_deref() == Some(directory) {
            if listed_virtual_root(candidate).is_some() || zip_virtual_root(candidate).is_some() {
                return resolve_navigation_entry_path(candidate)
                    .or_else(|| Some(candidate.to_path_buf()));
            }
            return Some(candidate.to_path_buf());
        }
    }
    fallback
}

pub(crate) fn new_shared_browser_worker_state() -> SharedBrowserWorkerState {
    Arc::new(Mutex::new(BrowserWorkerState::default()))
}

pub(crate) fn preload_browser_directory_for_path(
    state: &SharedBrowserWorkerState,
    path: &Path,
    navigation_sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
    cache: &mut FilesystemCache,
) {
    let Some(directory) = browser_directory_for_path(path, None) else {
        return;
    };
    let entry_paths = load_browser_entries(
        &directory,
        &BrowserScanOptions {
            navigation_sort,
            archive_mode,
            ..BrowserScanOptions::default()
        },
        cache,
    );
    if let Ok(mut state) = state.lock() {
        state.cached_directory = Some(directory);
        state.cached_navigation_sort = Some(navigation_sort);
        state.cached_archive_mode = Some(archive_mode);
        state.options.archive_mode = archive_mode;
        state.cached_entries = entry_paths;
    }
}

pub(crate) fn spawn_browser_query_worker(
    shared_cache: SharedFilesystemCache,
    shared_state: SharedBrowserWorkerState,
) -> (Sender<FilesystemCommand>, Receiver<FilesystemResult>) {
    let (command_tx, command_rx) = mpsc::channel::<FilesystemCommand>();
    let (result_tx, result_rx) = mpsc::channel::<FilesystemResult>();

    thread::spawn(move || {
        while let Ok(command) = command_rx.recv() {
            let mut latest = command;
            while let Ok(next) = command_rx.try_recv() {
                latest = next;
            }
            match latest {
                FilesystemCommand::OpenBrowserDirectory {
                    request_id,
                    dir,
                    selected,
                    options,
                } => {
                    let options = match resolve_scan_options(&shared_state, options) {
                        Some(options) => options,
                        None => {
                            let _ = result_tx.send(FilesystemResult::BrowserFailed { request_id });
                            continue;
                        }
                    };
                    if let Some(entries) = load_cached_browser_entries(
                        &shared_state,
                        &dir,
                        options.navigation_sort,
                        options.archive_mode,
                    ) {
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            scan_query_request(
                                &result_tx,
                                request_id,
                                dir.clone(),
                                selected.clone(),
                                options.clone(),
                                entries,
                            )
                        }));
                        let _ = result_tx.send(match result {
                            Ok(entries) => {
                                send_thumbnail_hint(&result_tx, request_id, &entries, &options);
                                FilesystemResult::BrowserFinish {
                                    request_id,
                                    directory: dir,
                                    entries,
                                    selected,
                                }
                            }
                            Err(_) => FilesystemResult::BrowserFailed { request_id },
                        });
                        continue;
                    }
                    let cached_entries = {
                        let Ok(mut cache) = shared_cache.lock() else {
                            break;
                        };
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            load_browser_entries(&dir, &options, &mut cache)
                        }));
                        match result {
                            Ok(entries) => entries,
                            Err(_) => {
                                let _ =
                                    result_tx.send(FilesystemResult::BrowserFailed { request_id });
                                continue;
                            }
                        }
                    };
                    store_cached_browser_entries(
                        &shared_state,
                        &dir,
                        options.navigation_sort,
                        options.archive_mode,
                        cached_entries.clone(),
                    );
                    let options_for_scan = options.clone();
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        scan_query_request(
                            &result_tx,
                            request_id,
                            dir.clone(),
                            selected.clone(),
                            options_for_scan,
                            cached_entries,
                        )
                    }));
                    match result {
                        Ok(entries) => {
                            send_thumbnail_hint(&result_tx, request_id, &entries, &options);
                            let _ = result_tx.send(FilesystemResult::BrowserFinish {
                                request_id,
                                directory: dir,
                                entries,
                                selected,
                            });
                        }
                        Err(_) => {
                            let _ = result_tx.send(FilesystemResult::BrowserFailed { request_id });
                        }
                    }
                }
                FilesystemCommand::ResolveSourceInput { .. } => {}
                _ => {}
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
    let mut cache = FilesystemCache::new(options.navigation_sort, options.archive_mode);
    scan_browser_directory_with_preview_cached(dir, options, &mut cache, &mut on_preview_chunk)
}

pub fn scan_browser_directory_with_preview_cached(
    dir: &Path,
    options: &BrowserScanOptions,
    cache: &mut FilesystemCache,
    mut on_preview_chunk: impl FnMut(Vec<BrowserEntry>),
) -> Vec<BrowserEntry> {
    let entries = load_browser_entries(dir, options, cache);
    finalize_browser_entries(entries, options, &mut on_preview_chunk)
}

fn scan_query_request(
    result_tx: &Sender<FilesystemResult>,
    request_id: u64,
    dir: PathBuf,
    selected: Option<PathBuf>,
    options: BrowserScanOptions,
    cached_entries: Vec<CachedBrowserEntry>,
) -> Vec<BrowserEntry> {
    let _ = result_tx.send(FilesystemResult::BrowserReset {
        request_id,
        directory: dir.clone(),
        selected: selected.clone(),
    });

    finalize_browser_entries(cached_entries, &options, &mut |entries| {
        let _ = result_tx.send(FilesystemResult::BrowserAppend {
            request_id,
            entries,
        });
    })
}

fn finalize_browser_entries(
    cached_entries: Vec<CachedBrowserEntry>,
    options: &BrowserScanOptions,
    on_preview_chunk: &mut impl FnMut(Vec<BrowserEntry>),
) -> Vec<BrowserEntry> {
    let collected = collect_browser_entries(cached_entries, options, on_preview_chunk);
    let mut entries = collected
        .into_iter()
        .map(|entry| {
            build_browser_entry(
                entry.path,
                entry.metadata,
                options.archive_mode,
                options.archive_as_container_in_sort,
            )
        })
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

fn load_browser_entries(
    dir: &Path,
    options: &BrowserScanOptions,
    cache: &mut FilesystemCache,
) -> Vec<CachedBrowserEntry> {
    cache.ensure_settings(options.navigation_sort, options.archive_mode);
    let entry_paths = cache.browser_entries(dir);
    let metadata_by_path = cache.browser_metadata_batch(&entry_paths);
    entry_paths
        .into_iter()
        .map(|path| CachedBrowserEntry {
            metadata: metadata_by_path.get(&path).cloned().unwrap_or_default(),
            path,
        })
        .collect()
}

fn resolve_scan_options(
    shared_state: &SharedBrowserWorkerState,
    options: Option<BrowserScanOptions>,
) -> Option<BrowserScanOptions> {
    let Ok(mut state) = shared_state.lock() else {
        return options;
    };
    if let Some(options) = options {
        state.options = options.clone();
        return Some(options);
    }
    Some(state.options.clone())
}

fn load_cached_browser_entries(
    shared_state: &SharedBrowserWorkerState,
    dir: &Path,
    navigation_sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
) -> Option<Vec<CachedBrowserEntry>> {
    let Ok(state) = shared_state.lock() else {
        return None;
    };
    if state.cached_directory.as_deref() != Some(dir)
        || state.cached_navigation_sort != Some(navigation_sort)
        || state.cached_archive_mode != Some(archive_mode)
    {
        return None;
    }
    Some(state.cached_entries.clone())
}

fn store_cached_browser_entries(
    shared_state: &SharedBrowserWorkerState,
    dir: &Path,
    navigation_sort: NavigationSortOption,
    archive_mode: ArchiveBrowseOption,
    entries: Vec<CachedBrowserEntry>,
) {
    let Ok(mut state) = shared_state.lock() else {
        return;
    };
    state.cached_directory = Some(dir.to_path_buf());
    state.cached_navigation_sort = Some(navigation_sort);
    state.cached_archive_mode = Some(archive_mode);
    state.options.archive_mode = archive_mode;
    state.cached_entries = entries;
}

fn send_thumbnail_hint(
    result_tx: &Sender<FilesystemResult>,
    request_id: u64,
    entries: &[BrowserEntry],
    options: &BrowserScanOptions,
) {
    if options.thumbnail_hint_count == 0 || options.thumbnail_hint_max_side == 0 {
        return;
    }
    let paths = entries
        .iter()
        .filter(|entry| !entry.is_container)
        .take(options.thumbnail_hint_count)
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return;
    }
    let _ = result_tx.send(FilesystemResult::ThumbnailHint {
        request_id,
        paths,
        max_side: options.thumbnail_hint_max_side,
    });
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

fn collect_browser_entries(
    cached_entries: Vec<CachedBrowserEntry>,
    options: &BrowserScanOptions,
    on_preview_chunk: &mut impl FnMut(Vec<BrowserEntry>),
) -> Vec<CachedBrowserEntry> {
    let mut collected = Vec::new();
    let mut preview_chunk = Vec::new();
    for entry in cached_entries {
        let preview_entry = build_preview_entry(
            entry.path.clone(),
            options.archive_mode,
            options.archive_as_container_in_sort,
        );
        if !matches_filters(
            &preview_entry,
            &options.filter_text,
            &options.extension_filter,
        ) {
            continue;
        }
        collected.push(entry);
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

fn build_browser_entry(
    path: PathBuf,
    metadata: BrowserMetadata,
    archive_mode: ArchiveBrowseOption,
    archive_as_container_in_sort: bool,
) -> BrowserEntry {
    let is_container = browser_entry_is_container(&path, archive_mode);
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

fn build_preview_entry(
    path: PathBuf,
    archive_mode: ArchiveBrowseOption,
    archive_as_container_in_sort: bool,
) -> BrowserEntry {
    let is_container = browser_entry_is_container(&path, archive_mode);
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

fn browser_entry_is_container(path: &Path, archive_mode: ArchiveBrowseOption) -> bool {
    if matches!(archive_mode, ArchiveBrowseOption::Archiver)
        && (listed_virtual_root(path).is_none() && zip_virtual_root(path).is_none())
        && !path.is_dir()
        && is_browser_container(path)
    {
        return false;
    }
    is_browser_container(path)
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

    #[test]
    fn selected_path_prefers_pending_navigation_in_same_directory() {
        let dir = std::env::temp_dir().join("wml2viewer_browser_selection");
        let current = dir.join("001.png");
        let pending = dir.join("002.png");

        let selected =
            browser_selected_path_for_directory(&dir, &current, Some(&pending), None, None);

        assert_eq!(selected, Some(pending));
    }

    #[test]
    fn selected_path_keeps_archive_entry_in_parent_directory() {
        let dir = std::env::temp_dir().join("wml2viewer_browser_archive_parent");
        let archive = dir.join("sample.zip");

        let selected = browser_selected_path_for_directory(&dir, &archive, None, None, None);

        assert_eq!(selected, Some(archive));
    }

    #[test]
    fn snapshot_sync_requests_pending_directory_change() {
        let dir1 = std::env::temp_dir().join("wml2viewer_browser_sync_1");
        let dir2 = std::env::temp_dir().join("wml2viewer_browser_sync_2");
        let current = dir1.join("001.png");
        let pending = dir2.join("002.png");
        let mut snapshot = BrowserSnapshotState {
            directory: Some(dir1.clone()),
            selected: Some(current.clone()),
            ..Default::default()
        };

        let sync = snapshot.sync_with_navigation(&current, Some(&pending), None);

        assert_eq!(sync, Some((dir2, Some(pending))));
    }

    #[test]
    fn snapshot_reset_uses_pending_selection_when_request_matches() {
        let dir = std::env::temp_dir().join("wml2viewer_browser_reset");
        let current = dir.join("001.png");
        let pending = dir.join("002.png");
        let mut snapshot = BrowserSnapshotState::default();
        snapshot.begin_request(7);

        let applied = snapshot.apply_query_result(
            FilesystemResult::BrowserReset {
                request_id: 7,
                directory: dir.clone(),
                selected: Some(current),
            },
            Path::new("ignored"),
            Some(&pending),
            None,
        );

        assert!(applied);
        assert_eq!(snapshot.directory, Some(dir));
        assert_eq!(snapshot.selected, Some(pending));
    }

    #[test]
    fn finish_keeps_incremental_entries_and_clears_pending() {
        let dir = std::env::temp_dir().join("wml2viewer_browser_finish");
        let entry = BrowserEntry {
            path: dir.join("001.png"),
            label: "001.png".to_string(),
            is_container: false,
            sort_as_container: false,
            metadata: BrowserMetadata::default(),
        };
        let mut snapshot = BrowserSnapshotState {
            entries: vec![entry.clone()],
            pending_request_id: Some(9),
            ..Default::default()
        };

        let applied = snapshot.apply_query_result(
            FilesystemResult::BrowserFinish {
                request_id: 9,
                directory: dir.clone(),
                entries: vec![entry.clone()],
                selected: Some(entry.path.clone()),
            },
            &entry.path,
            None,
            None,
        );

        assert!(applied);
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.directory, Some(dir));
        assert_eq!(snapshot.selected, Some(entry.path));
        assert_eq!(snapshot.pending_request_id, None);
    }

    #[test]
    fn failed_clears_pending_without_discarding_incremental_entries() {
        let dir = std::env::temp_dir().join("wml2viewer_browser_failed");
        let mut snapshot = BrowserSnapshotState {
            entries: vec![BrowserEntry {
                path: dir.join("001.png"),
                label: "001.png".to_string(),
                is_container: false,
                sort_as_container: false,
                metadata: BrowserMetadata::default(),
            }],
            pending_request_id: Some(10),
            ..Default::default()
        };

        let applied = snapshot.apply_query_result(
            FilesystemResult::BrowserFailed { request_id: 10 },
            dir.as_path(),
            None,
            None,
        );

        assert!(applied);
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.pending_request_id, None);
    }

    #[test]
    fn scan_options_are_persisted_in_shared_worker_state() {
        let shared = new_shared_browser_worker_state();
        let options = BrowserScanOptions {
            filter_text: "cover".to_string(),
            thumbnail_hint_count: 8,
            ..BrowserScanOptions::default()
        };

        let first = resolve_scan_options(&shared, Some(options.clone())).unwrap();
        let second = resolve_scan_options(&shared, None).unwrap();

        assert_eq!(first.filter_text, "cover");
        assert_eq!(second.filter_text, "cover");
        assert_eq!(second.thumbnail_hint_count, 8);
    }

    #[test]
    fn thumbnail_hint_targets_first_non_container_entries() {
        let (tx, rx) = mpsc::channel();
        let options = BrowserScanOptions {
            thumbnail_hint_count: 2,
            thumbnail_hint_max_side: 96,
            ..BrowserScanOptions::default()
        };
        let entries = vec![
            BrowserEntry {
                path: PathBuf::from("folder"),
                label: "folder".to_string(),
                is_container: true,
                sort_as_container: true,
                metadata: BrowserMetadata::default(),
            },
            BrowserEntry {
                path: PathBuf::from("001.png"),
                label: "001.png".to_string(),
                is_container: false,
                sort_as_container: false,
                metadata: BrowserMetadata::default(),
            },
            BrowserEntry {
                path: PathBuf::from("002.png"),
                label: "002.png".to_string(),
                is_container: false,
                sort_as_container: false,
                metadata: BrowserMetadata::default(),
            },
            BrowserEntry {
                path: PathBuf::from("003.png"),
                label: "003.png".to_string(),
                is_container: false,
                sort_as_container: false,
                metadata: BrowserMetadata::default(),
            },
        ];

        send_thumbnail_hint(&tx, 3, &entries, &options);

        match rx.try_recv().unwrap() {
            FilesystemResult::ThumbnailHint {
                request_id,
                paths,
                max_side,
            } => {
                assert_eq!(request_id, 3);
                assert_eq!(max_side, 96);
                assert_eq!(
                    paths,
                    vec![PathBuf::from("001.png"), PathBuf::from("002.png")]
                );
            }
            other => panic!("unexpected result: {:?}", std::mem::discriminant(&other)),
        }
    }
}
