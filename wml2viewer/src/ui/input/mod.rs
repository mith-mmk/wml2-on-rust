use crate::options::{KeyBinding, ViewerAction};
use crate::ui::viewer::ViewerApp;
use eframe::egui;
use std::time::Instant;

impl ViewerApp {
    pub(crate) fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.save_current_as(self.save_format);
        }

        if self.show_settings {
            return;
        }

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
                ViewerAction::ToggleMangaMode => {
                    self.options.manga_mode = !self.options.manga_mode;
                }
                ViewerAction::ToggleSettings => {
                    self.show_settings = !self.show_settings;
                }
                ViewerAction::ToggleFiler => {
                    self.show_filer = !self.show_filer;
                }
                ViewerAction::SaveAs => {
                    self.save_current_as(self.save_format);
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

    pub(crate) fn handle_pointer_input(&mut self, response: &egui::Response) {
        if self.show_settings {
            return;
        }

        if response.double_clicked() {
            let _ = self.toggle_zoom();
            return;
        }

        if response.clicked() {
            self.left_menu_pos = response
                .interact_pointer_pos()
                .unwrap_or_else(|| response.rect.left_top());
            self.show_left_menu = true;
        }
    }
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
        "F" => Some(egui::Key::F),
        "P" => Some(egui::Key::P),
        _ => None,
    }
}
