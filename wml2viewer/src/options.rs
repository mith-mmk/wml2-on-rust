/*!
! prelude options
*/

use std::collections::HashMap;

pub use crate::configs::resourses::{FontSizePreset, ResourceOptions};
pub use crate::dependent::plugins::PluginConfig;
pub use crate::ui::viewer::options::{
    BackgroundStyle, RenderOptions, ViewerOptions, WindowOptions, WindowSize, WindowStartPosition,
    ZoomOption,
};

#[derive(Clone, Default)]
pub struct AppConfig {
    pub viewer: ViewerOptions,
    pub window: WindowOptions,
    pub render: RenderOptions,
    pub resources: ResourceOptions,
    pub plugins: PluginConfig,
    pub input: InputOptions,
    pub navigation: NavigationOptions,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ViewerAction {
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomToggle,
    ToggleFullscreen,
    Reload,
    NextImage,
    PrevImage,
    FirstImage,
    LastImage,
    ToggleAnimation,
    ToggleGrayscale,
    ToggleMangaMode,
    ToggleSettings,
    ToggleFiler,
    SaveAs,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key: String,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyBinding {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }
}

#[derive(Clone, Default)]
pub struct InputOptions {
    pub key_mapping: HashMap<KeyBinding, ViewerAction>,
}

impl InputOptions {
    pub fn merged_with_defaults(&self) -> HashMap<KeyBinding, ViewerAction> {
        let mut map = default_key_mapping();
        for (binding, action) in &self.key_mapping {
            map.insert(binding.clone(), action.clone());
        }
        map
    }
}

fn default_key_mapping() -> HashMap<KeyBinding, ViewerAction> {
    let mut map = HashMap::new();
    map.insert(KeyBinding::new("Plus"), ViewerAction::ZoomIn);
    map.insert(KeyBinding::new("Minus"), ViewerAction::ZoomOut);
    map.insert(
        KeyBinding::new("Num0").with_shift(),
        ViewerAction::ZoomReset,
    );
    map.insert(KeyBinding::new("Enter"), ViewerAction::ToggleFullscreen);
    map.insert(KeyBinding::new("R").with_shift(), ViewerAction::Reload);
    map.insert(KeyBinding::new("Space"), ViewerAction::NextImage);
    map.insert(KeyBinding::new("ArrowRight"), ViewerAction::NextImage);
    map.insert(
        KeyBinding::new("Space").with_shift(),
        ViewerAction::PrevImage,
    );
    map.insert(KeyBinding::new("ArrowLeft"), ViewerAction::PrevImage);
    map.insert(KeyBinding::new("Home"), ViewerAction::FirstImage);
    map.insert(KeyBinding::new("End"), ViewerAction::LastImage);
    map.insert(
        KeyBinding::new("G").with_shift(),
        ViewerAction::ToggleGrayscale,
    );
    map.insert(
        KeyBinding::new("C").with_shift(),
        ViewerAction::ToggleMangaMode,
    );
    map.insert(KeyBinding::new("F"), ViewerAction::ToggleFiler);
    map.insert(KeyBinding::new("P"), ViewerAction::ToggleSettings);
    map
}

#[derive(Clone)]
pub struct NavigationOptions {
    pub end_of_folder: EndOfFolderOption,
    pub sort: NavigationSortOption,
}

impl Default for NavigationOptions {
    fn default() -> Self {
        Self {
            end_of_folder: EndOfFolderOption::Recursive,
            sort: NavigationSortOption::OsName,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndOfFolderOption {
    Stop,
    Next,
    Loop,
    Recursive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationSortOption {
    OsName,
    Name,
    Date,
    Size,
}

// pub(crate) reading: ReadingOptions,
// pub(crate) slideshow: slideshowOptions,
// pub(crate) navigation: navigationOptions,
// pub(crate) thumbnail: thumbnailOptions,
// pub(crate) loader: LoaderOptions,
// pub(crate) storage: StrageOptions,
// pub(crate) file_system: FileSystemOptions,
// pub(crate) input: InputOptions,
// pub(crate) runtime: RuntimeOptions,
// pub(crate) os_depend: OSDependOptions
