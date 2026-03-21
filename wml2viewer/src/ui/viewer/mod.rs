use crate::configs::config::save_app_config;
use crate::configs::resourses::{AppliedResources, apply_resources};
use crate::dependent::{default_download_dir, pick_save_directory};
use crate::drawers::canvas::Canvas;
use crate::drawers::image::{LoadedImage, SaveFormat, save_loaded_image};
use crate::filesystem::{
    FilesystemCommand, FilesystemResult, adjacent_entry, archive_prefers_low_io,
    set_archive_zip_workaround, spawn_filesystem_worker,
};
use crate::options::{
    AppConfig, EndOfFolderOption, KeyBinding, NavigationSortOption, PluginConfig, ResourceOptions,
    RuntimeOptions, ViewerAction,
};
use crate::ui::i18n::{UiTextKey, tr};
use crate::ui::menu::fileviewer::state::FilerState;
use crate::ui::menu::fileviewer::thumbnail::{
    ThumbnailCommand, ThumbnailResult, set_thumbnail_workaround, spawn_thumbnail_worker,
};
use crate::ui::menu::fileviewer::worker::{FilerCommand, FilerResult, spawn_filer_worker};
use crate::ui::render::{
    ActiveRenderRequest, RenderCommand, RenderResult, aligned_offset, canvas_to_color_image,
    downscale_for_texture_limit, spawn_render_worker, worker_send_error,
};
use crate::ui::viewer::options::{
    RenderOptions, ViewerOptions, WindowOptions, WindowStartPosition, WindowUiTheme,
};
use eframe::egui::{self, Pos2, TextureHandle, TextureOptions, vec2};
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};
pub mod options;
mod state;
use options::ZoomOption;
use state::{SaveDialogState, ViewerOverlayState};

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
    pub(crate) plugins: PluginConfig,
    pub(crate) storage: crate::options::StorageOptions,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) applied_locale: String,
    pub(crate) loaded_font_names: Vec<String>,
    pub(crate) resource_font_paths_input: String,
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
    pub(crate) thumbnail_tx: Sender<ThumbnailCommand>,
    pub(crate) thumbnail_rx: Receiver<ThumbnailResult>,
    pub(crate) next_thumbnail_request_id: u64,
    pub(crate) thumbnail_pending: HashSet<PathBuf>,
    pub(crate) thumbnail_cache: HashMap<PathBuf, TextureHandle>,
    pub(crate) navigator_ready: bool,
    pub(crate) overlay: ViewerOverlayState,
    pub(crate) last_navigation_at: Option<Instant>,
    pub(crate) show_settings: bool,
    pub(crate) show_restart_prompt: bool,
    pub(crate) settings_tab: SettingsTab,
    pub(crate) max_texture_side: usize,
    pub(crate) texture_display_scale: f32,
    pub(crate) pending_resize_after_load: bool,
    pub(crate) pending_fit_recalc: bool,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) show_left_menu: bool,
    pub(crate) left_menu_pos: Pos2,
    pub(crate) save_dialog: SaveDialogState,
    pub(crate) show_filer: bool,
    pub(crate) show_subfiler: bool,
    pub(crate) filer: FilerState,
    pub(crate) susie64_search_paths_input: String,
    pub(crate) system_search_paths_input: String,
    pub(crate) ffmpeg_search_paths_input: String,
    pub(crate) startup_window_sync_frames: usize,
    pub(crate) empty_mode: bool,
    pub(crate) companion_tx: Sender<RenderCommand>,
    pub(crate) companion_rx: Receiver<RenderResult>,
    pub(crate) companion_active_request: Option<ActiveRenderRequest>,
    pub(crate) companion_navigation_path: Option<PathBuf>,
    pub(crate) companion_source: Option<LoadedImage>,
    pub(crate) companion_rendered: Option<LoadedImage>,
    pub(crate) companion_texture: Option<TextureHandle>,
    pub(crate) companion_texture_display_scale: f32,
    pub(crate) preload_tx: Sender<RenderCommand>,
    pub(crate) preload_rx: Receiver<RenderResult>,
    pub(crate) next_preload_request_id: u64,
    pub(crate) active_preload_request_id: Option<u64>,
    pub(crate) pending_preload_navigation_path: Option<PathBuf>,
    pub(crate) preloaded_navigation_path: Option<PathBuf>,
    pub(crate) preloaded_load_path: Option<PathBuf>,
    pub(crate) preloaded_source: Option<LoadedImage>,
    pub(crate) preloaded_rendered: Option<LoadedImage>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    Viewer,
    Plugins,
    Resources,
    Render,
    Window,
    Navigation,
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

fn viewport_size_changed(current: egui::Vec2, previous: egui::Vec2) -> bool {
    if previous == egui::Vec2::ZERO {
        return true;
    }
    (current.x - previous.x).abs() > 1.0 || (current.y - previous.y).abs() > 1.0
}

fn default_save_file_name(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("image")
        .to_string()
}

fn ellipsize_end(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.to_string();
    }
    let head = chars.iter().take(max_chars.saturating_sub(3)).collect::<String>();
    format!("{head}...")
}

fn format_key_binding(binding: &KeyBinding) -> String {
    let mut parts = Vec::new();
    if binding.ctrl {
        parts.push("Ctrl");
    }
    if binding.shift {
        parts.push("Shift");
    }
    if binding.alt {
        parts.push("Alt");
    }
    parts.push(&binding.key);
    parts.join("+")
}

pub(crate) fn join_search_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) fn parse_search_paths(input: &str) -> Vec<PathBuf> {
    input
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(PathBuf::from)
        .collect()
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
        startup_load_path: Option<PathBuf>,
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
        set_archive_zip_workaround(config.runtime.workaround.archive.zip.clone());
        set_thumbnail_workaround(config.runtime.workaround.thumbnail.clone());
        let (worker_tx, worker_rx) = spawn_render_worker(source.clone());
        let (companion_tx, companion_rx) = spawn_render_worker(source.clone());
        let (preload_tx, preload_rx) = spawn_render_worker(source.clone());
        let (fs_tx, fs_rx) = spawn_filesystem_worker(config.navigation.sort);
        let (filer_tx, filer_rx) = spawn_filer_worker();
        let (thumbnail_tx, thumbnail_rx) = spawn_thumbnail_worker();
        let resource_font_paths_input = join_search_paths(&config.resources.font_paths);

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
            plugins: config.plugins,
            storage: config.storage,
            runtime: config.runtime,
            applied_locale: locale,
            loaded_font_names: loaded_fonts,
            resource_font_paths_input,
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
            thumbnail_tx,
            thumbnail_rx,
            next_thumbnail_request_id: 0,
            thumbnail_pending: HashSet::new(),
            thumbnail_cache: HashMap::new(),
            navigator_ready: false,
            overlay: ViewerOverlayState::default(),
            last_navigation_at: None,
            show_settings: false,
            show_restart_prompt: false,
            settings_tab: SettingsTab::Viewer,
            max_texture_side: cc.egui_ctx.input(|i| i.max_texture_side),
            texture_display_scale: 1.0,
            pending_resize_after_load: false,
            pending_fit_recalc: false,
            config_path,
            show_left_menu: false,
            left_menu_pos: Pos2::ZERO,
            save_dialog: SaveDialogState {
                file_name: default_save_file_name(&path),
                ..SaveDialogState::default()
            },
            show_filer: show_filer_on_start,
            show_subfiler: false,
            filer: FilerState::default(),
            susie64_search_paths_input: String::new(),
            system_search_paths_input: String::new(),
            ffmpeg_search_paths_input: String::new(),
            startup_window_sync_frames: 0,
            empty_mode: show_filer_on_start,
            companion_tx,
            companion_rx,
            companion_active_request: None,
            companion_navigation_path: None,
            companion_source: None,
            companion_rendered: None,
            companion_texture: None,
            companion_texture_display_scale: 1.0,
            preload_tx,
            preload_rx,
            next_preload_request_id: 0,
            active_preload_request_id: None,
            pending_preload_navigation_path: None,
            preloaded_navigation_path: None,
            preloaded_load_path: None,
            preloaded_source: None,
            preloaded_rendered: None,
        };

        this.save_dialog.output_dir = this
            .storage
            .path
            .clone()
            .or_else(default_download_dir)
            .or_else(|| path.parent().map(|parent| parent.to_path_buf()));
        this.susie64_search_paths_input = join_search_paths(&this.plugins.susie64.search_path);
        this.system_search_paths_input = join_search_paths(&this.plugins.system.search_path);
        this.ffmpeg_search_paths_input = join_search_paths(&this.plugins.ffmpeg.search_path);
        this.apply_window_theme(&cc.egui_ctx);

        let _ = this.init_filesystem(navigation_path);
        if let Some(dir) = this.current_directory() {
            this.request_filer_directory(dir, Some(this.current_navigation_path.clone()));
        }
        if let Some(path) = startup_load_path {
            let _ = this.request_load_path(path);
        }
        this
    }

    fn source_size(&self) -> egui::Vec2 {
        vec2(
            self.source.canvas.width() as f32,
            self.source.canvas.height() as f32,
        )
    }

    fn fit_target_size(&self) -> egui::Vec2 {
        if self.manga_spread_active() {
            if let Some(companion) = &self.companion_source {
                let separator = self.options.manga_separator.pixels.max(0.0);
                return vec2(
                    self.source.canvas.width() as f32 + companion.canvas.width() as f32 + separator,
                    self.source.canvas.height().max(companion.canvas.height()) as f32,
                );
            }
        }

        self.source_size()
    }

    fn paint_manga_separator(&self, ui: &mut egui::Ui, height: f32) {
        let width = self.options.manga_separator.pixels.max(0.0);
        if width <= 0.0 {
            return;
        }

        let (rect, _) = ui.allocate_exact_size(vec2(width, height.max(1.0)), egui::Sense::hover());
        match self.options.manga_separator.style {
            crate::ui::viewer::options::MangaSeparatorStyle::None => {}
            crate::ui::viewer::options::MangaSeparatorStyle::Solid => {
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(
                        self.options.manga_separator.color[0],
                        self.options.manga_separator.color[1],
                        self.options.manga_separator.color[2],
                        self.options.manga_separator.color[3],
                    ),
                );
            }
            crate::ui::viewer::options::MangaSeparatorStyle::Shadow => {
                let base = self.options.manga_separator.color;
                let steps = width.max(2.0) as usize;
                for step in 0..steps {
                    let t = (step as f32 + 0.5) / steps as f32;
                    let alpha = (1.0 - ((t - 0.5).abs() * 2.0)).max(0.0) * (base[3] as f32);
                    let x0 = rect.left() + (step as f32 / steps as f32) * rect.width();
                    let x1 = rect.left() + ((step + 1) as f32 / steps as f32) * rect.width();
                    let band = egui::Rect::from_min_max(
                        egui::pos2(x0, rect.top()),
                        egui::pos2(x1, rect.bottom()),
                    );
                    ui.painter().rect_filled(
                        band,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(
                            base[0],
                            base[1],
                            base[2],
                            alpha.round().clamp(0.0, 255.0) as u8,
                        ),
                    );
                }
            }
        }
    }

    pub(crate) fn text(&self, key: UiTextKey) -> &'static str {
        tr(&self.applied_locale, key)
    }

    pub(crate) fn apply_window_theme(&self, ctx: &egui::Context) {
        match self.window_options.ui_theme {
            WindowUiTheme::System => {}
            WindowUiTheme::Light => ctx.set_visuals(egui::Visuals::light()),
            WindowUiTheme::Dark => ctx.set_visuals(egui::Visuals::dark()),
        }
    }

    pub(crate) fn open_help(&self) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("help.html");
        let _ = std::fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new(".")));
        let mut bindings = self
            .keymap
            .iter()
            .map(|(binding, action)| (format_key_binding(binding), format!("{action:?}")))
            .collect::<Vec<_>>();
        bindings.sort_by(|left, right| left.0.cmp(&right.0));

        let rows = bindings
            .into_iter()
            .map(|(binding, action)| format!("<tr><td>{binding}</td><td>{action}</td></tr>"))
            .collect::<Vec<_>>()
            .join("\n");
        let html = format!(
            r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>wml2viewer Help</title>
  <style>
    body {{ font-family: sans-serif; margin: 32px; line-height: 1.5; }}
    table {{ border-collapse: collapse; width: 100%; }}
    th, td {{ border: 1px solid #ccc; padding: 8px 10px; text-align: left; }}
    code {{ background: #f4f4f4; padding: 2px 4px; border-radius: 4px; }}
  </style>
</head>
<body>
  <h1>wml2viewer Help</h1>
  <h2>Key Bindings</h2>
  <table>
    <thead><tr><th>Key</th><th>Action</th></tr></thead>
    <tbody>{rows}</tbody>
  </table>
  <h2>Startup Options</h2>
  <ul>
    <li><code>wml2viewer [path]</code></li>
    <li><code>wml2viewer --config &lt;path&gt; [path]</code></li>
    <li><code>wml2viewer --config=&lt;path&gt; [path]</code></li>
    <li><code>wml2viewer --clean system</code> (planned)</li>
  </ul>
</body>
</html>"#
        );
        let _ = std::fs::write(&path, html);

        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.display().to_string()])
            .spawn();
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&path).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
    }

    pub(crate) fn set_zoom(&mut self, zoom: f32) -> Result<(), Box<dyn Error>> {
        let zoom = zoom.clamp(0.1, 16.0);
        if (zoom - self.zoom).abs() < f32::EPSILON {
            return Ok(());
        }
        self.zoom = zoom;
        self.invalidate_preload();
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
            (self.color_image_from_canvas(&canvas), display_scale)
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

    pub(crate) fn current_directory(&self) -> Option<PathBuf> {
        if self.current_navigation_path.is_dir() {
            return Some(self.current_navigation_path.clone());
        }
        if let Some(parent) = self.current_navigation_path.parent() {
            let marker = parent.file_name().and_then(|name| name.to_str());
            if matches!(marker, Some("__wmlv__" | "__zipv__")) {
                return parent.parent().map(|path| path.to_path_buf());
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
            sort_field: self.filer.sort_field,
            ascending: self.filer.ascending,
            separate_dirs: self.filer.separate_dirs,
            filter_text: self.filer.filter_text.clone(),
            extension_filter: self.filer.extension_filter.clone(),
            name_sort_mode: self.filer.name_sort_mode,
        });
    }

    pub(crate) fn refresh_current_filer_directory(&mut self) {
        if let Some(dir) = self
            .filer
            .directory
            .clone()
            .or_else(|| self.current_directory())
        {
            self.request_filer_directory(dir, self.filer.selected.clone());
        }
    }

    pub(crate) fn set_filesystem_current(&mut self, path: PathBuf) {
        let request_id = self.alloc_fs_request_id();
        let _ = self
            .fs_tx
            .send(FilesystemCommand::SetCurrent { request_id, path });
    }

    pub(crate) fn save_current_as(&mut self, format: SaveFormat) {
        if self.save_dialog.in_progress {
            return;
        }
        let Some(parent) = self
            .save_dialog
            .output_dir
            .clone()
            .or_else(|| self.storage.path.clone())
            .or_else(default_download_dir)
            .or_else(|| self.current_path.parent().map(|path| path.to_path_buf()))
        else {
            self.save_dialog.message = Some("Cannot determine save directory".to_string());
            return;
        };

        let file_name = self.save_dialog.file_name.trim();
        let stem = if file_name.is_empty() {
            default_save_file_name(&self.current_path)
        } else {
            file_name.to_string()
        };
        let output = parent.join(format!("{stem}.{}", format.extension()));
        let source = self.source.clone();
        let (tx, rx) = mpsc::channel();
        self.save_dialog.in_progress = true;
        self.save_dialog.result_rx = Some(rx);
        std::thread::spawn(move || {
            let result = save_loaded_image(&output, &source, format)
                .map(|_| format!("Saved {}", output.display()))
                .map_err(|err| format!("Save failed: {err}"));
            let _ = tx.send(result);
        });
    }

    fn color_image_from_canvas(&self, canvas: &Canvas) -> egui::ColorImage {
        let mut image = canvas_to_color_image(canvas);
        if self.options.grayscale {
            for pixel in &mut image.pixels {
                let luma = (0.299 * pixel.r() as f32
                    + 0.587 * pixel.g() as f32
                    + 0.114 * pixel.b() as f32)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                *pixel = egui::Color32::from_rgba_unmultiplied(luma, luma, luma, pixel.a());
            }
        }
        image
    }

    pub(crate) fn open_save_dialog(&mut self) {
        self.save_dialog.open = true;
    }

    fn poll_save_result(&mut self) {
        let Some(rx) = &self.save_dialog.result_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(message)) => {
                self.save_dialog.message = Some(message);
                self.save_dialog.in_progress = false;
                self.save_dialog.open = false;
                self.save_dialog.result_rx = None;
            }
            Ok(Err(message)) => {
                self.save_dialog.message = Some(message);
                self.save_dialog.in_progress = false;
                self.save_dialog.result_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.save_dialog.message = Some("Save worker disconnected".to_string());
                self.save_dialog.in_progress = false;
                self.save_dialog.result_rx = None;
            }
        }
    }

    fn save_dialog_ui(&mut self, ctx: &egui::Context) {
        if !self.save_dialog.open {
            return;
        }

        let mut open = self.save_dialog.open;
        let mut close_requested = false;
        egui::Window::new(self.text(UiTextKey::Save))
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.text(UiTextKey::Directory));
                    ui.label(
                        self.save_dialog
                            .output_dir
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| self.text(UiTextKey::NotSelected).to_string()),
                    );
                });
                if ui.button(self.text(UiTextKey::ChooseFolder)).clicked() {
                    self.save_dialog.output_dir =
                        pick_save_directory().or_else(default_download_dir);
                    if self.storage.path_record {
                        self.storage.path = self.save_dialog.output_dir.clone();
                    }
                }
                ui.horizontal(|ui| {
                    ui.label(self.text(UiTextKey::NameLabel));
                    ui.add_enabled_ui(!self.save_dialog.in_progress, |ui| {
                        ui.text_edit_singleline(&mut self.save_dialog.file_name);
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(self.text(UiTextKey::Format));
                    ui.add_enabled_ui(!self.save_dialog.in_progress, |ui| {
                        egui::ComboBox::from_id_salt("save_format_dialog")
                            .selected_text(self.save_dialog.format.to_string())
                            .show_ui(ui, |ui| {
                                for format in SaveFormat::all() {
                                    ui.selectable_value(
                                        &mut self.save_dialog.format,
                                        format,
                                        format.to_string(),
                                    );
                                }
                            });
                    });
                });
                if self.save_dialog.in_progress {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        let dots = ".".repeat((self.frame_counter % 3) + 1);
                        ui.label(format!("Waiting{dots}"));
                    });
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.save_dialog.in_progress,
                            egui::Button::new(self.text(UiTextKey::Save)),
                        )
                        .clicked()
                    {
                        self.save_current_as(self.save_dialog.format);
                    }
                    if ui.button(self.text(UiTextKey::Cancel)).clicked() {
                        close_requested = true;
                    }
                });
            });
        if close_requested {
            open = false;
        }
        self.save_dialog.open = open;
    }

    fn status_panel_ui(&mut self, ctx: &egui::Context) {
        let Some(message) = &self.save_dialog.message else {
            return;
        };

        egui::TopBottomPanel::bottom("status_overlay")
            .resizable(false)
            .exact_height(24.0)
            .show(ctx, |ui| {
                let text = ellipsize_end(message, 160);
                ui.horizontal(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(text).small());
                });
            });
    }

    fn loading_overlay_ui(&mut self, ctx: &egui::Context) {
        let Some(message) = &self.overlay.loading_message else {
            return;
        };
        egui::TopBottomPanel::bottom("loading_overlay")
            .resizable(false)
            .exact_height(24.0)
            .show(ctx, |ui| {
                let text = ellipsize_end(message, 160);
                ui.horizontal(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(text).small());
                });
            });
    }

    fn alert_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(message) = self.overlay.alert_message.clone() else {
            return;
        };

        let mut open = true;
        let mut close_requested = false;
        egui::Window::new("Alert")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(message);
                if ui.button(self.text(UiTextKey::Close)).clicked() {
                    close_requested = true;
                }
            });
        if close_requested || !open {
            self.overlay.alert_message = None;
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
            && self.last_viewport_size.x >= self.last_viewport_size.y * 1.4
            && self.is_current_portrait_page()
            && self.companion_navigation_path.is_some()
            && self
                .companion_source
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

    fn request_companion_resize(&mut self) -> Result<(), Box<dyn Error>> {
        if self.companion_source.is_none() {
            return Ok(());
        }
        let request_id = self.alloc_request_id();
        self.companion_active_request = Some(ActiveRenderRequest::Resize(request_id));
        self.companion_tx
            .send(RenderCommand::ResizeCurrent {
                request_id,
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
            self.companion_source = None;
            self.companion_rendered = None;
            self.companion_texture = None;
            self.companion_active_request = None;
            self.pending_fit_recalc |= !matches!(self.render_options.zoom_option, ZoomOption::None);
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
        if self.try_take_preloaded(&path, &load_path) {
            return Ok(());
        }
        let request_id = self.alloc_request_id();
        self.active_request = Some(ActiveRenderRequest::Load(request_id));
        self.pending_fit_recalc = !matches!(self.render_options.zoom_option, ZoomOption::None);
        self.overlay.loading_message = Some(format!("Loading {}", load_path.display()));
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
        self.invalidate_preload();
        let request_id = self.alloc_request_id();
        self.active_request = Some(ActiveRenderRequest::Resize(request_id));
        self.overlay.loading_message = Some(format!("Rendering {:.0}%", self.zoom * 100.0));
        self.worker_tx
            .send(RenderCommand::ResizeCurrent {
                request_id,
                zoom: self.zoom,
                method: self.render_options.zoom_method,
            })
            .map_err(worker_send_error)?;
        if let Some(path) = self.companion_navigation_path.clone() {
            if self.companion_source.is_some() {
                let _ = self.request_companion_resize();
            } else {
                let _ = self.request_companion_load(path);
            }
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

    fn alloc_thumbnail_request_id(&mut self) -> u64 {
        self.next_thumbnail_request_id += 1;
        self.next_thumbnail_request_id
    }

    fn alloc_preload_request_id(&mut self) -> u64 {
        self.next_preload_request_id += 1;
        self.next_preload_request_id
    }

    fn invalidate_preload(&mut self) {
        self.active_preload_request_id = None;
        self.pending_preload_navigation_path = None;
        self.preloaded_navigation_path = None;
        self.preloaded_load_path = None;
        self.preloaded_source = None;
        self.preloaded_rendered = None;
    }

    fn init_filesystem(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let request_id = self.alloc_fs_request_id();
        self.active_fs_request_id = Some(request_id);
        self.overlay.loading_message = Some(format!("Scanning {}", path.display()));
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
        self.overlay.loading_message = Some("Scanning folder...".to_string());
        self.fs_tx.send(command).map_err(filesystem_send_error)?;
        Ok(())
    }

    fn apply_loaded_result(
        &mut self,
        path: Option<PathBuf>,
        source: LoadedImage,
        rendered: LoadedImage,
    ) {
        if let Some(path) = path {
            let request_id = self.alloc_fs_request_id();
            self.current_path = path.clone();
            self.save_dialog.file_name = default_save_file_name(&path);
            let _ = self.fs_tx.send(FilesystemCommand::SetCurrent {
                request_id,
                path: self.current_navigation_path.clone(),
            });
            if let Some(dir) = self.current_directory() {
                self.request_filer_directory(dir, Some(self.current_navigation_path.clone()));
            }
        }
        self.source = source;
        self.rendered = rendered;
        self.pending_fit_recalc |= !matches!(self.render_options.zoom_option, ZoomOption::None);
        self.current_frame = self
            .current_frame
            .min(self.rendered.frame_count().saturating_sub(1));
        self.completed_loops = 0;
        self.last_frame_at = Instant::now();
        self.upload_current_frame();
        if self.active_fs_request_id.is_none() {
            self.overlay.loading_message = None;
        }
        self.active_request = None;
        self.schedule_preload();
        if self.pending_resize_after_load {
            self.pending_resize_after_load = false;
            let _ = self.request_resize_current();
        }
    }

    fn next_preload_candidate(&self) -> Option<PathBuf> {
        let step = if self.manga_spread_active() { 2 } else { 1 };
        adjacent_entry(&self.current_navigation_path, self.navigation_sort, step)
    }

    fn schedule_preload(&mut self) {
        if self.empty_mode || self.active_request.is_some() {
            return;
        }
        if archive_prefers_low_io(&self.current_navigation_path) {
            return;
        }
        let Some(path) = self.next_preload_candidate() else {
            return;
        };
        if archive_prefers_low_io(&path) {
            return;
        }
        if self.preloaded_navigation_path.as_ref() == Some(&path)
            || self.pending_preload_navigation_path.as_ref() == Some(&path)
        {
            return;
        }
        let load_path = crate::filesystem::resolve_start_path(&path).unwrap_or(path.clone());
        let request_id = self.alloc_preload_request_id();
        self.active_preload_request_id = Some(request_id);
        self.pending_preload_navigation_path = Some(path);
        let _ = self.preload_tx.send(RenderCommand::LoadPath {
            request_id,
            path: load_path,
            zoom: self.zoom,
            method: self.render_options.zoom_method,
        });
    }

    fn try_take_preloaded(&mut self, path: &std::path::Path, load_path: &std::path::Path) -> bool {
        let matches_navigation = self
            .preloaded_navigation_path
            .as_ref()
            .map(|cached| cached == path)
            .unwrap_or(false);
        let _ = load_path;
        if !matches_navigation {
            return false;
        }

        let source = self.preloaded_source.take();
        let rendered = self.preloaded_rendered.take();
        let load_path = self.preloaded_load_path.take();
        self.preloaded_navigation_path = None;
        self.pending_preload_navigation_path = None;
        if let (Some(source), Some(rendered)) = (source, rendered) {
            self.overlay.loading_message = None;
            self.apply_loaded_result(load_path, source, rendered);
            return true;
        }
        false
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
                    self.apply_loaded_result(path, source, rendered);
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
                    let failed_during_load =
                        matches!(active_request, ActiveRenderRequest::Load(_));
                    self.overlay.alert_message = Some(message);
                    self.overlay.loading_message = None;
                    self.active_request = None;
                    if failed_during_load {
                        let _ = self.next_image();
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.overlay.alert_message = Some("render worker disconnected".to_string());
                    self.overlay.loading_message = None;
                    self.active_request = None;
                    break;
                }
            }
        }
    }

    fn poll_preload_worker(&mut self) {
        loop {
            match self.preload_rx.try_recv() {
                Ok(RenderResult::Loaded {
                    request_id,
                    path,
                    source,
                    rendered,
                }) => {
                    if self.active_preload_request_id != Some(request_id) {
                        continue;
                    }
                    self.active_preload_request_id = None;
                    self.preloaded_navigation_path = self.pending_preload_navigation_path.take();
                    self.preloaded_load_path = path;
                    self.preloaded_source = Some(source);
                    self.preloaded_rendered = Some(rendered);
                }
                Ok(RenderResult::Failed { request_id, .. }) => {
                    if self.active_preload_request_id == Some(request_id) {
                        self.active_preload_request_id = None;
                        self.pending_preload_navigation_path = None;
                        self.preloaded_navigation_path = None;
                        self.preloaded_load_path = None;
                        self.preloaded_source = None;
                        self.preloaded_rendered = None;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.invalidate_preload();
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
                    path,
                    source,
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
                    let layout_changed = path.is_some()
                        || self
                            .companion_source
                            .as_ref()
                            .map(|image| {
                                image.canvas.width() != source.canvas.width()
                                    || image.canvas.height() != source.canvas.height()
                            })
                            .unwrap_or(true);

                    let (canvas, display_scale) = downscale_for_texture_limit(
                        rendered.frame_canvas(0),
                        self.max_texture_side,
                        self.render_options.zoom_method,
                    );
                    let image = self.color_image_from_canvas(&canvas);
                    let texture = if let Some(texture) = &mut self.companion_texture {
                        texture.set(image, TextureOptions::LINEAR);
                        texture.clone()
                    } else {
                        self.egui_ctx
                            .load_texture("manga_companion", image, TextureOptions::LINEAR)
                    };
                    self.companion_texture = Some(texture);
                    self.companion_source = Some(source);
                    self.companion_rendered = Some(rendered);
                    self.companion_texture_display_scale = display_scale;
                    if layout_changed {
                        self.pending_fit_recalc |=
                            !matches!(self.render_options.zoom_option, ZoomOption::None);
                    }
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
                        self.companion_source = None;
                        self.companion_rendered = None;
                        self.companion_texture = None;
                        self.companion_active_request = None;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.companion_source = None;
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
                            self.overlay.loading_message = None;
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
                        self.overlay.loading_message =
                            Some("No displayable file found".to_string());
                        self.show_filer = true;
                        self.active_fs_request_id = None;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.overlay.loading_message =
                        Some("filesystem worker disconnected".to_string());
                    self.active_fs_request_id = None;
                    break;
                }
            }
        }
    }

    fn poll_filer_worker(&mut self) {
        loop {
            match self.filer_rx.try_recv() {
                Ok(FilerResult::Reset {
                    request_id,
                    directory,
                    selected,
                }) => {
                    if self.filer.pending_request_id != Some(request_id) {
                        continue;
                    }
                    self.filer.directory = Some(directory);
                    self.filer.entries.clear();
                    self.filer.selected = selected;
                }
                Ok(FilerResult::Append {
                    request_id,
                    entries,
                }) => {
                    if self.filer.pending_request_id != Some(request_id) {
                        continue;
                    }
                    self.filer.entries.extend(entries);
                }
                Ok(FilerResult::Snapshot {
                    request_id,
                    directory,
                    entries,
                    selected,
                }) => {
                    if self.filer.pending_request_id != Some(request_id) {
                        continue;
                    }
                    self.filer.pending_request_id = None;
                    self.filer.directory = Some(directory);
                    self.filer.entries = entries;
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

    fn poll_thumbnail_worker(&mut self) {
        loop {
            match self.thumbnail_rx.try_recv() {
                Ok(ThumbnailResult::Ready {
                    _request_id: _,
                    path,
                    image,
                }) => {
                    self.thumbnail_pending.remove(&path);
                    let texture = self.egui_ctx.load_texture(
                        format!("thumb:{}", path.display()),
                        image,
                        TextureOptions::LINEAR,
                    );
                    self.thumbnail_cache.insert(path, texture);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.thumbnail_pending.clear();
                    break;
                }
            }
        }
    }

    pub(crate) fn ensure_thumbnail(&mut self, path: &std::path::Path, max_side: u32) {
        if self.thumbnail_cache.contains_key(path) || self.thumbnail_pending.contains(path) {
            return;
        }
        let request_id = self.alloc_thumbnail_request_id();
        let path = path.to_path_buf();
        self.thumbnail_pending.insert(path.clone());
        let _ = self.thumbnail_tx.send(ThumbnailCommand::Generate {
            request_id,
            path,
            max_side,
        });
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
        self.poll_thumbnail_worker();
        self.poll_preload_worker();
        self.poll_save_result();
        self.sync_manga_companion(ctx);
        self.handle_keyboard(ctx);
        self.settings_ui(ctx);
        self.restart_prompt_ui(ctx);
        self.alert_dialog_ui(ctx);
        self.save_dialog_ui(ctx);
        self.left_click_menu_ui(ctx);
        self.filer_ui(ctx);
        self.subfiler_ui(ctx);
        self.status_panel_ui(ctx);

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

            let viewport = ui.max_rect().size();
            let startup_viewport_settling =
                self.frame_counter < 8 && viewport_size_changed(viewport, self.last_viewport_size);

            if startup_viewport_settling {
                self.last_viewport_size = viewport;
            } else if !self.empty_mode
                && (viewport_size_changed(viewport, self.last_viewport_size)
                    || self.pending_fit_recalc)
                && !matches!(self.render_options.zoom_option, ZoomOption::None)
            {
                self.last_viewport_size = viewport;
                self.pending_fit_recalc = false;

                let new_zoom = calc_fit_zoom(
                    viewport,
                    self.fit_target_size(),
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
                        ui.spacing_mut().item_spacing.x = 0.0;
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
                                    self.paint_manga_separator(
                                        ui,
                                        draw_size.y.max(companion_draw_size.y),
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
                                    self.paint_manga_separator(
                                        ui,
                                        draw_size.y.max(companion_draw_size.y),
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

                    if self.empty_mode {
                        ui.add_space(8.0);
                        ui.label(format!(
                            "{} {}",
                            self.text(UiTextKey::NoDisplayableFileFound),
                            self.text(UiTextKey::OpenDirectoryOrFileFromFiler)
                        ));
                    }
                });
        });
        self.loading_overlay_ui(ctx);
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
