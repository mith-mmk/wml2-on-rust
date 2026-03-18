use std::error::Error;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::dependent::default_config_dir;
use crate::drawers::affine::InterpolationAlgorithm;
use crate::options::{AppConfig, EndOfFolderOption, NavigationSortOption};
use crate::ui::viewer::options::{
    BackgroundStyle, RenderOptions, ViewerOptions, WindowOptions, WindowSize, WindowStartPosition,
    ZoomOption,
};

type ConfigResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct ConfigFile {
    viewer: ViewerConfigFile,
    window: WindowConfigFile,
    render: RenderConfigFile,
    navigation: NavigationConfigFile,
    runtime: RuntimeConfigFile,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct ViewerConfigFile {
    animation: bool,
    manga_mode: bool,
    manga_right_to_left: bool,
    background: BackgroundConfigFile,
}

impl Default for ViewerConfigFile {
    fn default() -> Self {
        Self {
            animation: true,
            manga_mode: false,
            manga_right_to_left: true,
            background: BackgroundConfigFile::Solid {
                rgba: [0, 0, 0, 255],
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BackgroundConfigFile {
    Solid {
        rgba: [u8; 4],
    },
    Tile {
        color1: [u8; 4],
        color2: [u8; 4],
        size: u32,
    },
}

impl Default for BackgroundConfigFile {
    fn default() -> Self {
        Self::Solid {
            rgba: [0, 0, 0, 255],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct WindowConfigFile {
    fullscreen: bool,
    size: WindowSizeConfigFile,
    start_position: WindowStartPositionConfigFile,
}

impl Default for WindowConfigFile {
    fn default() -> Self {
        Self {
            fullscreen: false,
            size: WindowSizeConfigFile::Relative(0.8),
            start_position: WindowStartPositionConfigFile::Center,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum WindowSizeConfigFile {
    Relative(f32),
    Exact { width: f32, height: f32 },
}

impl Default for WindowSizeConfigFile {
    fn default() -> Self {
        Self::Relative(0.8)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum WindowStartPositionConfigFile {
    Center,
    Exact { x: f32, y: f32 },
}

impl Default for WindowStartPositionConfigFile {
    fn default() -> Self {
        Self::Center
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct RenderConfigFile {
    zoom_option: ZoomOptionConfigFile,
    zoom_method: ZoomMethodConfigFile,
}

impl Default for RenderConfigFile {
    fn default() -> Self {
        Self {
            zoom_option: ZoomOptionConfigFile::FitScreen,
            zoom_method: ZoomMethodConfigFile::Bilinear,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ZoomOptionConfigFile {
    None,
    FitWidth,
    FitHeight,
    FitScreen,
    FitScreenIncludeSmaller,
    FitScreenOnlySmaller,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ZoomMethodConfigFile {
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct NavigationConfigFile {
    end_of_folder: EndOfFolderConfigFile,
    sort: NavigationSortConfigFile,
}

impl Default for NavigationConfigFile {
    fn default() -> Self {
        Self {
            end_of_folder: EndOfFolderConfigFile::Recursive,
            sort: NavigationSortConfigFile::OsName,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EndOfFolderConfigFile {
    Stop,
    Next,
    Loop,
    Recursive,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NavigationSortConfigFile {
    OsName,
    Name,
    Date,
    Size,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct RuntimeConfigFile {
    current_file: Option<PathBuf>,
}

pub fn load_app_config(config_path: Option<&Path>) -> ConfigResult<AppConfig> {
    let Some(file) = load_config_file(config_path)? else {
        return Ok(AppConfig::default());
    };
    Ok(file.into())
}

pub fn load_startup_path(config_path: Option<&Path>) -> ConfigResult<PathBuf> {
    if let Some(file) = load_config_file(config_path)? {
        if let Some(path) = file.runtime.current_file {
            return Ok(path);
        }
    }

    Ok(std::env::current_dir()?)
}

pub fn save_app_config(
    config: &AppConfig,
    current_path: Option<&Path>,
    config_override: Option<&Path>,
) -> ConfigResult<()> {
    let path = resolve_config_path(config_override);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = toml::to_string_pretty(&ConfigFile::from_parts(config.clone(), current_path))?;
    fs::write(path, text)?;
    Ok(())
}

fn resolve_config_path(config_override: Option<&Path>) -> PathBuf {
    config_override
        .map(|path| path.to_path_buf())
        .or_else(|| default_config_dir().map(|dir| dir.join("config.toml")))
        .unwrap_or_else(|| PathBuf::from("wml2viewer.toml"))
}

fn load_config_file(config_override: Option<&Path>) -> ConfigResult<Option<ConfigFile>> {
    let path = resolve_config_path(config_override);
    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(path)?;
    let file: ConfigFile = toml::from_str(&text)?;
    Ok(Some(file))
}

impl From<ConfigFile> for AppConfig {
    fn from(value: ConfigFile) -> Self {
        let mut config = AppConfig::default();
        config.viewer = ViewerOptions {
            align: config.viewer.align,
            background: value.viewer.background.into(),
            fade: config.viewer.fade,
            animation: value.viewer.animation,
            manga_mode: value.viewer.manga_mode,
            manga_right_to_left: value.viewer.manga_right_to_left,
        };
        config.window = value.window.into();
        config.render = value.render.into();
        config.navigation.end_of_folder = value.navigation.end_of_folder.into();
        config.navigation.sort = value.navigation.sort.into();
        config
    }
}

impl From<AppConfig> for ConfigFile {
    fn from(value: AppConfig) -> Self {
        Self::from_parts(value, None)
    }
}

impl ConfigFile {
    fn from_parts(value: AppConfig, current_path: Option<&Path>) -> Self {
        Self {
            viewer: ViewerConfigFile {
                animation: value.viewer.animation,
                manga_mode: value.viewer.manga_mode,
                manga_right_to_left: value.viewer.manga_right_to_left,
                background: value.viewer.background.into(),
            },
            window: value.window.into(),
            render: value.render.into(),
            navigation: NavigationConfigFile {
                end_of_folder: value.navigation.end_of_folder.into(),
                sort: value.navigation.sort.into(),
            },
            runtime: RuntimeConfigFile {
                current_file: current_path.map(|path| path.to_path_buf()),
            },
        }
    }
}

impl From<BackgroundConfigFile> for BackgroundStyle {
    fn from(value: BackgroundConfigFile) -> Self {
        match value {
            BackgroundConfigFile::Solid { rgba } => BackgroundStyle::Solid(rgba),
            BackgroundConfigFile::Tile {
                color1,
                color2,
                size,
            } => BackgroundStyle::Tile {
                color1,
                color2,
                size,
            },
        }
    }
}

impl From<BackgroundStyle> for BackgroundConfigFile {
    fn from(value: BackgroundStyle) -> Self {
        match value {
            BackgroundStyle::Solid(rgba) => Self::Solid { rgba },
            BackgroundStyle::Tile {
                color1,
                color2,
                size,
            } => Self::Tile {
                color1,
                color2,
                size,
            },
        }
    }
}

impl From<WindowConfigFile> for WindowOptions {
    fn from(value: WindowConfigFile) -> Self {
        Self {
            fullscreen: value.fullscreen,
            size: value.size.into(),
            start_position: value.start_position.into(),
        }
    }
}

impl From<WindowOptions> for WindowConfigFile {
    fn from(value: WindowOptions) -> Self {
        Self {
            fullscreen: value.fullscreen,
            size: value.size.into(),
            start_position: value.start_position.into(),
        }
    }
}

impl From<WindowSizeConfigFile> for WindowSize {
    fn from(value: WindowSizeConfigFile) -> Self {
        match value {
            WindowSizeConfigFile::Relative(ratio) => WindowSize::Relative(ratio),
            WindowSizeConfigFile::Exact { width, height } => WindowSize::Exact { width, height },
        }
    }
}

impl From<WindowSize> for WindowSizeConfigFile {
    fn from(value: WindowSize) -> Self {
        match value {
            WindowSize::Relative(ratio) => Self::Relative(ratio),
            WindowSize::Exact { width, height } => Self::Exact { width, height },
        }
    }
}

impl From<WindowStartPositionConfigFile> for WindowStartPosition {
    fn from(value: WindowStartPositionConfigFile) -> Self {
        match value {
            WindowStartPositionConfigFile::Center => WindowStartPosition::Center,
            WindowStartPositionConfigFile::Exact { x, y } => WindowStartPosition::Exact { x, y },
        }
    }
}

impl From<WindowStartPosition> for WindowStartPositionConfigFile {
    fn from(value: WindowStartPosition) -> Self {
        match value {
            WindowStartPosition::Center => Self::Center,
            WindowStartPosition::Exact { x, y } => Self::Exact { x, y },
        }
    }
}

impl From<RenderConfigFile> for RenderOptions {
    fn from(value: RenderConfigFile) -> Self {
        Self {
            zoom_option: value.zoom_option.into(),
            zoom_method: value.zoom_method.into(),
        }
    }
}

impl From<RenderOptions> for RenderConfigFile {
    fn from(value: RenderOptions) -> Self {
        Self {
            zoom_option: value.zoom_option.into(),
            zoom_method: value.zoom_method.into(),
        }
    }
}

impl From<ZoomOptionConfigFile> for ZoomOption {
    fn from(value: ZoomOptionConfigFile) -> Self {
        match value {
            ZoomOptionConfigFile::None => ZoomOption::None,
            ZoomOptionConfigFile::FitWidth => ZoomOption::FitWidth,
            ZoomOptionConfigFile::FitHeight => ZoomOption::FitHeight,
            ZoomOptionConfigFile::FitScreen => ZoomOption::FitScreen,
            ZoomOptionConfigFile::FitScreenIncludeSmaller => ZoomOption::FitScreenIncludeSmaller,
            ZoomOptionConfigFile::FitScreenOnlySmaller => ZoomOption::FitScreenOnlySmaller,
        }
    }
}

impl From<ZoomOption> for ZoomOptionConfigFile {
    fn from(value: ZoomOption) -> Self {
        match value {
            ZoomOption::None => Self::None,
            ZoomOption::FitWidth => Self::FitWidth,
            ZoomOption::FitHeight => Self::FitHeight,
            ZoomOption::FitScreen => Self::FitScreen,
            ZoomOption::FitScreenIncludeSmaller => Self::FitScreenIncludeSmaller,
            ZoomOption::FitScreenOnlySmaller => Self::FitScreenOnlySmaller,
        }
    }
}

impl From<ZoomMethodConfigFile> for InterpolationAlgorithm {
    fn from(value: ZoomMethodConfigFile) -> Self {
        match value {
            ZoomMethodConfigFile::Nearest => InterpolationAlgorithm::NearestNeighber,
            ZoomMethodConfigFile::Bilinear => InterpolationAlgorithm::Bilinear,
            ZoomMethodConfigFile::Bicubic => InterpolationAlgorithm::BicubicAlpha(None),
            ZoomMethodConfigFile::Lanczos3 => InterpolationAlgorithm::Lanzcos3,
        }
    }
}

impl From<InterpolationAlgorithm> for ZoomMethodConfigFile {
    fn from(value: InterpolationAlgorithm) -> Self {
        match value {
            InterpolationAlgorithm::NearestNeighber => Self::Nearest,
            InterpolationAlgorithm::Bilinear => Self::Bilinear,
            InterpolationAlgorithm::Bicubic | InterpolationAlgorithm::BicubicAlpha(_) => {
                Self::Bicubic
            }
            InterpolationAlgorithm::Lanzcos3 | InterpolationAlgorithm::Lanzcos(_) => Self::Lanczos3,
        }
    }
}

impl From<EndOfFolderConfigFile> for EndOfFolderOption {
    fn from(value: EndOfFolderConfigFile) -> Self {
        match value {
            EndOfFolderConfigFile::Stop => EndOfFolderOption::Stop,
            EndOfFolderConfigFile::Next => EndOfFolderOption::Next,
            EndOfFolderConfigFile::Loop => EndOfFolderOption::Loop,
            EndOfFolderConfigFile::Recursive => EndOfFolderOption::Recursive,
        }
    }
}

impl From<EndOfFolderOption> for EndOfFolderConfigFile {
    fn from(value: EndOfFolderOption) -> Self {
        match value {
            EndOfFolderOption::Stop => Self::Stop,
            EndOfFolderOption::Next => Self::Next,
            EndOfFolderOption::Loop => Self::Loop,
            EndOfFolderOption::Recursive => Self::Recursive,
        }
    }
}

impl From<NavigationSortConfigFile> for NavigationSortOption {
    fn from(value: NavigationSortConfigFile) -> Self {
        match value {
            NavigationSortConfigFile::OsName => NavigationSortOption::OsName,
            NavigationSortConfigFile::Name => NavigationSortOption::Name,
            NavigationSortConfigFile::Date => NavigationSortOption::Date,
            NavigationSortConfigFile::Size => NavigationSortOption::Size,
        }
    }
}

impl From<NavigationSortOption> for NavigationSortConfigFile {
    fn from(value: NavigationSortOption) -> Self {
        match value {
            NavigationSortOption::OsName => Self::OsName,
            NavigationSortOption::Name => Self::Name,
            NavigationSortOption::Date => Self::Date,
            NavigationSortOption::Size => Self::Size,
        }
    }
}
