use crate::configs::config::{load_app_config, save_app_config};
use crate::configs::resourses::{FontSizePreset, apply_resources};
use crate::dependent::plugins::discover_plugin_modules;
use crate::dependent::{default_download_dir, pick_save_directory};
use crate::drawers::affine::InterpolationAlgorithm;
use crate::options::{AppConfig, EndOfFolderOption, NavigationOptions};
use crate::ui::i18n::UiTextKey;
use crate::ui::render::interpolation_label;
use crate::ui::viewer::options::{BackgroundStyle, MangaSeparatorStyle, WindowUiTheme, ZoomOption};
use crate::ui::viewer::{SettingsTab, ViewerApp, join_search_paths, parse_search_paths};
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
        let mut apply_requested = false;
        let mut undo_requested = false;
        let mut reset_requested = false;
        let settings_text = self.text(UiTextKey::Settings);
        let viewer_text = self.text(UiTextKey::Viewer);
        let plugins_text = self.text(UiTextKey::Plugins);
        let resources_text = self.text(UiTextKey::Resources);
        let render_text = self.text(UiTextKey::Render);
        let window_text = self.text(UiTextKey::Window);
        let navigation_text = self.text(UiTextKey::Navigation);
        let animation_text = self.text(UiTextKey::Animation);
        let grayscale_text = self.text(UiTextKey::Grayscale);
        let manga_mode_text = self.text(UiTextKey::MangaMode);
        let manga_rtl_text = self.text(UiTextKey::MangaRightToLeft);
        let background_text = self.text(UiTextKey::Background);
        let locale_text = self.text(UiTextKey::Locale);
        let fonts_text = self.text(UiTextKey::Fonts);
        let font_size_text = self.text(UiTextKey::FontSize);
        let auto_text = self.text(UiTextKey::Auto);
        let theme_text = self.text(UiTextKey::Theme);
        let system_text = self.text(UiTextKey::System);
        let light_text = self.text(UiTextKey::Light);
        let dark_text = self.text(UiTextKey::Dark);
        let zoom_mode_text = self.text(UiTextKey::ZoomMode);
        let resize_text = self.text(UiTextKey::Resize);
        let fullscreen_text = self.text(UiTextKey::Fullscreen);
        let remember_size_text = self.text(UiTextKey::RememberSize);
        let remember_position_text = self.text(UiTextKey::RememberPosition);
        let window_relative_text = self.text(UiTextKey::WindowSizeRelative);
        let window_exact_text = self.text(UiTextKey::WindowSizeExact);
        let use_exact_size_text = self.text(UiTextKey::UseExactSize);
        let use_relative_size_text = self.text(UiTextKey::UseRelativeSize);
        let end_of_folder_text = self.text(UiTextKey::EndOfFolder);
        let reload_current_text = self.text(UiTextKey::ReloadCurrent);
        let close_text = self.text(UiTextKey::Close);
        let help_text = "Help";
        let black_text = self.text(UiTextKey::Black);
        let gray_text = self.text(UiTextKey::Gray);
        let tile_text = self.text(UiTextKey::Tile);
        let separator_text = self.text(UiTextKey::Separator);
        let separator_style_text = self.text(UiTextKey::SeparatorStyle);
        let separator_color_text = self.text(UiTextKey::SeparatorColor);
        let separator_pixels_text = self.text(UiTextKey::SeparatorPixels);
        let none_text = self.text(UiTextKey::None);
        let solid_text = self.text(UiTextKey::Solid);
        let shadow_text = self.text(UiTextKey::Shadow);
        let remember_save_path_text = self.text(UiTextKey::RememberSavePath);
        egui::Window::new(settings_text)
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Viewer, viewer_text);
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Plugins, plugins_text);
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::Resources,
                        resources_text,
                    );
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Render, render_text);
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Window, window_text);
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::Navigation,
                        navigation_text,
                    );
                });
                ui.separator();

                if self.settings_tab == SettingsTab::Viewer {
                    ui.group(|ui| {
                        config_changed |= ui
                            .checkbox(&mut self.options.animation, animation_text)
                            .changed();
                        config_changed |= ui
                            .checkbox(&mut self.options.grayscale, grayscale_text)
                            .changed();
                        config_changed |= ui
                            .checkbox(&mut self.options.manga_mode, manga_mode_text)
                            .changed();
                        config_changed |= ui
                            .checkbox(&mut self.options.manga_right_to_left, manga_rtl_text)
                            .changed();
                        ui.separator();
                        ui.label(separator_text);
                        ui.horizontal(|ui| {
                            ui.label(separator_style_text);
                            egui::ComboBox::from_id_salt("manga_separator_style")
                                .selected_text(match self.options.manga_separator.style {
                                    MangaSeparatorStyle::None => none_text,
                                    MangaSeparatorStyle::Solid => solid_text,
                                    MangaSeparatorStyle::Shadow => shadow_text,
                                })
                                .show_ui(ui, |ui| {
                                    config_changed |= ui
                                        .selectable_value(
                                            &mut self.options.manga_separator.style,
                                            MangaSeparatorStyle::None,
                                            none_text,
                                        )
                                        .changed();
                                    config_changed |= ui
                                        .selectable_value(
                                            &mut self.options.manga_separator.style,
                                            MangaSeparatorStyle::Solid,
                                            solid_text,
                                        )
                                        .changed();
                                    config_changed |= ui
                                        .selectable_value(
                                            &mut self.options.manga_separator.style,
                                            MangaSeparatorStyle::Shadow,
                                            shadow_text,
                                        )
                                        .changed();
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label(separator_pixels_text);
                            config_changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.options.manga_separator.pixels)
                                        .range(0.0..=64.0)
                                        .speed(0.25),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label(separator_color_text);
                            config_changed |= ui
                                .color_edit_button_srgba_unmultiplied(
                                    &mut self.options.manga_separator.color,
                                )
                                .changed();
                        });

                        ui.horizontal(|ui| {
                            ui.label(background_text);
                            if ui.button(black_text).clicked() {
                                self.options.background = BackgroundStyle::Solid([0, 0, 0, 255]);
                                config_changed = true;
                            }
                            if ui.button(gray_text).clicked() {
                                self.options.background = BackgroundStyle::Solid([48, 48, 48, 255]);
                                config_changed = true;
                            }
                            if ui.button(tile_text).clicked() {
                                self.options.background = BackgroundStyle::Tile {
                                    color1: [32, 32, 32, 255],
                                    color2: [80, 80, 80, 255],
                                    size: 16,
                                };
                                config_changed = true;
                            }
                        });
                    });
                }

                if self.settings_tab == SettingsTab::Plugins {
                    ui.group(|ui| {
                        ui.heading("susie64");
                        config_changed |= ui
                            .checkbox(&mut self.plugins.susie64.enable, "enable")
                            .changed();
                        ui.label("search_path");
                        if ui
                            .text_edit_singleline(&mut self.susie64_search_paths_input)
                            .changed()
                        {
                            self.plugins.susie64.search_path =
                                parse_search_paths(&self.susie64_search_paths_input);
                            config_changed = true;
                        }
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = pick_save_directory() {
                                self.plugins.susie64.search_path = vec![path];
                                self.susie64_search_paths_input =
                                    join_search_paths(&self.plugins.susie64.search_path);
                                config_changed = true;
                            }
                        }
                        if ui.button("Load modules").clicked() {
                            self.plugins.susie64.modules =
                                discover_plugin_modules("susie64", &self.plugins.susie64);
                            config_changed = true;
                        }
                        ui.label(format!("modules: {}", self.plugins.susie64.modules.len()));
                        ui.separator();
                        ui.heading("system");
                        config_changed |= ui
                            .checkbox(&mut self.plugins.system.enable, "enable")
                            .changed();
                        ui.label("search_path: OS API");
                        ui.label(format!("modules: {}", self.plugins.system.modules.len()));
                        ui.separator();
                        ui.heading("ffmpeg");
                        config_changed |= ui
                            .checkbox(&mut self.plugins.ffmpeg.enable, "enable")
                            .changed();
                        ui.label("search_path");
                        if ui
                            .text_edit_singleline(&mut self.ffmpeg_search_paths_input)
                            .changed()
                        {
                            self.plugins.ffmpeg.search_path =
                                parse_search_paths(&self.ffmpeg_search_paths_input);
                            config_changed = true;
                        }
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = pick_save_directory() {
                                self.plugins.ffmpeg.search_path = vec![path];
                                self.ffmpeg_search_paths_input =
                                    join_search_paths(&self.plugins.ffmpeg.search_path);
                                config_changed = true;
                            }
                        }
                        if ui.button("Load modules").clicked() {
                            self.plugins.ffmpeg.modules =
                                discover_plugin_modules("ffmpeg", &self.plugins.ffmpeg);
                            config_changed = true;
                        }
                        ui.label(format!("modules: {}", self.plugins.ffmpeg.modules.len()));
                    });
                }

                if self.settings_tab == SettingsTab::Resources {
                    ui.group(|ui| {
                        ui.label(format!("{}: {}", locale_text, self.applied_locale));
                        if !self.loaded_font_names.is_empty() {
                            ui.label(format!(
                                "{}: {}",
                                fonts_text,
                                self.loaded_font_names.join(", ")
                            ));
                        }
                        ui.horizontal(|ui| {
                            ui.label(font_size_text);
                            egui::ComboBox::from_id_salt("font_size")
                                .selected_text(font_size_label(self.resources.font_size))
                                .show_ui(ui, |ui| {
                                    config_changed |= ui
                                        .selectable_value(
                                            &mut self.resources.font_size,
                                            FontSizePreset::Auto,
                                            auto_text,
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
                }

                if self.settings_tab == SettingsTab::Render {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(zoom_mode_text);
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
                            ui.label(resize_text);
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
                }

                if self.settings_tab == SettingsTab::Window {
                    ui.group(|ui| {
                        if ui
                            .checkbox(&mut self.window_options.fullscreen, fullscreen_text)
                            .changed()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                                self.window_options.fullscreen,
                            ));
                            config_changed = true;
                        }
                        config_changed |= ui
                            .checkbox(&mut self.window_options.remember_size, remember_size_text)
                            .changed();
                        config_changed |= ui
                            .checkbox(
                                &mut self.window_options.remember_position,
                                remember_position_text,
                            )
                            .changed();
                        ui.horizontal(|ui| {
                            ui.label(theme_text);
                            egui::ComboBox::from_id_salt("window_theme")
                                .selected_text(match self.window_options.ui_theme {
                                    WindowUiTheme::System => system_text,
                                    WindowUiTheme::Light => light_text,
                                    WindowUiTheme::Dark => dark_text,
                                })
                                .show_ui(ui, |ui| {
                                    config_changed |= ui
                                        .selectable_value(
                                            &mut self.window_options.ui_theme,
                                            WindowUiTheme::System,
                                            system_text,
                                        )
                                        .changed();
                                    config_changed |= ui
                                        .selectable_value(
                                            &mut self.window_options.ui_theme,
                                            WindowUiTheme::Light,
                                            light_text,
                                        )
                                        .changed();
                                    config_changed |= ui
                                        .selectable_value(
                                            &mut self.window_options.ui_theme,
                                            WindowUiTheme::Dark,
                                            dark_text,
                                        )
                                        .changed();
                                });
                        });
                        match &mut self.window_options.size {
                            crate::ui::viewer::options::WindowSize::Relative(ratio) => {
                                ui.label(window_relative_text);
                                config_changed |= ui
                                    .add(egui::Slider::new(ratio, 0.2..=1.0).text("ratio"))
                                    .changed();
                                if ui.button(use_exact_size_text).clicked() {
                                    self.window_options.size =
                                        crate::ui::viewer::options::WindowSize::Exact {
                                            width: self.last_viewport_size.x.max(320.0),
                                            height: self.last_viewport_size.y.max(240.0),
                                        };
                                    config_changed = true;
                                }
                            }
                            crate::ui::viewer::options::WindowSize::Exact { width, height } => {
                                ui.label(window_exact_text);
                                config_changed |= ui
                                    .add(egui::DragValue::new(width).speed(1.0).prefix("W "))
                                    .changed();
                                config_changed |= ui
                                    .add(egui::DragValue::new(height).speed(1.0).prefix("H "))
                                    .changed();
                                if ui.button(use_relative_size_text).clicked() {
                                    self.window_options.size =
                                        crate::ui::viewer::options::WindowSize::Relative(0.8);
                                    config_changed = true;
                                }
                            }
                        }
                    });
                }

                if self.settings_tab == SettingsTab::Navigation {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(end_of_folder_text);
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
                        config_changed |= ui
                            .checkbox(&mut self.storage.path_record, remember_save_path_text)
                            .changed();
                    });
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        apply_requested = true;
                    }
                    if ui.button("Undo").clicked() {
                        undo_requested = true;
                    }
                    if ui.button("Reset").clicked() {
                        reset_requested = true;
                    }
                    if ui.button(reload_current_text).clicked() {
                        reload_requested = true;
                    }
                    if ui.button(help_text).clicked() {
                        self.open_help();
                    }
                    if ui.button(close_text).clicked() {
                        close_requested = true;
                    }
                });
            });
        if close_requested {
            open = false;
        }
        self.show_settings = open;
        if reset_requested {
            self.restore_config(AppConfig::default(), ctx);
        }
        if undo_requested {
            let config = load_app_config(self.config_path.as_deref()).unwrap_or_default();
            self.restore_config(config, ctx);
        }
        if zoom_option_changed {
            self.pending_fit_recalc = true;
        }
        if rerender_requested {
            let _ = self.request_resize_current();
        }
        if reload_requested {
            let _ = self.reload_current();
        }
        if config_changed || apply_requested {
            self.apply_window_theme(ctx);
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

    fn restore_config(&mut self, config: AppConfig, ctx: &egui::Context) {
        self.options = config.viewer;
        self.window_options = config.window;
        self.render_options = config.render;
        self.resources = config.resources;
        self.plugins = config.plugins;
        self.storage = config.storage;
        self.keymap = config.input.merged_with_defaults();
        self.end_of_folder = config.navigation.end_of_folder;
        self.navigation_sort = config.navigation.sort;
        self.save_output_dir = self
            .storage
            .path
            .clone()
            .or_else(default_download_dir)
            .or_else(|| self.current_path.parent().map(|path| path.to_path_buf()));
        self.susie64_search_paths_input = join_search_paths(&self.plugins.susie64.search_path);
        self.system_search_paths_input = join_search_paths(&self.plugins.system.search_path);
        self.ffmpeg_search_paths_input = join_search_paths(&self.plugins.ffmpeg.search_path);
        self.apply_window_theme(ctx);
        let applied = apply_resources(ctx, &self.resources);
        self.applied_locale = applied.locale;
        self.loaded_font_names = applied.loaded_fonts;
        self.pending_fit_recalc = true;
    }

    pub(crate) fn current_config(&self) -> AppConfig {
        AppConfig {
            viewer: self.options.clone(),
            window: self.window_options.clone(),
            render: self.render_options.clone(),
            plugins: self.plugins.clone(),
            storage: self.storage.clone(),
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
