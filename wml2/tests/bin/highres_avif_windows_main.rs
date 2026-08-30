//! Executable smoke test for the public high-resolution AVIF bridge.
//!
//! This is a real `main`, rather than a libtest function, so allocator and
//! process teardown are part of the exercised path on Windows.

#[cfg(windows)]
fn main() {
    if let Err(error) = run() {
        eprintln!("highres AVIF public bridge failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    use wml2::highres::{ResourceLimits, avif};

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../samples/WML2Viewer.avif"),
        manifest.join("samples/WML2Viewer.avif"),
        manifest.join("../avif/test_data/images/WML2Viewer.avif"),
    ];
    let sample = candidates
        .iter()
        .find(|path| path.is_file())
        .ok_or("WML2Viewer.avif sample is not available")?;
    let data = fs::read(sample)?;

    let native_limits = avif::NativeDecodeLimits::new(
        1 << 20,
        4096,
        4096,
        16_777_216,
        1 << 24,
        1 << 20,
        1 << 20,
        64,
        256,
        4096,
        32,
        64,
    );
    let resource_limits = ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(4096)
        .max_height(4096)
        .max_pixels(16_777_216)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 24)
        .max_frame_bytes(1 << 26)
        .max_total_live_decoded_bytes(1 << 27)
        .max_references(64)
        .max_frame_count(64)
        .max_metadata_bytes(1 << 20)
        .max_icc_bytes(1 << 20)
        .max_clut_bytes(1 << 24)
        .max_parser_entries(1 << 16)
        .max_parser_depth(64)
        .max_grid_cells(1 << 16)
        .max_derived_work(1 << 20)
        .max_derived_depth(32)
        .build()?;

    let frame = avif::decode_native(&data, &native_limits, &resource_limits)?;
    let planes = frame
        .pixels()
        .u16_planes()
        .ok_or("public native bridge did not retain U16 planes")?;
    if planes.is_empty() || planes.iter().any(|plane| plane.samples().is_empty()) {
        return Err("public native bridge returned empty native planes".into());
    }
    if frame.descriptor().width() == 0 || frame.descriptor().height() == 0 {
        return Err("public native bridge returned empty dimensions".into());
    }
    Ok(())
}
