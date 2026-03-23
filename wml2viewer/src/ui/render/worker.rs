use crate::drawers::affine::InterpolationAlgorithm;
use crate::drawers::image::{
    LoadedImage, load_canvas_from_bytes_with_hint, load_canvas_from_file, resize_loaded_image,
};
use crate::filesystem::{load_virtual_image_bytes, resolve_start_path};
use crate::ui::viewer::options::RenderScaleMode;
use std::error::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

pub(crate) enum RenderCommand {
    LoadPath {
        request_id: u64,
        path: PathBuf,
        zoom: f32,
        method: InterpolationAlgorithm,
        scale_mode: RenderScaleMode,
    },
    ResizeCurrent {
        request_id: u64,
        zoom: f32,
        method: InterpolationAlgorithm,
        scale_mode: RenderScaleMode,
    },
    Shutdown,
}

pub(crate) enum RenderResult {
    Loaded {
        request_id: u64,
        path: Option<PathBuf>,
        source: LoadedImage,
        rendered: LoadedImage,
    },
    Failed {
        request_id: u64,
        path: Option<PathBuf>,
        message: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActiveRenderRequest {
    Load(u64),
    Resize(u64),
}

pub(crate) fn spawn_render_worker(
    initial_source: LoadedImage,
) -> (Sender<RenderCommand>, Receiver<RenderResult>, JoinHandle<()>) {
    let (command_tx, command_rx) = mpsc::channel::<RenderCommand>();
    let (result_tx, result_rx) = mpsc::channel::<RenderResult>();

    let join = thread::spawn(move || {
        let mut current_source = initial_source;
        while let Ok(command) = command_rx.recv() {
            match command {
                RenderCommand::LoadPath {
                    request_id,
                    path,
                    zoom,
                    method,
                    scale_mode,
                } => {
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        (|| -> Result<(LoadedImage, LoadedImage, PathBuf), Box<dyn Error>> {
                            let load_path = resolve_start_path(&path).unwrap_or(path.clone());
                            let source = if let Some(bytes) = load_virtual_image_bytes(&load_path) {
                                load_canvas_from_bytes_with_hint(&bytes, Some(&load_path))?
                            } else {
                                load_canvas_from_file(&load_path)?
                            };
                            let rendered = match scale_mode {
                                RenderScaleMode::FastGpu => source.clone(),
                                RenderScaleMode::PreciseCpu => {
                                    resize_loaded_image(&source, zoom, method)?
                                }
                            };
                            Ok((source, rendered, load_path))
                        })()
                    }))
                    .unwrap_or_else(|_| {
                        Err(Box::new(std::io::Error::other(
                            "decoder panicked while loading image",
                        )))
                    });

                    match result {
                        Ok((source, rendered, load_path)) => {
                            current_source = source.clone();
                            let _ = result_tx.send(RenderResult::Loaded {
                                request_id,
                                path: Some(load_path),
                                source,
                                rendered,
                            });
                        }
                        Err(err) => {
                            let _ = result_tx.send(RenderResult::Failed {
                                request_id,
                                path: Some(path),
                                message: err.to_string(),
                            });
                        }
                    }
                }
                RenderCommand::ResizeCurrent {
                    request_id,
                    zoom,
                    method,
                    scale_mode,
                } => match catch_unwind(AssertUnwindSafe(|| {
                    match scale_mode {
                        RenderScaleMode::FastGpu => Ok(current_source.clone()),
                        RenderScaleMode::PreciseCpu => {
                            resize_loaded_image(&current_source, zoom, method)
                        }
                    }
                }))
                .unwrap_or_else(|_| {
                    Err(Box::new(std::io::Error::other(
                        "renderer panicked while resizing image",
                    )))
                }) {
                    Ok(rendered) => {
                        let _ = result_tx.send(RenderResult::Loaded {
                            request_id,
                            path: None,
                            source: current_source.clone(),
                            rendered,
                        });
                    }
                    Err(err) => {
                        let _ = result_tx.send(RenderResult::Failed {
                            request_id,
                            path: None,
                            message: err.to_string(),
                        });
                    }
                },
                RenderCommand::Shutdown => break,
            }
        }
    });

    (command_tx, result_rx, join)
}

pub(crate) fn worker_send_error(err: mpsc::SendError<RenderCommand>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(err.to_string()))
}
