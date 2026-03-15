use crate::configs::config::{load_app_config, load_startup_path};
use crate::drawers::image::{LoadedImage, load_canvas_from_file, resize_loaded_image};
use crate::filesystem::resolve_start_path;
use crate::options::*;
use crate::ui::viewer::ViewerApp;
use eframe::egui::{self};
use std::error::Error;
use std::path::{Path, PathBuf};

fn load_image(path: &Path) -> Result<LoadedImage, Box<dyn Error>> {
    Ok(load_canvas_from_file(path)?)
}

pub fn run(image_path: Option<PathBuf>, config_path: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let config = load_app_config(config_path.as_deref()).unwrap_or_default();
    let image_path = image_path.unwrap_or(
        load_startup_path(config_path.as_deref()).unwrap_or(std::env::current_dir()?),
    );
    let start_path = resolve_start_path(&image_path).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No supported image found for {}", image_path.display()),
        )
    })?;
    let image = load_image(&start_path)?;
    let rendered = resize_loaded_image(&image, 1.0, config.render.zoom_method)?;
    let title = format!("wml2viewer - {}", start_path.display());

    // ui::viewer::set_canvas_size(&str);
    // ui::menu::set_title(&str);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([320.0, 240.0])
            .with_min_inner_size([320.0, 240.0])
            .with_fullscreen(config.window.fullscreen),
        ..Default::default()
    };

    eframe::run_native(
        "wml2viewer",
        native_options,
        Box::new(move |cc| {
            let screen = cc.egui_ctx.input(|i| i.viewport().monitor_size.unwrap());

            let image_size = egui::vec2(image.canvas.width() as f32, image.canvas.height() as f32);
            let padding = egui::vec2(32.0, 96.0);
            let window_size = match config.window.size.clone() {
                WindowSize::Relative(ratio) => {
                    let ratio = ratio.clamp(0.1, 1.0);
                    egui::vec2(screen.x * ratio, screen.y * ratio)
                }
                WindowSize::Exact { width, height } => egui::vec2(width, height),
            };
            let window_size = egui::vec2(
                window_size
                    .x
                    .max((image_size.x + padding.x).min(screen.x * 0.9))
                    .min(screen.x),
                window_size
                    .y
                    .max((image_size.y + padding.y).min(screen.y * 0.9))
                    .min(screen.y),
            );

            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(window_size));
            match &config.window.start_position {
                WindowStartPosition::Center => {
                    let centered = egui::pos2(
                        ((screen.x - window_size.x) * 0.5).max(0.0),
                        ((screen.y - window_size.y) * 0.5).max(0.0),
                    );
                    cc.egui_ctx
                        .send_viewport_cmd(egui::ViewportCommand::OuterPosition(centered));
                }
                WindowStartPosition::Exact { x, y } => {
                    cc.egui_ctx
                        .send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                            *x, *y,
                        )));
                }
            }

            Ok(Box::new(ViewerApp::new(
                cc,
                image_path.clone(),
                start_path,
                image,
                rendered,
                config,
                config_path.clone(),
            )))
        }),
    )?;

    Ok(())
}
