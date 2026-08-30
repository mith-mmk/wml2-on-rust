use wml2::highres::{ResourceLimits, avif};

#[test]
fn native_bridge_exposes_standalone_types() {
    use avif::{DecodeError, DecoderError, NativeDecodeLimits, decode_native};

    let native_limits = NativeDecodeLimits::new(
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
        .build()
        .unwrap();
    let _decode: fn(
        &[u8],
        &NativeDecodeLimits,
        &ResourceLimits,
    ) -> Result<wml2::highres::ImageFrame, DecodeError> = decode_native;
    let _codec_error: Option<DecoderError> = None;
    let _ = (native_limits, resource_limits);
}
