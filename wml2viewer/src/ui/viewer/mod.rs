use crate::drawers::affine::InterpolationAlgorithm;
use crate::drawers::image::{LoadedImage, resize_loaded_image};
use std::time::{Duration, Instant};
use crate::drawers::canvas::Canvas;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, vec2};
use std::error::Error;
use std::path::PathBuf;

pub(crate) struct ViewerApp {
    source: LoadedImage,
    rendered: LoadedImage,
    texture: TextureHandle,
    image_size: egui::Vec2,
    file_label: String,
    zoom: f32,
    current_frame: usize,
    last_frame_at: Instant,
    completed_loops: u32,
}

impl ViewerApp {
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        path: PathBuf,
        source: LoadedImage,
        rendered: LoadedImage,
    ) -> Self {
        let color_image = canvas_to_color_image(rendered.frame_canvas(0));
        let image_size = vec2(color_image.size[0] as f32, color_image.size[1] as f32);
        let texture_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("image")
            .to_owned();
        let texture = cc
            .egui_ctx
            .load_texture(texture_name, color_image, TextureOptions::LINEAR);

        Self {
            source,
            rendered,
            texture,
            image_size,
            file_label: path.display().to_string(),
            zoom: 1.0,
            current_frame: 0,
            last_frame_at: Instant::now(),
            completed_loops: 0,
        }
    }

    pub(crate) fn set_zoom(&mut self, zoom: f32) -> Result<(), Box<dyn Error>> {
        let zoom = zoom.clamp(0.1, 16.0);
        if (zoom - self.zoom).abs() < f32::EPSILON {
            return Ok(());
        }

        self.rendered = resize_loaded_image(&self.source, zoom, InterpolationAlgorithm::Bilinear)?;
        self.current_frame = self
            .current_frame
            .min(self.rendered.frame_count().saturating_sub(1));
        self.zoom = zoom;
        self.completed_loops = 0;
        self.last_frame_at = Instant::now();
        self.upload_current_frame();
        Ok(())
    }

    pub(crate) fn upload_current_frame(&mut self) {
        let canvas = self.rendered.frame_canvas(self.current_frame);
        self.texture
            .set(canvas_to_color_image(canvas), TextureOptions::LINEAR);
        self.image_size = vec2(canvas.width() as f32, canvas.height() as f32);
    }

    pub(crate) fn update_animation(&mut self, ctx: &egui::Context) {
        if !self.rendered.is_animated() {
            return;
        }

        let frame_delay = self.rendered.frame_delay_ms(self.current_frame).max(16);
        let elapsed = self.last_frame_at.elapsed();
        let delay = Duration::from_millis(frame_delay);

        if elapsed >= delay {
            if let Some(next_frame) = self.next_frame_index() {
                self.current_frame = next_frame;
                self.last_frame_at = Instant::now();
                self.upload_current_frame();
            }
        }

        let remaining = delay.saturating_sub(self.last_frame_at.elapsed());
        ctx.request_repaint_after(remaining.max(Duration::from_millis(16)));
    }

    pub(crate) fn next_frame_index(&mut self) -> Option<usize> {
        let frame_count = self.rendered.frame_count();
        if frame_count <= 1 {
            return None;
        }

        if self.current_frame + 1 < frame_count {
            return Some(self.current_frame + 1);
        }

        match self.source.loop_count {
            Some(loop_count) if loop_count > 0 && self.completed_loops + 1 >= loop_count => None,
            _ => {
                self.completed_loops += 1;
                Some(0)
            }
        }
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_animation(ctx);

        egui::TopBottomPanel::top("image_info").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(&self.file_label);
                ui.label(format!(
                    "{} x {}",
                    self.source.canvas.width(),
                    self.source.canvas.height()
                ));
                ui.label(format!("frames: {}", self.source.frame_count()));
                if self.source.is_animated() {
                    ui.label(format!("frame: {}", self.current_frame + 1));
                }
                ui.separator();

                let mut next_zoom = self.zoom;
                if ui.button("-").clicked() {
                    next_zoom /= 1.25;
                }
                if ui.button("100%").clicked() {
                    next_zoom = 1.0;
                }
                if ui.button("+").clicked() {
                    next_zoom *= 1.25;
                }

                let mut zoom_percent = self.zoom * 100.0;
                if ui
                    .add(
                        egui::Slider::new(&mut zoom_percent, 10.0..=1600.0)
                            .logarithmic(true)
                            .suffix("%")
                            .text("zoom"),
                    )
                    .changed()
                {
                    next_zoom = zoom_percent / 100.0;
                }

                if (next_zoom - self.zoom).abs() >= f32::EPSILON {
                    if let Err(err) = self.set_zoom(next_zoom) {
                        eprintln!("failed to resize image: {err}");
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(egui::Image::from_texture(&self.texture).fit_to_original_size(1.0));
                });
        });
    }
}

fn canvas_to_color_image(canvas: &Canvas) -> ColorImage {
    ColorImage::from_rgba_unmultiplied(
        [canvas.width() as usize, canvas.height() as usize],
        canvas.buffer(),
    )
}