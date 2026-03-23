# wml2viewer 0.0.12 preview

Minimal native image viewer built with `egui` and `wml2`.

## Features

- Async startup: the UI opens first and the initial image is decoded in the background
- Viewer / filer / subfiler layout with bottom status overlay and separate dialogs
- Config dialog now keeps system integration actions in a dedicated `System` tab
- Config changes are staged and applied only when `Apply` is pressed
- `Apply` and `Cancel` both close the Settings dialog
- Plugin settings expose priority for `internal`, `system`, `ffmpeg`, and `susie64`
- Manga spread mode for portrait pages when the viewport is wide enough
- Filer with list / thumbnail / detail views and drive/root switching
- Filer name sorting now uses a dropdown, and ZIP is treated like a file by default in separated sort mode
- Filer side can be switched left/right from Settings
- ZIP and WML(listed files) virtual browsing
- Save dialog with output format selection
- Locale-aware UI resources and font fallback, with locale editable from Settings
- Locale `Auto` fills the staged value from the current system locale without applying immediately
- Plugin decode pipeline with priority resolution across `internal`, `system`, `ffmpeg`, and `susie64`
- ZIP startup now keeps the UI responsive by resolving archive contents after the window opens
- Startup now prioritizes the first viewer image before filer/filesystem worker synchronization
- Startup filesystem sync now follows the first resolved image path, reducing folder/ZIP startup stalls
- Windows release builds use the GUI subsystem so Explorer launch does not open a console window
- ZIP metadata loading now falls back to plain `BufReader<File>` if the cached reader path fails
- Navigation requests now keep a pending target, reducing stale-image state during folder/archive transitions
- Failed image loads now fall back to the loading texture instead of leaving the previous image onscreen
- Pointer defaults: left click advances after a short double-click wait, right click opens Settings, left double click toggles fit mode
- Render / filer / thumbnail workers automatically respawn if a worker thread disconnects
- Render workers now receive an explicit shutdown command on app exit

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
local_cache = false

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
- Local ZIP temp cache is now disabled by default to avoid slowing network/archive startup on SSD-heavy setups.
- Large BMP/archive thumbnails can be suppressed from Settings.
- Thumbnail failures are cleared from the pending queue so the filer can retry.
- Filer timestamps now use local system time instead of UTC, with locale-specific formatting.
- Filer `OS` name sort now uses Windows shell ordering on Windows and locale-aware normalized natural sort on other platforms.
- ZIP is treated as a file by default in separated sort mode; the hidden toggle is kept internally for later use.
- On Windows, file association registration is available from `Settings -> System`.
- `ffmpeg` decode currently shells out to `ffmpeg.exe`.
- `susie64` decode is Windows-only and currently targets image plugins.
- `system` decode now uses Windows WIC on Windows. macOS system codec runtime is still follow-up work.
- Filer and viewer also expose plugin-enabled extensions such as `avif` and `jp2` when the provider is enabled.
- Plugin setting changes show a restart recommendation popup.
- Manga companion pages stay inside the current folder or virtual archive branch.
- `bench_archive` continues even if some archive entries fail to decode, so ZIP metadata/read timing is still measurable.
- `ZipCacheReader` now uses larger chunks plus tail prefetch to reduce startup I/O on large archives.
- Windows font lookup now follows `%LOCALAPPDATA%\Microsoft\Windows\Fonts` then `%WINDIR%\Fonts`.
- Locale default system fonts stay first, and `resources.font_paths` lets you prepend custom fonts.

## Known Remaining Issues For 0.0.12

- Long ZIP extraction can still stall viewer responsiveness.
- `bench_archive` still needs work for very large archives such as `1.6GB`.
- On Windows, some plugin-folder/system-plugin flows may still surface command-line windows or leave `COM Surrogate` behind on forced exit.
- `LHA` support and keybinding UI are deferred to `0.0.13`.

## Benchmarks

```powershell
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_decode -- .\samples\WML2Viewer.avif 5
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_browser -- .\samples 3
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip default
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip online_cache
cargo run --manifest-path wml2viewer/Cargo.toml --example bench_archive -- .\some.zip temp_copy
```

`bench_archive` prints a normal error and returns a failure exit code instead of panicking when the input is unsupported.
