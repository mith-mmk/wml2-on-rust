# wml2viewer

Minimal native image viewer built with `egui` and `wml2`.

## Features

- Image viewing with manga spread mode
- Filer with list / thumbnail / detail views
- ZIP and WML virtual browsing
- Locale-aware UI resources and font fallback
- Save dialog with output format selection

## Run

```powershell
cargo run --manifest-path wml2viewer/Cargo.toml -- <path>
```

## Command line

- `wml2viewer [path]`
- `wml2viewer --config <path> [path]`
- `wml2viewer --clean system`

## Config

Config is stored under the platform-specific config directory.

Relevant runtime workaround example:

```toml
[runtime.workaround.archive.zip]
threshold_mb = 256
local_cache = true
```

## Notes

- Very large or network ZIP files use a low-I/O workaround.
- On Windows, file association registration is available from Settings.
