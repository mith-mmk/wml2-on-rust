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
fn pq_is_absolute_nits_and_hlg_is_scene_relative() {
    assert!((pq_eotf(1.0).unwrap() - 10_000.0).abs() < 0.1);
    let one_nit = pq_oetf(1.0).unwrap();
    assert!((pq_eotf(one_nit).unwrap() - 1.0).abs() < 0.01);
    assert!((hlg_scene_from_signal(0.5).unwrap() - 1.0 / 12.0).abs() < 1e-6);
    assert!((hlg_oetf(1.0 / 12.0).unwrap() - 0.5).abs() < 1e-6);
    assert!(pq_eotf(f32::NAN).is_err());
    assert!(HlgDisplayConditions::new(1000.0, 0.1, 1.2).is_ok());
    assert!(HlgDisplayConditions::new(f32::INFINITY, 0.0, 1.0).is_err());
    assert!(HlgDisplayConditions::new(1000.0, 1000.0, 1.2).is_err());
    assert!(hlg_oetf(f32::MAX).is_err());
}

#[test]
fn transfer_functions_match_independent_f64_reference_on_10_and_12_bit_grids() {
    fn pq_ref(signal: f64) -> f64 {
        let m1 = 2610.0 / 16384.0;
        let m2 = 2523.0 / 32.0;
        let c1 = 3424.0 / 4096.0;
        let c2 = 2413.0 / 128.0;
        let c3 = 2392.0 / 128.0;
        let v = signal.powf(1.0 / m2);
        10000.0 * ((v - c1).max(0.0) / (c2 - c3 * v)).powf(1.0 / m1)
    }
    fn hlg_ref(scene: f64) -> f64 {
        let a = 0.17883277_f64;
        let b = 1.0 - 4.0 * a;
        let c = 0.5 - a * (4.0 * a).ln();
        if scene <= 1.0 / 12.0 {
            (3.0 * scene).sqrt()
        } else {
            a * (12.0 * scene - b).ln() + c
        }
    }
    for bits in [10u32, 12] {
        let max = (1u32 << bits) - 1;
        for raw in 0..=max {
            let signal = raw as f64 / max as f64;
            let pq_error = (pq_eotf(signal as f32).unwrap() as f64 - pq_ref(signal)).abs();
            assert!(
                pq_error <= 0.2,
                "PQ {bits}-bit sample {raw} error {pq_error}"
            );
            let scene = raw as f64 / max as f64;
            let hlg_error = (hlg_oetf(scene as f32).unwrap() as f64 - hlg_ref(scene)).abs();
            assert!(
                hlg_error <= 2.0e-6,
                "HLG {bits}-bit sample {raw} error {hlg_error}"
            );
            let pq_roundtrip = pq_oetf(pq_eotf(signal as f32).unwrap()).unwrap() as f64;
            assert!(
                (pq_roundtrip - signal).abs() <= 1.0 / max as f64,
                "PQ {bits}-bit sample {raw} roundtrip {pq_roundtrip}"
            );
            let hlg_roundtrip =
                hlg_oetf(hlg_scene_from_signal(signal as f32).unwrap()).unwrap() as f64;
            assert!(
                (hlg_roundtrip - signal).abs() <= 1.0 / max as f64,
                "HLG {bits}-bit sample {raw} roundtrip {hlg_roundtrip}"
            );
        }
    }
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
