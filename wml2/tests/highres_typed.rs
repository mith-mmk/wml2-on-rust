use wml2::highres::*;

#[test]
fn odd_subsampled_planes_use_ceiling_dimensions_and_sample_strides() {
    let frame = ImageDescriptor::ycbcr(
        5,
        3,
        10,
        [
            Subsampling::FULL,
            Subsampling::new(2, 2).unwrap(),
            Subsampling::new(2, 1).unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(
        (
            frame.planes()[0].layout().width(),
            frame.planes()[0].layout().height()
        ),
        (5, 3)
    );
    assert_eq!(
        (
            frame.planes()[1].layout().width(),
            frame.planes()[1].layout().height()
        ),
        (3, 2)
    );
    assert_eq!(
        (
            frame.planes()[2].layout().width(),
            frame.planes()[2].layout().height()
        ),
        (3, 3)
    );

    let planes = frame
        .planes()
        .iter()
        .map(|plane| {
            Plane::new(
                plane.layout().clone(),
                vec![0u16; plane.layout().addressed_samples()],
            )
        })
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let image = ImageFrame::new(frame, PixelBuffer::u16(planes).unwrap()).unwrap();
    image.validate().unwrap();
}

#[test]
fn interleaved_layout_allows_padding_but_rejects_overlap_and_negative_like_strides() {
    let layout = PlaneLayout::new(2, 2, 8, 4, vec![0, 1, 2], Subsampling::FULL).unwrap();
    assert_eq!(layout.addressed_samples(), 15);
    assert!(PlaneLayout::new(2, 2, 8, 3, vec![0, 1, 1], Subsampling::FULL).is_err());
    assert!(PlaneLayout::new(2, 2, 8, 3, vec![0, 3], Subsampling::FULL).is_err());
}

#[test]
fn frame_validation_rejects_actual_layout_mismatch_before_alpha_indexing() {
    let descriptor_layout = PlaneLayout::interleaved(1, 1, 4, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        1,
        1,
        ChannelModel::RGB,
        vec![
            PlaneDescriptor::new(
                descriptor_layout,
                vec![
                    ChannelRole::Red,
                    ChannelRole::Green,
                    ChannelRole::Blue,
                    ChannelRole::Alpha,
                ],
                8,
            )
            .unwrap(),
        ],
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap();
    let actual_layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let actual = Plane::new(actual_layout, vec![0.0f32]).unwrap();
    assert!(matches!(
        ImageFrame::new(descriptor, PixelBuffer::f32(vec![actual]).unwrap()),
        Err(HighresError::InvalidLayout(_))
    ));

    let descriptor = ImageDescriptor::gray(1, 1, 8).unwrap();
    let actual_layout = PlaneLayout::new(1, 1, 2, 1, vec![0], Subsampling::FULL).unwrap();
    let actual = Plane::new(actual_layout, vec![0u8, 0]).unwrap();
    assert!(ImageFrame::new(descriptor, PixelBuffer::u8(vec![actual]).unwrap()).is_err());
}

#[test]
fn integer_precision_and_alpha_are_checked_at_frame_boundary() {
    let descriptor = ImageDescriptor::gray(2, 1, 10).unwrap();
    let plane = Plane::new(descriptor.planes()[0].layout().clone(), vec![0u16, 1024]).unwrap();
    assert!(matches!(
        ImageFrame::new(descriptor, PixelBuffer::u16(vec![plane]).unwrap()),
        Err(HighresError::InvalidSamples(_))
    ));

    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let alpha = PlaneDescriptor::planar(layout.clone(), ChannelRole::Alpha, 16).unwrap();
    let color = PlaneDescriptor::planar(layout, ChannelRole::Gray, 16).unwrap();
    let descriptor = ImageDescriptor::new(1, 1, ChannelModel::Gray, vec![color, alpha])
        .unwrap()
        .with_alpha(AlphaAssociation::Straight)
        .unwrap();
    let planes = descriptor
        .planes()
        .iter()
        .map(|p| Plane::new(p.layout().clone(), vec![0.5f32]))
        .collect::<Result<Vec<_>>>()
        .unwrap();
    ImageFrame::new(descriptor, PixelBuffer::f32(planes).unwrap()).unwrap();
}

#[test]
fn u16_represents_10_12_and_16_bit_samples_without_requantization() {
    for meaningful_bits in [10u8, 12, 16] {
        let descriptor = ImageDescriptor::gray(2, 1, meaningful_bits).unwrap();
        let max = if meaningful_bits == 16 {
            u16::MAX
        } else {
            (1u16 << meaningful_bits) - 1
        };
        let layout = descriptor.planes()[0].layout().clone();
        let plane = Plane::new(layout, vec![0u16, max]).unwrap();
        let frame = ImageFrame::new(descriptor, PixelBuffer::u16(vec![plane]).unwrap()).unwrap();
        assert_eq!(
            frame.descriptor().planes()[0].meaningful_bits(),
            meaningful_bits
        );
        assert_eq!(frame.pixels().u16_planes().unwrap()[0].samples()[1], max);
    }
}

#[test]
fn planar_and_interleaved_rgb_with_alpha_keep_sample_unit_stride() {
    let layout = PlaneLayout::new(2, 1, 10, 4, vec![0, 1, 2, 3], Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        2,
        1,
        ChannelModel::RGB,
        vec![
            PlaneDescriptor::new(
                layout.clone(),
                vec![
                    ChannelRole::Red,
                    ChannelRole::Green,
                    ChannelRole::Blue,
                    ChannelRole::Alpha,
                ],
                16,
            )
            .unwrap(),
        ],
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap();
    let samples = vec![1u16, 2, 3, 4, 5, 6, 7, 8, 0, 0];
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![Plane::new(layout, samples).unwrap()]).unwrap(),
    )
    .unwrap();
    assert_eq!(
        frame.pixels().u16_planes().unwrap()[0]
            .layout()
            .row_stride(),
        10
    );
    assert_eq!(frame.descriptor().alpha(), AlphaAssociation::Straight);
}

#[test]
fn ycbcr_supports_420_422_and_444_native_planes() {
    for subsampling in [
        [
            Subsampling::FULL,
            Subsampling::new(2, 2).unwrap(),
            Subsampling::new(2, 2).unwrap(),
        ],
        [
            Subsampling::FULL,
            Subsampling::new(2, 1).unwrap(),
            Subsampling::new(2, 1).unwrap(),
        ],
        [Subsampling::FULL, Subsampling::FULL, Subsampling::FULL],
    ] {
        let descriptor = ImageDescriptor::ycbcr(5, 3, 12, subsampling).unwrap();
        let planes = descriptor
            .planes()
            .iter()
            .map(|plane| {
                Plane::new(
                    plane.layout().clone(),
                    vec![0u16; plane.layout().addressed_samples()],
                )
            })
            .collect::<Result<Vec<_>>>()
            .unwrap();
        let frame = ImageFrame::new(descriptor, PixelBuffer::u16(planes).unwrap()).unwrap();
        assert_eq!(frame.descriptor().model(), ChannelModel::YCbCr);
    }
}

#[test]
fn f32_rejects_non_finite_and_alpha_outside_normalized_range() {
    let descriptor = ImageDescriptor::gray(1, 1, 32).unwrap();
    let plane = Plane::new(descriptor.planes()[0].layout().clone(), vec![f32::NAN]).unwrap();
    assert!(matches!(
        ImageFrame::new(descriptor, PixelBuffer::f32(vec![plane]).unwrap()),
        Err(HighresError::InvalidSamples(_))
    ));

    let layout = PlaneLayout::interleaved(1, 1, 4, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        1,
        1,
        ChannelModel::RGB,
        vec![
            PlaneDescriptor::new(
                layout.clone(),
                vec![
                    ChannelRole::Red,
                    ChannelRole::Green,
                    ChannelRole::Blue,
                    ChannelRole::Alpha,
                ],
                32,
            )
            .unwrap(),
        ],
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap();
    let plane = Plane::new(layout.clone(), vec![-1.0, 2.0, 3.0, 0.5]).unwrap();
    ImageFrame::new(descriptor.clone(), PixelBuffer::f32(vec![plane]).unwrap()).unwrap();
    let plane = Plane::new(layout, vec![-1.0, 2.0, 3.0, 2.0]).unwrap();
    assert!(ImageFrame::new(descriptor, PixelBuffer::f32(vec![plane]).unwrap()).is_err());
}

#[test]
fn metadata_defaults_and_crop_bounds_are_checked_after_mutation() {
    assert_eq!(PixelAspectRatio::default().horizontal(), 1);
    assert_eq!(PixelAspectRatio::default().vertical(), 1);
    let descriptor = ImageDescriptor::gray(4, 4, 8).unwrap();
    let layout = descriptor.planes()[0].layout().clone();
    let plane = Plane::new(layout, vec![0u8; 16]).unwrap();
    let mut frame = ImageFrame::new(descriptor, PixelBuffer::u8(vec![plane]).unwrap()).unwrap();
    let mut metadata = FrameMetadata::default();
    metadata.set_crop(Some(Rect::new(3, 3, 2, 1).unwrap()));
    frame = frame.with_metadata(metadata);
    assert!(matches!(
        frame.validate(),
        Err(HighresError::InvalidMetadata(_))
    ));
}

#[test]
fn color_signalling_retains_icc_nclx_and_av1_together() {
    let colors = ColorInformationSet::new()
        .with_icc_profile(vec![1, 2, 3])
        .unwrap()
        .with_nclx(NclxColorInformation::new(9, 16, 9, true))
        .with_av1(Av1ColorInformation::new(9, 18, 9, true));
    assert_eq!(colors.icc_profile(), Some(&[1, 2, 3][..]));
    assert_eq!(colors.nclx().unwrap().transfer(), 16);
    assert_eq!(colors.av1().unwrap().transfer(), 18);
    assert_eq!(colors.provenance().len(), 3);
}
