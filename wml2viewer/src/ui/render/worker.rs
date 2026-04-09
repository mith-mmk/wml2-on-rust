use crate::drawers::affine::InterpolationAlgorithm;
use crate::drawers::canvas::Canvas;
use crate::drawers::image::{
    LoadedImage, load_canvas_from_bytes_with_hint, load_canvas_from_file, resize_loaded_image,
};
use crate::filesystem::{OpenedImageSource, open_image_source_with_cancel, resolve_start_path};
use crate::ui::viewer::options::RenderScaleMode;
use std::error::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderWorkerPriority {
    Primary,
    Companion,
    Preload,
}

struct RenderIoCoordinator {
    primary_epoch: AtomicU64,
    primary_active: AtomicU64,
}

fn render_io_coordinator() -> &'static RenderIoCoordinator {
    static COORDINATOR: OnceLock<RenderIoCoordinator> = OnceLock::new();
    COORDINATOR.get_or_init(|| RenderIoCoordinator {
        primary_epoch: AtomicU64::new(0),
        primary_active: AtomicU64::new(0),
    })
}

fn should_abort_background_load(
    priority: RenderWorkerPriority,
    request_id: u64,
    latest_load_request_id: &AtomicU64,
    primary_epoch_snapshot: u64,
    coordinator: &RenderIoCoordinator,
) -> bool {
    if latest_load_request_id.load(Ordering::Acquire) != request_id {
        return true;
    }
    if priority == RenderWorkerPriority::Primary {
        return false;
    }
    coordinator.primary_active.load(Ordering::Acquire) > 0
        || coordinator.primary_epoch.load(Ordering::Acquire) != primary_epoch_snapshot
}

fn blank_loaded_image() -> LoadedImage {
    LoadedImage {
        canvas: Canvas::new(1, 1),
        animation: Vec::new(),
        loop_count: None,
    }
}

pub(crate) fn spawn_render_worker(
    initial_source: LoadedImage,
    priority: RenderWorkerPriority,
) -> (
    Sender<RenderCommand>,
    Receiver<RenderResult>,
    JoinHandle<()>,
) {
    let (command_tx, command_rx) = mpsc::channel::<RenderCommand>();
    let (result_tx, result_rx) = mpsc::channel::<RenderResult>();
    let current_source = Arc::new(Mutex::new(initial_source));
    let latest_load_request_id = Arc::new(AtomicU64::new(0));

    let join = thread::spawn(move || {
        while let Ok(command) = command_rx.recv() {
            match command {
                RenderCommand::LoadPath {
                    request_id,
                    path,
                    zoom,
                    method,
                    scale_mode,
                } => {
                    latest_load_request_id.store(request_id, Ordering::Release);
                    let result_tx = result_tx.clone();
                    let current_source = Arc::clone(&current_source);
                    let latest_load_request_id = Arc::clone(&latest_load_request_id);
                    let coordinator = render_io_coordinator();
                    let primary_epoch_snapshot = if priority == RenderWorkerPriority::Primary {
                        coordinator.primary_active.fetch_add(1, Ordering::AcqRel);
                        coordinator.primary_epoch.fetch_add(1, Ordering::AcqRel) + 1
                    } else {
                        coordinator.primary_epoch.load(Ordering::Acquire)
                    };
                    thread::spawn(move || {
                        struct PrimaryLoadGuard<'a> {
                            active: &'a AtomicU64,
                            enabled: bool,
                        }
                        impl Drop for PrimaryLoadGuard<'_> {
                            fn drop(&mut self) {
                                if self.enabled {
                                    self.active.fetch_sub(1, Ordering::AcqRel);
                                }
                            }
                        }

                        let _primary_guard = PrimaryLoadGuard {
                            active: &coordinator.primary_active,
                            enabled: priority == RenderWorkerPriority::Primary,
                        };
                        let should_cancel = || {
                            should_abort_background_load(
                                priority,
                                request_id,
                                &latest_load_request_id,
                                primary_epoch_snapshot,
                                coordinator,
                            )
                        };
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            (|| -> Result<Option<(LoadedImage, LoadedImage, PathBuf)>, Box<dyn Error>> {
                                if should_cancel() {
                                    return Ok(None);
                                }

                                let load_path = resolve_start_path(&path).unwrap_or(path.clone());
                                if should_cancel() {
                                    return Ok(None);
                                }

                                let source = match open_image_source_with_cancel(&load_path, &should_cancel) {
                                    Some(OpenedImageSource::Bytes {
                                        bytes,
                                        hint_path,
                                        ..
                                    }) => load_canvas_from_bytes_with_hint(&bytes, Some(&hint_path))?,
                                    Some(OpenedImageSource::File { path, .. }) => {
                                        load_canvas_from_file(&path)?
                                    }
                                    None => load_canvas_from_file(&load_path)?,
                                };
                                if should_cancel() {
                                    return Ok(None);
                                }

                                let rendered = match scale_mode {
                                    RenderScaleMode::FastGpu => source.clone(),
                                    RenderScaleMode::PreciseCpu => {
                                        resize_loaded_image(&source, zoom, method)?
                                    }
                                };
                                Ok(Some((source, rendered, load_path)))
                            })()
                        }))
                        .unwrap_or_else(|_| {
                            Err(Box::new(std::io::Error::other(
                                "decoder panicked while loading image",
                            )))
                        });

                        match result {
                            Ok(Some((source, rendered, load_path))) => {
                                if should_cancel() {
                                    return;
                                }
                                if let Ok(mut current) = current_source.lock() {
                                    *current = source.clone();
                                }
                                let _ = result_tx.send(RenderResult::Loaded {
                                    request_id,
                                    path: Some(load_path),
                                    source,
                                    rendered,
                                });
                            }
                            Ok(None) => {}
                            Err(err) => {
                                if !should_cancel() {
                                    let _ = result_tx.send(RenderResult::Failed {
                                        request_id,
                                        path: Some(path),
                                        message: err.to_string(),
                                    });
                                }
                            }
                        }
                    });
                }
                RenderCommand::ResizeCurrent {
                    request_id,
                    zoom,
                    method,
                    scale_mode,
                } => {
                    let result_tx = result_tx.clone();
                    let current_source = Arc::clone(&current_source);
                    thread::spawn(move || {
                        let source_snapshot = current_source
                            .lock()
                            .map(|current| current.clone())
                            .unwrap_or_else(|_| blank_loaded_image());
                        match catch_unwind(AssertUnwindSafe(|| match scale_mode {
                            RenderScaleMode::FastGpu => Ok(source_snapshot.clone()),
                            RenderScaleMode::PreciseCpu => {
                                resize_loaded_image(&source_snapshot, zoom, method)
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
                                    source: source_snapshot,
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
                        }
                    });
                }
                RenderCommand::Shutdown => break,
            }
        }
    });

    (command_tx, result_rx, join)
}

pub(crate) fn worker_send_error(err: mpsc::SendError<RenderCommand>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{RenderIoCoordinator, RenderWorkerPriority, should_abort_background_load};
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn preload_is_aborted_while_primary_is_active() {
        let latest = AtomicU64::new(7);
        let coordinator = RenderIoCoordinator {
            primary_epoch: AtomicU64::new(3),
            primary_active: AtomicU64::new(1),
        };

        assert!(should_abort_background_load(
            RenderWorkerPriority::Preload,
            7,
            &latest,
            3,
            &coordinator,
        ));
    }

    #[test]
    fn preload_is_aborted_after_primary_epoch_changes() {
        let latest = AtomicU64::new(9);
        let coordinator = RenderIoCoordinator {
            primary_epoch: AtomicU64::new(4),
            primary_active: AtomicU64::new(0),
        };

        assert!(should_abort_background_load(
            RenderWorkerPriority::Preload,
            9,
            &latest,
            3,
            &coordinator,
        ));
    }

    #[test]
    fn primary_load_is_not_aborted_by_primary_activity() {
        let latest = AtomicU64::new(11);
        let coordinator = RenderIoCoordinator {
            primary_epoch: AtomicU64::new(5),
            primary_active: AtomicU64::new(1),
        };

        assert!(!should_abort_background_load(
            RenderWorkerPriority::Primary,
            11,
            &latest,
            5,
            &coordinator,
        ));
        coordinator.primary_active.store(0, Ordering::Release);
    }
}
