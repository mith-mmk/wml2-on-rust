use crate::configs::config::save_app_config;
use crate::configs::resourses::{AppliedResources, apply_resources};
use crate::drawers::canvas::Canvas;
use crate::drawers::image::{LoadedImage, SaveFormat, save_loaded_image};
use crate::filesystem::{
    FilesystemCommand, FilesystemResult, adjacent_entry, spawn_filesystem_worker,
};
use crate::options::{
    AppConfig, EndOfFolderOption, KeyBinding, NavigationSortOption, ResourceOptions, ViewerAction,
};
use crate::ui::i18n::{UiTextKey, tr};
use crate::ui::menu::fileviewer::state::FilerState;
use crate::ui::menu::fileviewer::worker::{FilerCommand, FilerResult, spawn_filer_worker};
use crate::ui::render::{
    ActiveRenderRequest, RenderCommand, RenderResult, aligned_offset, canvas_to_color_image,
    downscale_for_texture_limit, spawn_render_worker, worker_send_error,
};
use crate::ui::viewer::options::{
    RenderOptions, ViewerOptions, WindowOptions, WindowStartPosition,
};
use eframe::egui::{self, Pos2, TextureHandle, TextureOptions, vec2};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};
pub mod options;
use options::ZoomOption;

const NAVIGATION_REPEAT_INTERVAL: Duration = Duration::from_millis(180);

pub(crate) struct ViewerApp {
    pub(crate) current_navigation_path: PathBuf,
    pub(crate) current_path: PathBuf,
    pub(crate) source: LoadedImage,
    pub(crate) rendered: LoadedImage,
    pub(crate) texture: TextureHandle,
    pub(crate) egui_ctx: egui::Context,

    pub(crate) zoom: f32,

    pub(crate) current_frame: usize,
    pub(crate) last_frame_at: Instant,
    pub(crate) completed_loops: u32,

    pub(crate) fit_zoom: f32,
    pub(crate) last_viewport_size: egui::Vec2,
    pub(crate) frame_counter: usize,

    pub(crate) render_options: RenderOptions,
    pub(crate) options: ViewerOptions,
    pub(crate) window_options: WindowOptions,
    pub(crate) resources: ResourceOptions,
    pub(crate) applied_locale: String,
    pub(crate) loaded_font_names: Vec<String>,
    pub(crate) keymap: HashMap<KeyBinding, ViewerAction>,
    pub(crate) end_of_folder: EndOfFolderOption,
    pub(crate) navigation_sort: NavigationSortOption,
    pub(crate) worker_tx: Sender<RenderCommand>,
    pub(crate) worker_rx: Receiver<RenderResult>,
    pub(crate) next_request_id: u64,
    pub(crate) active_request: Option<ActiveRenderRequest>,
    pub(crate) fs_tx: Sender<FilesystemCommand>,
    pub(crate) fs_rx: Receiver<FilesystemResult>,
    pub(crate) next_fs_request_id: u64,
    pub(crate) active_fs_request_id: Option<u64>,
    pub(crate) filer_tx: Sender<FilerCommand>,
    pub(crate) filer_rx: Receiver<FilerResult>,
    pub(crate) next_filer_request_id: u64,
    pub(crate) navigator_ready: bool,
    pub(crate) loading_message: Option<String>,
    pub(crate) last_navigation_at: Option<Instant>,
    pub(crate) show_settings: bool,
    pub(crate) max_texture_side: usize,
    pub(crate) texture_display_scale: f32,
    pub(crate) pending_resize_after_load: bool,
    pub(crate) pending_fit_recalc: bool,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) show_left_menu: bool,
    pub(crate) left_menu_pos: Pos2,
    pub(crate) save_format: SaveFormat,
    pub(crate) save_message: Option<String>,
    pub(crate) show_filer: bool,
    pub(crate) filer: FilerState,
    pub(crate) startup_window_sync_frames: usize,
    pub(crate) empty_mode: bool,
    pub(crate) companion_tx: Sender<RenderCommand>,
    pub(crate) companion_rx: Receiver<RenderResult>,
    pub(crate) companion_active_request: Option<ActiveRenderRequest>,
    pub(crate) companion_navigation_path: Option<PathBuf>,
    pub(crate) companion_rendered: Option<LoadedImage>,
    pub(crate) companion_texture: Option<TextureHandle>,
    pub(crate) companion_texture_display_scale: f32,
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
        show_filer_on_start: bool,
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
        let AppliedResources {
            locale,
            loaded_fonts,
        } = apply_resources(&cc.egui_ctx, &config.resources);
        let (worker_tx, worker_rx) = spawn_render_worker(source.clone());
        let (companion_tx, companion_rx) = spawn_render_worker(source.clone());
        let (fs_tx, fs_rx) = spawn_filesystem_worker(config.navigation.sort);
        let (filer_tx, filer_rx) = spawn_filer_worker();

        let mut this = Self {
            current_navigation_path: navigation_path.clone(),
            current_path: path.clone(),
            source,
            rendered,
            texture,
            egui_ctx: cc.egui_ctx.clone(),

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
            resources: config.resources,
            applied_locale: locale,
            loaded_font_names: loaded_fonts,
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
            filer_tx,
            filer_rx,
            next_filer_request_id: 0,
            navigator_ready: false,
            loading_message: None,
            last_navigation_at: None,
            show_settings: false,
            max_texture_side: cc.egui_ctx.input(|i| i.max_texture_side),
            texture_display_scale: 1.0,
            pending_resize_after_load: false,
            pending_fit_recalc: false,
            config_path,
            show_left_menu: false,
            left_menu_pos: Pos2::ZERO,
            save_format: SaveFormat::Png,
            save_message: None,
            show_filer: show_filer_on_start,
            filer: FilerState::default(),
            startup_window_sync_frames: 0,
            empty_mode: show_filer_on_start,
            companion_tx,
            companion_rx,
            companion_active_request: None,
            companion_navigation_path: None,
            companion_rendered: None,
            companion_texture: None,
            companion_texture_display_scale: 1.0,
        };

        let _ = this.init_filesystem(navigation_path);
        if let Some(dir) = this.current_directory() {
            this.request_filer_directory(dir, Some(this.current_navigation_path.clone()));
        }
        this
    }

    fn source_size(&self) -> egui::Vec2 {
        vec2(
            self.source.canvas.width() as f32,
            self.source.canvas.height() as f32,
        )
    }

    pub(crate) fn text(&self, key: UiTextKey) -> &'static str {
        tr(&self.applied_locale, key)
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

    pub(crate) fn toggle_zoom(&mut self) -> Result<(), Box<dyn Error>> {
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

    pub(crate) fn reload_current(&mut self) -> Result<(), Box<dyn Error>> {
        self.request_load_path(self.current_navigation_path.clone())
    }

    fn current_directory(&self) -> Option<PathBuf> {
        if self.current_navigation_path.is_dir() {
            return Some(self.current_navigation_path.clone());
        }
        if let Some(parent) = self.current_navigation_path.parent() {
            let marker = parent.file_name().and_then(|name| name.to_str());
            if matches!(marker, Some("__wmlv__" | "__zipv__")) {
                return parent
                    .parent()
                    .and_then(|path| path.parent())
                    .map(|path| path.to_path_buf());
            }
            return Some(parent.to_path_buf());
        }
        self.current_path.parent().map(|path| path.to_path_buf())
    }

    pub(crate) fn request_filer_directory(&mut self, dir: PathBuf, selected: Option<PathBuf>) {
        let request_id = self.alloc_filer_request_id();
        self.filer.pending_request_id = Some(request_id);
        let _ = self.filer_tx.send(FilerCommand::OpenDirectory {
            request_id,
            dir,
            sort: self.navigation_sort,
            selected,
        });
    }

    pub(crate) fn save_current_as(&mut self, format: SaveFormat) {
        let Some(parent) = self.current_path.parent() else {
            self.save_message = Some("Cannot determine save directory".to_string());
            return;
        };

        let stem = self
            .current_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("image");
        let output = parent.join(format!("{stem}.{}", format.extension()));
        match save_loaded_image(&output, &self.source, format) {
            Ok(()) => {
                self.save_message = Some(format!("Saved {}", output.display()));
            }
            Err(err) => {
                self.save_message = Some(format!("Save failed: {err}"));
            }
        }
    }

    fn is_current_portrait_page(&self) -> bool {
        self.source.canvas.height() >= self.source.canvas.width()
    }

    fn desired_manga_companion_path(&self) -> Option<PathBuf> {
        if !self.options.manga_mode || self.empty_mode || !self.is_current_portrait_page() {
            return None;
        }

        adjacent_entry(&self.current_navigation_path, self.navigation_sort, 1)
    }

    fn manga_spread_active(&self) -> bool {
        self.options.manga_mode
            && self.is_current_portrait_page()
            && self.companion_navigation_path.is_some()
            && self
                .companion_rendered
                .as_ref()
                .map(|image| image.canvas.height() >= image.canvas.width())
                .unwrap_or(false)
    }

    fn request_companion_load(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let request_id = self.alloc_request_id();
        self.companion_active_request = Some(ActiveRenderRequest::Load(request_id));
        self.companion_navigation_path = Some(path.clone());
        let load_path = crate::filesystem::resolve_start_path(&path).unwrap_or(path);
        self.companion_tx
            .send(RenderCommand::LoadPath {
                request_id,
                path: load_path,
                zoom: self.zoom,
                method: self.render_options.zoom_method,
            })
            .map_err(worker_send_error)?;
        Ok(())
    }

    fn sync_manga_companion(&mut self, ctx: &egui::Context) {
        let desired = self.desired_manga_companion_path();
        if desired == self.companion_navigation_path && self.companion_rendered.is_some() {
            return;
        }

        if desired.is_none() {
            self.companion_navigation_path = None;
            self.companion_rendered = None;
            self.companion_texture = None;
            self.companion_active_request = None;
            return;
        }

        if self.companion_active_request.is_none() {
            let _ = self.request_companion_load(desired.unwrap());
            ctx.request_repaint();
        }
    }

    fn manga_navigation_target(&self, forward: bool) -> Option<PathBuf> {
        if !self.manga_spread_active() {
            return None;
        }

        let step = if forward { 2 } else { -2 };
        adjacent_entry(&self.current_navigation_path, self.navigation_sort, step)
    }

    pub(crate) fn next_image(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.can_trigger_navigation() {
            return Ok(());
        }
        if let Some(target) = self.manga_navigation_target(true) {
            self.current_navigation_path = target.clone();
            self.request_load_path(target)?;
            self.last_navigation_at = Some(Instant::now());
            return Ok(());
        }
        self.request_navigation(FilesystemCommand::Next {
            request_id: 0,
            policy: self.end_of_folder,
        })?;
        self.last_navigation_at = Some(Instant::now());
        Ok(())
    }

    pub(crate) fn prev_image(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.can_trigger_navigation() {
            return Ok(());
        }
        if let Some(target) = self.manga_navigation_target(false) {
            self.current_navigation_path = target.clone();
            self.request_load_path(target)?;
            self.last_navigation_at = Some(Instant::now());
            return Ok(());
        }
        self.request_navigation(FilesystemCommand::Prev {
            request_id: 0,
            policy: self.end_of_folder,
        })?;
        self.last_navigation_at = Some(Instant::now());
        Ok(())
    }

    pub(crate) fn first_image(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.can_trigger_navigation() {
            return Ok(());
        }
        self.request_navigation(FilesystemCommand::First { request_id: 0 })?;
        self.last_navigation_at = Some(Instant::now());
        Ok(())
    }

    pub(crate) fn last_image(&mut self) -> Result<(), Box<dyn Error>> {
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

    pub(crate) fn request_load_path(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
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

    pub(crate) fn request_resize_current(&mut self) -> Result<(), Box<dyn Error>> {
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
        if let Some(path) = self.companion_navigation_path.clone() {
            let _ = self.request_companion_load(path);
        }
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

    fn alloc_filer_request_id(&mut self) -> u64 {
        self.next_filer_request_id += 1;
        self.next_filer_request_id
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
                        if let Some(dir) = self.current_directory() {
                            self.request_filer_directory(
                                dir,
                                Some(self.current_navigation_path.clone()),
                            );
                        }
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

    fn poll_companion_worker(&mut self) {
        loop {
            match self.companion_rx.try_recv() {
                Ok(RenderResult::Loaded {
                    request_id,
                    path: _,
                    source: _,
                    rendered,
                }) => {
                    let Some(active_request) = self.companion_active_request else {
                        continue;
                    };
                    let request_matches = match active_request {
                        ActiveRenderRequest::Load(active_id)
                        | ActiveRenderRequest::Resize(active_id) => active_id == request_id,
                    };
                    if !request_matches {
                        continue;
                    }

                    let (canvas, display_scale) = downscale_for_texture_limit(
                        rendered.frame_canvas(0),
                        self.max_texture_side,
                        self.render_options.zoom_method,
                    );
                    let image = canvas_to_color_image(&canvas);
                    let texture = if let Some(texture) = &mut self.companion_texture {
                        texture.set(image, TextureOptions::LINEAR);
                        texture.clone()
                    } else {
                        self.egui_ctx
                            .load_texture("manga_companion", image, TextureOptions::LINEAR)
                    };
                    self.companion_texture = Some(texture);
                    self.companion_rendered = Some(rendered);
                    self.companion_texture_display_scale = display_scale;
                    self.companion_active_request = None;
                }
                Ok(RenderResult::Failed { request_id, .. }) => {
                    let Some(active_request) = self.companion_active_request else {
                        continue;
                    };
                    let request_matches = match active_request {
                        ActiveRenderRequest::Load(active_id)
                        | ActiveRenderRequest::Resize(active_id) => active_id == request_id,
                    };
                    if request_matches {
                        self.companion_rendered = None;
                        self.companion_texture = None;
                        self.companion_active_request = None;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.companion_rendered = None;
                    self.companion_texture = None;
                    self.companion_active_request = None;
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
                        self.loading_message = Some("No displayable file found".to_string());
                        self.show_filer = true;
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

    fn poll_filer_worker(&mut self) {
        loop {
            match self.filer_rx.try_recv() {
                Ok(FilerResult::Snapshot {
                    request_id,
                    directory,
                    entries,
                    navigation_entries,
                    selected,
                }) => {
                    if self.filer.pending_request_id != Some(request_id) {
                        continue;
                    }
                    self.filer.pending_request_id = None;
                    self.filer.directory = Some(directory);
                    self.filer.entries = entries;
                    self.filer.navigation_entries = navigation_entries;
                    self.filer.selected = selected;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.filer.pending_request_id = None;
                    break;
                }
            }
        }
    }

    fn sync_window_state(&mut self, ctx: &egui::Context) {
        let viewport = ctx.input(|i| i.viewport().clone());
        self.startup_window_sync_frames += 1;

        if let Some(fullscreen) = viewport.fullscreen {
            self.window_options.fullscreen = fullscreen;
        }

        if self.window_options.fullscreen || self.startup_window_sync_frames < 20 {
            return;
        }

        if self.window_options.remember_size {
            if let Some(inner_rect) = viewport.inner_rect {
                self.window_options.size = crate::ui::viewer::options::WindowSize::Exact {
                    width: inner_rect.width(),
                    height: inner_rect.height(),
                };
            }
        }

        if self.window_options.remember_position {
            if let Some(outer_rect) = viewport.outer_rect {
                self.window_options.start_position = WindowStartPosition::Exact {
                    x: outer_rect.min.x,
                    y: outer_rect.min.y,
                };
            }
        }
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_window_state(ctx);
        self.update_window_title(ctx);
        self.poll_worker();
        self.poll_companion_worker();
        self.poll_filesystem();
        self.poll_filer_worker();
        self.sync_manga_companion(ctx);
        self.handle_keyboard(ctx);
        self.settings_ui(ctx);
        self.left_click_menu_ui(ctx);
        self.filer_ui(ctx);

        let zoom_delta = ctx.input(|i| i.zoom_delta());

        if zoom_delta != 1.0 && !self.show_settings {
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

            if !self.empty_mode
                && (viewport != self.last_viewport_size || self.pending_fit_recalc)
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
                    let spread_active = self.manga_spread_active();
                    let companion = self
                        .companion_rendered
                        .as_ref()
                        .zip(self.companion_texture.as_ref());

                    let companion_draw_size = companion.map(|(companion_rendered, _)| {
                        vec2(
                            companion_rendered.canvas.width() as f32
                                * self.companion_texture_display_scale,
                            companion_rendered.canvas.height() as f32
                                * self.companion_texture_display_scale,
                        )
                    });
                    let total_draw_size = if spread_active {
                        if let Some(companion_draw_size) = companion_draw_size {
                            vec2(
                                draw_size.x + companion_draw_size.x,
                                draw_size.y.max(companion_draw_size.y),
                            )
                        } else {
                            draw_size
                        }
                    } else {
                        draw_size
                    };
                    let offset = aligned_offset(viewport, total_draw_size, self.options.align);

                    ui.add_space(offset.y.max(0.0));

                    let inner = ui.horizontal(|ui| {
                        ui.add_space(offset.x.max(0.0));
                        if spread_active {
                            if let Some((_, companion_texture)) = companion {
                                let companion_draw_size = companion_draw_size.unwrap_or(draw_size);
                                let draw_companion_first = self.options.manga_right_to_left;
                                if draw_companion_first {
                                    let first = ui.add(
                                        egui::Image::from_texture(companion_texture)
                                            .fit_to_exact_size(companion_draw_size),
                                    );
                                    ui.add(
                                        egui::Image::from_texture(&self.texture)
                                            .fit_to_exact_size(draw_size),
                                    );
                                    Some(first)
                                } else {
                                    let first = ui.add(
                                        egui::Image::from_texture(&self.texture)
                                            .fit_to_exact_size(draw_size),
                                    );
                                    ui.add(
                                        egui::Image::from_texture(companion_texture)
                                            .fit_to_exact_size(companion_draw_size),
                                    );
                                    Some(first)
                                }
                            } else {
                                Some(
                                    ui.add(
                                        egui::Image::from_texture(&self.texture)
                                            .fit_to_exact_size(draw_size),
                                    ),
                                )
                            }
                        } else {
                            Some(
                                ui.add(
                                    egui::Image::from_texture(&self.texture)
                                        .fit_to_exact_size(draw_size),
                                ),
                            )
                        }
                    });
                    if let Some(response) = inner.inner {
                        self.handle_pointer_input(&response);
                    }

                    if let Some(message) = &self.loading_message {
                        ui.add_space(8.0);
                        ui.label(message);
                    }
                    if self.empty_mode {
                        ui.add_space(8.0);
                        ui.label(format!(
                            "{} {}",
                            self.text(UiTextKey::NoDisplayableFileFound),
                            self.text(UiTextKey::OpenDirectoryOrFileFromFiler)
                        ));
                    }
                    if let Some(message) = &self.save_message {
                        ui.add_space(4.0);
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

fn filesystem_send_error(err: mpsc::SendError<FilesystemCommand>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(err.to_string()))
}
