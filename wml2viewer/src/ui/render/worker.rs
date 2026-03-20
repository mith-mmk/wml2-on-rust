use crate::drawers::affine::InterpolationAlgorithm;
use crate::drawers::image::{
    LoadedImage, load_canvas_from_bytes_with_hint, load_canvas_from_file, resize_loaded_image,
};
use crate::filesystem::load_virtual_image_bytes;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub(crate) enum RenderCommand {
    LoadPath {
        request_id: u64,
        path: PathBuf,
        zoom: f32,
        method: InterpolationAlgorithm,
    },
    ResizeCurrent {
        request_id: u64,
        zoom: f32,
        method: InterpolationAlgorithm,
    },
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
) -> (Sender<RenderCommand>, Receiver<RenderResult>) {
    let (command_tx, command_rx) = mpsc::channel::<RenderCommand>();
    let (result_tx, result_rx) = mpsc::channel::<RenderResult>();

    thread::spawn(move || {
        let mut current_source = initial_source;
        while let Ok(command) = command_rx.recv() {
            match command {
                RenderCommand::LoadPath {
                    request_id,
                    path,
                    zoom,
                    method,
                } => {
                    let result = (|| -> Result<(LoadedImage, LoadedImage), Box<dyn Error>> {
                        let source = if let Some(bytes) = load_virtual_image_bytes(&path) {
                            load_canvas_from_bytes_with_hint(&bytes, Some(&path))?
                        } else {
                            load_canvas_from_file(&path)?
                        };
                        let rendered = resize_loaded_image(&source, zoom, method)?;
                        Ok((source, rendered))
                    })();

                    match result {
                        Ok((source, rendered)) => {
                            current_source = source.clone();
                            let _ = result_tx.send(RenderResult::Loaded {
                                request_id,
                                path: Some(path),
                                source,
                                rendered,
                            });
                        }
                        Err(err) => {
                            let _ = result_tx.send(RenderResult::Failed {
                                request_id,
                                message: err.to_string(),
                            });
                        }
                    }
                }
                RenderCommand::ResizeCurrent {
                    request_id,
                    zoom,
                    method,
                } => match resize_loaded_image(&current_source, zoom, method) {
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
                            message: err.to_string(),
                        });
                    }
                },
            }
        }
    });

    (command_tx, result_rx)
}

pub(crate) fn worker_send_error(err: mpsc::SendError<RenderCommand>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(err.to_string()))
}
