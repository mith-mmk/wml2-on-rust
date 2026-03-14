/* OS dependent drivers */
// ここで名称ごと吸収する
#[cfg(target_family = "windows")]
use windows;
#[cfg(target_family = "macos")]
use darwin;

// BSD系もそのまま流用できそう
#[cfg(target_family = "linux")]
use linux;

#[cfg(target_family = "android")]
use android;

#[cfg(target_family = "ios")]
use ios;

///それ以外
#[not(cfg(target_family = "android", "linux", "windows", "ios", "macos"))]
use other;

