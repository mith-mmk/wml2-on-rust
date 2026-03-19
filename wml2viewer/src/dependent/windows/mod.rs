use crate::dependent::normalize_locale_tag;
use std::path::PathBuf;

const LOCALE_NAME_MAX_LENGTH: i32 = 85;

#[link(name = "Kernel32")]
unsafe extern "system" {
    fn GetUserDefaultLocaleName(locale_name: *mut u16, locale_name_count: i32) -> i32;
}

pub fn system_locale() -> Option<String> {
    let mut buf = [0_u16; LOCALE_NAME_MAX_LENGTH as usize];
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH) };
    if len <= 1 {
        return None;
    }

    String::from_utf16(&buf[..(len as usize - 1)])
        .ok()
        .map(|locale| normalize_locale_tag(Some(&locale)))
}

pub fn locale_font_candidates(locale: &str) -> Vec<PathBuf> {
    let mut fonts = Vec::new();
    if locale.starts_with("ja") {
        fonts.extend([
            PathBuf::from(r"C:\Windows\Fonts\YuGothicUI-Regular.ttf"),
            PathBuf::from(r"C:\Windows\Fonts\YuGothR.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\YuGothM.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\NotoSansJP-Regular.otf"),
            PathBuf::from(r"C:\Windows\Fonts\NotoSansCJK-Regular.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\meiryo.ttc"),
        ]);
    } else if locale.starts_with("zh") {
        fonts.extend([
            PathBuf::from(r"C:\Windows\Fonts\msjh.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\NotoSansTC-Regular.otf"),
            PathBuf::from(r"C:\Windows\Fonts\NotoSansCJK-Regular.ttc"),
        ]);
    }
    fonts.extend([
        PathBuf::from(r"C:\Windows\Fonts\arial.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\segoeui.ttf"),
    ]);
    fonts
}

pub fn emoji_font_candidates() -> Vec<PathBuf> {
    vec![PathBuf::from(r"C:\Windows\Fonts\seguiemj.ttf")]
}

pub fn available_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let path = PathBuf::from(&drive);
        if path.exists() {
            roots.push(path);
        }
    }

    if let Some(home) = std::env::var_os("USERPROFILE") {
        let home = PathBuf::from(home);
        if !roots.iter().any(|root| root == &home) {
            roots.push(home);
        }
    }

    roots
}
