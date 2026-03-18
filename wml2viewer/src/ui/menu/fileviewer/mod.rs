pub(crate) mod state;
pub(crate) mod worker;

use crate::drawers::image::SaveFormat;
use crate::ui::i18n::UiTextKey;
use crate::ui::viewer::ViewerApp;
use eframe::egui;

impl ViewerApp {
    pub(crate) fn left_click_menu_ui(&mut self, ctx: &egui::Context) {
        if !self.show_left_menu {
            return;
        }

        let mut open = true;
        egui::Window::new("Menu")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(self.left_menu_pos)
            .open(&mut open)
            .show(ctx, |ui| {
                if ui.button(self.text(UiTextKey::Next)).clicked() {
                    let _ = self.next_image();
                    self.show_left_menu = false;
                }
                if ui.button(self.text(UiTextKey::Previous)).clicked() {
                    let _ = self.prev_image();
                    self.show_left_menu = false;
                }
                if ui.button(self.text(UiTextKey::ToggleSettings)).clicked() {
                    self.show_settings = !self.show_settings;
                    self.show_left_menu = false;
                }
                if ui.button(self.text(UiTextKey::ToggleFiler)).clicked() {
                    self.show_filer = !self.show_filer;
                    self.show_left_menu = false;
                }
                if ui.button(self.text(UiTextKey::ToggleManga)).clicked() {
                    self.options.manga_mode = !self.options.manga_mode;
                    self.show_left_menu = false;
                }
                ui.separator();
                ui.label(self.text(UiTextKey::SaveAs));
                for format in SaveFormat::all() {
                    if ui
                        .selectable_label(self.save_format == format, format.to_string())
                        .clicked()
                    {
                        self.save_format = format;
                        self.save_current_as(format);
                        self.show_left_menu = false;
                    }
                }
            });
        self.show_left_menu = open;
    }

    pub(crate) fn filer_ui(&mut self, ctx: &egui::Context) {
        if !self.show_filer {
            return;
        }

        egui::SidePanel::left("filer_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading(self.text(UiTextKey::Filer));
                let current_root = self
                    .filer
                    .directory
                    .as_ref()
                    .and_then(|dir| self.filer.roots.iter().find(|root| dir.starts_with(root)))
                    .cloned()
                    .or_else(|| self.filer.roots.first().cloned());
                egui::ComboBox::from_id_salt("filer_roots")
                    .selected_text(
                        current_root
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| "(root)".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for root in self.filer.roots.clone() {
                            if ui
                                .selectable_label(
                                    current_root.as_ref() == Some(&root),
                                    root.display().to_string(),
                                )
                                .clicked()
                            {
                                self.request_filer_directory(root, None);
                            }
                        }
                    });
                if let Some(dir) = &self.filer.directory {
                    ui.label(dir.display().to_string());
                    if let Some(parent) = dir.parent() {
                        if ui.button("..").clicked() {
                            self.request_filer_directory(parent.to_path_buf(), None);
                        }
                    }
                }
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let entries = self.filer.entries.clone();
                    for entry in entries {
                        let selected = self.filer.selected.as_ref() == Some(&entry.path)
                            || self.current_navigation_path == entry.path;
                        let response = ui.selectable_label(selected, entry.label.clone());
                        if let Some(size) = entry.metadata.size {
                            let modified = entry
                                .metadata
                                .modified
                                .map(|value| format!("\n{value:?}"))
                                .unwrap_or_default();
                            response
                                .clone()
                                .on_hover_text(format!("{size} bytes{modified}"));
                        }
                        if response.clicked() {
                            if entry.is_dir {
                                self.request_filer_directory(entry.path, None);
                                continue;
                            }
                            self.filer.selected = Some(entry.path.clone());
                            self.current_navigation_path = entry.path.clone();
                            self.empty_mode = false;
                            let _ = self.request_load_path(entry.path);
                        }
                    }
                });
            });
    }
}
