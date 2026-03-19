# wml2viewer preview

Minimal native image viewer built with `egui` and `wml2`.

## Features

- Image viewing with manga spread mode
- Filer with list / thumbnail / detail views
- ZIP and WML(listed files) virtual browsing
- Locale-aware UI resources and font fallback
- Save dialog with output format selection

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
```

## Notes

- Very large or network ZIP files use a low-I/O workaround.
- On Windows, file association registration is available from Settings.
