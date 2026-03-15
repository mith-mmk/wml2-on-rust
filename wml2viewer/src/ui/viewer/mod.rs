use crate::configs::config::save_app_config;
use crate::drawers::affine::InterpolationAlgorithm;
use crate::drawers::canvas::Canvas;
use crate::drawers::image::{ImageAlign, LoadedImage, resize_canvas, resize_loaded_image};
use crate::filesystem::{FilesystemCommand, FilesystemResult, spawn_filesystem_worker};
use crate::options::{
    AppConfig, EndOfFolderOption, KeyBinding, NavigationOptions, NavigationSortOption, ViewerAction,
};
use crate::ui::viewer::options::{
    BackgroundStyle, RenderOptions, ViewerOptions, WindowOptions, WindowStartPosition,
};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions, vec2};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};
pub mod options;
use options::ZoomOption;

const NAVIGATION_REPEAT_INTERVAL: Duration = Duration::from_millis(180);

enum RenderCommand {
    LoadPath {
        request_id: u64,
        path: PathBuf,
        zoom: f32,
        method: crate::drawers::affine::InterpolationAlgorithm,
    },
    ResizeCurrent {
        request_id: u64,
        zoom: f32,
        method: crate::drawers::affine::InterpolationAlgorithm,
    },
}

enum RenderResult {
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
enum ActiveRenderRequest {
    Load(u64),
    Resize(u64),
}

pub(crate) struct ViewerApp {
    current_navigation_path: PathBuf,
    current_path: PathBuf,
    source: LoadedImage,
    rendered: LoadedImage,
    texture: TextureHandle,

    zoom: f32,

    current_frame: usize,
    last_frame_at: Instant,
    completed_loops: u32,

    fit_zoom: f32,
    last_viewport_size: egui::Vec2,
    frame_counter: usize,

    render_options: RenderOptions,
    options: ViewerOptions,
    window_options: WindowOptions,
    keymap: HashMap<KeyBinding, ViewerAction>,
    end_of_folder: EndOfFolderOption,
    navigation_sort: NavigationSortOption,
    worker_tx: Sender<RenderCommand>,
    worker_rx: Receiver<RenderResult>,
    next_request_id: u64,
    active_request: Option<ActiveRenderRequest>,
    fs_tx: Sender<FilesystemCommand>,
    fs_rx: Receiver<FilesystemResult>,
    next_fs_request_id: u64,
    active_fs_request_id: Option<u64>,
    navigator_ready: bool,
    loading_message: Option<String>,
    last_navigation_at: Option<Instant>,
    show_settings: bool,
    max_texture_side: usize,
    texture_display_scale: f32,
    pending_resize_after_load: bool,
    pending_fit_recalc: bool,
    config_path: Option<PathBuf>,
}

fn calc_fit_zoom(ctx_size: egui::Vec2, image_size: egui::Vec2, option: &ZoomOption) -> f32 {
    let image_width = image_size.x.max(1.0);
    let image_height = image_size.y.max(1.0);

    let canvas_width = ctx_size.x;
    let canvas_height = ctx_size.y;

    let zoom_w = canvas_width / image_width;
    let zoom_h = canvas_height / image_height;
    let fit = zoom_w.min(zoom_h);

    match option {
        ZoomOption::None => 1.0,
        ZoomOption::FitWidth => zoom_w.min(1.0),
        ZoomOption::FitHeight => zoom_h.min(1.0),
        ZoomOption::FitScreen => fit.min(1.0),
        ZoomOption::FitScreenIncludeSmaller => fit,
        ZoomOption::FitScreenOnlySmaller => fit.min(1.0),
    }
}

impl ViewerApp {
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        navigation_path: PathBuf,
        path: PathBuf,
        source: LoadedImage,
        rendered: LoadedImage,
        config: AppConfig,
        config_path: Option<PathBuf>,
    ) -> Self {
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
        let (worker_tx, worker_rx) = spawn_render_worker(source.clone());
        let (fs_tx, fs_rx) = spawn_filesystem_worker(config.navigation.sort);

        let mut this = Self {
            current_navigation_path: navigation_path.clone(),
            current_path: path.clone(),
            source,
            rendered,
            texture,

            zoom,

            current_frame: 0,
            last_frame_at: Instant::now(),
            completed_loops: 0,

            fit_zoom: 1.0,
            last_viewport_size: egui::Vec2::ZERO,
            frame_counter: 0,

            render_options: config.render,
            options: config.viewer,
            window_options: config.window,
            keymap: config.input.merged_with_defaults(),
            end_of_folder: config.navigation.end_of_folder,
            navigation_sort: config.navigation.sort,
            worker_tx,
            worker_rx,
            next_request_id: 0,
            active_request: None,
            fs_tx,
            fs_rx,
            next_fs_request_id: 0,
            active_fs_request_id: None,
            navigator_ready: false,
            loading_message: None,
            last_navigation_at: None,
            show_settings: false,
            max_texture_side: cc.egui_ctx.input(|i| i.max_texture_side),
            texture_display_scale: 1.0,
            pending_resize_after_load: false,
            pending_fit_recalc: false,
            config_path,
        };

        let _ = this.init_filesystem(navigation_path);
        this
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
        self.zoom = zoom;
        self.request_resize_current()?;
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
        let (image, display_scale) = {
            let canvas = self.current_canvas();
            let (canvas, display_scale) = downscale_for_texture_limit(
                canvas,
                self.max_texture_side,
                self.render_options.zoom_method,
            );
            (canvas_to_color_image(&canvas), display_scale)
        };

        self.texture_display_scale = display_scale;
        self.texture.set(image, TextureOptions::LINEAR);
    }

    fn update_window_title(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "wml2viewer - {}",
            self.current_path.display()
        )));
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

    fn reload_current(&mut self) -> Result<(), Box<dyn Error>> {
        self.request_load_path(self.current_navigation_path.clone())
    }

    fn next_image(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.can_trigger_navigation() {
            return Ok(());
        }
        self.request_navigation(FilesystemCommand::Next {
            request_id: 0,
            policy: self.end_of_folder,
        })?;
        self.last_navigation_at = Some(Instant::now());
        Ok(())
    }

    fn prev_image(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.can_trigger_navigation() {
            return Ok(());
        }
        self.request_navigation(FilesystemCommand::Prev {
            request_id: 0,
            policy: self.end_of_folder,
        })?;
        self.last_navigation_at = Some(Instant::now());
        Ok(())
    }

    fn first_image(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.can_trigger_navigation() {
            return Ok(());
        }
        self.request_navigation(FilesystemCommand::First { request_id: 0 })?;
        self.last_navigation_at = Some(Instant::now());
        Ok(())
    }

    fn last_image(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.can_trigger_navigation() {
            return Ok(());
        }
        self.request_navigation(FilesystemCommand::Last { request_id: 0 })?;
        self.last_navigation_at = Some(Instant::now());
        Ok(())
    }

    fn can_trigger_navigation(&self) -> bool {
        if self.active_request.is_some() || self.active_fs_request_id.is_some() {
            return false;
        }
        self.last_navigation_at
            .map(|last| last.elapsed() >= NAVIGATION_REPEAT_INTERVAL)
            .unwrap_or(true)
    }

    fn request_load_path(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let load_path = crate::filesystem::resolve_start_path(&path).unwrap_or(path.clone());
        let request_id = self.alloc_request_id();
        self.active_request = Some(ActiveRenderRequest::Load(request_id));
        self.loading_message = Some(format!("Loading {}", load_path.display()));
        self.worker_tx
            .send(RenderCommand::LoadPath {
                request_id,
                path: load_path,
                zoom: self.zoom,
                method: self.render_options.zoom_method,
            })
            .map_err(worker_send_error)?;
        Ok(())
    }

    fn request_resize_current(&mut self) -> Result<(), Box<dyn Error>> {
        if matches!(self.active_request, Some(ActiveRenderRequest::Load(_))) {
            self.pending_resize_after_load = true;
            return Ok(());
        }
        let request_id = self.alloc_request_id();
        self.active_request = Some(ActiveRenderRequest::Resize(request_id));
        self.loading_message = Some(format!("Rendering {:.0}%", self.zoom * 100.0));
        self.worker_tx
            .send(RenderCommand::ResizeCurrent {
                request_id,
                zoom: self.zoom,
                method: self.render_options.zoom_method,
            })
            .map_err(worker_send_error)?;
        Ok(())
    }

    fn alloc_request_id(&mut self) -> u64 {
        self.next_request_id += 1;
        self.next_request_id
    }

    fn alloc_fs_request_id(&mut self) -> u64 {
        self.next_fs_request_id += 1;
        self.next_fs_request_id
    }

    fn init_filesystem(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let request_id = self.alloc_fs_request_id();
        self.active_fs_request_id = Some(request_id);
        self.loading_message = Some(format!("Scanning {}", path.display()));
        self.fs_tx
            .send(FilesystemCommand::Init { request_id, path })
            .map_err(filesystem_send_error)?;
        Ok(())
    }

    fn request_navigation(&mut self, mut command: FilesystemCommand) -> Result<(), Box<dyn Error>> {
        if !self.navigator_ready {
            return Ok(());
        }
        let request_id = self.alloc_fs_request_id();
        self.active_fs_request_id = Some(request_id);
        command = match command {
            FilesystemCommand::Init { path, .. } => FilesystemCommand::Init { request_id, path },
            FilesystemCommand::SetCurrent { path, .. } => {
                FilesystemCommand::SetCurrent { request_id, path }
            }
            FilesystemCommand::Next { policy, .. } => {
                FilesystemCommand::Next { request_id, policy }
            }
            FilesystemCommand::Prev { policy, .. } => {
                FilesystemCommand::Prev { request_id, policy }
            }
            FilesystemCommand::First { .. } => FilesystemCommand::First { request_id },
            FilesystemCommand::Last { .. } => FilesystemCommand::Last { request_id },
        };
        self.loading_message = Some("Scanning folder...".to_string());
        self.fs_tx.send(command).map_err(filesystem_send_error)?;
        Ok(())
    }

    fn poll_worker(&mut self) {
        loop {
            match self.worker_rx.try_recv() {
                Ok(RenderResult::Loaded {
                    request_id,
                    path,
                    source,
                    rendered,
                }) => {
                    let Some(active_request) = self.active_request else {
                        continue;
                    };
                    let request_matches = match active_request {
                        ActiveRenderRequest::Load(active_id)
                        | ActiveRenderRequest::Resize(active_id) => active_id == request_id,
                    };
                    if !request_matches {
                        continue;
                    }
                    if let Some(path) = path {
                        let request_id = self.alloc_fs_request_id();
                        self.current_path = path.clone();
                        let _ = self.fs_tx.send(FilesystemCommand::SetCurrent {
                            request_id,
                            path: self.current_navigation_path.clone(),
                        });
                    }
                    self.source = source;
                    self.rendered = rendered;
                    self.current_frame = self
                        .current_frame
                        .min(self.rendered.frame_count().saturating_sub(1));
                    self.completed_loops = 0;
                    self.last_frame_at = Instant::now();
                    self.upload_current_frame();
                    if self.active_fs_request_id.is_none() {
                        self.loading_message = None;
                    }
                    self.active_request = None;
                    if self.pending_resize_after_load {
                        self.pending_resize_after_load = false;
                        let _ = self.request_resize_current();
                    }
                }
                Ok(RenderResult::Failed {
                    request_id,
                    message,
                }) => {
                    let Some(active_request) = self.active_request else {
                        continue;
                    };
                    let request_matches = match active_request {
                        ActiveRenderRequest::Load(active_id)
                        | ActiveRenderRequest::Resize(active_id) => active_id == request_id,
                    };
                    if !request_matches {
                        continue;
                    }
                    self.loading_message = Some(message);
                    self.active_request = None;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.loading_message = Some("render worker disconnected".to_string());
                    self.active_request = None;
                    break;
                }
            }
        }
    }

    fn poll_filesystem(&mut self) {
        loop {
            match self.fs_rx.try_recv() {
                Ok(FilesystemResult::NavigatorReady { request_id }) => {
                    if self.active_fs_request_id == Some(request_id) {
                        self.navigator_ready = true;
                        if self.active_request.is_none() {
                            self.loading_message = None;
                        }
                        self.active_fs_request_id = None;
                    }
                }
                Ok(FilesystemResult::CurrentSet) => {}
                Ok(FilesystemResult::PathResolved {
                    request_id,
                    navigation_path,
                    load_path,
                }) => {
                    if self.active_fs_request_id == Some(request_id) {
                        self.current_navigation_path = navigation_path;
                        let _ = self.request_load_path(load_path);
                        self.active_fs_request_id = None;
                    }
                }
                Ok(FilesystemResult::NoPath { request_id }) => {
                    if self.active_fs_request_id == Some(request_id) {
                        if self.active_request.is_none() {
                            self.loading_message = None;
                        }
                        self.active_fs_request_id = None;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.loading_message = Some("filesystem worker disconnected".to_string());
                    self.active_fs_request_id = None;
                    break;
                }
            }
        }
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
                    self.window_options.fullscreen = !fullscreen;
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
                ViewerAction::ToggleSettings => {
                    self.show_settings = !self.show_settings;
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

    fn handle_pointer_input(&mut self, ctx: &egui::Context) {
        let double_clicked = ctx.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        });
        if double_clicked {
            let _ = self.toggle_zoom();
        }
    }

    fn settings_ui(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let mut open = self.show_settings;
        let mut reload_requested = false;
        let mut rerender_requested = false;
        let mut zoom_option_changed = false;
        let mut config_changed = false;
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Viewer");
                if ui
                    .checkbox(&mut self.options.animation, "Animation")
                    .changed()
                {
                    config_changed = true;
                }

                ui.horizontal(|ui| {
                    ui.label("End of folder");
                    let before = self.end_of_folder;
                    egui::ComboBox::from_id_salt("end_of_folder")
                        .selected_text(end_of_folder_label(self.end_of_folder))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.end_of_folder,
                                EndOfFolderOption::Stop,
                                "STOP",
                            );
                            ui.selectable_value(
                                &mut self.end_of_folder,
                                EndOfFolderOption::Loop,
                                "LOOP",
                            );
                            ui.selectable_value(
                                &mut self.end_of_folder,
                                EndOfFolderOption::Next,
                                "NEXT",
                            );
                            ui.selectable_value(
                                &mut self.end_of_folder,
                                EndOfFolderOption::Recursive,
                                "RECURSIVE",
                            );
                        });
                    if self.end_of_folder != before {
                        config_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Zoom mode");
                    let before = self.render_options.zoom_option.clone();
                    egui::ComboBox::from_id_salt("zoom_option")
                        .selected_text(zoom_option_label(&self.render_options.zoom_option))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.render_options.zoom_option,
                                ZoomOption::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.render_options.zoom_option,
                                ZoomOption::FitWidth,
                                "FitWidth",
                            );
                            ui.selectable_value(
                                &mut self.render_options.zoom_option,
                                ZoomOption::FitHeight,
                                "FitHeight",
                            );
                            ui.selectable_value(
                                &mut self.render_options.zoom_option,
                                ZoomOption::FitScreen,
                                "FitScreen",
                            );
                            ui.selectable_value(
                                &mut self.render_options.zoom_option,
                                ZoomOption::FitScreenIncludeSmaller,
                                "FitScreenIncludeSmaller",
                            );
                            ui.selectable_value(
                                &mut self.render_options.zoom_option,
                                ZoomOption::FitScreenOnlySmaller,
                                "FitScreenOnlySmaller",
                            );
                        });
                    if self.render_options.zoom_option != before {
                        zoom_option_changed = true;
                        config_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Resize");
                    let before = self.render_options.zoom_method;
                    egui::ComboBox::from_id_salt("zoom_method")
                        .selected_text(interpolation_label(self.render_options.zoom_method))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.render_options.zoom_method,
                                InterpolationAlgorithm::NearestNeighber,
                                "Nearest",
                            );
                            ui.selectable_value(
                                &mut self.render_options.zoom_method,
                                InterpolationAlgorithm::Bilinear,
                                "Bilinear",
                            );
                            ui.selectable_value(
                                &mut self.render_options.zoom_method,
                                InterpolationAlgorithm::BicubicAlpha(None),
                                "Bicubic",
                            );
                            ui.selectable_value(
                                &mut self.render_options.zoom_method,
                                InterpolationAlgorithm::Lanzcos3,
                                "Lanczos3",
                            );
                        });
                    if self.render_options.zoom_method != before {
                        rerender_requested = true;
                        config_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Background");
                    if ui.button("Black").clicked() {
                        self.options.background = BackgroundStyle::Solid([0, 0, 0, 255]);
                        config_changed = true;
                    }
                    if ui.button("Gray").clicked() {
                        self.options.background = BackgroundStyle::Solid([48, 48, 48, 255]);
                        config_changed = true;
                    }
                    if ui.button("Tile").clicked() {
                        self.options.background = BackgroundStyle::Tile {
                            color1: [32, 32, 32, 255],
                            color2: [80, 80, 80, 255],
                            size: 16,
                        };
                        config_changed = true;
                    }
                });

                ui.separator();
                if ui.button("Reload current").clicked() {
                    reload_requested = true;
                }
            });
        self.show_settings = open;
        if zoom_option_changed {
            self.pending_fit_recalc = true;
        }
        if rerender_requested {
            let _ = self.request_resize_current();
        }
        if reload_requested {
            let _ = self.reload_current();
        }
        if config_changed {
            let _ = save_app_config(
                &self.current_config(),
                Some(&self.current_path),
                self.config_path.as_deref(),
            );
        }
    }

    fn current_config(&self) -> AppConfig {
        AppConfig {
            viewer: self.options.clone(),
            window: self.window_options.clone(),
            render: self.render_options.clone(),
            input: Default::default(),
            navigation: NavigationOptions {
                end_of_folder: self.end_of_folder,
                sort: self.navigation_sort,
            },
        }
    }

    fn sync_window_state(&mut self, ctx: &egui::Context) {
        let viewport = ctx.input(|i| i.viewport().clone());

        if let Some(fullscreen) = viewport.fullscreen {
            self.window_options.fullscreen = fullscreen;
        }

        if self.window_options.fullscreen {
            return;
        }

        if let Some(inner_rect) = viewport.inner_rect {
            self.window_options.size = crate::ui::viewer::options::WindowSize::Exact {
                width: inner_rect.width(),
                height: inner_rect.height(),
            };
        }

        if let Some(outer_rect) = viewport.outer_rect {
            self.window_options.start_position = WindowStartPosition::Exact {
                x: outer_rect.min.x,
                y: outer_rect.min.y,
            };
        }
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_window_state(ctx);
        self.update_window_title(ctx);
        self.poll_worker();
        self.poll_filesystem();
        self.handle_keyboard(ctx);
        self.handle_pointer_input(ctx);
        self.settings_ui(ctx);

        let zoom_delta = ctx.input(|i| i.zoom_delta());

        if zoom_delta != 1.0 {
            let _ = self.set_zoom(self.zoom * zoom_delta);
        }

        self.frame_counter += 1;
        self.update_animation(ctx);

        let panel = egui::CentralPanel::default().frame(egui::Frame::NONE);
        panel.show(ctx, |ui| {
            self.paint_background(ui, ui.max_rect());
            if self.active_request.is_some() || self.active_fs_request_id.is_some() {
                ctx.request_repaint_after(Duration::from_millis(16));
            }

            let viewport = ui.available_size();

            if (viewport != self.last_viewport_size || self.pending_fit_recalc)
                && !matches!(self.render_options.zoom_option, ZoomOption::None)
            {
                self.last_viewport_size = viewport;
                self.pending_fit_recalc = false;

                let new_zoom = calc_fit_zoom(
                    viewport,
                    self.source_size(),
                    &self.render_options.zoom_option,
                );
                self.fit_zoom = new_zoom.clamp(0.1, 16.0);
                let _ = self.set_zoom(new_zoom);
            }

            let draw_size = vec2(
                self.current_canvas().width() as f32 * self.texture_display_scale,
                self.current_canvas().height() as f32 * self.texture_display_scale,
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

                    if let Some(message) = &self.loading_message {
                        ui.add_space(8.0);
                        ui.label(message);
                    }
                });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = save_app_config(
            &self.current_config(),
            Some(&self.current_path),
            self.config_path.as_deref(),
        );
    }
}

fn canvas_to_color_image(canvas: &Canvas) -> ColorImage {
    ColorImage::from_rgba_unmultiplied(
        [canvas.width() as usize, canvas.height() as usize],
        canvas.buffer(),
    )
}

fn downscale_for_texture_limit<'a>(
    canvas: &'a Canvas,
    max_texture_side: usize,
    method: InterpolationAlgorithm,
) -> (std::borrow::Cow<'a, Canvas>, f32) {
    let width = canvas.width() as usize;
    let height = canvas.height() as usize;
    let max_side = width.max(height);
    if max_side <= max_texture_side || max_texture_side == 0 {
        return (std::borrow::Cow::Borrowed(canvas), 1.0);
    }

    let scale = max_texture_side as f32 / max_side as f32;
    match resize_canvas(canvas, scale, method) {
        Ok(resized) => (std::borrow::Cow::Owned(resized), scale),
        Err(_) => (std::borrow::Cow::Borrowed(canvas), 1.0),
    }
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
        "P" => Some(egui::Key::P),
        _ => None,
    }
}

fn end_of_folder_label(option: EndOfFolderOption) -> &'static str {
    match option {
        EndOfFolderOption::Stop => "STOP",
        EndOfFolderOption::Next => "NEXT",
        EndOfFolderOption::Loop => "LOOP",
        EndOfFolderOption::Recursive => "RECURSIVE",
    }
}

fn zoom_option_label(option: &ZoomOption) -> &'static str {
    match option {
        ZoomOption::None => "None",
        ZoomOption::FitWidth => "FitWidth",
        ZoomOption::FitHeight => "FitHeight",
        ZoomOption::FitScreen => "FitScreen",
        ZoomOption::FitScreenIncludeSmaller => "FitScreenIncludeSmaller",
        ZoomOption::FitScreenOnlySmaller => "FitScreenOnlySmaller",
    }
}

fn interpolation_label(method: InterpolationAlgorithm) -> &'static str {
    match method {
        InterpolationAlgorithm::NearestNeighber => "Nearest",
        InterpolationAlgorithm::Bilinear => "Bilinear",
        InterpolationAlgorithm::Bicubic => "Bicubic",
        InterpolationAlgorithm::BicubicAlpha(_) => "Bicubic",
        InterpolationAlgorithm::Lanzcos3 => "Lanczos3",
        InterpolationAlgorithm::Lanzcos(_) => "Lanczos",
    }
}

fn spawn_render_worker(
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
                        let source = crate::drawers::image::load_canvas_from_file(&path)?;
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

fn worker_send_error(err: mpsc::SendError<RenderCommand>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(err.to_string()))
}

fn filesystem_send_error(err: mpsc::SendError<FilesystemCommand>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(err.to_string()))
}
