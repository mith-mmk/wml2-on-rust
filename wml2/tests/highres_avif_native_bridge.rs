use std::path::{Path, PathBuf};

use avif_codec::{decode_frame_bytes_strict_with_limits, parse_native_info};
use wml2::highres::{ChannelModel, ChannelRole, ResourceLimits, avif};

fn sample_path(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("test/images/external/avif");
    let supported = root.join("supported").join(name);
    if supported.is_file() {
        supported
    } else {
        root.join("unsupported").join(name)
    }
}

fn native_limits(input: usize) -> avif::NativeDecodeLimits {
    avif::NativeDecodeLimits::new(
        input,
        8192,
        8192,
        1 << 28,
        1 << 30,
        1 << 30,
        1 << 30,
        128,
        1024,
        1 << 24,
        128,
        1024,
    )
}

fn resource_limits(input: usize) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(input)
        .max_width(8192)
        .max_height(8192)
        .max_pixels(1 << 28)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 30)
        .max_frame_bytes(1 << 30)
        .max_total_live_decoded_bytes(1 << 30)
        .max_references(128)
        .max_frame_count(1024)
        .max_metadata_bytes(1 << 24)
        .max_icc_bytes(1 << 24)
        .max_clut_bytes(1 << 28)
        .max_parser_entries(1 << 20)
        .max_parser_depth(128)
        .max_grid_cells(1 << 20)
        .max_derived_work(1 << 28)
        .max_derived_depth(64)
        .build()
        .expect("fixture limits are valid")
}

fn box_bytes(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(payload.len() + 8);
    output.extend_from_slice(&u32::try_from(payload.len() + 8).unwrap().to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(payload);
    output
}

/// Re-wrap a real AV1 item in a minimal ordinary AVIF container while varying
/// the order and kind of its colour properties.  The AV1 bytes remain those
/// from the conformance fixture, so this exercises strict decode and the
/// public WML2 bridge rather than only the metadata parser.
fn synthetic_colored_avif(source: &[u8], color_order: &[&[u8; 4]]) -> Vec<u8> {
    let source_info = avif_codec::container::parse_avif(source).unwrap();
    let width = source_info.width.unwrap();
    let height = source_info.height.unwrap();
    let bit_depth = source_info
        .pixel_information
        .as_ref()
        .and_then(|pixi| pixi.bits_per_channel.first().copied())
        .unwrap_or(8);
    let av1_config = source_info.av1_config.as_ref().unwrap();
    let payload = source_info.primary_item_payload;
    let source_frame = decode_frame_bytes_strict_with_limits(source, &native_limits(source.len()))
        .unwrap()
        .into_frame();
    let source_description = source_frame
        .color_config
        .color_description
        .expect("source fixture must expose AV1 colour signalling");
    let source_full_range = matches!(
        source_frame.color_config.color_range,
        avif_codec::av1::ColorRange::Full
    );

    let mut ftyp_payload = Vec::from(&b"avif"[..]);
    ftyp_payload.extend_from_slice(&[0, 0, 0, 0]);
    ftyp_payload.extend_from_slice(b"avif");
    let ftyp = box_bytes(b"ftyp", &ftyp_payload);

    let mut properties = Vec::new();
    let mut ispe = vec![0, 0, 0, 0];
    ispe.extend_from_slice(&width.to_be_bytes());
    ispe.extend_from_slice(&height.to_be_bytes());
    properties.push(box_bytes(b"ispe", &ispe));
    let mut pixi = vec![0, 0, 0, 0, 3];
    pixi.extend_from_slice(&[bit_depth, bit_depth, bit_depth]);
    properties.push(box_bytes(b"pixi", &pixi));
    properties.push(box_bytes(b"av1C", av1_config));
    for color_type in color_order {
        let payload = match *color_type {
            b"nclx" => {
                let mut value = Vec::with_capacity(7);
                value.extend_from_slice(
                    &u16::from(source_description.color_primaries).to_be_bytes(),
                );
                value.extend_from_slice(
                    &u16::from(source_description.transfer_characteristics).to_be_bytes(),
                );
                value.extend_from_slice(
                    &u16::from(source_description.matrix_coefficients).to_be_bytes(),
                );
                value.push(u8::from(source_full_range) << 7);
                value
            }
            b"prof" => vec![1, 2, 3, 4],
            b"rICC" => vec![5, 6, 7, 8],
            b"zzzz" => vec![9, 8, 7],
            _ => panic!("unsupported synthetic colour property"),
        };
        let mut color = Vec::from(*color_type);
        color.extend_from_slice(&payload);
        properties.push(box_bytes(b"colr", &color));
    }
    let property_count = properties.len();
    let ipco_payload = properties.into_iter().flatten().collect::<Vec<_>>();
    let ipco = box_bytes(b"ipco", &ipco_payload);
    let mut ipma_payload = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 1];
    ipma_payload.push(u8::try_from(property_count).unwrap());
    ipma_payload.extend(1..=u8::try_from(property_count).unwrap());
    let ipma = box_bytes(b"ipma", &ipma_payload);
    let mut iprp_payload = ipco;
    iprp_payload.extend_from_slice(&ipma);
    let iprp = box_bytes(b"iprp", &iprp_payload);

    let mut pitm_payload = vec![0, 0, 0, 0];
    pitm_payload.extend_from_slice(&[0, 1]);
    let pitm = box_bytes(b"pitm", &pitm_payload);
    let mut infe_payload = vec![2, 0, 0, 0, 0, 1, 0, 0];
    infe_payload.extend_from_slice(b"av01");
    infe_payload.extend_from_slice(b"primary\0");
    let infe = box_bytes(b"infe", &infe_payload);
    let mut iinf_payload = vec![0, 0, 0, 0, 0, 1];
    iinf_payload.extend_from_slice(&infe);
    let iinf = box_bytes(b"iinf", &iinf_payload);

    let mut meta_prefix = vec![0, 0, 0, 0];
    meta_prefix.extend_from_slice(&pitm);
    meta_prefix.extend_from_slice(&iinf);
    meta_prefix.extend_from_slice(&iprp);
    let iloc_size = 8 + 8 + 14;
    let primary_offset = ftyp.len() + 8 + meta_prefix.len() + iloc_size + 8;
    let mut iloc_payload = vec![0, 0, 0, 0, 0x44, 0, 0, 1, 0, 1, 0, 0, 0, 1];
    iloc_payload.extend_from_slice(&u32::try_from(primary_offset).unwrap().to_be_bytes());
    iloc_payload.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_be_bytes());
    let iloc = box_bytes(b"iloc", &iloc_payload);
    let mut meta_payload = meta_prefix;
    meta_payload.extend_from_slice(&iloc);

    let mut output = ftyp;
    output.extend_from_slice(&box_bytes(b"meta", &meta_payload));
    output.extend_from_slice(&box_bytes(b"mdat", &payload));
    output
}

fn decode_fixture(name: &str) -> (avif_codec::DecodedFrame, wml2::highres::ImageFrame) {
    let path = sample_path(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!("fixture {} is required ({}): {error}", path.display(), name)
    });
    let limits = native_limits(bytes.len());
    let reference = decode_frame_bytes_strict_with_limits(&bytes, &limits)
        .unwrap_or_else(|error| panic!("strict decoder failed for {name}: {error:?}"))
        .into_frame();
    let output = avif::decode_native(&bytes, &limits, &resource_limits(bytes.len()))
        .unwrap_or_else(|error| panic!("highres bridge failed for {name}: {error}"));
    (reference, output)
}

fn assert_native_plane_match(
    reference: &avif_codec::DecodedFrame,
    output: &wml2::highres::ImageFrame,
) {
    assert_eq!(output.descriptor().width() as usize, reference.width);
    assert_eq!(output.descriptor().height() as usize, reference.height);
    assert_eq!(
        output.descriptor().planes().len(),
        reference.buffers.planes.len()
    );
    let planes = output
        .pixels()
        .u16_planes()
        .expect("native AVIF bridge must retain U16 samples");
    assert_eq!(planes.len(), reference.buffers.planes.len());
    for (index, (source, actual)) in reference.buffers.planes.iter().zip(planes).enumerate() {
        assert_eq!(actual.samples(), source.samples, "plane {index} samples");
        assert_eq!(actual.layout().width(), source.layout.width as u32);
        assert_eq!(actual.layout().height(), source.layout.height as u32);
        assert_eq!(
            actual.layout().subsampling().x(),
            1 << source.layout.subsampling_x
        );
        assert_eq!(
            actual.layout().subsampling().y(),
            1 << source.layout.subsampling_y
        );
        assert_eq!(actual.layout().row_stride(), source.layout.width);
        assert_eq!(actual.layout().pixel_stride(), 1);
        assert_eq!(actual.layout().channel_offsets(), &[0]);
        assert_eq!(
            output.descriptor().planes()[index].layout(),
            actual.layout()
        );
        assert_eq!(
            output.descriptor().planes()[index].meaningful_bits(),
            reference.bit_depth
        );
    }
}

fn assert_native_metadata(
    reference: &avif_codec::DecodedFrame,
    output: &wml2::highres::ImageFrame,
) {
    let descriptor = output.descriptor();
    let expected_model = if reference.color_config.monochrome {
        ChannelModel::Gray
    } else if reference
        .color_config
        .color_description
        .is_some_and(|description| description.matrix_coefficients == 0)
    {
        ChannelModel::RGB
    } else {
        ChannelModel::YCbCr
    };
    assert_eq!(descriptor.model(), expected_model);
    assert_eq!(
        descriptor.alpha() != wml2::highres::AlphaAssociation::None,
        reference
            .buffers
            .planes
            .iter()
            .any(|plane| plane.layout.plane == 3)
    );
    let expected_roles: &[ChannelRole] = if reference.color_config.monochrome {
        &[ChannelRole::Gray]
    } else if expected_model == ChannelModel::RGB {
        &[ChannelRole::Green, ChannelRole::Blue, ChannelRole::Red]
    } else {
        &[ChannelRole::Y, ChannelRole::Cb, ChannelRole::Cr]
    };
    for (index, plane) in descriptor.planes().iter().enumerate() {
        if plane.roles()[0] == ChannelRole::Alpha {
            assert_eq!(
                index,
                expected_roles.len(),
                "alpha must follow colour planes"
            );
        } else {
            assert_eq!(
                plane.roles(),
                &expected_roles[index..index + 1],
                "role {index}"
            );
        }
    }
    let av1 = output
        .metadata()
        .source_color()
        .av1()
        .expect("AV1 colour signalling must be retained");
    let description = reference
        .color_config
        .color_description
        .expect("fixture has AV1 colour description");
    assert_eq!(av1.primaries(), u16::from(description.color_primaries));
    assert_eq!(
        av1.transfer(),
        u16::from(description.transfer_characteristics)
    );
    assert_eq!(av1.matrix(), u16::from(description.matrix_coefficients));
    assert_eq!(
        av1.full_range(),
        matches!(
            reference.color_config.color_range,
            avif_codec::av1::ColorRange::Full
        )
    );
}

#[test]
fn native_bridge_matches_strict_reference_for_gray_and_all_subsampling() {
    for name in [
        "fox.profile0.8bpc.yuv420.monochrome.avif",
        "fox.profile0.8bpc.yuv420.avif",
        "fox.profile2.8bpc.yuv422.avif",
        "fox.profile1.8bpc.yuv444.avif",
    ] {
        let (reference, output) = decode_fixture(name);
        assert_native_plane_match(&reference, &output);
        assert_native_metadata(&reference, &output);
    }
}

#[test]
fn native_bridge_matches_strict_reference_for_ten_and_twelve_bit() {
    for name in [
        "fox.profile1.10bpc.yuv444.avif",
        "fox.profile2.12bpc.yuv444.avif",
    ] {
        let (reference, output) = decode_fixture(name);
        assert_native_plane_match(&reference, &output);
        assert_native_metadata(&reference, &output);
    }
}

#[test]
fn native_bridge_retains_alpha_geometry_and_color_information() {
    let (reference, output) =
        decode_fixture("plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif");
    assert_native_plane_match(&reference, &output);
    assert_native_metadata(&reference, &output);
    assert!(
        output
            .descriptor()
            .planes()
            .iter()
            .any(|plane| plane.roles().contains(&ChannelRole::Alpha))
    );
    assert!(output.metadata().source_color().av1().is_some());

    let (rotated, rotated_output) = decode_fixture("kimono.rotate90.avif");
    assert_native_plane_match(&rotated, &rotated_output);
    assert_ne!(
        rotated_output.metadata().rotation(),
        wml2::highres::Rotation::None
    );
    assert_eq!(rotated_output.metadata().render_geometry().len(), 1);
}

#[test]
fn native_bridge_retains_nclx_without_implicit_color_conversion() {
    let (reference, output) = decode_fixture("colors_sdr_srgb.avif");
    assert_native_plane_match(&reference, &output);
    assert!(output.metadata().source_color().av1().is_some());
    let parsed = parse_native_info(
        &std::fs::read(sample_path("colors_sdr_srgb.avif")).unwrap(),
        &native_limits(1 << 30),
    )
    .expect("rich parser should retain ICC fixture metadata");
    assert_eq!(
        parsed.color_information().nclx,
        output
            .metadata()
            .source_color()
            .nclx()
            .map(|value| avif_codec::NclxColorInformation {
                color_primaries: value.primaries(),
                transfer_characteristics: value.transfer(),
                matrix_coefficients: value.matrix(),
                full_range_flag: value.full_range(),
            })
    );
}

#[test]
fn native_bridge_keeps_selected_color_ticket_in_ordered_prof_and_ricc_cases() {
    let source_path = sample_path("fox.profile1.8bpc.yuv444.avif");
    let source = std::fs::read(source_path).unwrap();
    for (icc_kind, order) in [
        (b"prof", [b"nclx", b"prof", b"zzzz"]),
        (b"rICC", [b"rICC", b"zzzz", b"nclx"]),
    ] {
        let bytes = synthetic_colored_avif(&source, &order);
        let native = parse_native_info(&bytes, &native_limits(bytes.len()))
            .expect("synthetic ordinary av01 metadata must parse");
        assert_eq!(native.color_information().icc_color_type, Some(*icc_kind));
        assert!(native.color_information().icc_profile.is_some());
        assert!(native.color_information().nclx.is_some());
        assert_eq!(native.color_information().unknown_colr.len(), 1);
        let strict = decode_frame_bytes_strict_with_limits(&bytes, &native_limits(bytes.len()))
            .expect("synthetic ordinary av01 must strict-decode")
            .into_frame();
        let bridged = avif::decode_native(
            &bytes,
            &native_limits(bytes.len()),
            &resource_limits(bytes.len()),
        )
        .expect("WML2 bridge must not retain a dead ICC ticket");
        assert_native_plane_match(&strict, &bridged);
        assert_eq!(
            bridged.metadata().source_color().icc_profile(),
            native.color_information().icc_profile.as_deref()
        );
        let expected_color = native.color_information();
        let expected_icc_kind = expected_color.icc_color_type.map(|kind| match &kind {
            [b'p', b'r', b'o', b'f'] => wml2::highres::IccColorType::Prof,
            [b'r', b'I', b'C', b'C'] => wml2::highres::IccColorType::Ricc,
            _ => panic!("synthetic ICC kind should be supported"),
        });
        let expected_nclx = expected_color.nclx.map(|value| {
            wml2::highres::NclxColorInformation::new(
                value.color_primaries,
                value.transfer_characteristics,
                value.matrix_coefficients,
                value.full_range_flag,
            )
        });
        let active_color = bridged.descriptor().color_information();
        assert_eq!(
            active_color.icc_profile(),
            expected_color.icc_profile.as_deref()
        );
        assert_eq!(active_color.icc_color_type(), expected_icc_kind);
        assert_eq!(active_color.nclx(), expected_nclx);
        assert_eq!(
            active_color.unknown_colr().len(),
            expected_color.unknown_colr.len()
        );
        for (actual, expected) in active_color
            .unknown_colr()
            .iter()
            .zip(&expected_color.unknown_colr)
        {
            assert_eq!(actual.color_type, expected.color_type);
            assert_eq!(actual.payload, expected.payload);
        }
        let source_color = bridged.metadata().source_color();
        assert_eq!(source_color.icc_color_type(), expected_icc_kind);
        assert_eq!(source_color.nclx(), expected_nclx);
        assert_eq!(
            source_color.unknown_colr().len(),
            expected_color.unknown_colr.len()
        );
        for (actual, expected) in source_color
            .unknown_colr()
            .iter()
            .zip(&expected_color.unknown_colr)
        {
            assert_eq!(actual.color_type, expected.color_type);
            assert_eq!(actual.payload, expected.payload);
        }
    }
}

#[test]
fn native_bridge_rejects_derived_images_and_sequences_at_typed_boundary() {
    for name in [
        "color_grid_alpha_grid_tile_shared_in_dimg.avif",
        "colors-animated-8bpc.avif",
    ] {
        let path = sample_path(name);
        assert!(
            path.is_file(),
            "required staging fixture is missing: {}",
            path.display()
        );
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!("required staging fixture cannot be read: {path:?}: {error}")
        });
        let limits = native_limits(bytes.len());
        let result = avif::decode_native(&bytes, &limits, &resource_limits(bytes.len()));
        match result {
            Err(wml2::highres::avif::DecodeError::Codec(
                avif_codec::DecoderError::Unsupported(message),
            )) => assert!(
                (name.starts_with("color_grid") && message.contains("primary av01"))
                    || (name.starts_with("colors-animated") && message.contains("AVIS")),
                "unexpected unsupported reason for {name}: {message}"
            ),
            other => panic!("derived/sequence fixture had wrong result for {name}: {other:?}"),
        }
    }
}
