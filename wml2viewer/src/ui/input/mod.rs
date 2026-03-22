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

        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.open_help();
            return;
        }

        if self.show_settings || self.save_dialog.open || self.overlay.alert_message.is_some() {
            if !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                if ctx.wants_keyboard_input() {
                    return;
                }
            } else if self.overlay.alert_message.is_some() {
                self.overlay.alert_message = None;
                return;
            }
        }

        if ctx.wants_keyboard_input() {
            return;
        }

        let triggered = collect_triggered_actions(ctx, &self.keymap);
        for action in triggered {
            if self.show_settings && !matches!(action, ViewerAction::ToggleSettings) {
                continue;
            }
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
                    if self.show_settings {
                        self.close_settings_dialog();
                    } else {
                        self.open_settings_dialog();
                    }
                }
                ViewerAction::ToggleFiler => {
                    self.show_filer = !self.show_filer;
                    self.pending_fit_recalc = true;
                }
                ViewerAction::ToggleSubfiler => {
                    self.show_subfiler = !self.show_subfiler;
                }
                ViewerAction::SaveAs => {
                    self.open_save_dialog();
                }
            }
        }
    }

    pub(crate) fn handle_pointer_input(&mut self, response: &egui::Response) {
        if self.save_dialog.open || self.overlay.alert_message.is_some() {
            return;
        }

        if response.double_clicked_by(egui::PointerButton::Secondary) {
            let _ = self.toggle_fit_zoom_mode();
            return;
        }

        if response.middle_clicked() {
            self.left_menu_pos = response
                .interact_pointer_pos()
                .unwrap_or_else(|| response.rect.left_top());
            self.show_left_menu = true;
            return;
        }

        if response.secondary_clicked() {
            let _ = self.next_image();
            return;
        }

        if self.show_settings {
            return;
        }

        if response.clicked() {
            self.open_settings_dialog();
        }
    }
}
