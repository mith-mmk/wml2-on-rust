use crate::drawers::affine::InterpolationAlgorithm;

use crate::drawers::image::{LoadedImage, load_canvas_from_file, resize_loaded_image};
use crate::ui::viewer::{ViewerApp};

use eframe::egui::{self};
use std::error::Error;
use std::path::{Path, PathBuf};


fn load_image(path: &Path) -> Result<LoadedImage, Box<dyn Error>> {
    Ok(load_canvas_from_file(path)?)
}

fn initial_window_size(width: usize, height: usize) -> [f32; 2] {
    [
        (width as f32 + 32.0).clamp(480.0, 1600.0),
        (height as f32 + 96.0).clamp(360.0, 1200.0),
    ]
}


pub fn run(image_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let image = load_image(&image_path)?;
    let rendered = resize_loaded_image(&image, 1.0, InterpolationAlgorithm::Bilinear)?;
    let title = format!("wml2viewer - {}", image_path.display());

    let initial_size = initial_window_size(
        image.canvas.width() as usize,
        image.canvas.height() as usize,
    );

    // ui::viewer::set_canvas_size(&str);
    // ui::menu::set_title(&str);


    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(initial_size)
            .with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "wml2viewer",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(ViewerApp::new(cc, image_path, image, rendered)))
        }),
    )?;

    Ok(())
}