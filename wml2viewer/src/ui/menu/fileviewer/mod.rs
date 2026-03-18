use crate::drawers::image::SaveFormat;
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
                if ui.button("Next").clicked() {
                    let _ = self.next_image();
                    self.show_left_menu = false;
                }
                if ui.button("Previous").clicked() {
                    let _ = self.prev_image();
                    self.show_left_menu = false;
                }
                if ui.button("Toggle Settings").clicked() {
                    self.show_settings = !self.show_settings;
                    self.show_left_menu = false;
                }
                if ui.button("Toggle Filer").clicked() {
                    self.show_filer = !self.show_filer;
                    self.show_left_menu = false;
                }
                if ui.button("Toggle Manga").clicked() {
                    self.options.manga_mode = !self.options.manga_mode;
                    self.show_left_menu = false;
                }
                ui.separator();
                ui.label("Save As");
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
                ui.heading("Filer");
                let current_root = self
                    .filer_directory
                    .as_ref()
                    .and_then(|dir| self.filer_roots.iter().find(|root| dir.starts_with(root)))
                    .cloned()
                    .or_else(|| self.filer_roots.first().cloned());
                egui::ComboBox::from_id_salt("filer_roots")
                    .selected_text(
                        current_root
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| "(root)".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for root in self.filer_roots.clone() {
                            if ui
                                .selectable_label(
                                    current_root.as_ref() == Some(&root),
                                    root.display().to_string(),
                                )
                                .clicked()
                            {
                                self.set_filer_directory(root);
                            }
                        }
                    });
                if let Some(dir) = &self.filer_directory {
                    ui.label(dir.display().to_string());
                    if let Some(parent) = dir.parent() {
                        if ui.button("..").clicked() {
                            self.set_filer_directory(parent.to_path_buf());
                        }
                    }
                }
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let entries = self.filer_entries.clone();
                    for entry in entries {
                        let label = entry
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "(entry)".to_string());
                        let selected = self.filer_selected.as_ref() == Some(&entry)
                            || self.current_navigation_path == entry;
                        if ui.selectable_label(selected, label).clicked() {
                            if entry.is_dir() {
                                self.set_filer_directory(entry);
                                continue;
                            }
                            self.filer_selected = Some(entry.clone());
                            self.current_navigation_path = entry.clone();
                            self.empty_mode = false;
                            let _ = self.request_load_path(entry);
                        }
                    }
                });
            });
    }
}
