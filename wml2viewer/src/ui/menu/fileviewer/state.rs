use crate::dependent::available_roots;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Clone, Debug, Default)]
pub(crate) struct FilerMetadata {
    pub(crate) size: Option<u64>,
    pub(crate) modified: Option<SystemTime>,
}

#[derive(Clone, Debug)]
pub(crate) struct FilerEntry {
    pub(crate) path: PathBuf,
    pub(crate) label: String,
    pub(crate) is_dir: bool,
    pub(crate) metadata: FilerMetadata,
}

#[derive(Debug)]
pub(crate) struct FilerState {
    pub(crate) entries: Vec<FilerEntry>,
    pub(crate) navigation_entries: Vec<PathBuf>,
    pub(crate) directory: Option<PathBuf>,
    pub(crate) selected: Option<PathBuf>,
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) pending_request_id: Option<u64>,
}

impl Default for FilerState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            navigation_entries: Vec::new(),
            directory: None,
            selected: None,
            roots: available_roots(),
            pending_request_id: None,
        }
    }
}
