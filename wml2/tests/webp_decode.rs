mod common;

use bin_rs::reader::BytesReader;
use common::{bundled_test_image_path, sample_bytes, sample_config_hint, sample_path};
use wml2::draw::{
    CallbackResponse, DecodeOptions, DrawCallback, DrawOptions, ImageBuffer, NextOptions,
    TerminateOptions, VerboseOptions, image_decoder, image_from_file, image_load, image_to,
};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

type Error = Box<dyn std::error::Error>;

#[derive(Default)]
struct RecordingDrawer {
    events: Vec<String>,
    terminate_count: usize,
    next_count: usize,
}

impl DrawCallback for RecordingDrawer {
    fn init(
        &mut self,
        width: usize,
        height: usize,
        _option: Option<wml2::draw::InitOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.events.push(format!("init:{width}x{height}"));
        Ok(Some(CallbackResponse::cont()))
    }

    fn draw(
        &mut self,
        start_x: usize,
        start_y: usize,
        width: usize,
        height: usize,
        _data: &[u8],
        _option: Option<DrawOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.events
            .push(format!("draw:{start_x},{start_y}:{width}x{height}"));
        Ok(Some(CallbackResponse::cont()))
    }

    fn terminate(
        &mut self,
        _term: Option<TerminateOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.terminate_count += 1;
        self.events.push("terminate".to_string());
        Ok(Some(CallbackResponse::cont()))
    }

    fn next(&mut self, _next: Option<NextOptions>) -> Result<Option<CallbackResponse>, Error> {
        self.next_count += 1;
        self.events.push("next".to_string());
        Ok(Some(CallbackResponse::cont()))
    }

    fn verbose(
        &mut self,
        _verbose: &str,
        _option: Option<VerboseOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        Ok(Some(CallbackResponse::cont()))
    }

    fn set_metadata(
        &mut self,
        key: &str,
        _value: DataMap,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.events.push(format!("metadata:{key}"));
        Ok(Some(CallbackResponse::cont()))
    }
}

fn animated_sample_bytes() -> Vec<u8> {
    vec![
        82, 73, 70, 70, 192, 0, 0, 0, 87, 69, 66, 80, 86, 80, 56, 88, 10, 0, 0, 0, 2, 0, 0, 0, 3,
        0, 0, 3, 0, 0, 65, 78, 73, 77, 6, 0, 0, 0, 255, 255, 255, 255, 1, 0, 65, 78, 77, 70, 72, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 3, 0, 0, 100, 0, 0, 2, 86, 80, 56, 32, 48, 0, 0, 0, 208,
        1, 0, 157, 1, 42, 4, 0, 4, 0, 2, 0, 52, 37, 160, 2, 116, 186, 1, 248, 0, 3, 176, 0, 254,
        240, 232, 247, 255, 32, 185, 97, 117, 200, 215, 255, 32, 63, 227, 42, 124, 101, 79, 248,
        242, 0, 0, 0, 65, 78, 77, 70, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 3, 0, 0, 100, 0, 0,
        0, 86, 80, 56, 32, 44, 0, 0, 0, 148, 1, 0, 157, 1, 42, 4, 0, 4, 0, 0, 0, 52, 37, 160, 2,
        116, 186, 0, 3, 152, 0, 254, 249, 147, 111, 255, 144, 31, 255, 144, 31, 255, 144, 31, 255,
        32, 63, 226, 23, 123, 32, 48, 0,
    ]
}

fn assert_webp_metadata(image: &wml2::draw::ImageBuffer, width: usize, height: usize, codec: &str) {
    let metadata = image.metadata.as_ref().unwrap();
    assert!(matches!(
        metadata.get("Format"),
        Some(DataMap::Ascii(format)) if format == "WEBP"
    ));
    assert!(matches!(
        metadata.get("width"),
        Some(DataMap::UInt(actual)) if *actual == width as u64
    ));
    assert!(matches!(
        metadata.get("height"),
        Some(DataMap::UInt(actual)) if *actual == height as u64
    ));
    assert!(matches!(
        metadata.get("WebP codec"),
        Some(DataMap::Ascii(actual)) if actual == codec
    ));
}

#[test]
fn decode_webp_still_samples_from_file() {
    let cases = [
        ("sample.webp", 1920, 1080, "Lossy"),
        ("sample_lossy.webp", 1152, 896, "Lossy"),
        ("sample_lossless.webp", 1152, 896, "Lossless"),
    ];

    for (name, width, height, codec) in cases {
        let Some(path) = sample_path(name) else {
            eprintln!(
                "skipping missing sample: {name} (configure {})",
                sample_config_hint().display()
            );
            continue;
        };
        let image = image_from_file(path.to_string_lossy().into_owned()).unwrap();
        assert_eq!(image.width, width);
        assert_eq!(image.height, height);
        assert!(
            image
                .buffer
                .as_ref()
                .map(|buffer| !buffer.is_empty())
                .unwrap_or(false)
        );
        assert_webp_metadata(&image, width, height, codec);
    }
}

#[test]
fn decode_webp_still_samples_from_bytes() {
    let cases = [
        ("sample.webp", 1920, 1080, "Lossy"),
        ("sample_lossy.webp", 1152, 896, "Lossy"),
        ("sample_lossless.webp", 1152, 896, "Lossless"),
    ];

    for (name, width, height, codec) in cases {
        let Some(bytes) = sample_bytes(name) else {
            eprintln!(
                "skipping missing sample: {name} (configure {})",
                sample_config_hint().display()
            );
            continue;
        };
        let image = image_load(&bytes).unwrap();
        assert_eq!(image.width, width);
        assert_eq!(image.height, height);
        assert!(
            image
                .buffer
                .as_ref()
                .map(|buffer| !buffer.is_empty())
                .unwrap_or(false)
        );
        assert_webp_metadata(&image, width, height, codec);
    }
}

#[test]
fn decode_animated_webp_and_collect_frames() {
    let bytes = animated_sample_bytes();
    let image = image_load(&bytes).unwrap();

    assert_eq!(image.width, 4);
    assert_eq!(image.height, 4);
    assert_eq!(image.first_wait_time, Some(100));
    assert_eq!(image.animation.as_ref().map(|frames| frames.len()), Some(2));

    let metadata = image.metadata.as_ref().unwrap();
    assert!(matches!(
        metadata.get("WebP animated"),
        Some(DataMap::Ascii(flag)) if flag == "true"
    ));
    assert!(matches!(
        metadata.get("Animation frames"),
        Some(DataMap::UInt(count)) if *count == 2
    ));
    assert!(matches!(
        metadata.get("Animation frame durations"),
        Some(DataMap::UIntAllay(durations)) if durations == &vec![100, 100]
    ));
}

#[test]
fn decode_tracked_animated_webp_sample_from_file() {
    let Some(path) = sample_path("sample_animation.webp") else {
        eprintln!(
            "skipping missing sample: sample_animation.webp (configure {})",
            sample_config_hint().display()
        );
        return;
    };

    let image = image_from_file(path.to_string_lossy().into_owned()).unwrap();
    let metadata = image.metadata.as_ref().unwrap();

    assert!(
        image
            .animation
            .as_ref()
            .map(|frames| frames.len())
            .unwrap_or(0)
            > 1
    );
    assert!(image.first_wait_time.is_some());
    assert!(matches!(
        metadata.get("WebP animated"),
        Some(DataMap::Ascii(flag)) if flag == "true"
    ));
    assert!(matches!(
        metadata.get("Animation frames"),
        Some(DataMap::UInt(count)) if *count > 1
    ));
}

#[test]
fn decode_tracked_animated_webp_sample_from_bytes() {
    let Some(bytes) = sample_bytes("sample_animation.webp") else {
        eprintln!(
            "skipping missing sample: sample_animation.webp (configure {})",
            sample_config_hint().display()
        );
        return;
    };

    let image = image_load(&bytes).unwrap();
    let metadata = image.metadata.as_ref().unwrap();

    assert!(
        image
            .animation
            .as_ref()
            .map(|frames| frames.len())
            .unwrap_or(0)
            > 1
    );
    assert!(image.first_wait_time.is_some());
    assert!(matches!(
        metadata.get("WebP animated"),
        Some(DataMap::Ascii(flag)) if flag == "true"
    ));
    assert!(matches!(
        metadata.get("Animation frames"),
        Some(DataMap::UInt(count)) if *count > 1
    ));
}

#[test]
fn decode_viewer_error_webp_sample() {
    let path = bundled_test_image_path("WML2Viewer_error.webp");
    let image = image_from_file(path.to_string_lossy().into_owned()).unwrap();
    let metadata = image.metadata.as_ref().unwrap();

    assert_eq!(image.width, 900);
    assert_eq!(image.height, 900);
    assert!(image.first_wait_time.is_some());
    assert!(
        image
            .buffer
            .as_ref()
            .map(|buffer| !buffer.is_empty())
            .unwrap_or(false)
            || image
                .animation
                .as_ref()
                .map(|frames| !frames.is_empty())
                .unwrap_or(false)
    );
    assert!(matches!(
        metadata.get("WebP animated"),
        Some(DataMap::Ascii(flag)) if flag == "true"
    ));
}

#[test]
fn webp_decoder_keeps_draw_callback_contract_for_still_images() {
    let mut source = ImageBuffer::from_buffer(
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    );
    let webp = image_to(&mut source, ImageFormat::Webp, None).unwrap();
    let mut reader = BytesReader::new(&webp);
    let mut drawer = RecordingDrawer::default();
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: &mut drawer,
    };

    image_decoder(&mut reader, &mut options).unwrap();

    assert_eq!(drawer.terminate_count, 1);
    assert_eq!(drawer.next_count, 0);
    assert!(
        drawer
            .events
            .first()
            .is_some_and(|event| event == "init:2x2")
    );
    assert!(drawer.events.iter().any(|event| event == "draw:0,0:2x2"));
    assert!(drawer.events.iter().any(|event| event == "metadata:Format"));
    assert_eq!(drawer.events.last().map(String::as_str), Some("terminate"));
}

#[test]
fn webp_decoder_keeps_draw_callback_contract_for_animation() {
    let bytes = animated_sample_bytes();
    let mut reader = BytesReader::new(&bytes);
    let mut drawer = RecordingDrawer::default();
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: &mut drawer,
    };

    image_decoder(&mut reader, &mut options).unwrap();

    assert_eq!(drawer.terminate_count, 1);
    assert_eq!(drawer.next_count, 2);
    assert!(
        drawer
            .events
            .first()
            .is_some_and(|event| event == "init:4x4")
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event == "metadata:Animation frames")
    );
    assert_eq!(drawer.events.last().map(String::as_str), Some("terminate"));
}
