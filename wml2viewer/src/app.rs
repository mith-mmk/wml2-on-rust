use crate::drawers::image::{LoadedImage, load_canvas_from_file};
use crate::ui::viewer::ViewerApp;
use crate::ui::viewer::ViewerOptions;
use crate::ui::viewer::ZoomOption;
use eframe::egui::{self};
use std::error::Error;
use std::path::{Path, PathBuf};

fn load_image(path: &Path) -> Result<LoadedImage, Box<dyn Error>> {
    Ok(load_canvas_from_file(path)?)
}

pub fn run(image_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let image = load_image(&image_path)?;
    let rendered = image.clone();
    let title = format!("wml2viewer - {}", image_path.display());

    // ui::viewer::set_canvas_size(&str);
    // ui::menu::set_title(&str);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([320.0, 240.0])
            .with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "wml2viewer",
        native_options,
        Box::new(move |cc| {
            let screen = cc.egui_ctx.input(|i| i.viewport().monitor_size.unwrap());

            let image_size = egui::vec2(image.canvas.width() as f32, image.canvas.height() as f32);

            let padding = egui::vec2(32.0, 96.0);

            let window_size = egui::vec2(
                (image_size.x + padding.x).min(screen.x * 0.9),
                (image_size.y + padding.y).min(screen.y * 0.9),
            );

            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(window_size));

            Ok(Box::new(ViewerApp::new(
                cc,
                image_path,
                image,
                rendered,
                Some(ViewerOptions {
                    zoom_option: ZoomOption::FitScreenOnlySmaller,
                }),
            )))
        }),
    )?;

    Ok(())
}
