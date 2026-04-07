use crate::dependent::ui_available_roots;
pub(crate) use crate::filesystem::{
    BrowserEntry as FilerEntry, BrowserNameSortMode as NameSortMode, BrowserScanOptions,
    BrowserSnapshotState as FilerSnapshotState, BrowserSortField as FilerSortField,
};
use crate::options::NavigationSortOption;
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

impl FilerState {
    pub(crate) fn browser_scan_options(
        &self,
        navigation_sort: NavigationSortOption,
    ) -> BrowserScanOptions {
        BrowserScanOptions {
            navigation_sort,
            sort_field: self.sort_field,
            ascending: self.ascending,
            separate_dirs: self.separate_dirs,
            archive_as_container_in_sort: self.archive_as_container_in_sort,
            filter_text: self.filter_text.clone(),
            extension_filter: self.extension_filter.clone(),
            name_sort_mode: self.name_sort_mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_scan_options_follow_filer_state() {
        let state = FilerState {
            sort_field: FilerSortField::Modified,
            ascending: false,
            separate_dirs: false,
            archive_as_container_in_sort: true,
            filter_text: "cover".to_string(),
            extension_filter: "png".to_string(),
            name_sort_mode: NameSortMode::CaseInsensitive,
            ..Default::default()
        };

        let options = state.browser_scan_options(NavigationSortOption::Date);

        assert_eq!(options.navigation_sort, NavigationSortOption::Date);
        assert_eq!(options.sort_field, FilerSortField::Modified);
        assert!(!options.ascending);
        assert!(!options.separate_dirs);
        assert!(options.archive_as_container_in_sort);
        assert_eq!(options.filter_text, "cover");
        assert_eq!(options.extension_filter, "png");
        assert_eq!(options.name_sort_mode, NameSortMode::CaseInsensitive);
    }
}
