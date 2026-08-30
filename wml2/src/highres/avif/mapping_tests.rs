use super::mapping::consume_native_frame_with_metadata;
use crate::highres::{
    AlphaAssociation, Av1ColorInformation, ChannelRole, ColorInformationSet, ColorProvenance,
    FrameMetadata, NclxColorInformation, ProcessingError, ResourceLimits, UnknownColorInformation,
};

fn limits() -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1024 * 1024)
        .max_width(4096)
        .max_height(4096)
        .max_pixels(4096 * 4096)
        .max_channels(4)
        .max_planes(4)
        .max_plane_bytes(1024 * 1024)
        .max_frame_bytes(4 * 1024 * 1024)
        .max_total_live_decoded_bytes(8 * 1024 * 1024)
        .max_references(32)
        .max_frame_count(1024)
        .max_metadata_bytes(1024 * 1024)
        .max_icc_bytes(1024 * 1024)
        .max_clut_bytes(1024 * 1024)
        .max_parser_entries(1024)
        .max_parser_depth(16)
        .max_grid_cells(64)
        .max_derived_work(1024 * 1024)
        .max_derived_depth(16)
        .build()
        .unwrap()
}

fn config(identity: bool, _alpha: bool) -> avif_codec::av1::ColorConfig {
    avif_codec::av1::ColorConfig {
        high_bitdepth: true,
        twelve_bit: false,
        bit_depth: 10,
        monochrome: false,
        color_description: Some(avif_codec::av1::ColorDescription {
            color_primaries: 1,
            transfer_characteristics: 13,
            matrix_coefficients: if identity { 0 } else { 1 },
        }),
        color_range: avif_codec::av1::ColorRange::Full,
        subsampling_x: false,
        subsampling_y: false,
        chroma_sample_position: None,
        separate_uv_delta_q: false,
    }
}

fn plane(id: u8, samples: Vec<u16>) -> avif_codec::av1::PlaneBuffer {
    let width = samples.len();
    avif_codec::av1::PlaneBuffer {
        layout: avif_codec::av1::PlaneLayout {
            plane: id,
            width,
            height: 1,
            subsampling_x: 0,
            subsampling_y: 0,
            sample_count: width,
        },
        samples,
    }
}

fn plane_2d(
    id: u8,
    width: usize,
    height: usize,
    subsampling_x: u8,
    subsampling_y: u8,
) -> avif_codec::av1::PlaneBuffer {
    avif_codec::av1::PlaneBuffer {
        layout: avif_codec::av1::PlaneLayout {
            plane: id,
            width,
            height,
            subsampling_x,
            subsampling_y,
            sample_count: width * height,
        },
        samples: vec![0; width * height],
    }
}

#[test]
fn maps_identity_planes_as_gbr_and_keeps_alpha_by_native_id() {
    let frame = avif_codec::DecodedFrame {
        width: 2,
        height: 1,
        render_width: 2,
        render_height: 1,
        bit_depth: 10,
        color_config: config(true, true),
        color_information: None,
        alpha_premultiplied: true,
        buffers: avif_codec::av1::FrameBuffers {
            width: 2,
            height: 1,
            planes: vec![
                plane(0, vec![1, 2]),
                plane(1, vec![3, 4]),
                plane(2, vec![5, 6]),
                plane(3, vec![0, 1023]),
            ],
        },
    };
    let colors = ColorInformationSet::new()
        .with_icc_profile(vec![1, 2, 3])
        .unwrap()
        .with_nclx(NclxColorInformation::new(9, 16, 9, true))
        .with_av1(Av1ColorInformation::new(1, 13, 0, true));
    let output = consume_native_frame_with_metadata(
        frame,
        FrameMetadata::new(colors.clone()),
        &limits(),
        None,
    )
    .unwrap();
    assert_eq!(
        output.descriptor().model(),
        crate::highres::ChannelModel::RGB
    );
    assert_eq!(output.descriptor().alpha(), AlphaAssociation::Premultiplied);
    assert_eq!(
        output.descriptor().planes()[0].roles(),
        &[ChannelRole::Green]
    );
    assert_eq!(output.descriptor().planes()[2].roles(), &[ChannelRole::Red]);
    assert_eq!(
        output.descriptor().planes()[3].roles(),
        &[ChannelRole::Alpha]
    );
    assert_eq!(
        output.pixels().u16_planes().unwrap()[3].samples(),
        &[0, 1023]
    );
    assert_eq!(output.metadata().coded_dimensions(), Some((2, 1)));
    assert_eq!(output.metadata().render_dimensions(), Some((2, 1)));
    assert_eq!(output.descriptor().color_information(), &colors);
    assert_eq!(output.metadata().source_color(), &colors);
}

#[test]
fn maps_odd_420_planes_from_native_log2_subsampling() {
    let mut color_config = config(false, false);
    color_config.subsampling_x = true;
    color_config.subsampling_y = true;
    let frame = avif_codec::DecodedFrame {
        width: 3,
        height: 3,
        render_width: 3,
        render_height: 3,
        bit_depth: 12,
        color_config: avif_codec::av1::ColorConfig {
            bit_depth: 12,
            twelve_bit: true,
            ..color_config
        },
        color_information: None,
        alpha_premultiplied: false,
        buffers: avif_codec::av1::FrameBuffers {
            width: 3,
            height: 3,
            planes: vec![
                plane_2d(0, 3, 3, 0, 0),
                plane_2d(1, 2, 2, 1, 1),
                plane_2d(2, 2, 2, 1, 1),
            ],
        },
    };
    let output =
        consume_native_frame_with_metadata(frame, FrameMetadata::default(), &limits(), None)
            .unwrap();
    assert_eq!(
        output.descriptor().planes()[0].layout().subsampling().x(),
        1
    );
    assert_eq!(
        output.descriptor().planes()[1].layout().subsampling().x(),
        2
    );
    assert_eq!(
        output.descriptor().planes()[1].layout().subsampling().y(),
        2
    );
    assert_eq!(output.descriptor().planes()[1].layout().width(), 2);
    assert_eq!(output.descriptor().planes()[1].layout().height(), 2);
    assert_eq!(output.descriptor().planes()[0].meaningful_bits(), 12);
}

#[test]
fn preserves_supported_native_precision_without_requantizing() {
    for bit_depth in [8u8, 10, 12] {
        let mut color_config = config(true, false);
        color_config.bit_depth = bit_depth;
        color_config.high_bitdepth = bit_depth >= 10;
        color_config.twelve_bit = bit_depth == 12;
        let max = if bit_depth == 16 {
            u16::MAX
        } else {
            (1u16 << bit_depth) - 1
        };
        let frame = avif_codec::DecodedFrame {
            width: 1,
            height: 1,
            render_width: 1,
            render_height: 1,
            bit_depth,
            color_config,
            color_information: None,
            alpha_premultiplied: false,
            buffers: avif_codec::av1::FrameBuffers {
                width: 1,
                height: 1,
                planes: vec![
                    plane(0, vec![max]),
                    plane(1, vec![max]),
                    plane(2, vec![max]),
                ],
            },
        };
        let output =
            consume_native_frame_with_metadata(frame, FrameMetadata::default(), &limits(), None)
                .unwrap();
        assert!(
            output
                .descriptor()
                .planes()
                .iter()
                .all(|plane| plane.meaningful_bits() == bit_depth)
        );
        assert_eq!(output.pixels().u16_planes().unwrap()[0].samples(), &[max]);
    }
}

#[test]
fn rejects_unproven_16bit_native_frame() {
    let mut color_config = config(true, false);
    color_config.bit_depth = 16;
    color_config.high_bitdepth = true;
    let frame = avif_codec::DecodedFrame {
        width: 1,
        height: 1,
        render_width: 1,
        render_height: 1,
        bit_depth: 16,
        color_config,
        color_information: None,
        alpha_premultiplied: false,
        buffers: avif_codec::av1::FrameBuffers {
            width: 1,
            height: 1,
            planes: vec![
                plane(0, vec![u16::MAX]),
                plane(1, vec![u16::MAX]),
                plane(2, vec![u16::MAX]),
            ],
        },
    };
    assert!(matches!(
        consume_native_frame_with_metadata(frame, FrameMetadata::default(), &limits(), None),
        Err(ProcessingError::Unsupported(_))
    ));
}

#[test]
fn maps_monochrome_decoder_subsampling_flags_without_rejecting_luma() {
    let mut color_config = config(false, true);
    color_config.monochrome = true;
    color_config.subsampling_x = true;
    color_config.subsampling_y = true;
    let frame = avif_codec::DecodedFrame {
        width: 3,
        height: 1,
        render_width: 3,
        render_height: 1,
        bit_depth: 10,
        color_config,
        color_information: None,
        alpha_premultiplied: false,
        buffers: avif_codec::av1::FrameBuffers {
            width: 3,
            height: 1,
            planes: vec![plane(0, vec![1, 2, 3]), plane(3, vec![0, 1023, 1023])],
        },
    };
    let output =
        consume_native_frame_with_metadata(frame, FrameMetadata::default(), &limits(), None)
            .unwrap();
    assert_eq!(
        output.descriptor().model(),
        crate::highres::ChannelModel::Gray
    );
    assert_eq!(
        output.descriptor().planes()[0].roles(),
        &[ChannelRole::Gray]
    );
    assert_eq!(
        output.descriptor().planes()[1].roles(),
        &[ChannelRole::Alpha]
    );
}

#[test]
fn rejects_duplicate_or_unsupported_native_plane_ids_without_partial_success() {
    let base = avif_codec::DecodedFrame {
        width: 1,
        height: 1,
        render_width: 1,
        render_height: 1,
        bit_depth: 8,
        color_config: config(false, false),
        color_information: None,
        alpha_premultiplied: false,
        buffers: avif_codec::av1::FrameBuffers {
            width: 1,
            height: 1,
            planes: vec![
                plane(0, vec![1]),
                plane(0, vec![2]),
                plane(1, vec![3]),
                plane(2, vec![4]),
            ],
        },
    };
    assert!(matches!(
        consume_native_frame_with_metadata(base, FrameMetadata::default(), &limits(), None),
        Err(ProcessingError::Invalid(_))
    ));
}

#[test]
fn rejects_native_frame_configuration_precision_mismatch() {
    let frame = avif_codec::DecodedFrame {
        width: 1,
        height: 1,
        render_width: 1,
        render_height: 1,
        bit_depth: 8,
        color_config: config(false, false),
        color_information: None,
        alpha_premultiplied: false,
        buffers: avif_codec::av1::FrameBuffers {
            width: 1,
            height: 1,
            planes: vec![plane(0, vec![0]), plane(1, vec![0]), plane(2, vec![0])],
        },
    };
    assert!(matches!(
        consume_native_frame_with_metadata(frame, FrameMetadata::default(), &limits(), None),
        Err(ProcessingError::Invalid(_))
    ));
}

#[test]
fn rejects_native_subsampling_that_disagrees_with_av1_flags() {
    let mut color_config = config(false, false);
    color_config.subsampling_x = true;
    color_config.subsampling_y = true;
    let frame = avif_codec::DecodedFrame {
        width: 2,
        height: 2,
        render_width: 2,
        render_height: 2,
        bit_depth: 10,
        color_config,
        color_information: None,
        alpha_premultiplied: false,
        buffers: avif_codec::av1::FrameBuffers {
            width: 2,
            height: 2,
            planes: vec![
                plane_2d(0, 2, 2, 0, 0),
                plane_2d(1, 2, 2, 0, 0),
                plane_2d(2, 2, 2, 0, 0),
            ],
        },
    };
    assert!(matches!(
        consume_native_frame_with_metadata(frame, FrameMetadata::default(), &limits(), None),
        Err(ProcessingError::Invalid(_))
    ));
}

fn rich_info() -> avif_codec::RichAvifInfo {
    avif_codec::RichAvifInfo {
        info: avif_codec::AvifInfo {
            major_brand: *b"avif",
            compatible_brands: Vec::new(),
            primary_item_id: Some(1),
            width: Some(1),
            height: Some(1),
            pixel_information: None,
            color_information: None,
            alpha_premultiplied: false,
            alpha_auxiliary_items: Vec::new(),
            alpha_grid: None,
            primary_grid: None,
            clean_aperture: None,
            rotation: None,
            mirror: None,
            av1_config: None,
            primary_item_payload: Vec::new(),
            sequence_sample_payloads: Vec::new(),
        },
        color_information: avif_codec::ColorInformationSet::default(),
    }
}

fn native_rgb_8() -> avif_codec::DecodedFrame {
    avif_codec::DecodedFrame {
        width: 1,
        height: 1,
        render_width: 1,
        render_height: 1,
        bit_depth: 8,
        color_config: avif_codec::av1::ColorConfig {
            high_bitdepth: false,
            twelve_bit: false,
            bit_depth: 8,
            monochrome: false,
            color_description: Some(avif_codec::av1::ColorDescription {
                color_primaries: 1,
                transfer_characteristics: 13,
                matrix_coefficients: 1,
            }),
            color_range: avif_codec::av1::ColorRange::Full,
            subsampling_x: false,
            subsampling_y: false,
            chroma_sample_position: None,
            separate_uv_delta_q: false,
        },
        color_information: None,
        alpha_premultiplied: false,
        buffers: avif_codec::av1::FrameBuffers {
            width: 1,
            height: 1,
            planes: vec![plane(0, vec![1]), plane(1, vec![2]), plane(2, vec![3])],
        },
    }
}

#[test]
fn rich_pixi_precision_is_checked_before_mapping() {
    let mut info = rich_info();
    info.info.pixel_information = Some(avif_codec::PixelInformation {
        bits_per_channel: vec![10, 10, 10],
        extended_channels: None,
    });
    assert!(matches!(
        super::mapping::consume_native_frame(native_rgb_8(), &info, &limits(), None),
        Err(ProcessingError::Invalid(_))
    ));
}

#[test]
fn ordered_geometry_is_retained_by_native_bridge() {
    let mut info = rich_info();
    info.info.rotation = Some(avif_codec::ImageRotation { angle: 1 });
    info.info.mirror = Some(avif_codec::ImageMirror { axis: 0 });
    let frame = super::mapping::consume_native_frame(native_rgb_8(), &info, &limits(), None)
        .expect("ordered geometry is metadata, not a pixel transform");
    assert_eq!(
        frame.metadata().rotation(),
        crate::highres::Rotation::Degrees90
    );
    assert!(frame.metadata().mirror_horizontal());
    assert_eq!(frame.metadata().coded_geometry().len(), 2);
}

#[test]
fn mapper_charges_retained_icc_capacity() {
    let mut info = rich_info();
    let mut profile = Vec::with_capacity(4096);
    profile.push(1);
    info.color_information.icc_profile = Some(profile);
    info.color_information.icc_color_type = Some(*b"prof");
    let mut constrained = limits();
    constrained.max_icc_bytes = 64;
    assert!(matches!(
        super::mapping::consume_native_frame(native_rgb_8(), &info, &constrained, None),
        Err(ProcessingError::ResourceLimit(_))
    ));
}

#[test]
fn mapper_rejects_frame_budget_before_owned_descriptor_build() {
    let mut constrained = limits();
    constrained.max_frame_bytes = 128;
    constrained.max_total_live_decoded_bytes = 128;
    assert!(matches!(
        super::mapping::consume_native_frame(native_rgb_8(), &rich_info(), &constrained, None),
        Err(ProcessingError::ResourceLimit(_))
    ));
}

#[test]
fn mapper_rejects_live_budget_before_retained_icc_copy() {
    let mut info = rich_info();
    info.color_information.icc_profile = Some(vec![1; 4096]);
    info.color_information.icc_color_type = Some(*b"prof");
    let mut constrained = limits();
    constrained.max_frame_bytes = 1;
    constrained.max_total_live_decoded_bytes = 1;
    assert!(matches!(
        super::mapping::consume_native_frame(native_rgb_8(), &info, &constrained, None),
        Err(ProcessingError::ResourceLimit(_))
    ));
}

#[test]
fn construction_failpoint_is_reported_as_allocation() {
    let _failpoint = super::mapping::construction_failpoint(0);
    let result = consume_native_frame_with_metadata(
        native_rgb_8(),
        FrameMetadata::default(),
        &limits(),
        None,
    );
    assert!(matches!(result, Err(ProcessingError::Allocation(_))));
}

#[test]
fn native_map_plan_accepts_exact_peak_and_rejects_one_byte_less() {
    let frame = native_rgb_8();
    let metadata = FrameMetadata::default();
    let metadata_bytes = metadata.metadata_bytes().unwrap();
    let baseline = limits();
    let (frame_bytes, live_bytes) =
        super::mapping::inspect_native_frame_for_test(&frame, metadata_bytes, &baseline).unwrap();

    let mut below = baseline;
    below.max_frame_bytes = frame_bytes.saturating_sub(1);
    assert!(matches!(
        super::mapping::inspect_native_frame_for_test(&frame, metadata_bytes, &below),
        Err(ProcessingError::ResourceLimit(_))
    ));

    let mut exact = baseline;
    exact.max_frame_bytes = frame_bytes;
    exact.max_total_live_decoded_bytes = live_bytes;
    assert!(super::mapping::inspect_native_frame_for_test(&frame, metadata_bytes, &exact).is_ok());
}

#[test]
fn native_mapping_peak_includes_external_and_native_outer_ownership() {
    let frame = native_rgb_8();
    let info = rich_info();
    let external_live_bytes = 17;
    let baseline = limits();
    let peak = super::mapping::inspect_native_mapping_peak_for_test(
        &frame,
        &info,
        &baseline,
        external_live_bytes,
    )
    .expect("native ownership peak should be finite");
    assert!(peak.pre_release_live_bytes >= peak.post_release_live_bytes);
    assert!(peak.peak_live_bytes >= peak.pre_release_live_bytes);
    assert!(peak.peak_live_bytes >= peak.post_release_live_bytes);

    let mut exact = baseline;
    exact.max_frame_bytes = peak.final_frame_bytes;
    exact.max_total_live_decoded_bytes = peak.peak_live_bytes;
    super::mapping::consume_native_frame_with_external_live(
        frame,
        &info,
        &exact,
        None,
        external_live_bytes,
    )
    .expect("exact ownership peak should map");

    let mut below = exact;
    below.max_total_live_decoded_bytes = peak.peak_live_bytes.saturating_sub(1);
    let error = super::mapping::consume_native_frame_with_external_live(
        native_rgb_8(),
        &info,
        &below,
        None,
        external_live_bytes,
    )
    .expect_err("one byte below the ownership peak must fail in preflight");
    assert!(matches!(error, ProcessingError::ResourceLimit(_)));
}

#[test]
fn rich_native_mapping_peak_accounts_all_owned_metadata_and_retries() {
    let mut info = rich_info();
    let mut profile = Vec::with_capacity(4096);
    profile.push(1);
    info.color_information.icc_profile = Some(profile);
    info.color_information.icc_color_type = Some(*b"prof");
    info.color_information.nclx = Some(avif_codec::NclxColorInformation {
        color_primaries: 1,
        transfer_characteristics: 13,
        matrix_coefficients: 1,
        full_range_flag: true,
    });
    let mut unknown_payload = Vec::with_capacity(32);
    unknown_payload.extend_from_slice(&[0xa5, 0x5a]);
    info.color_information
        .unknown_colr
        .push(avif_codec::ColorInformation {
            color_type: *b"zzzz",
            payload: unknown_payload,
        });
    info.info.pixel_information = Some(avif_codec::PixelInformation {
        bits_per_channel: vec![8, 8, 8],
        extended_channels: None,
    });
    info.info.rotation = Some(avif_codec::ImageRotation { angle: 1 });
    info.info.mirror = Some(avif_codec::ImageMirror { axis: 0 });

    let external_live_bytes = 17;
    let baseline = limits();
    let peak = super::mapping::inspect_native_mapping_peak_for_test(
        &native_rgb_8(),
        &info,
        &baseline,
        external_live_bytes,
    )
    .expect("rich native ownership peak should be checked");
    assert!(peak.pre_release_live_bytes > 0);
    assert!(peak.post_release_live_bytes > 0);
    assert!(peak.peak_live_bytes >= peak.pre_release_live_bytes);
    assert!(peak.peak_live_bytes >= peak.post_release_live_bytes);

    let mut exact = baseline;
    exact.max_frame_bytes = peak.final_frame_bytes;
    exact.max_total_live_decoded_bytes = peak.peak_live_bytes;
    super::mapping::consume_native_frame_with_external_live(
        native_rgb_8(),
        &info,
        &exact,
        None,
        external_live_bytes,
    )
    .expect("the measured rich ownership peak must be admitted exactly");

    // The preflight rejection proves that the mapping candidate is not entered
    // one byte below the combined peak.  An exact-budget failpoint is the
    // positive control that the same path does request a fresh candidate.
    let mut below = exact;
    below.max_total_live_decoded_bytes = peak.peak_live_bytes.saturating_sub(1);
    let failpoint = super::mapping::construction_failpoint(0);
    let error = super::mapping::consume_native_frame_with_external_live(
        native_rgb_8(),
        &info,
        &below,
        None,
        external_live_bytes,
    )
    .expect_err("one byte below rich ownership peak must reject before mapping");
    assert!(
        matches!(error, ProcessingError::ResourceLimit(_)),
        "unexpected M-1 error: {error:?}; peak={:?}",
        peak
    );
    let error = super::mapping::consume_native_frame_with_external_live(
        native_rgb_8(),
        &info,
        &exact,
        None,
        external_live_bytes,
    )
    .expect_err("exact rich mapping should reach the allocation failpoint");
    assert!(matches!(error, ProcessingError::Allocation(_)));
    drop(failpoint);

    super::mapping::consume_native_frame_with_external_live(
        native_rgb_8(),
        &info,
        &exact,
        None,
        external_live_bytes,
    )
    .expect("the exact rich budget must remain retryable after candidate failure");
}

#[test]
fn native_map_plan_checks_each_preconstruction_limit() {
    enum LimitCase {
        Width,
        Height,
        Pixels,
        Channels,
        Planes,
        PlaneBytes,
        FrameBytes,
        LiveBytes,
        MetadataBytes,
        IccBytes,
    }

    let frame = native_rgb_8();
    let cases = [
        LimitCase::Width,
        LimitCase::Height,
        LimitCase::Pixels,
        LimitCase::Channels,
        LimitCase::Planes,
        LimitCase::PlaneBytes,
        LimitCase::FrameBytes,
        LimitCase::LiveBytes,
        LimitCase::MetadataBytes,
        LimitCase::IccBytes,
    ];
    for case in cases {
        let mut constrained = limits();
        let mut info = rich_info();
        match case {
            LimitCase::Width => constrained.max_width = 0,
            LimitCase::Height => constrained.max_height = 0,
            LimitCase::Pixels => constrained.max_pixels = 0,
            LimitCase::Channels => constrained.max_channels = 2,
            LimitCase::Planes => constrained.max_planes = 2,
            LimitCase::PlaneBytes => constrained.max_plane_bytes = 1,
            LimitCase::FrameBytes => constrained.max_frame_bytes = 1,
            LimitCase::LiveBytes => constrained.max_total_live_decoded_bytes = 1,
            LimitCase::MetadataBytes => constrained.max_metadata_bytes = 0,
            LimitCase::IccBytes => {
                let mut profile = Vec::with_capacity(4096);
                profile.push(1);
                info.color_information.icc_profile = Some(profile);
                info.color_information.icc_color_type = Some(*b"prof");
                constrained.max_icc_bytes = 64;
            }
        }
        let result = super::mapping::inspect_native_for_test(&frame, &info, &constrained);
        assert!(
            matches!(result, Err(ProcessingError::ResourceLimit(_))),
            "limit case rejected after construction: {:?}",
            std::mem::discriminant(&case)
        );
    }
}

#[test]
fn construction_ledger_rejects_reserve_before_capacity_growth() {
    let mut constrained = limits();
    constrained.max_frame_bytes = 8;
    constrained.max_total_live_decoded_bytes = 8;
    let (result, capacity) = super::mapping::reserve_construction_for_test(&constrained, 16);
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert_eq!(capacity, 0, "budget rejection must precede Vec growth");
}

#[test]
fn construction_ledger_exact_empty_and_one_byte_under_boundaries() {
    let mut exact = limits();
    exact.max_frame_bytes = 8;
    exact.max_total_live_decoded_bytes = 8;
    let (result, capacity) = super::mapping::reserve_construction_for_test(&exact, 8);
    assert!(result.is_ok());
    assert_eq!(capacity, 8);

    let mut below = limits();
    below.max_frame_bytes = 7;
    below.max_total_live_decoded_bytes = 7;
    let (result, capacity) = super::mapping::reserve_construction_for_test(&below, 8);
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert_eq!(capacity, 0);

    let mut empty = limits();
    empty.max_frame_bytes = 0;
    empty.max_total_live_decoded_bytes = 0;
    let (result, capacity) = super::mapping::reserve_construction_for_test(&empty, 0);
    assert!(result.is_ok());
    assert_eq!(capacity, 0);
}

#[test]
fn construction_ledger_rejects_size_overflow_before_reserve() {
    let (result, capacity) =
        super::mapping::reserve_u16_construction_for_test(&limits(), usize::MAX);
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert_eq!(capacity, 0);
}

#[test]
fn construction_failpoint_leaves_candidate_empty() {
    let _failpoint = super::mapping::construction_failpoint(0);
    let (result, capacity) = super::mapping::reserve_construction_for_test(&limits(), 8);
    assert!(matches!(result, Err(ProcessingError::Allocation(_))));
    assert_eq!(
        capacity, 0,
        "failed reserve must not retain a candidate Vec"
    );
}

#[test]
fn mapper_reports_descriptor_budget_failure_after_stack_preflight() {
    let frame = native_rgb_8();
    let info = rich_info();
    let metadata = super::metadata::frame_metadata(&info).unwrap();
    let retained = metadata.metadata_bytes().unwrap();
    let samples = frame
        .buffers
        .planes
        .iter()
        .map(|plane| plane.samples.capacity() * std::mem::size_of::<u16>())
        .sum::<usize>();
    let headers = std::mem::size_of::<FrameMetadata>()
        + std::mem::size_of::<crate::highres::ImageDescriptor>()
        + std::mem::size_of::<crate::highres::PixelBuffer>();
    let mut after = metadata;
    after
        .source_color_mut()
        .set_av1(crate::highres::Av1ColorInformation::new(1, 13, 1, true));
    let provenance_delta = after.metadata_bytes().unwrap() - retained;
    let descriptor_bytes =
        frame.buffers.planes.len() * std::mem::size_of::<crate::highres::PlaneDescriptor>();
    let mut constrained = limits();
    constrained.max_frame_bytes =
        retained + samples + headers + provenance_delta + descriptor_bytes - 1;
    assert!(matches!(
        super::mapping::inspect_native_for_test(&frame, &info, &constrained),
        Err(ProcessingError::ResourceLimit(_))
    ));
    assert!(matches!(
        super::mapping::consume_native_frame(frame, &info, &constrained, None),
        Err(ProcessingError::ResourceLimit(_))
    ));
}

#[test]
fn mapper_reports_active_colour_budget_failure() {
    let colors = ColorInformationSet::new()
        .with_icc_profile(vec![1; 4096])
        .unwrap();
    let metadata = FrameMetadata::new(colors);
    let mut constrained = limits();
    constrained.max_frame_bytes = 6000;
    assert!(matches!(
        consume_native_frame_with_metadata(native_rgb_8(), metadata, &constrained, None),
        Err(ProcessingError::ResourceLimit(_))
    ));
}

#[test]
fn provenance_failpoint_is_reported_as_allocation() {
    let _failpoint = super::mapping::construction_failpoint(0);
    assert!(matches!(
        super::mapping::consume_native_frame(native_rgb_8(), &rich_info(), &limits(), None),
        Err(ProcessingError::Allocation(_))
    ));
}

#[test]
fn metadata_replacement_rejects_actual_overcapacity_without_mutation() {
    let mut colors = ColorInformationSet::from_owned_parts(
        vec![ColorProvenance::ContainerNclx],
        None,
        None,
        Some(NclxColorInformation::new(1, 13, 1, true)),
        None,
        Vec::new(),
    );
    let old_bytes = colors.owned_bytes().unwrap();
    let owner = colors.provenance().as_ptr();
    let mut constrained = limits();
    constrained.max_total_live_decoded_bytes = old_bytes + 2;
    let mut ledger =
        super::allocation::ConstructionLedger::new(old_bytes, 0, &constrained).unwrap();
    let before = (
        ledger.frame_bytes,
        ledger.live_bytes,
        ledger.metadata_bytes_for_test(),
    );
    let result = ledger.try_grow_metadata_vec(colors.provenance_mut_bridge(), 1, 0, |count| {
        let mut candidate = Vec::<ColorProvenance>::new();
        candidate
            .try_reserve_exact(count + 1)
            .map_err(|_| ProcessingError::Allocation("candidate allocation failed".into()))?;
        Ok(candidate)
    });
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert_eq!(colors.provenance().as_ptr(), owner);
    assert_eq!(colors.provenance(), &[ColorProvenance::ContainerNclx]);
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
        ),
        before
    );
}

#[test]
fn fresh_metadata_candidate_rejects_actual_overcapacity_transactionally() {
    let mut constrained = limits();
    constrained.max_frame_bytes = 2;
    constrained.max_total_live_decoded_bytes = 2;
    constrained.max_metadata_bytes = 2;
    let mut ledger = super::allocation::ConstructionLedger::new(0, 0, &constrained).unwrap();
    let before = (
        ledger.frame_bytes,
        ledger.live_bytes,
        ledger.metadata_bytes_for_test(),
    );
    let result = ledger.try_new_metadata_vec_with(2, |count| {
        let mut candidate = Vec::<u8>::new();
        candidate
            .try_reserve_exact(count + 1)
            .map_err(|_| ProcessingError::Allocation("candidate allocation failed".into()))?;
        Ok(candidate)
    });
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
        ),
        before
    );
}

#[test]
fn metadata_replacement_moves_outer_without_moving_nested_payload() {
    let payload = vec![4, 5, 6];
    let payload_pointer = payload.as_ptr();
    let mut colors = ColorInformationSet::from_owned_parts(
        Vec::new(),
        None,
        None,
        None,
        None,
        vec![UnknownColorInformation {
            color_type: *b"test",
            payload,
        }],
    );
    let old_bytes = colors.owned_bytes().unwrap();
    let mut ledger = super::allocation::ConstructionLedger::new(old_bytes, 0, &limits()).unwrap();
    ledger
        .try_grow_metadata_vec(
            colors.unknown_colr_mut_bridge(),
            1,
            0,
            super::allocation::try_new_metadata_candidate::<UnknownColorInformation>,
        )
        .unwrap();
    assert_eq!(colors.unknown_colr_capacity(), 2);
    assert_eq!(colors.unknown_colr()[0].payload.as_ptr(), payload_pointer);
    assert_eq!(colors.unknown_colr()[0].payload, [4, 5, 6]);
}

#[test]
fn av1_update_existing_description_has_no_metadata_delta() {
    let description = native_rgb_8().color_config.color_description.unwrap();
    let mut metadata = FrameMetadata::new(
        ColorInformationSet::new().with_av1(Av1ColorInformation::new(1, 13, 1, true)),
    );
    let before_metadata = metadata.metadata_bytes().unwrap();
    let provenance = metadata.source_color().provenance().as_ptr();
    let mut ledger =
        super::allocation::ConstructionLedger::new(before_metadata, 0, &limits()).unwrap();
    let before = (
        ledger.frame_bytes,
        ledger.live_bytes,
        ledger.metadata_bytes_for_test(),
    );
    super::mapping::update_av1_metadata_with_ledger(&mut metadata, description, false, &mut ledger)
        .unwrap();
    assert_eq!(metadata.metadata_bytes().unwrap(), before_metadata);
    assert_eq!(metadata.source_color().provenance().as_ptr(), provenance);
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
        ),
        before
    );
    assert!(!metadata.source_color().av1().unwrap().full_range());
}

#[test]
fn av1_update_adds_description_transactionally() {
    let description = native_rgb_8().color_config.color_description.unwrap();
    let payload = vec![7, 8, 9];
    let payload_pointer = payload.as_ptr();
    let mut metadata = FrameMetadata::new(ColorInformationSet::from_owned_parts(
        Vec::new(),
        None,
        None,
        None,
        None,
        vec![UnknownColorInformation {
            color_type: *b"test",
            payload,
        }],
    ));
    let before_metadata = metadata.metadata_bytes().unwrap();
    let mut ledger =
        super::allocation::ConstructionLedger::new(before_metadata, 0, &limits()).unwrap();
    super::mapping::update_av1_metadata_with_ledger(&mut metadata, description, true, &mut ledger)
        .unwrap();
    assert_eq!(
        metadata.metadata_bytes().unwrap(),
        before_metadata + std::mem::size_of::<Av1ColorInformation>() + 1
    );
    assert_eq!(
        metadata.source_color().unknown_colr()[0].payload.as_ptr(),
        payload_pointer
    );
    assert!(metadata.source_color().av1().unwrap().full_range());
    assert_eq!(
        ledger.metadata_bytes_for_test(),
        metadata.metadata_bytes().unwrap()
    );
}

#[test]
fn av1_update_failure_preserves_metadata_and_ledger() {
    let description = native_rgb_8().color_config.color_description.unwrap();
    let mut metadata = FrameMetadata::default();
    let mut constrained = limits();
    constrained.max_metadata_bytes = 8;
    let mut ledger = super::allocation::ConstructionLedger::new(0, 0, &constrained).unwrap();
    let before = (
        ledger.frame_bytes,
        ledger.live_bytes,
        ledger.metadata_bytes_for_test(),
        metadata.metadata_bytes().unwrap(),
    );
    let result = super::mapping::update_av1_metadata_with_ledger(
        &mut metadata,
        description,
        true,
        &mut ledger,
    );
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert!(metadata.source_color().av1().is_none());
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
            metadata.metadata_bytes().unwrap(),
        ),
        before
    );

    let mut metadata = FrameMetadata::default();
    let mut ledger = super::allocation::ConstructionLedger::new(0, 0, &limits()).unwrap();
    let before = (
        ledger.frame_bytes,
        ledger.live_bytes,
        ledger.metadata_bytes_for_test(),
    );
    let _failpoint = super::mapping::construction_failpoint(0);
    let result = super::mapping::update_av1_metadata_with_ledger(
        &mut metadata,
        description,
        true,
        &mut ledger,
    );
    assert!(matches!(result, Err(ProcessingError::Allocation(_))));
    assert!(metadata.source_color().av1().is_none());
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
        ),
        before
    );
}

#[test]
fn av1_update_accounts_av1_and_provenance_presence_independently() {
    let description = native_rgb_8().color_config.color_description.unwrap();
    for av1_present in [false, true] {
        for provenance_present in [false, true] {
            let provenance = if provenance_present {
                vec![ColorProvenance::Av1]
            } else {
                Vec::new()
            };
            let av1 = av1_present.then(|| Av1ColorInformation::new(1, 13, 1, true));
            let mut metadata = FrameMetadata::new(ColorInformationSet::from_owned_parts(
                provenance,
                None,
                None,
                None,
                av1,
                Vec::new(),
            ));
            let before = metadata.metadata_bytes().unwrap();
            let expected_delta = usize::from(!av1_present)
                * std::mem::size_of::<Av1ColorInformation>()
                + usize::from(!provenance_present);
            let mut ledger =
                super::allocation::ConstructionLedger::new(before, 0, &limits()).unwrap();
            super::mapping::update_av1_metadata_with_ledger(
                &mut metadata,
                description,
                true,
                &mut ledger,
            )
            .unwrap();
            assert_eq!(
                metadata.metadata_bytes().unwrap(),
                before + expected_delta,
                "av1_present={av1_present}, provenance_present={provenance_present}"
            );
            assert_eq!(
                ledger.metadata_bytes_for_test(),
                before + expected_delta,
                "av1_present={av1_present}, provenance_present={provenance_present}"
            );
            assert!(
                metadata
                    .source_color()
                    .provenance()
                    .contains(&ColorProvenance::Av1)
            );
            assert!(metadata.source_color().av1().is_some());
        }
    }
}

#[test]
fn icc_copy_checks_requested_and_actual_owner_capacity() {
    let mut constrained = limits();
    constrained.max_icc_bytes = 1;
    let mut ledger = super::allocation::ConstructionLedger::new(0, 0, &constrained).unwrap();
    let mut called = false;
    let before = (
        ledger.frame_bytes,
        ledger.live_bytes,
        ledger.metadata_bytes_for_test(),
    );
    let result = ledger.try_copy_icc_with(&[1, 2], |_count| {
        called = true;
        Ok(Vec::new())
    });
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert!(
        !called,
        "requested ICC limit must precede candidate allocation"
    );
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
        ),
        before
    );

    let result = ledger.try_copy_icc_with(&[1], |count| {
        let mut candidate = Vec::new();
        candidate
            .try_reserve_exact(count + 1)
            .map_err(|_| ProcessingError::Allocation("candidate allocation failed".into()))?;
        Ok(candidate)
    });
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
        ),
        before
    );

    let copy = ledger.try_copy_icc(&[1]).unwrap();
    assert_eq!(copy, [1]);
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
        ),
        (1, 1, 1)
    );
}

#[test]
fn icc_copy_allocator_failure_preserves_ledger() {
    let mut ledger = super::allocation::ConstructionLedger::new(0, 0, &limits()).unwrap();
    let before = (
        ledger.frame_bytes,
        ledger.live_bytes,
        ledger.metadata_bytes_for_test(),
    );
    let _failpoint = super::mapping::construction_failpoint(0);
    let result = ledger.try_copy_icc(&[1]);
    assert!(matches!(result, Err(ProcessingError::Allocation(_))));
    assert_eq!(
        (
            ledger.frame_bytes,
            ledger.live_bytes,
            ledger.metadata_bytes_for_test(),
        ),
        before
    );
}
