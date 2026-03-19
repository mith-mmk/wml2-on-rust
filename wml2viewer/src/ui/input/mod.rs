pub(crate) mod dispatch;

use crate::options::ViewerAction;
use crate::ui::input::dispatch::collect_triggered_actions;
use crate::ui::viewer::ViewerApp;
use eframe::egui;
use std::time::Instant;

impl ViewerApp {
    pub(crate) fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.open_save_dialog();
        }

        if self.show_settings {
            return;
        }

        let triggered = collect_triggered_actions(ctx, &self.keymap);
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
                ViewerAction::ToggleGrayscale => {
                    self.options.grayscale = !self.options.grayscale;
                    self.upload_current_frame();
                    if self.companion_rendered.is_some() {
                        self.pending_fit_recalc = true;
                    }
                }
                ViewerAction::ToggleMangaMode => {
                    self.options.manga_mode = !self.options.manga_mode;
                    self.pending_fit_recalc = true;
                }
                ViewerAction::ToggleSettings => {
                    self.show_settings = !self.show_settings;
                }
                ViewerAction::ToggleFiler => {
                    self.show_filer = !self.show_filer;
                    self.pending_fit_recalc = true;
                }
                ViewerAction::SaveAs => {
                    self.open_save_dialog();
                }
            }
        }
    }

    pub(crate) fn handle_pointer_input(&mut self, response: &egui::Response) {
        if self.show_settings {
            return;
        }

        if response.double_clicked() {
            let _ = self.toggle_zoom();
            return;
        }

        if response.secondary_clicked() {
            self.left_menu_pos = response
                .interact_pointer_pos()
                .unwrap_or_else(|| response.rect.left_top());
            self.show_left_menu = true;
            return;
        }

        if response.clicked() {
            let _ = self.next_image();
        }
    }
}
