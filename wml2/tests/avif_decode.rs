use std::path::PathBuf;

use bin_rs::reader::BytesReader;
use wml2::draw::{
    CallbackResponse, DecodeOptions, DrawCallback, DrawOptions, InitOptions, NextOptions,
    TerminateOptions, VerboseOptions, image_decoder,
};
use wml2::metadata::DataMap;

type Error = Box<dyn std::error::Error>;

#[derive(Default)]
struct RecordingDrawer {
    events: Vec<String>,
}

impl DrawCallback for RecordingDrawer {
    fn init(
        &mut self,
        width: usize,
        height: usize,
        _option: Option<InitOptions>,
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
        self.events.push("terminate".to_string());
        Ok(Some(CallbackResponse::cont()))
    }

    fn next(&mut self, _next: Option<NextOptions>) -> Result<Option<CallbackResponse>, Error> {
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
        value: DataMap,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.events.push(format!("metadata:{key}={value:?}"));
        Ok(Some(CallbackResponse::cont()))
    }
}

fn sample_avif() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("samples")
            .join("WML2Viewer.avif"),
    )
    .expect("sample AVIF should exist")
}

#[cfg(feature = "avif")]
#[test]
fn format_check_recognizes_avif_sample() {
    let data = sample_avif();

    assert!(matches!(
        wml2::util::format_check(&data),
        wml2::util::ImageFormat::Avif
    ));
    assert!(wml2::get_can_decode(&data).unwrap());
    assert!(
        wml2::get_decoder_extentions()
            .iter()
            .any(|extension| extension == "avif")
    );
}

#[cfg(feature = "avif")]
#[test]
fn avif_decoder_emits_image_callbacks() {
    let data = sample_avif();
    let mut reader = BytesReader::new(&data);
    let mut drawer = RecordingDrawer::default();
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: &mut drawer,
    };

    image_decoder(&mut reader, &mut options).unwrap();

    assert!(drawer.events.iter().any(|event| event == "init:900x900"));
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event == "draw:0,0:900x900")
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:Format="))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AVIF bits per channel="))
    );
    assert!(drawer.events.iter().any(|event| event == "terminate"));
}

#[cfg(not(feature = "avif"))]
#[test]
fn avif_decoder_is_feature_gated() {
    let data = sample_avif();
    let mut reader = BytesReader::new(&data);
    let mut drawer = RecordingDrawer::default();
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: &mut drawer,
    };

    let err = image_decoder(&mut reader, &mut options).unwrap_err();

    assert!(err.to_string().contains("No Support format"));
    assert!(drawer.events.is_empty());
}
