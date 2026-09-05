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

#[test]
fn omitted_av1_description_retains_independent_range() {
    for (bytes, full) in [
        (
            include_bytes!("fixtures/review/av1-limited-omitted.avif").as_slice(),
            false,
        ),
        (
            include_bytes!("fixtures/review/av1-full-omitted.avif").as_slice(),
            true,
        ),
    ] {
        let raw = avif_codec::decode_frame_bytes_strict_with_limits(bytes, &limits()).unwrap();
        let (raw, rich) = raw.into_frame_and_rich();
        assert!(raw.color_config.color_description.is_none());
        assert!(rich.color_information.nclx.is_none());
        let frame = decode_native(bytes, &limits()).unwrap();
        let color = frame.descriptor().color_information().av1().unwrap();
        assert!(!color.description_present());
        assert_eq!(color.full_range(), full);
        assert_eq!(frame.metadata().source_color().av1(), Some(color));
        let explicit = wml2::highres::Av1ColorInformation::new(2, 2, 2, full);
        assert!(explicit.description_present());
        assert_ne!(explicit, color);
        let mut conflicting = bytes.to_vec();
        let property = conflicting
            .windows(4)
            .position(|tag| tag == b"free")
            .unwrap();
        conflicting[property..property + 4].copy_from_slice(b"colr");
        conflicting[property + 14] = if full { 0 } else { 0x80 };
        let frame = decode_native(&conflicting, &limits()).unwrap();
        let colors = frame.descriptor().color_information();
        assert_eq!(colors.av1().unwrap().full_range(), full);
        assert_eq!(colors.nclx().unwrap().full_range(), !full);
    }
}
