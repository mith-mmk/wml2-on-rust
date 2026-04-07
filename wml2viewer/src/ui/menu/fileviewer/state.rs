use crate::dependent::ui_available_roots;
pub(crate) use crate::filesystem::{
    BrowserEntry as FilerEntry, BrowserNameSortMode as NameSortMode,
    BrowserSnapshotState as FilerSnapshotState, BrowserSortField as FilerSortField,
};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum FilerViewMode {
    #[default]
    List,
    ThumbnailSmall,
    ThumbnailMedium,
    ThumbnailLarge,
    Detail,
}

#[derive(Debug)]
pub(crate) struct FilerState {
    pub(crate) snapshot: FilerSnapshotState,
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) view_mode: FilerViewMode,
    pub(crate) sort_field: FilerSortField,
    pub(crate) ascending: bool,
    pub(crate) separate_dirs: bool,
    pub(crate) archive_as_container_in_sort: bool,
    pub(crate) filter_text: String,
    pub(crate) extension_filter: String,
    pub(crate) name_sort_mode: NameSortMode,
    pub(crate) url_input: String,
    pub(crate) thumbnail_scale: f32,
}

impl Default for FilerState {
    fn default() -> Self {
        Self {
            snapshot: FilerSnapshotState::default(),
            roots: ui_available_roots(),
            view_mode: FilerViewMode::List,
            sort_field: FilerSortField::Name,
            ascending: true,
            separate_dirs: true,
            archive_as_container_in_sort: false,
            filter_text: String::new(),
            extension_filter: String::new(),
            name_sort_mode: NameSortMode::Os,
            url_input: String::new(),
            thumbnail_scale: 1.0,
        }
    }
}
