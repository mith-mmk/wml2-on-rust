use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use wml2::draw::{ImageBuffer, image_from, image_to};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn ffmpeg_path() -> PathBuf {
    env::var_os("AVIF_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

fn temporary_avif_path() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "wml2-avif-{}-{stamp}-{id}.avif",
        std::process::id()
    ))
}

fn gradient(width: usize, height: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&[(x * 7) as u8, (y * 9) as u8, ((x + y) * 5) as u8, 255]);
        }
    }
    rgba
}

fn rgb_error(source: &[u8], decoded: &[u8]) -> (f64, u8) {
    let mut total = 0u64;
    let mut maximum = 0u8;
    let mut samples = 0u64;
    for (source_pixel, decoded_pixel) in source.chunks_exact(4).zip(decoded.chunks_exact(4)) {
        for channel in 0..3 {
            let error = source_pixel[channel].abs_diff(decoded_pixel[channel]);
            total += u64::from(error);
            maximum = maximum.max(error);
            samples += 1;
        }
    }
    (total as f64 / samples as f64, maximum)
}

#[test]
fn avif_encoder_round_trips_through_wml2_decoder() {
    let width = 17;
    let height = 13;
    let source = gradient(width, height);
    let mut image = ImageBuffer::from_buffer(width, height, source);
    let encoded = image_to(&mut image, ImageFormat::Avif, None).expect("AVIF encode");
    assert!(encoded.starts_with(b"\0\0\0\x1cftypavif"));

    let decoded = image_from(&encoded).expect("AVIF decode");
    assert_eq!((decoded.width, decoded.height), (width, height));
    assert_eq!(
        decoded.buffer.as_ref().map(Vec::len),
        Some(width * height * 4)
    );
}

#[test]
fn avif_encoder_error_is_bounded_against_current_decoder() {
    let width = 17;
    let height = 13;
    let source = gradient(width, height);
    let mut image = ImageBuffer::from_buffer(width, height, source.clone());
    let encoded = image_to(&mut image, ImageFormat::Avif, None).expect("AVIF encode");
    let decoded = image_from(&encoded).expect("AVIF decode");
    let output = decoded.buffer.expect("decoded RGBA");
    assert_eq!(output.len(), source.len());
    let (average, maximum) = rgb_error(&source, &output);
    assert!(average <= 20.0, "average RGB error {average} exceeds limit");
    assert!(maximum <= 96, "maximum RGB error {maximum} exceeds limit");
}

#[test]
fn avif_encoder_accepts_libavif_option_aliases() {
    let mut image = ImageBuffer::from_buffer(4, 4, gradient(4, 4));
    let mut options = HashMap::new();
    options.insert("qcolor".to_string(), DataMap::UInt(80));
    options.insert("max".to_string(), DataMap::UInt(80));
    options.insert("maxalpha".to_string(), DataMap::UInt(80));
    options.insert("speed".to_string(), DataMap::Ascii("default".to_string()));
    options.insert("jobs".to_string(), DataMap::Ascii("all".to_string()));
    let encoded = image_to(&mut image, ImageFormat::Avif, Some(options)).expect("AVIF encode");
    assert!(encoded.windows(4).any(|window| window == b"av1C"));
}

#[test]
fn avif_encoder_consumes_source_metadata_options() {
    let source = gradient(2, 2);
    let mut image = ImageBuffer::from_buffer(2, 2, source.clone());
    image.metadata = Some(HashMap::from([("lossless".to_string(), DataMap::UInt(1))]));
    let encoded = image_to(&mut image, ImageFormat::Avif, None).expect("lossless AVIF encode");
    let decoded = image_from(&encoded).expect("lossless AVIF decode");
    assert_eq!(decoded.buffer, Some(source));
}

#[test]
fn ffmpeg_generated_avif_decodes_with_current_avif_rust() {
    let ffmpeg = ffmpeg_path();
    if !Command::new(&ffmpeg)
        .arg("-version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("ffmpeg is not available; skipping external AVIF input oracle");
        return;
    }
    let output = temporary_avif_path();
    let result = Command::new(&ffmpeg)
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=4x4:d=1",
            "-frames:v",
            "1",
            "-c:v",
            "libaom-av1",
            "-still-picture",
            "1",
            "-f",
            "avif",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    if !result.status.success() {
        eprintln!(
            "ffmpeg has no usable libaom AVIF encoder; skipping external AVIF input oracle: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let _ = fs::remove_file(&output);
        return;
    }
    let encoded = fs::read(&output).expect("ffmpeg should write an AVIF file");
    let decoded = image_from(&encoded).expect("current avif-rust should decode FFmpeg AVIF");
    assert_eq!((decoded.width, decoded.height), (4, 4));
    assert_eq!(decoded.buffer.as_ref().map(Vec::len), Some(4 * 4 * 4));
    let _ = fs::remove_file(output);
}

#[test]
fn avif_encoder_preserves_alpha_item_through_wml2_decode() {
    let mut rgba = gradient(4, 4);
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[3] = 96;
    }
    let mut image = ImageBuffer::from_buffer(4, 4, rgba);
    let encoded = image_to(&mut image, ImageFormat::Avif, None).expect("AVIF encode");
    let decoded = image_from(&encoded).expect("AVIF decode");
    let alpha = decoded.buffer.expect("decoded RGBA");
    assert!(alpha.chunks_exact(4).all(|pixel| pixel[3] < 255));
}

#[test]
fn avif_encoder_lossless_alpha_round_trips_through_wml2_decode() {
    let mut source = gradient(4, 4);
    for (index, pixel) in source.chunks_exact_mut(4).enumerate() {
        pixel[3] = (index * 17) as u8;
    }
    let mut image = ImageBuffer::from_buffer(4, 4, source.clone());
    image.metadata = Some(HashMap::from([("lossless".to_string(), DataMap::UInt(1))]));
    let encoded = image_to(&mut image, ImageFormat::Avif, None).expect("lossless AVIF encode");
    let decoded = image_from(&encoded).expect("lossless AVIF decode");
    assert_eq!(decoded.buffer, Some(source));
}

#[test]
fn avif_encoder_rejects_non_srgb_color_metadata() {
    let mut image = ImageBuffer::from_buffer(2, 2, gradient(2, 2));
    let mut options = HashMap::new();
    options.insert("cicp".to_string(), DataMap::UIntAllay(vec![9, 16, 9, 1]));
    let error = image_to(&mut image, ImageFormat::Avif, Some(options)).unwrap_err();
    assert!(error.to_string().contains("standard sRGB"));
}
