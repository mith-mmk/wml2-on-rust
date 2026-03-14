use crate::drawers::canvas::Canvas;
use crate::drawers::image::{ImageAlign, LoadedImage, resize_loaded_image};
use crate::filesystem::FileNavigator;
use crate::options::{AppConfig, KeyBinding, ViewerAction};
use crate::ui::viewer::options::{BackgroundStyle, RenderOptions, ViewerOptions};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions, vec2};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};
pub mod options;
use options::ZoomOption;

#[derive(Clone)]
pub(crate) struct ViewerApp {
    source: LoadedImage,
    rendered: LoadedImage,
    texture: TextureHandle,

    zoom: f32,

    current_frame: usize,
    last_frame_at: Instant,
    completed_loops: u32,

    fit_zoom: f32,
    navigator: FileNavigator,
    last_viewport_size: egui::Vec2,
    frame_counter: usize,

    render_options: RenderOptions,
    options: ViewerOptions,
    keymap: HashMap<KeyBinding, ViewerAction>,
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
        ZoomOption::FitScreenIncludeSmaller => zoom_w.min(zoom_h),
        ZoomOption::FitScreenOnlySmaller => {
            let fit = zoom_w.min(zoom_h);
            if fit < 1.0 { fit } else { 1.0 }
        }
    }
}

impl ViewerApp {
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        path: PathBuf,
        source: LoadedImage,
        rendered: LoadedImage,
        navigator: FileNavigator,
        config: AppConfig,
    ) -> Self {
        /* todo! Windowのx,y座標を固定 */
        let color_image = canvas_to_color_image(rendered.frame_canvas(0));

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

            fit_zoom: 1.0,
            navigator,
            last_viewport_size: egui::Vec2::ZERO,
            frame_counter: 0,

            render_options: config.render,
            options: config.viewer,
            keymap: config.input.merged_with_defaults(),
        }
    }

    fn source_size(&self) -> egui::Vec2 {
        vec2(
            self.source.canvas.width() as f32,
            self.source.canvas.height() as f32,
        )
    }
    pub(crate) fn set_zoom(&mut self, zoom: f32) -> Result<(), Box<dyn Error>> {
        let zoom = zoom.clamp(0.1, 16.0);
        if (zoom - self.zoom).abs() < f32::EPSILON {
            return Ok(());
        }

        self.rendered = resize_loaded_image(&self.source, zoom, self.render_options.zoom_method)?;
        self.current_frame = self
            .current_frame
            .min(self.rendered.frame_count().saturating_sub(1));
        self.zoom = zoom;
        self.last_frame_at = Instant::now();
        self.completed_loops = 0;
        self.upload_current_frame();
        Ok(())
    }

    fn toggle_zoom(&mut self) -> Result<(), Box<dyn Error>> {
        let target_zoom = if (self.zoom - 1.0).abs() < 0.01 {
            self.fit_zoom
        } else {
            1.0
        };
        self.set_zoom(target_zoom)
    }

    fn animation_enabled(&self) -> bool {
        self.options.animation && self.rendered.is_animated()
    }

    fn current_canvas(&self) -> &Canvas {
        if self.animation_enabled() {
            self.rendered.frame_canvas(self.current_frame)
        } else {
            &self.rendered.canvas
        }
    }

    pub(crate) fn upload_current_frame(&mut self) {
        let canvas = self.current_canvas();

        self.texture
            .set(canvas_to_color_image(canvas), TextureOptions::LINEAR);
    }

    pub(crate) fn update_animation(&mut self, ctx: &egui::Context) {
        if !self.animation_enabled() {
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

    fn load_path(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let source = crate::drawers::image::load_canvas_from_file(&path)?;
        let rendered = resize_loaded_image(&source, self.zoom, self.render_options.zoom_method)?;
        self.source = source;
        self.rendered = rendered;
        self.current_frame = 0;
        self.completed_loops = 0;
        self.last_frame_at = Instant::now();
        self.upload_current_frame();
        Ok(())
    }

    fn reload_current(&mut self) -> Result<(), Box<dyn Error>> {
        self.load_path(self.navigator.current().to_path_buf())
    }

    fn next_image(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(path) = self.navigator.next() {
            self.load_path(path)?;
        }
        Ok(())
    }

    fn prev_image(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(path) = self.navigator.prev() {
            self.load_path(path)?;
        }
        Ok(())
    }

    fn first_image(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(path) = self.navigator.first() {
            self.load_path(path)?;
        }
        Ok(())
    }

    fn last_image(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(path) = self.navigator.last() {
            self.load_path(path)?;
        }
        Ok(())
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let triggered = self.collect_triggered_actions(ctx);
        for action in triggered {
            match action {
                ViewerAction::ZoomIn => {
                    let _ = self.set_zoom(self.zoom * 1.25);
                }
                ViewerAction::ZoomOut => {
                    let _ = self.set_zoom(self.zoom / 1.25);
                }
                ViewerAction::ZoomReset => {
                    let _ = self.set_zoom(1.0);
                }
                ViewerAction::ZoomToggle => {
                    let _ = self.toggle_zoom();
                }
                ViewerAction::ToggleFullscreen => {
                    let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
                }
                ViewerAction::Reload => {
                    let _ = self.reload_current();
                }
                ViewerAction::NextImage => {
                    let _ = self.next_image();
                }
                ViewerAction::PrevImage => {
                    let _ = self.prev_image();
                }
                ViewerAction::FirstImage => {
                    let _ = self.first_image();
                }
                ViewerAction::LastImage => {
                    let _ = self.last_image();
                }
                ViewerAction::ToggleAnimation => {
                    self.options.animation = !self.options.animation;
                    self.current_frame = 0;
                    self.last_frame_at = Instant::now();
                    self.upload_current_frame();
                }
            }
        }
    }

    fn collect_triggered_actions(&self, ctx: &egui::Context) -> Vec<ViewerAction> {
        self.keymap
            .iter()
            .filter_map(|(binding, action)| {
                self.binding_pressed(ctx, binding).then(|| action.clone())
            })
            .collect()
    }

    fn binding_pressed(&self, ctx: &egui::Context, binding: &KeyBinding) -> bool {
        ctx.input(|i| {
            let modifiers = i.modifiers;
            if modifiers.shift != binding.shift
                || modifiers.ctrl != binding.ctrl
                || modifiers.alt != binding.alt
            {
                return false;
            }
            match key_name_to_egui(&binding.key) {
                Some(key) => i.key_pressed(key),
                None => false,
            }
        })
    }

    fn paint_background(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        match &self.options.background {
            BackgroundStyle::Solid(color) => {
                ui.painter().rect_filled(rect, 0.0, rgba_to_color32(*color));
            }
            BackgroundStyle::Tile {
                color1,
                color2,
                size,
            } => {
                let size = (*size).max(1) as f32;
                let color1 = rgba_to_color32(*color1);
                let color2 = rgba_to_color32(*color2);
                let mut y = rect.top();
                let mut row = 0_u32;
                while y < rect.bottom() {
                    let mut x = rect.left();
                    let mut col = 0_u32;
                    while x < rect.right() {
                        let color = if (row + col).is_multiple_of(2) {
                            color1
                        } else {
                            color2
                        };
                        let tile = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(size.min(rect.right() - x), size.min(rect.bottom() - y)),
                        );
                        ui.painter().rect_filled(tile, 0.0, color);
                        x += size;
                        col += 1;
                    }
                    y += size;
                    row += 1;
                }
            }
        }
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard(ctx);

        let zoom_delta = ctx.input(|i| i.zoom_delta());

        if zoom_delta != 1.0 {
            let _ = self.set_zoom(self.zoom * zoom_delta);
        }

        self.frame_counter += 1;
        self.update_animation(ctx);

        let panel = egui::CentralPanel::default().frame(egui::Frame::NONE);
        panel.show(ctx, |ui| {
            self.paint_background(ui, ui.max_rect());
            // inputに引っ越し
            if ui.input(|i| {
                i.pointer
                    .button_double_clicked(egui::PointerButton::Primary)
            }) {
                let _ = self.toggle_zoom();
            }

            let viewport = ui.available_size();

            if viewport != self.last_viewport_size
                && !matches!(self.render_options.zoom_option, ZoomOption::None)
            {
                self.last_viewport_size = viewport;

                let new_zoom = calc_fit_zoom(
                    viewport,
                    self.source_size(),
                    &self.render_options.zoom_option,
                );
                self.fit_zoom = new_zoom.clamp(0.1, 16.0);
                let _ = self.set_zoom(new_zoom);
            }

            let draw_size = vec2(
                self.current_canvas().width() as f32,
                self.current_canvas().height() as f32,
            );
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let offset = aligned_offset(viewport, draw_size, self.options.align);

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

fn aligned_offset(viewport: egui::Vec2, draw_size: egui::Vec2, align: ImageAlign) -> egui::Vec2 {
    let horizontal = match align {
        ImageAlign::Center | ImageAlign::Up | ImageAlign::Bottom => {
            (viewport.x - draw_size.x) * 0.5
        }
        ImageAlign::Right | ImageAlign::RightUp | ImageAlign::RightBottom => {
            viewport.x - draw_size.x
        }
        _ => 0.0,
    };
    let vertical = match align {
        ImageAlign::Center | ImageAlign::Left | ImageAlign::Right => {
            (viewport.y - draw_size.y) * 0.5
        }
        ImageAlign::LeftBottom | ImageAlign::RightBottom | ImageAlign::Bottom => {
            viewport.y - draw_size.y
        }
        _ => 0.0,
    };

    egui::vec2(horizontal, vertical)
}

fn rgba_to_color32([r, g, b, a]: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn key_name_to_egui(key: &str) -> Option<egui::Key> {
    match key {
        "Plus" => Some(egui::Key::Plus),
        "Minus" => Some(egui::Key::Minus),
        "Num0" => Some(egui::Key::Num0),
        "Enter" => Some(egui::Key::Enter),
        "R" => Some(egui::Key::R),
        "Space" => Some(egui::Key::Space),
        "ArrowRight" => Some(egui::Key::ArrowRight),
        "ArrowLeft" => Some(egui::Key::ArrowLeft),
        "Home" => Some(egui::Key::Home),
        "End" => Some(egui::Key::End),
        "G" => Some(egui::Key::G),
        "C" => Some(egui::Key::C),
        _ => None,
    }
}
