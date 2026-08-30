use wml2::highres::*;

fn limits() -> ResourceLimits {
    limits_with_total(1 << 27)
}

fn limits_with_total(max_total_live_decoded_bytes: usize) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(4096)
        .max_height(4096)
        .max_pixels(16_777_216)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 24)
        .max_frame_bytes(1 << 26)
        .max_total_live_decoded_bytes(max_total_live_decoded_bytes)
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
        .unwrap()
}

fn rgb_frame(color: ColorInformationSet, domain: SampleDomain) -> ImageFrame {
    let descriptor = ImageDescriptor::rgb(1, 1, 8)
        .unwrap()
        .with_color_information(color)
        .with_domain(domain);
    let planes = (0..3)
        .map(|_| {
            let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
            Plane::new(layout, vec![0u8]).unwrap()
        })
        .collect::<Vec<_>>();
    ImageFrame::new(descriptor, PixelBuffer::u8(planes).unwrap()).unwrap()
}

fn gray_frame(color: ColorInformationSet) -> ImageFrame {
    let descriptor = ImageDescriptor::gray(1, 1, 8)
        .unwrap()
        .with_color_information(color)
        .with_domain(SampleDomain::Encoded);
    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let plane = Plane::new(layout, vec![0u8]).unwrap();
    ImageFrame::new(descriptor, PixelBuffer::u8(vec![plane]).unwrap()).unwrap()
}

fn rgb_native() -> NativeSampleEncoding {
    NativeSampleEncoding::new(SampleRange::Full, MatrixCoefficients::Identity, None)
}

fn linear_rgb_frame(domain: SampleDomain) -> ImageFrame {
    let descriptor = ImageDescriptor::rgb(1, 1, 32)
        .unwrap()
        .with_domain(domain)
        .with_primaries(RgbPrimaries::srgb());
    let planes = (0..3)
        .map(|_| {
            let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
            Plane::new(layout, vec![0.25f32]).unwrap()
        })
        .collect::<Vec<_>>();
    ImageFrame::new(descriptor, PixelBuffer::f32(planes).unwrap()).unwrap()
}

fn valid_rgb_profile() -> Vec<u8> {
    let mut profile = vec![0u8; 204];
    let profile_len = profile.len() as u32;
    profile[..4].copy_from_slice(&profile_len.to_be_bytes());
    profile[8] = 4;
    profile[12..16].copy_from_slice(b"mntr");
    profile[16..20].copy_from_slice(b"RGB ");
    profile[20..24].copy_from_slice(b"XYZ ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[64..68].copy_from_slice(&1u32.to_be_bytes());
    profile[128..132].copy_from_slice(&6u32.to_be_bytes());
    for index in 0..6 {
        let record = 132 + index * 12;
        profile[record + 4..record + 8].copy_from_slice(&204u32.to_be_bytes());
    }
    profile
}

fn minimum_total_budget(frame: &ImageFrame, options: &ColorConvertOptions<'_>) -> usize {
    let mut low = 0usize;
    let mut high = 1 << 20;
    while low < high {
        let middle = low + (high - low) / 2;
        if options
            .validate_for(frame, &limits_with_total(middle))
            .is_ok()
        {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert!(low < 1 << 20, "valid plan did not fit the test budget");
    low
}

#[test]
fn active_metadata_prefers_icc_without_falling_back_to_nclx() {
    let color = ColorInformationSet::new()
        .with_icc_profile(valid_rgb_profile())
        .unwrap()
        .with_icc_color_type(IccColorType::Prof)
        .with_nclx(NclxColorInformation::new(1, 13, 0, true));
    let frame = rgb_frame(color, SampleDomain::Encoded);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_native_sample_encoding(rgb_native());
    options.validate_for(&frame, &limits()).unwrap();
}

#[test]
fn explicit_cicp_override_is_independent_of_active_icc() {
    let color = ColorInformationSet::new()
        .with_icc_profile(valid_rgb_profile())
        .unwrap()
        .with_nclx(NclxColorInformation::new(1, 13, 0, true));
    let frame = rgb_frame(color, SampleDomain::Encoded);
    let options = ColorConvertOptions::new(Destination::encoded_cicp_rgb(
        NclxColorInformation::new(1, 13, 0, true),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        1, 13, 0, true,
    )))
    .with_native_sample_encoding(rgb_native());
    options.validate_for(&frame, &limits()).unwrap();
}

#[test]
fn convert_frame_rejects_active_icc_but_explicit_cicp_succeeds() {
    let color = ColorInformationSet::new()
        .with_icc_profile(valid_rgb_profile())
        .unwrap()
        .with_icc_color_type(IccColorType::Prof)
        .with_nclx(NclxColorInformation::new(1, 13, 0, true));
    let frame = rgb_frame(color, SampleDomain::Encoded);
    let destination = Destination::linear_rgb(SampleDomain::LinearRelative, RgbPrimaries::srgb());
    let active = ColorConvertOptions::new(destination).with_native_sample_encoding(rgb_native());
    assert!(matches!(
        convert_frame(&frame, &active, &limits()),
        Err(ProcessingError::Unsupported(_))
    ));

    let explicit = ColorConvertOptions::new(destination)
        .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
            1, 13, 0, true,
        )))
        .with_native_sample_encoding(rgb_native());
    let converted = convert_frame(&frame, &explicit, &limits()).unwrap();
    assert_eq!(
        converted.descriptor().domain(),
        SampleDomain::LinearRelative
    );
}

#[test]
fn native_range_and_matrix_are_checked_separately_from_source_route() {
    let color = ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 1, false));
    let frame = rgb_frame(color, SampleDomain::Encoded);
    let good = ColorConvertOptions::new(Destination::encoded_cicp_rgb(NclxColorInformation::new(
        1, 13, 1, false,
    )))
    .with_native_sample_encoding(NativeSampleEncoding::new(
        SampleRange::Limited,
        MatrixCoefficients::Identity,
        None,
    ));
    assert!(matches!(
        good.validate_for(&frame, &limits()),
        Err(ProcessingError::Invalid(_))
    ));
}

#[test]
fn missing_or_conflicting_interpretation_is_an_error() {
    let frame = rgb_frame(ColorInformationSet::new(), SampleDomain::Encoded);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_native_sample_encoding(rgb_native());
    assert!(options.validate_for(&frame, &limits()).is_err());

    let malformed = ColorInformationSet::new()
        .with_icc_profile(vec![1, 2, 3])
        .unwrap();
    let frame = rgb_frame(malformed, SampleDomain::Encoded);
    assert!(options.validate_for(&frame, &limits()).is_err());
}

#[test]
fn linear_samples_use_explicit_primaries_without_a_second_transfer() {
    let frame = rgb_frame(ColorInformationSet::new(), SampleDomain::LinearRelative);
    let descriptor = frame
        .descriptor()
        .clone()
        .with_primaries(RgbPrimaries::srgb());
    let planes = (0..3)
        .map(|_| {
            let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
            Plane::new(layout, vec![0.25f32]).unwrap()
        })
        .collect::<Vec<_>>();
    let frame = ImageFrame::new(descriptor, PixelBuffer::f32(planes).unwrap()).unwrap();
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    options.validate_for(&frame, &limits()).unwrap();
}

#[test]
fn explicit_identity_cicp_preserves_a_declared_hdr_domain() {
    let frame = linear_rgb_frame(SampleDomain::LinearAbsoluteNits);
    let source = SourceInterpretation::Cicp(NclxColorInformation::new(1, 8, 0, true));
    let same_domain = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearAbsoluteNits,
        RgbPrimaries::srgb(),
    ))
    .with_source(source);
    same_domain.validate_for(&frame, &limits()).unwrap();

    let relative_destination = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(source);
    assert!(matches!(
        relative_destination.validate_for(&frame, &limits()),
        Err(ProcessingError::Unsupported(_))
    ));
}

#[test]
fn linear_destination_domains_are_checked_without_allocating_output() {
    let color = ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 0, true));
    for domain in [
        SampleDomain::LinearRelative,
        SampleDomain::LinearAbsoluteNits,
        SampleDomain::HlgSceneLinear,
        SampleDomain::HlgDisplayLinear(HlgDisplayConditions::new(1000.0, 0.1, 1.2).unwrap()),
    ] {
        let frame = rgb_frame(color.clone(), SampleDomain::Encoded);
        let options =
            ColorConvertOptions::new(Destination::linear_rgb(domain, RgbPrimaries::srgb()))
                .with_native_sample_encoding(rgb_native());
        let result = options.validate_for(&frame, &limits());
        if domain == SampleDomain::LinearRelative {
            result.unwrap();
        } else {
            assert!(matches!(result, Err(ProcessingError::Unsupported(_))));
        }
    }
}

#[test]
fn domain_route_rejects_undefined_hdr_and_encoded_crossings() {
    let icc_color = ColorInformationSet::new()
        .with_icc_profile(valid_rgb_profile())
        .unwrap()
        .with_icc_color_type(IccColorType::Prof);
    let icc_frame = rgb_frame(icc_color, SampleDomain::Encoded);
    for destination_domain in [
        SampleDomain::LinearAbsoluteNits,
        SampleDomain::HlgSceneLinear,
    ] {
        let options = ColorConvertOptions::new(Destination::linear_rgb(
            destination_domain,
            RgbPrimaries::srgb(),
        ))
        .with_native_sample_encoding(rgb_native());
        assert!(matches!(
            options.validate_for(&icc_frame, &limits()),
            Err(ProcessingError::Unsupported(_))
        ));
    }

    let pq_color = ColorInformationSet::new().with_nclx(NclxColorInformation::new(9, 16, 0, true));
    let pq_frame = rgb_frame(pq_color, SampleDomain::Encoded);
    let pq_to_hlg = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::HlgSceneLinear,
        RgbPrimaries::srgb(),
    ))
    .with_native_sample_encoding(NativeSampleEncoding::new(
        SampleRange::Full,
        MatrixCoefficients::Identity,
        None,
    ));
    let pq_to_hlg_result = pq_to_hlg.validate_for(&pq_frame, &limits());
    assert!(
        matches!(&pq_to_hlg_result, Err(ProcessingError::Unsupported(_))),
        "unexpected PQ to HLG result: {pq_to_hlg_result:?}"
    );
    let pq_to_sdr = ColorConvertOptions::new(Destination::encoded_cicp_rgb(
        NclxColorInformation::new(1, 13, 0, true),
    ))
    .with_native_sample_encoding(NativeSampleEncoding::new(
        SampleRange::Full,
        MatrixCoefficients::Identity,
        None,
    ));
    assert!(matches!(
        pq_to_sdr.validate_for(&pq_frame, &limits()),
        Err(ProcessingError::Unsupported(_))
    ));

    let absolute_frame = linear_rgb_frame(SampleDomain::LinearAbsoluteNits);
    let destination_profile = valid_rgb_profile();
    let linear_to_icc = ColorConvertOptions::new(Destination::icc_rgb(
        &destination_profile,
        IccColorType::Prof,
    ));
    assert!(matches!(
        linear_to_icc.validate_for(&absolute_frame, &limits()),
        Err(ProcessingError::Unsupported(_))
    ));
}

#[test]
fn gray_preserves_non_identity_signaling_and_explicitly_fills_unknown_matrix() {
    for matrix in [1, 5, 9] {
        let color =
            ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, matrix, true));
        let frame = gray_frame(color);
        let options = ColorConvertOptions::new(Destination::encoded_cicp_rgb(
            NclxColorInformation::new(1, 13, matrix, true),
        ));
        options.validate_for(&frame, &limits()).unwrap();
    }

    let mut av1_gray =
        gray_frame(ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 1, true)));
    av1_gray
        .metadata_mut()
        .set_av1_description(Some(Av1Description::new(true).with_flags(
            Some(true),
            Some(true),
            Some(true),
            Some(true),
            Some(0),
        )));
    let av1_options = ColorConvertOptions::new(Destination::encoded_cicp_rgb(
        NclxColorInformation::new(1, 13, 1, true),
    ));
    av1_options.validate_for(&av1_gray, &limits()).unwrap();

    let unknown =
        gray_frame(ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 2, true)));
    let destination = Destination::encoded_cicp_rgb(NclxColorInformation::new(1, 13, 0, true));
    let without_fill = ColorConvertOptions::new(destination);
    without_fill.validate_for(&unknown, &limits()).unwrap();

    let unknown_rgb = rgb_frame(
        ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 2, true)),
        SampleDomain::Encoded,
    );
    assert!(without_fill.validate_for(&unknown_rgb, &limits()).is_err());
    let with_fill = without_fill.with_native_sample_encoding(rgb_native());
    with_fill.validate_for(&unknown_rgb, &limits()).unwrap();
}

#[test]
fn unimplemented_intent_and_alpha_routes_are_explicit_errors() {
    let color = ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 0, true));
    let frame = rgb_frame(color, SampleDomain::Encoded);
    let intent = ColorConvertOptions::new(Destination::encoded_cicp_rgb(
        NclxColorInformation::new(1, 13, 0, true),
    ))
    .with_native_sample_encoding(rgb_native())
    .with_rendering_intent(RenderingIntent::Perceptual);
    assert!(matches!(
        intent.validate_for(&frame, &limits()),
        Err(ProcessingError::Unsupported(_))
    ));
    let alpha = intent
        .with_rendering_intent(RenderingIntent::RelativeColorimetric)
        .with_alpha_policy(AlphaPolicy::ChangeAssociation {
            input: AlphaAssociation::Straight,
            output: AlphaAssociation::Premultiplied,
            multiplication_domain: SampleDomain::LinearRelative,
            zero_alpha: ZeroAlphaPolicy::SetZero,
        });
    assert!(matches!(
        alpha.validate_for(&frame, &limits()),
        Err(ProcessingError::Unsupported(_))
    ));
}

#[test]
fn av1_positions_and_h273_codes_are_distinct_and_checked() {
    let left = ChromaLocation::av1_position(1).unwrap();
    let top_left = ChromaLocation::av1_position(2).unwrap();
    assert_eq!(left.h273_code(), 0);
    assert_eq!(top_left.h273_code(), 2);
    assert_ne!(left, top_left);
    assert_eq!(
        ChromaLocation::from_h273_code(5).unwrap().y_phase(),
        ChromaPhase::One
    );
    assert!(ChromaLocation::new(ChromaPhase::Zero, ChromaPhase::Half, 2).is_err());
}

#[test]
fn borrowed_profile_aliases_existing_metadata_without_extra_charge() {
    let color = ColorInformationSet::new()
        .with_icc_profile(valid_rgb_profile())
        .unwrap()
        .with_icc_color_type(IccColorType::Prof)
        .with_nclx(NclxColorInformation::new(1, 13, 0, true));
    let frame = rgb_frame(color, SampleDomain::Encoded);
    let metadata_profile = frame.metadata().source_color().icc_profile().unwrap();
    let active = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_native_sample_encoding(rgb_native());
    let explicit_metadata = active.with_source(SourceInterpretation::Icc {
        profile: metadata_profile,
        color_type: IccColorType::Prof,
    });

    assert_eq!(
        minimum_total_budget(&frame, &active),
        minimum_total_budget(&frame, &explicit_metadata),
        "an ICC slice already owned by frame metadata must not be charged again"
    );
}
