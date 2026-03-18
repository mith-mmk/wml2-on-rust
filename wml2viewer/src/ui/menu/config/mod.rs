use crate::configs::config::save_app_config;
use crate::configs::resourses::{FontSizePreset, apply_resources};
use crate::drawers::affine::InterpolationAlgorithm;
use crate::options::{AppConfig, EndOfFolderOption, NavigationOptions};
use crate::ui::render::interpolation_label;
use crate::ui::viewer::ViewerApp;
use crate::ui::viewer::options::BackgroundStyle;
use crate::ui::viewer::options::ZoomOption;
use eframe::egui;

impl ViewerApp {
    pub(crate) fn settings_ui(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let mut open = self.show_settings;
        let mut reload_requested = false;
        let mut rerender_requested = false;
        let mut zoom_option_changed = false;
        let mut config_changed = false;
        let mut close_requested = false;
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.collapsing("Viewer", |ui| {
                    config_changed |= ui
                        .checkbox(&mut self.options.animation, "Animation")
                        .changed();
                    config_changed |= ui
                        .checkbox(&mut self.options.manga_mode, "Manga mode")
                        .changed();
                    config_changed |= ui
                        .checkbox(&mut self.options.manga_right_to_left, "Manga right-to-left")
                        .changed();

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
                });

                ui.collapsing("Resources", |ui| {
                    ui.label(format!("Locale: {}", self.applied_locale));
                    if !self.loaded_font_names.is_empty() {
                        ui.label(format!("Fonts: {}", self.loaded_font_names.join(", ")));
                    }
                    ui.horizontal(|ui| {
                        ui.label("Font size");
                        egui::ComboBox::from_id_salt("font_size")
                            .selected_text(font_size_label(self.resources.font_size))
                            .show_ui(ui, |ui| {
                                config_changed |= ui
                                    .selectable_value(
                                        &mut self.resources.font_size,
                                        FontSizePreset::Auto,
                                        "Auto",
                                    )
                                    .changed();
                                config_changed |= ui
                                    .selectable_value(
                                        &mut self.resources.font_size,
                                        FontSizePreset::S,
                                        "S",
                                    )
                                    .changed();
                                config_changed |= ui
                                    .selectable_value(
                                        &mut self.resources.font_size,
                                        FontSizePreset::M,
                                        "M",
                                    )
                                    .changed();
                                config_changed |= ui
                                    .selectable_value(
                                        &mut self.resources.font_size,
                                        FontSizePreset::L,
                                        "L",
                                    )
                                    .changed();
                                config_changed |= ui
                                    .selectable_value(
                                        &mut self.resources.font_size,
                                        FontSizePreset::LL,
                                        "LL",
                                    )
                                    .changed();
                            });
                    });
                });

                ui.collapsing("Render", |ui| {
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
                });

                ui.collapsing("Window", |ui| {
                    if ui
                        .checkbox(&mut self.window_options.fullscreen, "Fullscreen")
                        .changed()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                            self.window_options.fullscreen,
                        ));
                        config_changed = true;
                    }
                    config_changed |= ui
                        .checkbox(&mut self.window_options.remember_size, "Remember size")
                        .changed();
                    config_changed |= ui
                        .checkbox(
                            &mut self.window_options.remember_position,
                            "Remember position",
                        )
                        .changed();
                    match &mut self.window_options.size {
                        crate::ui::viewer::options::WindowSize::Relative(ratio) => {
                            ui.label("Window size: relative");
                            config_changed |= ui
                                .add(egui::Slider::new(ratio, 0.2..=1.0).text("ratio"))
                                .changed();
                            if ui.button("Use exact size").clicked() {
                                self.window_options.size =
                                    crate::ui::viewer::options::WindowSize::Exact {
                                        width: self.last_viewport_size.x.max(320.0),
                                        height: self.last_viewport_size.y.max(240.0),
                                    };
                                config_changed = true;
                            }
                        }
                        crate::ui::viewer::options::WindowSize::Exact { width, height } => {
                            ui.label("Window size: exact");
                            config_changed |= ui
                                .add(egui::DragValue::new(width).speed(1.0).prefix("W "))
                                .changed();
                            config_changed |= ui
                                .add(egui::DragValue::new(height).speed(1.0).prefix("H "))
                                .changed();
                            if ui.button("Use relative size").clicked() {
                                self.window_options.size =
                                    crate::ui::viewer::options::WindowSize::Relative(0.8);
                                config_changed = true;
                            }
                        }
                    }
                });

                ui.collapsing("Navigation", |ui| {
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
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Reload current").clicked() {
                        reload_requested = true;
                    }
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
        if close_requested {
            open = false;
        }
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
            let applied = apply_resources(ctx, &self.resources);
            self.applied_locale = applied.locale;
            self.loaded_font_names = applied.loaded_fonts;
            let _ = save_app_config(
                &self.current_config(),
                Some(&self.current_path),
                self.config_path.as_deref(),
            );
        }
    }

    pub(crate) fn current_config(&self) -> AppConfig {
        AppConfig {
            viewer: self.options.clone(),
            window: self.window_options.clone(),
            render: self.render_options.clone(),
            input: Default::default(),
            resources: self.resources.clone(),
            navigation: NavigationOptions {
                end_of_folder: self.end_of_folder,
                sort: self.navigation_sort,
            },
        }
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

fn font_size_label(option: FontSizePreset) -> &'static str {
    match option {
        FontSizePreset::Auto => "Auto",
        FontSizePreset::S => "S",
        FontSizePreset::M => "M",
        FontSizePreset::L => "L",
        FontSizePreset::LL => "LL",
    }
}
