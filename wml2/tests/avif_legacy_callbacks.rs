#![cfg(feature = "avif")]
//! Legacy AVIF callback compatibility checks.
//!
//! To provision the required external AVIS fixture, run
//! `pwsh -File test/avif_external_compat.ps1 -DownloadMissing` from the
//! repository root. The AVIF-only gate is
//! `cargo test -p wml2 --features avif --test avif_legacy_callbacks -- --ignored`;
//! repeat it with `--features "avif highres"` to exercise the highres build.

use std::path::PathBuf;

use bin_rs::reader::BytesReader;
use wml2::draw::{
    CallbackResponse, DecodeOptions, DrawCallback, DrawOptions, InitOptions, NextOptions,
    TerminateOptions, VerboseOptions, image_decoder, image_load,
};
use wml2::metadata::DataMap;

type Error = Box<dyn std::error::Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AbortAt {
    Metadata,
    Init,
    Next,
    Draw,
}

#[derive(Debug)]
struct CallbackProbe {
    abort_at: Option<AbortAt>,
    events: Vec<&'static str>,
    draw_buffers: Vec<Vec<u8>>,
}

impl CallbackProbe {
    fn continuing() -> Self {
        Self {
            abort_at: None,
            events: Vec::new(),
            draw_buffers: Vec::new(),
        }
    }

    fn aborting(abort_at: AbortAt) -> Self {
        Self {
            abort_at: Some(abort_at),
            events: Vec::new(),
            draw_buffers: Vec::new(),
        }
    }

    fn response(&self, point: AbortAt) -> Option<CallbackResponse> {
        (self.abort_at == Some(point)).then(CallbackResponse::abort)
    }
}

impl DrawCallback for CallbackProbe {
    fn init(
        &mut self,
        _width: usize,
        _height: usize,
        _option: Option<InitOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.events.push("init");
        Ok(self.response(AbortAt::Init))
    }

    fn draw(
        &mut self,
        _start_x: usize,
        _start_y: usize,
        _width: usize,
        _height: usize,
        data: &[u8],
        _option: Option<DrawOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.events.push("draw");
        self.draw_buffers.push(data.to_vec());
        Ok(self.response(AbortAt::Draw))
    }

    fn terminate(
        &mut self,
        _term: Option<TerminateOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.events.push("terminate");
        Ok(Some(CallbackResponse::cont()))
    }

    fn next(&mut self, _next: Option<NextOptions>) -> Result<Option<CallbackResponse>, Error> {
        self.events.push("next");
        Ok(self.response(AbortAt::Next))
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
        _key: &str,
        _value: DataMap,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.events.push("metadata");
        Ok(self.response(AbortAt::Metadata))
    }
}

fn fixture(relative_path: &str) -> Vec<u8> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repository_root = manifest_dir
        .parent()
        .expect("wml2 must be nested below the repository root");
    let path = repository_root.join(relative_path);
    assert!(
        path.is_file(),
        "required AVIF callback fixture is missing: {}",
        path.display()
    );
    std::fs::read(path).expect("required AVIF callback fixture should be readable")
}

fn decode_with_probe(data: &[u8], probe: &mut CallbackProbe) -> Result<(), Error> {
    let mut reader = BytesReader::new(data);
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: probe,
    };
    image_decoder(&mut reader, &mut options).map(|_| ())
}

#[test]
fn avif_legacy_callbacks_keep_rgba8_and_normal_order() {
    let data = fixture("samples/WML2Viewer.avif");
    let expected = image_load(&data).expect("reference RGBA8 decode should succeed");
    let mut probe = CallbackProbe::continuing();
    decode_with_probe(&data, &mut probe).expect("callback decode should succeed");

    let init = probe
        .events
        .iter()
        .position(|event| *event == "init")
        .expect("normal decode should initialize first");
    let draw = probe
        .events
        .iter()
        .position(|event| *event == "draw")
        .expect("normal decode should draw");
    let terminate = probe
        .events
        .iter()
        .position(|event| *event == "terminate")
        .expect("normal decode should terminate");
    assert!(
        probe.events[..init]
            .iter()
            .all(|event| *event == "metadata")
    );
    assert!(init < draw && draw < terminate);
    assert_eq!(
        probe.draw_buffers,
        vec![expected.buffer.expect("RGBA8 buffer")]
    );
}

#[test]
fn avif_legacy_abort_at_init_skips_draw_and_terminate() {
    let data = fixture("samples/WML2Viewer.avif");
    let mut probe = CallbackProbe::aborting(AbortAt::Init);
    decode_with_probe(&data, &mut probe).expect("init abort should be a successful stop");

    assert!(probe.events.contains(&"metadata"));
    assert_eq!(probe.events.last(), Some(&"init"));
    assert!(!probe.events.contains(&"draw"));
    assert!(!probe.events.contains(&"terminate"));
}

#[test]
fn avif_legacy_abort_at_draw_skips_terminate() {
    let data = fixture("samples/WML2Viewer.avif");
    let mut probe = CallbackProbe::aborting(AbortAt::Draw);
    decode_with_probe(&data, &mut probe).expect("draw abort should be a successful stop");

    assert!(probe.events.contains(&"metadata"));
    assert_eq!(probe.events.last(), Some(&"draw"));
    assert!(!probe.events.contains(&"terminate"));
    assert_eq!(probe.draw_buffers.len(), 1);
}

#[test]
#[ignore = "requires the bootstrapped external AVIS fixture"]
fn avif_legacy_abort_at_next_skips_current_frame_draw_and_terminate() {
    let data = fixture("test/images/external/avif/unsupported/star-8bpc.avif");
    let mut probe = CallbackProbe::aborting(AbortAt::Next);
    decode_with_probe(&data, &mut probe).expect("next abort should be a successful stop");

    let init = probe
        .events
        .iter()
        .position(|event| *event == "init")
        .expect("animation should initialize first");
    assert!(
        probe.events[..init]
            .iter()
            .all(|event| *event == "metadata")
    );
    assert_eq!(&probe.events[init..], &["init", "next"]);
    assert!(probe.draw_buffers.is_empty());
}

#[test]
fn avif_legacy_metadata_abort_matches_continue_legacy_behavior() {
    let data = fixture("samples/WML2Viewer.avif");
    let mut continuing = CallbackProbe::continuing();
    decode_with_probe(&data, &mut continuing).expect("continue decode should succeed");
    let mut metadata_abort = CallbackProbe::aborting(AbortAt::Metadata);
    decode_with_probe(&data, &mut metadata_abort)
        .expect("metadata Abort should preserve the legacy successful decode");

    assert_eq!(metadata_abort.events, continuing.events);
    assert_eq!(metadata_abort.draw_buffers, continuing.draw_buffers);
    assert!(metadata_abort.events.contains(&"metadata"));
    assert!(metadata_abort.events.contains(&"draw"));
    assert!(metadata_abort.events.contains(&"terminate"));
    assert!(!metadata_abort.draw_buffers.is_empty());
}
