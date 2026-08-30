use wml2::highres::{
    AlphaAssociation, ChannelModel, ChannelRole, ColorInformationSet, ColorProvenance,
    FrameMetadata, IccColorType, IccConversionOptions, ImageDescriptor, ImageFrame, PixelBuffer,
    Plane, PlaneDescriptor, PlaneLayout, ResourceLimits, SampleDomain, Subsampling,
    convert_frame_with_icc,
};

fn limits(max_frame_bytes: usize, max_live_bytes: usize) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(64)
        .max_height(64)
        .max_pixels(4096)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 20)
        .max_frame_bytes(max_frame_bytes)
        .max_total_live_decoded_bytes(max_live_bytes)
        .max_references(64)
        .max_frame_count(64)
        .max_metadata_bytes(1 << 20)
        .max_icc_bytes(1 << 20)
        .max_clut_bytes(1 << 20)
        .max_parser_entries(1 << 16)
        .max_parser_depth(64)
        .max_grid_cells(1 << 16)
        .max_derived_work(1 << 20)
        .max_derived_depth(32)
        .build()
        .unwrap()
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn raw_profile(space: &[u8; 4], tags: Vec<(&[u8; 4], Vec<u8>)>) -> Vec<u8> {
    let mut bytes = vec![0; 132 + tags.len() * 12];
    bytes[8..12].copy_from_slice(&[4, 0, 0, 0]);
    bytes[12..16].copy_from_slice(b"mntr");
    bytes[16..20].copy_from_slice(space);
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    put_u32(&mut bytes, 64, 1);
    put_u32(&mut bytes, 128, tags.len() as u32);
    for (index, (signature, data)) in tags.into_iter().enumerate() {
        let offset = (bytes.len() + 3) & !3;
        bytes.resize(offset + data.len(), 0);
        bytes[offset..offset + data.len()].copy_from_slice(&data);
        bytes[132 + index * 12..136 + index * 12].copy_from_slice(signature);
        put_u32(&mut bytes, 136 + index * 12, offset as u32);
        put_u32(&mut bytes, 140 + index * 12, data.len() as u32);
    }
    let length = bytes.len() as u32;
    put_u32(&mut bytes, 0, length);
    bytes
}

fn xyz_tag(values: [f32; 3]) -> Vec<u8> {
    let mut tag = vec![0; 20];
    tag[..4].copy_from_slice(b"XYZ ");
    for (index, value) in values.into_iter().enumerate() {
        put_u32(
            &mut tag,
            8 + index * 4,
            ((value * 65536.0).round() as i32) as u32,
        );
    }
    tag
}

fn identity_curve() -> Vec<u8> {
    let mut curve = vec![0; 16];
    curve[..4].copy_from_slice(b"curv");
    put_u32(&mut curve, 8, 2);
    curve[12..14].copy_from_slice(&0u16.to_be_bytes());
    curve[14..16].copy_from_slice(&u16::MAX.to_be_bytes());
    curve
}

fn rgb_profile() -> Vec<u8> {
    let curve = identity_curve();
    raw_profile(
        b"RGB ",
        vec![
            (b"rXYZ", xyz_tag([0.9642, 0.0, 0.0])),
            (b"gXYZ", xyz_tag([0.0, 1.0, 0.0])),
            (b"bXYZ", xyz_tag([0.0, 0.0, 0.8249])),
            (b"rTRC", curve.clone()),
            (b"gTRC", curve.clone()),
            (b"bTRC", curve),
        ],
    )
}

fn identity_chad_tag() -> Vec<u8> {
    let mut tag = vec![0; 44];
    tag[..4].copy_from_slice(b"sf32");
    for (index, value) in [
        1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
    ]
    .into_iter()
    .enumerate()
    {
        put_u32(
            &mut tag,
            8 + index * 4,
            ((value * 65536.0).round() as i32) as u32,
        );
    }
    tag
}

fn tag_rich_rgb_profile() -> Vec<u8> {
    let curve = identity_curve();
    raw_profile(
        b"RGB ",
        vec![
            (b"rXYZ", xyz_tag([0.9642, 0.0, 0.0])),
            (b"gXYZ", xyz_tag([0.0, 1.0, 0.0])),
            (b"bXYZ", xyz_tag([0.0, 0.0, 0.8249])),
            (b"rTRC", curve.clone()),
            (b"gTRC", curve.clone()),
            (b"bTRC", curve),
            (b"wtpt", xyz_tag([0.9642, 1.0, 0.8249])),
            (b"chad", identity_chad_tag()),
        ],
    )
}

fn gray_profile() -> Vec<u8> {
    raw_profile(b"GRAY", vec![(b"kTRC", identity_curve())])
}

fn rgb_frame(samples: [[f32; 2]; 3], alpha: Option<[f32; 2]>) -> ImageFrame {
    let mut descriptors = Vec::new();
    let mut planes = Vec::new();
    for (role, values) in [
        (ChannelRole::Red, samples[0]),
        (ChannelRole::Green, samples[1]),
        (ChannelRole::Blue, samples[2]),
    ] {
        let layout = PlaneLayout::planar(2, 1, Subsampling::FULL).unwrap();
        descriptors.push(PlaneDescriptor::planar(layout.clone(), role, 32).unwrap());
        planes.push(Plane::new(layout, values.to_vec()).unwrap());
    }
    if let Some(values) = alpha {
        let layout = PlaneLayout::planar(2, 1, Subsampling::FULL).unwrap();
        descriptors.push(PlaneDescriptor::planar(layout.clone(), ChannelRole::Alpha, 32).unwrap());
        planes.push(Plane::new(layout, values.to_vec()).unwrap());
    }
    let descriptor = ImageDescriptor::new(2, 1, ChannelModel::RGB, descriptors)
        .unwrap()
        .with_alpha(if alpha.is_some() {
            AlphaAssociation::Straight
        } else {
            AlphaAssociation::None
        })
        .unwrap()
        .with_domain(SampleDomain::Encoded);
    ImageFrame::new(descriptor, PixelBuffer::f32(planes).unwrap()).unwrap()
}

fn gray_frame(samples: [f32; 2]) -> ImageFrame {
    let layout = PlaneLayout::planar(2, 1, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::gray(2, 1, 32)
        .unwrap()
        .with_domain(SampleDomain::Encoded);
    let plane = Plane::new(layout, samples.to_vec()).unwrap();
    ImageFrame::new(descriptor, PixelBuffer::f32(vec![plane]).unwrap()).unwrap()
}

fn gray_frame_with_alpha(samples: [f32; 2], alpha: [f32; 2]) -> ImageFrame {
    let mut descriptors = Vec::new();
    let mut planes = Vec::new();
    for (role, values) in [(ChannelRole::Gray, samples), (ChannelRole::Alpha, alpha)] {
        let layout = PlaneLayout::planar(2, 1, Subsampling::FULL).unwrap();
        descriptors.push(PlaneDescriptor::planar(layout.clone(), role, 32).unwrap());
        planes.push(Plane::new(layout, values.to_vec()).unwrap());
    }
    let descriptor = ImageDescriptor::new(2, 1, ChannelModel::Gray, descriptors)
        .unwrap()
        .with_alpha(AlphaAssociation::Straight)
        .unwrap()
        .with_domain(SampleDomain::Encoded);
    ImageFrame::new(descriptor, PixelBuffer::f32(planes).unwrap()).unwrap()
}

#[test]
fn explicit_rgb_icc_route_preserves_alpha_and_emits_encoded_f32() {
    let profile = rgb_profile();
    let source = rgb_frame([[0.0, 0.25], [0.5, 0.75], [1.0, 0.125]], Some([0.2, 0.8]));
    let output = convert_frame_with_icc(
        &source,
        IccConversionOptions::new(&profile, &profile),
        &limits(1 << 20, 1 << 20),
    )
    .expect("identity RGB ICC transform should succeed");
    assert_eq!(output.descriptor().model(), ChannelModel::RGB);
    assert_eq!(output.descriptor().domain(), SampleDomain::Encoded);
    assert_eq!(output.pixels().format(), wml2::highres::PixelFormat::F32);
    let planes = output.pixels().f32_planes().unwrap();
    for (actual, expected) in planes[0..3]
        .iter()
        .zip(source.pixels().f32_planes().unwrap())
    {
        for (actual, expected) in actual.samples().iter().zip(expected.samples()) {
            assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
        }
    }
    assert_eq!(planes[3].samples(), &[0.2, 0.8]);
    assert_eq!(
        output.descriptor().color_information().icc_profile(),
        Some(profile.as_slice())
    );
    assert_eq!(
        output.descriptor().color_information().icc_color_type(),
        Some(IccColorType::Prof)
    );
    assert!(
        output
            .descriptor()
            .color_information()
            .provenance()
            .contains(&ColorProvenance::Explicit)
    );
    assert!(
        !output
            .descriptor()
            .color_information()
            .provenance()
            .contains(&ColorProvenance::EmbeddedIcc)
    );
    assert!(
        output
            .metadata()
            .source_color()
            .provenance()
            .contains(&ColorProvenance::Explicit)
    );
}

#[test]
fn explicit_source_profile_is_retained_as_distinct_provenance() {
    let source_profile = rgb_profile();
    let destination_profile = rgb_profile();
    let source =
        rgb_frame([[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]], None).with_metadata(FrameMetadata::new(
            ColorInformationSet::new()
                .with_icc_profile(vec![0x7f])
                .unwrap()
                .with_icc_color_type(IccColorType::Ricc),
        ));
    let output = convert_frame_with_icc(
        &source,
        IccConversionOptions::new(&source_profile, &destination_profile)
            .with_source_color_type(IccColorType::Ricc)
            .with_destination_color_type(IccColorType::Prof),
        &limits(1 << 20, 1 << 20),
    )
    .expect("explicit ICC route should retain source provenance");
    assert_eq!(
        output.metadata().source_color().icc_profile(),
        Some(source_profile.as_slice())
    );
    assert_eq!(
        output.metadata().source_color().icc_color_type(),
        Some(IccColorType::Ricc)
    );
    assert_eq!(
        output.descriptor().color_information().icc_profile(),
        Some(destination_profile.as_slice())
    );
    assert_eq!(
        output.descriptor().color_information().icc_color_type(),
        Some(IccColorType::Prof)
    );
    assert!(
        output
            .descriptor()
            .color_information()
            .provenance()
            .contains(&ColorProvenance::Explicit)
    );
    assert!(
        !output
            .descriptor()
            .color_information()
            .provenance()
            .contains(&ColorProvenance::EmbeddedIcc)
    );
    assert!(
        output
            .metadata()
            .source_color()
            .provenance()
            .contains(&ColorProvenance::Explicit)
    );
    assert!(
        output
            .metadata()
            .source_color()
            .provenance()
            .contains(&ColorProvenance::EmbeddedIcc)
    );
}

#[test]
fn icc_rejects_out_of_domain_without_publishing_output() {
    let profile = rgb_profile();
    let source = rgb_frame([[1.5, 0.0], [0.0, 0.0], [0.0, 0.0]], None);
    let result = convert_frame_with_icc(
        &source,
        IccConversionOptions::new(&profile, &profile),
        &limits(1 << 20, 1 << 20),
    );
    assert!(result.is_err());
    assert_eq!(source.pixels().f32_planes().unwrap()[0].samples()[0], 1.5);
}

#[test]
fn icc_history_replaces_stale_cicp_with_explicit_authorities() {
    let source_profile = rgb_profile();
    let destination_profile = rgb_profile();
    let source = rgb_frame([[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]], Some([0.25, 0.75])).with_metadata(
        FrameMetadata::new(
            ColorInformationSet::new()
                .with_nclx(wml2::highres::NclxColorInformation::new(9, 16, 9, false)),
        ),
    );
    let source_metadata_before = source.metadata().clone();
    let output = convert_frame_with_icc(
        &source,
        IccConversionOptions::new(&source_profile, &destination_profile)
            .with_source_color_type(IccColorType::Ricc)
            .with_destination_color_type(IccColorType::Prof),
        &limits(1 << 20, 1 << 20),
    )
    .expect("explicit ICC conversion should succeed");
    let history = output
        .metadata()
        .last_conversion()
        .expect("ICC conversion must publish history");
    assert_eq!(history.source(), wml2::highres::ConversionSource::Icc);
    assert_eq!(history.source_cicp(), None);
    assert_eq!(source.metadata(), &source_metadata_before);
    assert_eq!(history.source_domain(), SampleDomain::Encoded);
    assert_eq!(
        history.source_primaries_authority(),
        wml2::highres::ConversionPrimaries::IccDefined
    );
    assert_eq!(
        history.source_primaries_info(),
        wml2::highres::ConversionPrimaries::IccDefined
    );
    assert_eq!(history.source_primaries_if_defined(), None);
    assert_eq!(
        history.source_icc().unwrap().color_type(),
        IccColorType::Ricc
    );
    assert_eq!(
        history.destination(),
        wml2::highres::ConversionDestination::Icc
    );
    assert_eq!(
        history.destination_icc().unwrap().color_type(),
        IccColorType::Prof
    );
    assert_eq!(history.destination_domain(), SampleDomain::Encoded);
    assert_eq!(
        history.destination_primaries_authority(),
        wml2::highres::ConversionPrimaries::IccDefined
    );
    assert_eq!(
        history.destination_primaries_info(),
        wml2::highres::ConversionPrimaries::IccDefined
    );
    assert_eq!(history.destination_primaries_if_defined(), None);
    assert_eq!(
        history.white_adaptation(),
        wml2::highres::ConversionWhiteAdaptation::IccD50Pcs
    );
    assert_eq!(
        history.alpha(),
        wml2::highres::ConversionAlpha::PreserveAssociation
    );
}

#[test]
fn tag_rich_icc_live_budget_has_exact_and_one_under_boundaries() {
    let profile = tag_rich_rgb_profile();
    let source = rgb_frame([[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]], None).with_metadata(
        FrameMetadata::new(
            ColorInformationSet::new()
                .with_icc_profile(vec![0xa5; 23])
                .unwrap()
                .with_icc_color_type(IccColorType::Prof),
        ),
    );
    let source_before = source.clone();
    let source_profile_ptr = source
        .metadata()
        .source_color()
        .icc_profile()
        .expect("rich source ICC")
        .as_ptr();
    let source_sample_ptr = source.pixels().f32_planes().unwrap()[0].samples().as_ptr();
    let options = IccConversionOptions::new(&profile, &profile);
    let mut low = 0usize;
    let mut high = 1 << 20;
    assert!(convert_frame_with_icc(&source, options, &limits(1 << 20, high)).is_ok());
    while low < high {
        let midpoint = low + (high - low) / 2;
        if convert_frame_with_icc(&source, options, &limits(1 << 20, midpoint)).is_ok() {
            high = midpoint;
        } else {
            low = midpoint + 1;
        }
    }
    assert!(low > 0);
    assert!(matches!(
        convert_frame_with_icc(&source, options, &limits(1 << 20, low - 1)),
        Err(wml2::highres::ProcessingError::ResourceLimit(_))
    ));
    assert!(convert_frame_with_icc(&source, options, &limits(1 << 20, low)).is_ok());
    assert_eq!(source, source_before);
    assert_eq!(
        source
            .metadata()
            .source_color()
            .icc_profile()
            .unwrap()
            .as_ptr(),
        source_profile_ptr
    );
    assert_eq!(
        source.pixels().f32_planes().unwrap()[0].samples().as_ptr(),
        source_sample_ptr
    );
}

#[test]
fn icc_transform_and_output_are_bounded_before_allocation() {
    let profile = rgb_profile();
    let source = rgb_frame([[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]], None);
    let result = convert_frame_with_icc(
        &source,
        IccConversionOptions::new(&profile, &profile),
        &limits(1, 1),
    );
    assert!(matches!(
        result,
        Err(wml2::highres::ProcessingError::ResourceLimit(_))
    ));
}

#[test]
fn icc_live_budget_has_exact_boundary_without_publishing_output() {
    let profile = rgb_profile();
    let source = rgb_frame([[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]], None);
    let options = IccConversionOptions::new(&profile, &profile);
    let mut low = 0usize;
    let mut high = 1 << 20;
    assert!(convert_frame_with_icc(&source, options, &limits(1 << 20, high)).is_ok());
    while low < high {
        let midpoint = low + (high - low) / 2;
        if convert_frame_with_icc(&source, options, &limits(1 << 20, midpoint)).is_ok() {
            high = midpoint;
        } else {
            low = midpoint + 1;
        }
    }
    assert!(low > 0);
    assert!(matches!(
        convert_frame_with_icc(&source, options, &limits(1 << 20, low - 1)),
        Err(wml2::highres::ProcessingError::ResourceLimit(_))
    ));
    assert!(convert_frame_with_icc(&source, options, &limits(1 << 20, low)).is_ok());
    assert_eq!(
        source.pixels().f32_planes().unwrap()[0].samples(),
        &[0.1, 0.2]
    );
}

#[test]
fn explicit_gray_icc_route_remains_gray_and_encoded() {
    let profile = gray_profile();
    let source = gray_frame([0.125, 0.875]);
    let output = convert_frame_with_icc(
        &source,
        IccConversionOptions::new(&profile, &profile),
        &limits(1 << 20, 1 << 20),
    )
    .expect("identity Gray ICC transform should succeed");
    assert_eq!(output.descriptor().model(), ChannelModel::Gray);
    assert_eq!(output.descriptor().domain(), SampleDomain::Encoded);
    assert_eq!(
        output.pixels().f32_planes().unwrap()[0].samples(),
        &[0.125, 0.875]
    );
}

#[test]
fn explicit_gray_icc_route_preserves_alpha_without_an_outer_sample_vec() {
    let profile = gray_profile();
    let source = gray_frame_with_alpha([0.125, 0.875], [0.2, 0.8]);
    let output = convert_frame_with_icc(
        &source,
        IccConversionOptions::new(&profile, &profile),
        &limits(1 << 20, 1 << 20),
    )
    .expect("identity Gray ICC transform with alpha should succeed");
    assert_eq!(output.descriptor().model(), ChannelModel::Gray);
    assert_eq!(output.descriptor().alpha(), AlphaAssociation::Straight);
    let planes = output.pixels().f32_planes().unwrap();
    assert_eq!(planes.len(), 2);
    assert_eq!(planes[0].samples(), &[0.125, 0.875]);
    assert_eq!(planes[1].samples(), &[0.2, 0.8]);
}
