# wml2viewer preview

Minimal native image viewer built with `egui` and `wml2`.

## Features

- Async startup: the UI opens first and the initial image is decoded in the background
- Viewer / filer / subfiler layout with bottom status overlay and separate dialogs
- Config dialog now keeps system integration actions in a dedicated `System` tab
- Config changes are staged and applied only when `Apply` is pressed
- Manga spread mode for portrait pages when the viewport is wide enough
- Filer with list / thumbnail / detail views and drive/root switching
- Filer side can be switched left/right from Settings
- ZIP and WML(listed files) virtual browsing
- Save dialog with output format selection
- Locale-aware UI resources and font fallback, with locale editable from Settings
- Locale `Auto` fills the staged value from the current system locale without applying immediately
- Plugin decode pipeline with priority resolution across `internal`, `system`, `ffmpeg`, and `susie64`
- ZIP startup now keeps the UI responsive by resolving archive contents after the window opens
- Render / filer / thumbnail workers automatically respawn if a worker thread disconnects

## Run

```powershell
cargo run --manifest-path wml2viewer/Cargo.toml -- <path>
```

## Command line
- `wml2viewer` run default
- `wml2viewer [path]` run with image 
- `wml2viewer --config <path> [path]` run set conifg <path> toml file
- `wml2viewer --clean system` clean system data

## help
- https://mith-mmk.github.io/wml2/help.html


## Config

Config is stored under the platform-specific config directory.

Relevant runtime workaround example:

```toml
[runtime.workaround.archive.zip]
threshold_mb = 256
local_cache = true

[filesystem.thumbnail]
suppress_large_files = true

[resources]
font_paths = ["C:/Windows/Fonts/NotoSansJP-Regular.otf"]
```

Plugin config example:

```toml
[plugins.ffmpeg]
enable = true
search_path = ["../test/plugins/ffmpeg"]

[plugins.susie64]
enable = true
search_path = ["../test/plugins/susie64"]
```

## Notes

- Very large or network ZIP files use a low-I/O workaround.
- Large BMP/archive thumbnails can be suppressed from Settings.
- Thumbnail failures are cleared from the pending queue so the filer can retry.
- On Windows, file association registration is available from `Settings -> System`.
- `ffmpeg` decode currently shells out to `ffmpeg.exe`.
- `susie64` decode is Windows-only and currently targets image plugins.
- `system` decode now uses Windows WIC on Windows. macOS system codec runtime is still follow-up work.
- Filer and viewer also expose plugin-enabled extensions such as `avif` and `jp2` when the provider is enabled.
- Plugin setting changes show a restart recommendation popup.
- Manga companion pages stay inside the current folder or virtual archive branch.
- Windows font lookup now follows `%LOCALAPPDATA%\Microsoft\Windows\Fonts` then `%WINDIR%\Fonts`.
- Locale default system fonts stay first, and `resources.font_paths` lets you prepend custom fonts.

## Benchmarks

```powershell
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_decode -- .\samples\WML2Viewer.avif 5
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_browser -- .\samples 3
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip default
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip online_cache
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip temp_copy
```

`bench_archive` prints a normal error and returns a failure exit code instead of panicking when the input is unsupported.
