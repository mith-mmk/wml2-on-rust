use std::path::PathBuf;

use wml2::highres::avif::{NativeDecodeLimits, decode_native};
use wml2::highres::{ChannelModel, PixelBuffer};

fn limits() -> NativeDecodeLimits {
    NativeDecodeLimits::new(
        64 * 1024 * 1024,
        16 * 1024,
        16 * 1024,
        16 * 1024 * 1024,
        64 * 1024 * 1024,
        8 * 1024 * 1024,
        8 * 1024 * 1024,
        128,
        512,
        128,
        16,
        128,
    )
}

#[test]
fn sample_maps_to_native_typed_planes_without_geometry_application() {
    let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("samples")
        .join("WML2Viewer.avif");
    let data = std::fs::read(sample).expect("the permanent WML2Viewer sample should be present");
    let frame = decode_native(&data, &limits()).expect("native AVIF sample should decode");

    assert!(matches!(frame.pixels(), PixelBuffer::U16(_)));
    assert!(frame.descriptor().width() > 0);
    assert!(frame.descriptor().height() > 0);
    assert!(matches!(
        frame.descriptor().model(),
        ChannelModel::RGB | ChannelModel::YCbCr | ChannelModel::Gray
    ));
    assert_eq!(
        frame.metadata().coded_dimensions(),
        Some((frame.descriptor().width(), frame.descriptor().height()))
    );
}
