use wml2::highres::*;

fn limits(
    max_plane_bytes: usize,
    max_frame_bytes: usize,
    max_metadata_bytes: usize,
) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(4096)
        .max_height(4096)
        .max_pixels(4096 * 4096)
        .max_channels(8)
        .max_planes(4)
        .max_plane_bytes(max_plane_bytes)
        .max_frame_bytes(max_frame_bytes)
        .max_total_live_decoded_bytes(max_frame_bytes)
        .max_references(8)
        .max_frame_count(128)
        .max_metadata_bytes(max_metadata_bytes)
        .max_icc_bytes(1 << 20)
        .max_clut_bytes(1 << 20)
        .max_parser_entries(1024)
        .max_parser_depth(64)
        .max_grid_cells(256)
        .max_derived_work(1 << 20)
        .max_derived_depth(64)
        .build()
        .unwrap()
}

#[test]
fn domain_and_primaries_are_explicit_and_checked() {
    let descriptor = ImageDescriptor::gray(1, 1, 16).unwrap();
    assert!(descriptor.domain().require_explicit().is_err());
    assert!(RgbPrimaries::new((f32::NAN, 0.3), (0.3, 0.6), (0.15, 0.06), (0.3127, 0.329)).is_err());
    assert!(HlgDisplayConditions::new(1000.0, 1000.0, 1.2).is_err());
    assert!(
        SampleDomain::HlgDisplayLinear(HlgDisplayConditions::new(1000.0, 0.1, 1.2).unwrap())
            .require_explicit()
            .is_ok()
    );
}

#[test]
fn raw_fractional_geometry_and_colour_properties_are_retained_in_order() {
    assert!(RawRational::new(1, 0).is_err());
    let width = RawRational::new(1920, 1).unwrap();
    let height = RawRational::new(1080, 1).unwrap();
    let aperture = CleanAperture::new(width, height, RawRational::new(1, 2).unwrap(), height);
    assert_eq!(aperture.width().numerator_bits(), 1920);
    assert_eq!(aperture.horizontal_offset().denominator(), 2);

    let mut colors = ColorInformationSet::new()
        .with_icc_profile(vec![1, 2, 3])
        .unwrap()
        .with_icc_color_type(IccColorType::Ricc)
        .with_nclx(NclxColorInformation::new(9, 16, 9, true));
    colors.push_unknown_colr(UnknownColorInformation {
        color_type: *b"myst",
        payload: vec![4, 5],
    });
    assert_eq!(colors.icc_color_type(), Some(IccColorType::Ricc));
    assert_eq!(colors.nclx().unwrap().primaries(), 9);
    assert_eq!(colors.unknown_colr()[0].color_type, *b"myst");

    let mut metadata = FrameMetadata::new(colors);
    metadata.set_clean_aperture(Some(aperture));
    metadata.set_coded_geometry(vec![
        GeometryOperation::Rotate(Rotation::Degrees90),
        GeometryOperation::MirrorHorizontal,
    ]);
    metadata.set_render_geometry(vec![GeometryOperation::MirrorHorizontal]);
    metadata.set_sequence_information(Some(SequenceInformation::new(
        7,
        RawRational::new(1001, 24000).unwrap(),
        RepetitionCount::Finite(2),
    )));
    assert_eq!(metadata.coded_geometry().len(), 2);
    assert_eq!(metadata.render_geometry().len(), 1);
    assert_eq!(metadata.sequence_information().unwrap().frame_count(), 7);
}

#[test]
fn limits_count_owned_capacity_and_aggregate_metadata() {
    let descriptor = ImageDescriptor::gray(1, 1, 8).unwrap();
    let layout = descriptor.planes()[0].layout().clone();
    let mut samples = Vec::with_capacity(32);
    samples.push(0u8);
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u8(vec![Plane::new(layout, samples).unwrap()]).unwrap(),
    )
    .unwrap();
    assert!(frame.validate_with_limits(&limits(1, 1, 1024)).is_err());

    let descriptor = ImageDescriptor::gray(1, 1, 8).unwrap();
    let layout = descriptor.planes()[0].layout().clone();
    let mut frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u8(vec![Plane::new(layout, vec![0]).unwrap()]).unwrap(),
    )
    .unwrap();
    frame
        .metadata_mut()
        .tags_mut()
        .insert("raw".into(), wml2::metadata::DataMap::Raw(vec![0; 16]));
    assert!(frame.validate_with_limits(&limits(1, 1, 8)).is_err());
}

#[test]
fn metadata_capacity_and_geometry_element_size_are_budgeted() {
    let mut profile = Vec::with_capacity(4096);
    profile.push(1);
    let descriptor = ImageDescriptor::gray(1, 1, 8)
        .unwrap()
        .with_color_information(
            ColorInformationSet::new()
                .with_icc_profile(profile)
                .unwrap(),
        );
    let layout = descriptor.planes()[0].layout().clone();
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u8(vec![Plane::new(layout, vec![0]).unwrap()]).unwrap(),
    )
    .unwrap();
    assert!(frame.validate_with_limits(&limits(1, 1, 64)).is_err());

    let descriptor = ImageDescriptor::gray(1, 1, 8).unwrap();
    let layout = descriptor.planes()[0].layout().clone();
    let mut frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u8(vec![Plane::new(layout, vec![0]).unwrap()]).unwrap(),
    )
    .unwrap();
    frame
        .metadata_mut()
        .set_coded_geometry(Vec::with_capacity(10));
    assert!(frame.validate_with_limits(&limits(1, 1, 16)).is_err());
}

#[test]
fn pixi_channels_and_rgb_white_point_are_checked() {
    assert!(
        PixelInformation::new(
            vec![8, 8, 8],
            Some(vec![PixelChannelInformation::new(0, 0, None)]),
        )
        .is_err()
    );
    assert!(RgbPrimaries::new((0.64, 0.33), (0.30, 0.60), (0.15, 0.06), (0.3, 0.0),).is_err());
}

#[test]
fn all_resource_limit_fields_are_required() {
    assert!(ResourceLimits::builder().max_width(1).build().is_err());
}
