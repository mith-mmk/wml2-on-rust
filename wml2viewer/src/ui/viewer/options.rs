//! Viewer option models derived from `SPEC.md`.

use crate::drawers::affine::InterpolationAlgorithm;
use crate::drawers::image::ImageAlign;

#[derive(Clone)]
pub struct ViewerOptions {
    pub align: ImageAlign,
    pub background: BackgroundStyle,
    pub fade: bool,
    pub animation: bool,
}

impl Default for ViewerOptions {
    fn default() -> Self {
        Self {
            align: ImageAlign::Center,
            background: BackgroundStyle::Solid([0, 0, 0, 255]),
            fade: false,
            animation: true,
        }
    }
}

#[derive(Clone)]
pub enum BackgroundStyle {
    Solid([u8; 4]),
    Tile {
        color1: [u8; 4],
        color2: [u8; 4],
        size: u32,
    },
}

#[derive(Clone)]
pub struct RenderOptions {
    pub zoom_option: ZoomOption,
    pub zoom_method: InterpolationAlgorithm,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            zoom_option: ZoomOption::FitScreen,
            zoom_method: InterpolationAlgorithm::Bilinear,
        }
    }
}

#[derive(Clone)]
pub struct WindowOptions {
    pub fullscreen: bool,
    pub size: WindowSize,
    pub start_position: WindowStartPosition,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            fullscreen: false,
            size: WindowSize::Relative(0.8),
            start_position: WindowStartPosition::Center,
        }
    }
}

#[derive(Clone)]
pub enum WindowSize {
    Relative(f32),
    Exact { width: f32, height: f32 },
}

#[derive(Clone)]
pub enum WindowStartPosition {
    Center,
    Exact { x: f32, y: f32 },
}

#[derive(Clone, PartialEq, Eq)]
pub enum ZoomOption {
    None,
    FitWidth,
    FitHeight,
    FitScreen,
    FitScreenIncludeSmaller,
    FitScreenOnlySmaller,
}
