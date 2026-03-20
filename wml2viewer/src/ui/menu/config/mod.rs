use crate::configs::config::{load_app_config, save_app_config};
use crate::configs::resourses::{FontSizePreset, apply_resources};
use crate::dependent::plugins::{discover_plugin_modules, set_runtime_plugin_config};
use crate::dependent::{
    clean_system_integration, default_download_dir, pick_save_directory,
    register_system_file_associations,
};
use crate::drawers::affine::InterpolationAlgorithm;
use crate::filesystem::set_archive_zip_workaround;
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
        let register_system_text = self.text(UiTextKey::RegisterSystem);
        let clean_system_text = self.text(UiTextKey::CleanSystem);
        let window_relative_text = self.text(UiTextKey::WindowSizeRelative);
        let window_exact_text = self.text(UiTextKey::WindowSizeExact);
        let use_exact_size_text = self.text(UiTextKey::UseExactSize);
        let use_relative_size_text = self.text(UiTextKey::UseRelativeSize);
        let end_of_folder_text = self.text(UiTextKey::EndOfFolder);
        let reload_current_text = self.text(UiTextKey::ReloadCurrent);
        let close_text = self.text(UiTextKey::Close);
        let help_text = self.text(UiTextKey::Help);
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
        let apply_text = self.text(UiTextKey::Apply);
        let undo_text = self.text(UiTextKey::Undo);
        let reset_text = self.text(UiTextKey::Reset);
        let enable_text = self.text(UiTextKey::Enable);
        let search_path_text = self.text(UiTextKey::SearchPath);
        let browse_text = self.text(UiTextKey::Browse);
        let load_modules_text = self.text(UiTextKey::LoadModules);
        let modules_text = self.text(UiTextKey::Modules);
        let search_path_os_api_text = self.text(UiTextKey::SearchPathOsApi);
        let registered_file_associations_text = self.text(UiTextKey::RegisteredFileAssociations);
        let failed_file_associations_text = self.text(UiTextKey::FailedFileAssociations);
        let cleaned_system_integration_text = self.text(UiTextKey::CleanedSystemIntegration);
        let workaround_text = self.text(UiTextKey::Workaround);
        let archive_text = self.text(UiTextKey::Archive);
        let threshold_mb_text = self.text(UiTextKey::ThresholdMb);
        let local_cache_text = self.text(UiTextKey::LocalCache);
        let fit_width_text = self.text(UiTextKey::FitWidth);
        let fit_height_text = self.text(UiTextKey::FitHeight);
        let fit_screen_text = self.text(UiTextKey::FitScreen);
        let fit_screen_include_smaller_text = self.text(UiTextKey::FitScreenIncludeSmaller);
        let fit_screen_only_smaller_text = self.text(UiTextKey::FitScreenOnlySmaller);
        let nearest_text = self.text(UiTextKey::Nearest);
        let bilinear_text = self.text(UiTextKey::Bilinear);
        let bicubic_text = self.text(UiTextKey::Bicubic);
        let lanczos3_text = self.text(UiTextKey::Lanczos3);
        let stop_text = self.text(UiTextKey::Stop);
        let loop_text = self.text(UiTextKey::Loop);
        let next_text = self.text(UiTextKey::Next);
        let recursive_text = self.text(UiTextKey::Recursive);
        egui::Window::new(settings_text)
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Viewer, viewer_text);
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Render, render_text);
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Window, window_text);
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::Navigation,
                        navigation_text,
                    );
                    ui.selectable_value(&mut self.settings_tab, SettingsTab::Plugins, plugins_text);
                    ui.selectable_value(
                        &mut self.settings_tab,
                        SettingsTab::Resources,
                        resources_text,
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
                            .checkbox(&mut self.plugins.susie64.enable, enable_text)
                            .changed();
                        ui.label(search_path_text);
                        if ui
                            .text_edit_singleline(&mut self.susie64_search_paths_input)
                            .changed()
                        {
                            self.plugins.susie64.search_path =
                                parse_search_paths(&self.susie64_search_paths_input);
                            config_changed = true;
                        }
                        if ui.button(browse_text).clicked() {
                            if let Some(path) = pick_save_directory() {
                                self.plugins.susie64.search_path = vec![path];
                                self.susie64_search_paths_input =
                                    join_search_paths(&self.plugins.susie64.search_path);
                                config_changed = true;
                            }
                        }
                        if ui.button(load_modules_text).clicked() {
                            self.plugins.susie64.modules =
                                discover_plugin_modules("susie64", &self.plugins.susie64);
                            config_changed = true;
                        }
                        ui.label(format!(
                            "{}: {}",
                            modules_text,
                            self.plugins.susie64.modules.len()
                        ));
                        ui.separator();
                        ui.heading("system");
                        config_changed |= ui
                            .checkbox(&mut self.plugins.system.enable, enable_text)
                            .changed();
                        ui.label(search_path_os_api_text);
                        ui.label(format!(
                            "{}: {}",
                            modules_text,
                            self.plugins.system.modules.len()
                        ));
                        ui.separator();
                        ui.heading("ffmpeg");
                        config_changed |= ui
                            .checkbox(&mut self.plugins.ffmpeg.enable, enable_text)
                            .changed();
                        ui.label(search_path_text);
                        if ui
                            .text_edit_singleline(&mut self.ffmpeg_search_paths_input)
                            .changed()
                        {
                            self.plugins.ffmpeg.search_path =
                                parse_search_paths(&self.ffmpeg_search_paths_input);
                            config_changed = true;
                        }
                        if ui.button(browse_text).clicked() {
                            if let Some(path) = pick_save_directory() {
                                self.plugins.ffmpeg.search_path = vec![path];
                                self.ffmpeg_search_paths_input =
                                    join_search_paths(&self.plugins.ffmpeg.search_path);
                                config_changed = true;
                            }
                        }
                        if ui.button(load_modules_text).clicked() {
                            self.plugins.ffmpeg.modules =
                                discover_plugin_modules("ffmpeg", &self.plugins.ffmpeg);
                            config_changed = true;
                        }
                        ui.label(format!(
                            "{}: {}",
                            modules_text,
                            self.plugins.ffmpeg.modules.len()
                        ));
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
                        ui.separator();
                        ui.label(workaround_text);
                        ui.horizontal(|ui| {
                            ui.label(format!("{archive_text} ZIP"));
                            ui.label(threshold_mb_text);
                            config_changed |= ui
                                .add(
                                    egui::DragValue::new(
                                        &mut self.runtime.workaround.archive.zip.threshold_mb,
                                    )
                                    .range(16..=16_384)
                                    .speed(8.0),
                                )
                                .changed();
                            ui.checkbox(
                                &mut self.runtime.workaround.archive.zip.local_cache,
                                local_cache_text,
                            );
                        });
                    });
                }

                if self.settings_tab == SettingsTab::Render {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(zoom_mode_text);
                            let before = self.render_options.zoom_option.clone();
                            egui::ComboBox::from_id_salt("zoom_option")
                                .selected_text(zoom_option_label(
                                    &self.applied_locale,
                                    &self.render_options.zoom_option,
                                ))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_option,
                                        ZoomOption::None,
                                        none_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_option,
                                        ZoomOption::FitWidth,
                                        fit_width_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_option,
                                        ZoomOption::FitHeight,
                                        fit_height_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_option,
                                        ZoomOption::FitScreen,
                                        fit_screen_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_option,
                                        ZoomOption::FitScreenIncludeSmaller,
                                        fit_screen_include_smaller_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_option,
                                        ZoomOption::FitScreenOnlySmaller,
                                        fit_screen_only_smaller_text,
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
                                        nearest_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_method,
                                        InterpolationAlgorithm::Bilinear,
                                        bilinear_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_method,
                                        InterpolationAlgorithm::BicubicAlpha(None),
                                        bicubic_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.render_options.zoom_method,
                                        InterpolationAlgorithm::Lanzcos3,
                                        lanczos3_text,
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
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(register_system_text).clicked() {
                                match std::env::current_exe()
                                    .ok()
                                    .and_then(|exe| register_system_file_associations(&exe).ok())
                                {
                                    Some(()) => {
                                        self.save_dialog.message =
                                            Some(registered_file_associations_text.to_string());
                                    }
                                    None => {
                                        self.save_dialog.message =
                                            Some(failed_file_associations_text.to_string());
                                    }
                                }
                            }
                            if ui.button(clean_system_text).clicked() {
                                match clean_system_integration() {
                                    Ok(()) => {
                                        self.save_dialog.message =
                                            Some(cleaned_system_integration_text.to_string());
                                    }
                                    Err(err) => {
                                        self.save_dialog.message = Some(err.to_string());
                                    }
                                }
                            }
                        });
                    });
                }

                if self.settings_tab == SettingsTab::Navigation {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(end_of_folder_text);
                            let before = self.end_of_folder;
                            egui::ComboBox::from_id_salt("end_of_folder")
                                .selected_text(end_of_folder_label(
                                    &self.applied_locale,
                                    self.end_of_folder,
                                ))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.end_of_folder,
                                        EndOfFolderOption::Stop,
                                        stop_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.end_of_folder,
                                        EndOfFolderOption::Loop,
                                        loop_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.end_of_folder,
                                        EndOfFolderOption::Next,
                                        next_text,
                                    );
                                    ui.selectable_value(
                                        &mut self.end_of_folder,
                                        EndOfFolderOption::Recursive,
                                        recursive_text,
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
                    if ui.button(apply_text).clicked() {
                        apply_requested = true;
                    }
                    if ui.button(undo_text).clicked() {
                        undo_requested = true;
                    }
                    if ui.button(reset_text).clicked() {
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
            set_archive_zip_workaround(self.runtime.workaround.archive.zip.clone());
            set_runtime_plugin_config(self.plugins.clone());
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
        self.runtime = config.runtime;
        self.keymap = config.input.merged_with_defaults();
        self.end_of_folder = config.navigation.end_of_folder;
        self.navigation_sort = config.navigation.sort;
        self.save_dialog.output_dir = self
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
        set_archive_zip_workaround(self.runtime.workaround.archive.zip.clone());
        set_runtime_plugin_config(self.plugins.clone());
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
            runtime: self.runtime.clone(),
            input: Default::default(),
            resources: self.resources.clone(),
            navigation: NavigationOptions {
                end_of_folder: self.end_of_folder,
                sort: self.navigation_sort,
            },
        }
    }
}

fn end_of_folder_label(locale: &str, option: EndOfFolderOption) -> &'static str {
    match option {
        EndOfFolderOption::Stop => crate::ui::i18n::tr(locale, UiTextKey::Stop),
        EndOfFolderOption::Next => crate::ui::i18n::tr(locale, UiTextKey::Next),
        EndOfFolderOption::Loop => crate::ui::i18n::tr(locale, UiTextKey::Loop),
        EndOfFolderOption::Recursive => crate::ui::i18n::tr(locale, UiTextKey::Recursive),
    }
}

fn zoom_option_label(locale: &str, option: &ZoomOption) -> &'static str {
    match option {
        ZoomOption::None => crate::ui::i18n::tr(locale, UiTextKey::None),
        ZoomOption::FitWidth => crate::ui::i18n::tr(locale, UiTextKey::FitWidth),
        ZoomOption::FitHeight => crate::ui::i18n::tr(locale, UiTextKey::FitHeight),
        ZoomOption::FitScreen => crate::ui::i18n::tr(locale, UiTextKey::FitScreen),
        ZoomOption::FitScreenIncludeSmaller => {
            crate::ui::i18n::tr(locale, UiTextKey::FitScreenIncludeSmaller)
        }
        ZoomOption::FitScreenOnlySmaller => {
            crate::ui::i18n::tr(locale, UiTextKey::FitScreenOnlySmaller)
        }
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
