use crate::drawers::affine::InterpolationAlgorithm;
use crate::drawers::image::{load_canvas_from_bytes, load_canvas_from_file, resize_loaded_image};
use crate::filesystem::{load_virtual_image_bytes, virtual_image_size};
use crate::ui::render::canvas_to_color_image;
use eframe::egui::ColorImage;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub(crate) enum ThumbnailCommand {
    Generate {
        request_id: u64,
        path: PathBuf,
        max_side: u32,
    },
}

pub(crate) enum ThumbnailResult {
    Ready {
        _request_id: u64,
        path: PathBuf,
        image: ColorImage,
    },
}

pub(crate) fn spawn_thumbnail_worker() -> (Sender<ThumbnailCommand>, Receiver<ThumbnailResult>) {
    let (command_tx, command_rx) = mpsc::channel::<ThumbnailCommand>();
    let (result_tx, result_rx) = mpsc::channel::<ThumbnailResult>();

    thread::spawn(move || {
        while let Ok(command) = command_rx.recv() {
            match command {
                ThumbnailCommand::Generate {
                    request_id,
                    path,
                    max_side,
                } => {
                    if should_skip_thumbnail(&path) {
                        continue;
                    }
                    let loaded = if let Some(bytes) = load_virtual_image_bytes(&path) {
                        load_canvas_from_bytes(&bytes)
                    } else {
                        load_canvas_from_file(&path)
                    };
                    let Ok(image) = loaded else {
                        continue;
                    };

                    let scale = (max_side as f32
                        / image.canvas.width().max(image.canvas.height()) as f32)
                        .clamp(0.05, 1.0);
                    let Ok(resized) =
                        resize_loaded_image(&image, scale, InterpolationAlgorithm::Bilinear)
                    else {
                        continue;
                    };
                    let color_image = canvas_to_color_image(&resized.canvas);
                    let _ = result_tx.send(ThumbnailResult::Ready {
                        _request_id: request_id,
                        path,
                        image: color_image,
                    });
                }
            }
        }
    });

    (command_tx, result_rx)
}

fn should_skip_thumbnail(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();
    let size = virtual_image_size(path).unwrap_or(0);
    (ext == "bmp" && size > 8 * 1024 * 1024) || size > 128 * 1024 * 1024
}
