use crate::dependent::normalize_locale_tag;
use std::path::PathBuf;

pub fn system_locale() -> Option<String> {
    std::env::var("LC_ALL")
        .ok()
        .or_else(|| std::env::var("LC_MESSAGES").ok())
        .or_else(|| std::env::var("LANG").ok())
        .map(|locale| normalize_locale_tag(Some(&locale)))
}

pub fn locale_font_candidates(locale: &str) -> Vec<PathBuf> {
    let mut fonts = Vec::new();
    if locale.starts_with("ja") {
        fonts.extend([
            PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
            PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf"),
            PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansJP-Regular.otf"),
        ]);
    } else if locale.starts_with("zh") {
        fonts.extend([
            PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansTC-Regular.otf"),
            PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
        ]);
    }
    fonts.extend([
        PathBuf::from("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSans-Regular.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    ]);
    fonts
}

pub fn emoji_font_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf"),
        PathBuf::from("/usr/share/fonts/noto/NotoColorEmoji.ttf"),
    ]
}
