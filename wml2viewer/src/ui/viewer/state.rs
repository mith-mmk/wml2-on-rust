use crate::drawers::image::SaveFormat;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

#[derive(Default)]
pub(crate) struct ViewerOverlayState {
    pub(crate) loading_message: Option<String>,
}

pub(crate) struct SaveDialogState {
    pub(crate) format: SaveFormat,
    pub(crate) output_dir: Option<PathBuf>,
    pub(crate) file_name: String,
    pub(crate) message: Option<String>,
    pub(crate) open: bool,
    pub(crate) in_progress: bool,
    pub(crate) result_rx: Option<Receiver<Result<String, String>>>,
}

impl Default for SaveDialogState {
    fn default() -> Self {
        Self {
            format: SaveFormat::Png,
            output_dir: None,
            file_name: String::new(),
            message: None,
            open: false,
            in_progress: false,
            result_rx: None,
        }
    }
}
