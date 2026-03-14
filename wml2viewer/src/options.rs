/*!
! prelude options
*/

pub use crate::ui::viewer::options::{
    BackgroundStyle, RenderOptions, ViewerOptions, WindowOptions, WindowSize, ZoomOption,
};

#[derive(Clone, Default)]
pub struct AppConfig {
    pub viewer: ViewerOptions,
    pub window: WindowOptions,
    pub render: RenderOptions,
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
