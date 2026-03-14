use crate::drawers::canvas::Canvas;
use crate::drawers::image::LoadedImage;
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, vec2};
use std::error::Error;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct ViewerOptions {
    pub(crate) zoom_option: ZoomOption,
}

#[derive(Clone)]
pub(crate) enum ZoomOption {
    None,
    FitWidth,
    FitHeight,
    FitScreen,
    FitScreenIncludeSmaller,
    FitScreenOnlySmaller,
}

pub(crate) struct ViewerApp {
    source: LoadedImage,
    rendered: LoadedImage,
    texture: TextureHandle,

    zoom: f32,

    current_frame: usize,
    last_frame_at: Instant,
    completed_loops: u32,

    last_viewport_size: egui::Vec2,
    frame_counter: usize,

    options: ViewerOptions,
}

fn calc_fit_zoom(ctx_size: egui::Vec2, image_size: egui::Vec2, option: &ZoomOption) -> f32 {
    let image_width = image_size.x.max(1.0);
    let image_height = image_size.y.max(1.0);

    let canvas_width = ctx_size.x;
    let canvas_height = ctx_size.y;

    let zoom_w = canvas_width / image_width;
    let zoom_h = canvas_height / image_height;

    match option {
        ZoomOption::None => 1.0,
        ZoomOption::FitWidth => zoom_w,
        ZoomOption::FitHeight => zoom_h,
        ZoomOption::FitScreen => zoom_w.min(zoom_h),
        ZoomOption::FitScreenIncludeSmaller => zoom_w.min(zoom_h).min(1.0),
        ZoomOption::FitScreenOnlySmaller => {
            let fit = zoom_w.min(zoom_h);
            if fit > 1.0 { fit } else { 1.0 }
        }
    }
}
impl ViewerApp {
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        path: PathBuf,
        source: LoadedImage,
        rendered: LoadedImage,
        options: Option<ViewerOptions>,
    ) -> Self {
        let color_image = canvas_to_color_image(rendered.frame_canvas(0));

        //let source_size = vec2(source.canvas.width() as f32, source.canvas.height() as f32);

        let zoom_option = options
            .as_ref()
            .map(|o| o.zoom_option.clone())
            .unwrap_or(ZoomOption::FitScreen);

        let zoom = 1.0;

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

            zoom,

            current_frame: 0,
            last_frame_at: Instant::now(),
            completed_loops: 0,

            last_viewport_size: egui::Vec2::ZERO,
            frame_counter: 0,

            options: options.unwrap_or(ViewerOptions { zoom_option }),
        }
    }

    fn source_size(&self) -> egui::Vec2 {
        vec2(
            self.source.canvas.width() as f32,
            self.source.canvas.height() as f32,
        )
    }
    pub(crate) fn set_zoom(&mut self, zoom: f32) -> Result<(), Box<dyn Error>> {
        self.zoom = zoom.clamp(0.1, 16.0);
        Ok(())
    }

    pub(crate) fn upload_current_frame(&mut self) {
        let canvas = self.rendered.frame_canvas(self.current_frame);

        self.texture
            .set(canvas_to_color_image(canvas), TextureOptions::LINEAR);
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
        self.frame_counter += 1;
        self.update_animation(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let viewport = ui.available_size();

            if viewport != self.last_viewport_size {
                self.last_viewport_size = viewport;

                let new_zoom =
                    calc_fit_zoom(viewport, self.source_size(), &self.options.zoom_option);

                let _ = self.set_zoom(new_zoom);
            }

            let draw_size = self.source_size() * self.zoom;
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let offset = (viewport - draw_size) * 0.5;

                    ui.add_space(offset.y.max(0.0));

                    ui.horizontal(|ui| {
                        ui.add_space(offset.x.max(0.0));

                        ui.add(
                            egui::Image::from_texture(&self.texture).fit_to_exact_size(draw_size),
                        );
                    });
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
