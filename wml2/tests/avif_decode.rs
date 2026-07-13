use std::path::PathBuf;

use bin_rs::reader::BytesReader;
use wml2::draw::{
    CallbackResponse, DecodeOptions, DrawCallback, DrawOptions, InitOptions, NextOptions,
    TerminateOptions, VerboseOptions, image_decoder,
};
use wml2::metadata::DataMap;
use wml2::util::{ImageFormat, format_check};

type Error = Box<dyn std::error::Error>;

#[derive(Default)]
struct RecordingDrawer {
    events: Vec<String>,
    draw_buffers: Vec<Vec<u8>>,
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
        data: &[u8],
        _option: Option<DrawOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.draw_buffers.push(data.to_vec());
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

fn filter_disabled_fixture() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("avif")
            .join("test_data")
            .join("images")
            .join("filter-disabled-gbr.avif"),
    )
    .expect("filter-disabled AVIF fixture should exist; run avif/scripts/bootstrap_oracles.ps1")
}

#[test]
fn format_check_recognizes_avif_sample() {
    let data = sample_avif();

    assert!(matches!(format_check(&data), ImageFormat::Avif));
    assert!(wml2::get_can_decode(&data).unwrap());
    assert!(
        wml2::get_decoder_extentions()
            .iter()
            .any(|extension| extension == "avif")
    );
}

#[test]
fn avif_decoder_decodes_sample_with_active_filters() {
    let data = sample_avif();
    let mut reader = BytesReader::new(&data);
    let mut drawer = RecordingDrawer::default();
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: &mut drawer,
    };

    image_decoder(&mut reader, &mut options).expect("supported filtered AVIF should decode");
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
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 config version=UInt(1)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 frame width=UInt(900)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 render height=UInt(900)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 payload after header offset=UInt("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 base q idx=UInt("))
    );
    for key in [
        "AV1 y dc quant",
        "AV1 y ac quant",
        "AV1 u dc quant",
        "AV1 u ac quant",
        "AV1 v dc quant",
        "AV1 v ac quant",
    ] {
        assert!(
            drawer
                .events
                .iter()
                .any(|event| event.starts_with(&format!("metadata:{key}=UInt("))),
            "missing {key} metadata"
        );
    }
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 superblock size=UInt(128)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 superblock columns=UInt(8)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 superblock rows=UInt(8)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 tile columns=UInt(1)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 tile rows=UInt(1)"))
    );
    assert!(drawer.events.iter().any(|event| {
        event.starts_with("metadata:AV1 tile group payload bytes=UInt(")
            && !event.starts_with("metadata:AV1 tile group payload bytes=UInt(0)")
    }));
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 tile group in frame OBU=UInt(1)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 tile group start=UInt(0)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 tile group end=UInt(0)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 tile payload count=UInt(1)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 entropy init bits=UInt(15)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 entropy tile count=UInt(1)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 root partition symbol=UInt("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 root partition=Ascii("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first block skip=UInt("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first block y mode=Ascii("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first block uv mode=Ascii("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first block transform count=UInt("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first transform size=Ascii("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first block residual skipped=UInt("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first block zero transform count=UInt("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first transform txb skip context=UInt("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first transform all zero symbol=UInt("))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first transform all zero=UInt("))
    );
    let first_transform_all_zero = drawer
        .events
        .iter()
        .find_map(|event| match event.as_str() {
            "metadata:AV1 first transform all zero=UInt(0)" => Some(false),
            "metadata:AV1 first transform all zero=UInt(1)" => Some(true),
            _ => None,
        })
        .expect("first transform all-zero metadata should be emitted");
    if !first_transform_all_zero {
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first non-zero transform index=UInt(")
        }));
    }
    let has_non_zero_transform = drawer
        .events
        .iter()
        .any(|event| event.starts_with("metadata:AV1 first non-zero transform index=UInt("));
    if has_non_zero_transform {
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first non-zero transform size=Ascii(")
        }));
        assert!(
            drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform tx type read=UInt(")
            })
        );
        let first_transform_tx_type = drawer.events.iter().find_map(|event| {
            event
                .strip_prefix("metadata:AV1 first transform tx type=Ascii(")
                .and_then(|value| value.strip_suffix(')'))
                .map(|value| value.trim_matches('"').to_string())
        });
        if drawer
            .events
            .iter()
            .any(|event| event == "metadata:AV1 first transform tx type read=UInt(1)")
        {
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform tx type set=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform tx type symbol=UInt(")
            }));
            assert!(first_transform_tx_type.is_some());
        }
        assert!(
            drawer
                .events
                .iter()
                .any(|event| event.starts_with("metadata:AV1 first transform eob multisize=UInt("))
        );
        assert!(
            drawer
                .events
                .iter()
                .any(|event| event.starts_with("metadata:AV1 first transform eob pt symbol=UInt("))
        );
        assert!(
            drawer
                .events
                .iter()
                .any(|event| event.starts_with("metadata:AV1 first transform eob base=UInt("))
        );
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform eob extra literal bits=UInt(")
        }));
        assert!(
            drawer
                .events
                .iter()
                .any(|event| event.starts_with("metadata:AV1 first transform eob=UInt("))
        );
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform coeff base eob context=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform coeff base eob symbol=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform coeff base eob level=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform regular coeff base count=UInt(")
        }));
        let regular_coeff_base_count = drawer
            .events
            .iter()
            .find_map(|event| {
                event
                    .strip_prefix("metadata:AV1 first transform regular coeff base count=UInt(")
                    .and_then(|value| value.strip_suffix(')'))
                    .and_then(|value| value.parse::<usize>().ok())
            })
            .expect("regular coeff_base count metadata should be emitted");
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform regular coeff base decoded count=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform coeff base non-zero count=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform coeff base range count=UInt(")
        }));
        let coeff_base_range_count = drawer
            .events
            .iter()
            .find_map(|event| {
                event
                    .strip_prefix("metadata:AV1 first transform coeff base range count=UInt(")
                    .and_then(|value| value.strip_suffix(')'))
                    .and_then(|value| value.parse::<usize>().ok())
            })
            .expect("coeff_base range count metadata should be emitted");
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform coeff br decoded count=UInt(")
        }));
        if coeff_base_range_count > 0 {
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff br scan index=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff br position=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff br context=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff br symbol=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff br level=UInt(")
            }));
        }
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform coeff sign decoded count=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform coeff golomb decoded count=UInt(")
        }));
        let sign_decoded_count = drawer
            .events
            .iter()
            .find_map(|event| {
                event
                    .strip_prefix("metadata:AV1 first transform coeff sign decoded count=UInt(")
                    .and_then(|value| value.strip_suffix(')'))
                    .and_then(|value| value.parse::<usize>().ok())
            })
            .expect("coeff sign decoded count metadata should be emitted");
        if sign_decoded_count > 0 {
            let has_dc_sign = drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform dc sign symbol=UInt(")
            });
            let has_ac_sign = drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first ac sign bit=UInt(")
            });
            assert!(has_dc_sign || has_ac_sign);
        }
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform signed coeff non-zero count=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform first signed coeff scan index=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform first signed coeff position=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform first signed coeff value=Ascii(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform dequant non-zero count=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform first dequant coeff position=UInt(")
        }));
        assert!(drawer.events.iter().any(|event| {
            event.starts_with("metadata:AV1 first transform first dequant coeff value=Ascii(")
        }));
        if matches!(
            first_transform_tx_type.as_deref(),
            Some("DctDct" | "Identity" | "VerticalDct" | "HorizontalDct")
        ) {
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform residual preview tx type=Ascii(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event
                    .starts_with("metadata:AV1 first transform residual preview sample count=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with(
                    "metadata:AV1 first transform first residual preview sample=Ascii(",
                )
            }));
        } else {
            assert!(!drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform residual preview tx type=Ascii(")
            }));
        }
        if regular_coeff_base_count > 0 {
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff base scan index=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff base position=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff base context=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with(
                    "metadata:AV1 first transform first coeff base reference magnitude=UInt(",
                )
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff base symbol=UInt(")
            }));
            assert!(drawer.events.iter().any(|event| {
                event.starts_with("metadata:AV1 first transform first coeff base level=UInt(")
            }));
        }
    }
    assert!(drawer.events.iter().any(|event| {
        event.starts_with("metadata:AV1 first block residual bit position=UInt(")
    }));
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 plane count=UInt(3)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 decode tile count=UInt(1)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first tile pixel width=UInt(900)"))
    );
    assert!(
        drawer
            .events
            .iter()
            .any(|event| event.starts_with("metadata:AV1 first tile pixel height=UInt(900)"))
    );
    let callback_events = drawer
        .events
        .iter()
        .filter(|event| {
            event.starts_with("init:")
                || event.starts_with("draw:")
                || event.as_str() == "terminate"
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        callback_events.first().map(String::as_str),
        Some("init:900x900")
    );
    assert!(
        callback_events
            .iter()
            .any(|event| event.starts_with("draw:0,0:900x900"))
    );
    assert_eq!(
        callback_events.last().map(String::as_str),
        Some("terminate")
    );
    assert_eq!(drawer.draw_buffers.len(), 1);
    assert_eq!(drawer.draw_buffers[0].len(), 900 * 900 * 4);
}

#[test]
fn avif_decoder_keeps_callback_order_for_filter_disabled_fixture() {
    let data = filter_disabled_fixture();
    let mut reader = BytesReader::new(&data);
    let mut drawer = RecordingDrawer::default();
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: &mut drawer,
    };

    image_decoder(&mut reader, &mut options).expect("filter-disabled AVIF should decode");

    let callback_events = drawer
        .events
        .iter()
        .filter(|event| {
            event.starts_with("init:")
                || event.starts_with("draw:")
                || event.as_str() == "terminate"
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        callback_events.first().map(String::as_str),
        Some("init:16x16")
    );
    assert!(
        callback_events
            .iter()
            .any(|event| event.starts_with("draw:0,0:16x16"))
    );
    assert_eq!(
        callback_events.last().map(String::as_str),
        Some("terminate")
    );
    assert_eq!(
        callback_events
            .iter()
            .filter(|event| event.starts_with("init:"))
            .count(),
        1
    );
    assert_eq!(
        callback_events
            .iter()
            .filter(|event| event.as_str() == "terminate")
            .count(),
        1
    );
    assert_eq!(drawer.draw_buffers.len(), 1);
    assert_eq!(drawer.draw_buffers[0].len(), 16 * 16 * 4);
}
