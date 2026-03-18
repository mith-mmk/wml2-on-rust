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

    String::from_utf16(&buf[..(len as usize - 1)]).ok()
}

pub fn locale_font_candidates(locale: &str) -> Vec<PathBuf> {
    let mut fonts = Vec::new();
    if locale.starts_with("ja") {
        fonts.extend([
            PathBuf::from(r"C:\Windows\Fonts\YuGothR.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\YuGothM.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\meiryo.ttc"),
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
