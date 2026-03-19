use crate::dependent::normalize_locale_tag;
use std::path::PathBuf;
use std::process::Command;

const LOCALE_NAME_MAX_LENGTH: i32 = 85;

#[link(name = "Kernel32")]
unsafe extern "system" {
    fn GetUserDefaultLocaleName(locale_name: *mut u16, locale_name_count: i32) -> i32;
}

pub fn system_locale() -> Option<String> {
    let mut buf = [0_u16; LOCALE_NAME_MAX_LENGTH as usize];
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH) };
    if len > 1 {
        return String::from_utf16(&buf[..(len as usize - 1)])
            .ok()
            .map(|locale| normalize_locale_tag(Some(&locale)));
    }

    std::env::var("LC_ALL")
        .ok()
        .or_else(|| std::env::var("LANG").ok())
        .or_else(|| std::env::var("LANGUAGE").ok())
        .map(|locale| normalize_locale_tag(Some(&locale)))
}

pub fn locale_font_candidates(locale: &str) -> Vec<PathBuf> {
    let mut fonts = Vec::new();
    if locale.starts_with("ja") {
        fonts.extend([
            PathBuf::from(r"C:\Windows\Fonts\YuGothR.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\YuGothM.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\YuGothB.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\meiryo.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\msgothic.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\NotoSansJP-Regular.otf"),
            PathBuf::from(r"C:\Windows\Fonts\NotoSansCJK-Regular.ttc"),
        ]);
    } else if locale.starts_with("zh") {
        fonts.extend([
            PathBuf::from(r"C:\Windows\Fonts\msjh.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\msyh.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\NotoSansTC-Regular.otf"),
            PathBuf::from(r"C:\Windows\Fonts\NotoSansCJK-Regular.ttc"),
        ]);
    }
    fonts.extend([
        PathBuf::from(r"C:\Windows\Fonts\segoeui.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\arialuni.ttf"),
        PathBuf::from(r"C:\Windows\Fonts\arial.ttf"),
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

pub fn pick_directory_dialog() -> Option<PathBuf> {
    let script = concat!(
        "Add-Type -AssemblyName System.Windows.Forms; ",
        "$dialog = New-Object System.Windows.Forms.FolderBrowserDialog; ",
        "$dialog.ShowNewFolderButton = $true; ",
        "if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { ",
        "  [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; ",
        "  Write-Output $dialog.SelectedPath",
        " }"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-STA", "-Command", script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

pub fn download_url_to_temp(url: &str) -> Option<PathBuf> {
    let temp_path = std::env::temp_dir().join(format!(
        "wml2viewer_url_{}.bin",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos()
    ));
    let script = format!(
        "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri '{}' -OutFile '{}'; Write-Output '{}'",
        url.replace('\'', "''"),
        temp_path.display().to_string().replace('\'', "''"),
        temp_path.display().to_string().replace('\'', "''")
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(temp_path)
}
