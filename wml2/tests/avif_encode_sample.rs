use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use wml2::draw::{image_from, image_from_file, image_to};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn ffmpeg_path() -> PathBuf {
    env::var_os("AVIF_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

fn temporary_path(extension: &str) -> PathBuf {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "wml2-avif-sample-{}-{id}.{extension}",
        std::process::id()
    ))
}

fn sample_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/images/bundled/sample_lossless.webp")
}

#[test]
fn avif_encoder_round_trips_bundled_lossless_webp_sample() {
    let input = sample_path();
    if !input.is_file() {
        eprintln!("skipping missing bundled sample: {}", input.display());
        return;
    }

    let mut image = image_from_file(input.to_string_lossy().into_owned()).expect("decode sample");
    let source = image.buffer.clone().expect("sample RGBA buffer");
    let source_dimensions = (image.width, image.height);
    let source_has_alpha = source.chunks_exact(4).any(|pixel| pixel[3] != 255);

    let options = HashMap::from([(String::from("speed"), DataMap::UInt(10))]);
    let encoded =
        image_to(&mut image, ImageFormat::Avif, Some(options)).expect("encode sample as AVIF");
    assert!(encoded.windows(4).any(|window| window == b"av1C"));

    let decoded = image_from(&encoded).expect("decode encoded sample with avif-rust");
    assert_eq!((decoded.width, decoded.height), source_dimensions);
    let decoded_buffer = decoded.buffer.expect("decoded AVIF RGBA buffer");
    assert_eq!(decoded_buffer.len(), source.len());
    assert_eq!(
        decoded_buffer.chunks_exact(4).any(|pixel| pixel[3] != 255),
        source_has_alpha
    );

    let ffmpeg = ffmpeg_path();
    if !Command::new(&ffmpeg)
        .arg("-version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("ffmpeg is not available; skipping sample decode oracle");
        return;
    }

    let input_avif = temporary_path("avif");
    let raw = temporary_path("rgba");
    fs::write(&input_avif, encoded).expect("write encoded sample");
    let result = Command::new(&ffmpeg)
        .args([
            "-v",
            "error",
            "-y",
            "-i",
            input_avif.to_str().unwrap(),
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            raw.to_str().unwrap(),
        ])
        .output()
        .expect("run ffmpeg sample decode");
    assert!(
        result.status.success(),
        "ffmpeg sample decode failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::metadata(&raw).expect("ffmpeg raw output").len(),
        (source_dimensions.0 * source_dimensions.1 * 4) as u64
    );
    let _ = fs::remove_file(input_avif);
    let _ = fs::remove_file(raw);
}
