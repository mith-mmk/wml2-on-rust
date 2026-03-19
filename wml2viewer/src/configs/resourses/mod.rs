use crate::dependent::{
    emoji_font_candidates, locale_font_candidates, normalize_locale_tag, resource_locale_fallbacks,
    system_locale,
};
use eframe::egui::{self, FontFamily, FontId, TextStyle};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontSizePreset {
    #[default]
    Auto,
    S,
    M,
    L,
    LL,
}

#[derive(Clone, Debug)]
pub struct ResourceOptions {
    pub locale: Option<String>,
    pub font_size: FontSizePreset,
}

impl Default for ResourceOptions {
    fn default() -> Self {
        Self {
            locale: system_locale(),
            font_size: FontSizePreset::S,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppliedResources {
    pub locale: String,
    pub loaded_fonts: Vec<String>,
}

pub fn apply_resources(ctx: &egui::Context, options: &ResourceOptions) -> AppliedResources {
    let locale = normalized_locale(options.locale.as_deref());
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded_fonts = Vec::new();

    for (name, data) in load_font_data(candidate_font_paths(&locale)) {
        fonts.font_data.insert(name.clone(), data.into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, name.clone());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, name.clone());
        loaded_fonts.push(name);
    }

    ctx.set_fonts(fonts);
    apply_text_styles(ctx, options.font_size);

    AppliedResources {
        locale,
        loaded_fonts,
    }
}

pub fn normalized_locale(locale: Option<&str>) -> String {
    normalize_locale_tag(locale)
}

fn candidate_font_paths(locale: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for candidate in resource_locale_fallbacks(locale) {
        paths.extend(locale_font_candidates(&candidate));
    }
    paths.extend(emoji_font_candidates());
    paths
}

fn load_font_data(paths: Vec<PathBuf>) -> Vec<(String, egui::FontData)> {
    let mut loaded = Vec::new();
    let mut seen = BTreeMap::<String, ()>::new();
    for path in paths {
        if !path.exists() {
            continue;
        }
        let key = path.to_string_lossy().into_owned();
        if seen.insert(key, ()).is_some() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("custom-font")
            .to_string();
        loaded.push((name, egui::FontData::from_owned(bytes)));
    }
    loaded
}

fn apply_text_styles(ctx: &egui::Context, preset: FontSizePreset) {
    let pixels_per_point = ctx.pixels_per_point().max(1.0);
    let monitor_size = ctx.input(|i| {
        i.viewport()
            .monitor_size
            .unwrap_or(egui::vec2(1280.0, 720.0))
    });
    let body_size = match preset {
        FontSizePreset::Auto => auto_font_size(monitor_size, pixels_per_point),
        FontSizePreset::S => 13.0,
        FontSizePreset::M => 15.0,
        FontSizePreset::L => 17.0,
        FontSizePreset::LL => 19.0,
    };

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new((body_size - 2.0).max(10.0), FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(body_size, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(body_size, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(body_size, FontFamily::Monospace),
        ),
        (
            TextStyle::Heading,
            FontId::new(body_size + 4.0, FontFamily::Proportional),
        ),
    ]
    .into_iter()
    .collect();
    ctx.set_style(style);
}

fn auto_font_size(monitor_size: egui::Vec2, pixels_per_point: f32) -> f32 {
    let logical_min = monitor_size.x.min(monitor_size.y) / pixels_per_point.max(1.0);
    if logical_min >= 1400.0 {
        19.0
    } else if logical_min >= 1100.0 {
        17.0
    } else if logical_min >= 800.0 {
        15.0
    } else {
        13.0
    }
}
