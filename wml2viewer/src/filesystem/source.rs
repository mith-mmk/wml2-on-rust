use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::dependent::download_http_url;

use super::path::{
    is_listed_file_path, is_zip_file_path, listed_virtual_identity_from_virtual_path,
    listed_virtual_root, resolve_start_path, resolve_virtual_listed_child, zip_virtual_root,
};
use super::zip_file::{
    load_zip_entries, load_zip_entry_bytes, zip_entry_record, zip_prefers_low_io,
};

const HTTP_TEMP_PREFIX: &str = "wml2viewer_url_";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum SourceKind {
    LocalPath,
    HttpTempFile,
    ListedFile,
    ListedVirtualChild,
    ZipArchive,
    ZipVirtualChild,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SourceId {
    pub kind: SourceKind,
    pub path: PathBuf,
    pub entry_index: Option<usize>,
    pub listed_identity: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SourceSignature {
    pub source: SourceId,
    pub exists: bool,
    pub is_dir: bool,
    pub len: Option<u64>,
    pub modified_nanos: Option<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OpenedImageSource {
    File {
        path: PathBuf,
        size_hint: Option<u64>,
    },
    Bytes {
        hint_path: PathBuf,
        bytes: Vec<u8>,
        size_hint: Option<u64>,
        prefers_low_io: bool,
    },
}

pub(crate) fn source_id_for_path(path: &Path) -> SourceId {
    if let Some((root, index)) = zip_virtual_child_source(path) {
        return SourceId {
            kind: SourceKind::ZipVirtualChild,
            path: root,
            entry_index: Some(index),
            listed_identity: None,
        };
    }
    if let Some((root, identity)) = listed_virtual_child_source(path) {
        return SourceId {
            kind: SourceKind::ListedVirtualChild,
            path: root,
            entry_index: None,
            listed_identity: identity,
        };
    }
    if is_zip_file_path(path) {
        return SourceId {
            kind: SourceKind::ZipArchive,
            path: path.to_path_buf(),
            entry_index: None,
            listed_identity: None,
        };
    }
    if is_listed_file_path(path) {
        return SourceId {
            kind: SourceKind::ListedFile,
            path: path.to_path_buf(),
            entry_index: None,
            listed_identity: None,
        };
    }
    SourceId {
        kind: if is_http_temp_file(path) {
            SourceKind::HttpTempFile
        } else {
            SourceKind::LocalPath
        },
        path: path.to_path_buf(),
        entry_index: None,
        listed_identity: None,
    }
}

pub(crate) fn source_signature_for_path(path: &Path) -> Option<SourceSignature> {
    let source = source_id_for_path(path);
    let metadata = fs::metadata(&source.path).ok()?;
    Some(SourceSignature {
        source,
        exists: true,
        is_dir: metadata.is_dir(),
        len: metadata.is_file().then_some(metadata.len()),
        modified_nanos: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos()),
    })
}

pub fn resolve_source_input_path(path: &Path) -> Option<PathBuf> {
    let url = source_url_from_input(path)?;
    if url.starts_with("http://") || url.starts_with("https://") {
        return download_http_url(&url);
    }
    Some(PathBuf::from(url))
}

fn source_url_from_input(path: &Path) -> Option<String> {
    let text = path.to_string_lossy().trim().to_string();
    (!text.is_empty()).then_some(text)
}

pub(crate) fn open_image_source(path: &Path) -> Option<OpenedImageSource> {
    let resolved = normalize_open_path(path)?;
    if let Some((archive, index)) = zip_virtual_child_source(&resolved) {
        let size_hint = zip_entry_record(&archive, index).map(|entry| entry.size);
        let bytes = load_zip_entry_bytes(&archive, index)?;
        return Some(OpenedImageSource::Bytes {
            hint_path: resolved,
            bytes,
            size_hint,
            prefers_low_io: zip_prefers_low_io(&archive),
        });
    }

    let metadata = fs::metadata(&resolved).ok()?;
    metadata.is_file().then_some(OpenedImageSource::File {
        path: resolved,
        size_hint: Some(metadata.len()),
    })
}

pub(crate) fn source_image_size(path: &Path) -> Option<u64> {
    let resolved = normalize_open_path(path)?;
    if let Some((archive, index)) = zip_virtual_child_source(&resolved) {
        return zip_entry_record(&archive, index).map(|entry| entry.size);
    }
    let metadata = fs::metadata(&resolved).ok()?;
    metadata.is_file().then_some(metadata.len())
}

pub(crate) fn source_entry_name(path: &Path) -> Option<String> {
    if let Some((archive, index)) = zip_virtual_child_source(path) {
        return load_zip_entries(&archive)
            .and_then(|entries| entries.into_iter().find(|entry| entry.index == index))
            .map(|entry| entry.name);
    }
    if let Some(target) = resolve_virtual_listed_child(path) {
        return source_entry_name(&target);
    }
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

pub(crate) fn source_metadata_path(path: &Path) -> Option<PathBuf> {
    if let Some((archive, _)) = zip_virtual_child_source(path) {
        return Some(archive);
    }
    if let Some(target) = resolve_virtual_listed_child(path) {
        return source_metadata_path(&target);
    }
    Some(path.to_path_buf())
}

pub(crate) fn source_prefers_low_io(path: &Path) -> bool {
    let Some(resolved) = normalize_open_path(path) else {
        return false;
    };
    if let Some((archive, _)) = zip_virtual_child_source(&resolved) {
        return zip_prefers_low_io(&archive);
    }
    is_zip_file_path(&resolved) && zip_prefers_low_io(&resolved)
}

fn normalize_open_path(path: &Path) -> Option<PathBuf> {
    if zip_virtual_child_source(path).is_some() {
        return Some(path.to_path_buf());
    }
    if let Some(target) = resolve_virtual_listed_child(path) {
        return normalize_open_path(&target);
    }
    if is_zip_file_path(path) || is_listed_file_path(path) || path.is_dir() {
        let next = resolve_start_path(path)?;
        if next == path {
            return Some(next);
        }
        return normalize_open_path(&next);
    }
    Some(path.to_path_buf())
}

fn zip_virtual_child_source(path: &Path) -> Option<(PathBuf, usize)> {
    let root = zip_virtual_root(path)?;
    let name = path.file_name()?.to_string_lossy();
    let index = name
        .split_once("__")
        .map(|(index, _)| index)
        .unwrap_or(name.as_ref())
        .parse::<usize>()
        .ok()?;
    Some((root, index))
}

fn listed_virtual_child_source(path: &Path) -> Option<(PathBuf, Option<u64>)> {
    let root = listed_virtual_root(path)?;
    let identity = listed_virtual_identity_from_virtual_path(path);
    Some((root, identity))
}

fn is_http_temp_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with(HTTP_TEMP_PREFIX))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn source_id_classifies_zip_virtual_children() {
        let path = PathBuf::from("archive.zip")
            .join("__zipv__")
            .join("00000003__page.png");

        let source = source_id_for_path(&path);

        assert_eq!(source.kind, SourceKind::ZipVirtualChild);
        assert_eq!(source.path, PathBuf::from("archive.zip"));
        assert_eq!(source.entry_index, Some(3));
    }

    #[test]
    fn source_id_classifies_http_temp_files() {
        let path = PathBuf::from("C:/temp/wml2viewer_url_12345.png");

        let source = source_id_for_path(&path);

        assert_eq!(source.kind, SourceKind::HttpTempFile);
    }

    #[test]
    fn resolve_source_input_path_keeps_local_paths() {
        let path = PathBuf::from("C:/images/sample.png");

        assert_eq!(resolve_source_input_path(&path), Some(path));
    }

    #[test]
    fn source_url_from_input_detects_http_urls() {
        let path = PathBuf::from("https://example.com/image.webp");

        assert_eq!(
            source_url_from_input(&path),
            Some("https://example.com/image.webp".to_string())
        );
    }

    fn make_temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wml2viewer-source-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn open_image_source_resolves_listed_virtual_child_to_real_file() {
        let dir = make_temp_dir();
        let listed = dir.join("pages.wmltxt");
        let page = dir.join("001.png");
        fs::write(&page, b"png").unwrap();
        fs::write(
            &listed,
            format!("#!WMLViewer2 ListedFile 1.0\n{}\n", page.display()),
        )
        .unwrap();

        let virtual_child = listed.join("__wmlv__").join(format!(
            "{:08}__{:016x}__{}",
            0usize,
            1u64,
            page.file_name().unwrap().to_string_lossy()
        ));

        let source = open_image_source(&virtual_child).unwrap();

        assert_eq!(
            source,
            OpenedImageSource::File {
                path: page.clone(),
                size_hint: Some(3),
            }
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn source_metadata_path_points_zip_children_to_archive() {
        let path = PathBuf::from("archive.zip")
            .join("__zipv__")
            .join("00000003__page.png");

        assert_eq!(
            source_metadata_path(&path),
            Some(PathBuf::from("archive.zip"))
        );
    }
}
