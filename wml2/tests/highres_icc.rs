use wml2::highres::icc::transform_frame;
use wml2::highres::{
    AlphaAssociation, ChannelModel, ChannelRole, ColorInformationSet, ImageDescriptor, ImageFrame,
    PixelBuffer, Plane, PlaneDescriptor, PlaneLayout, Result, Subsampling,
};

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn put_i32(data: &mut [u8], offset: usize, value: i32) {
    put_u32(data, offset, value as u32);
}

fn xyz(x: f32, y: f32, z: f32) -> Vec<u8> {
    let mut tag = vec![0u8; 20];
    put_u32(&mut tag, 0, u32::from_be_bytes(*b"XYZ "));
    put_i32(&mut tag, 8, (x * 65536.0).round() as i32);
    put_i32(&mut tag, 12, (y * 65536.0).round() as i32);
    put_i32(&mut tag, 16, (z * 65536.0).round() as i32);
    tag
}

fn identity_curve() -> Vec<u8> {
    let mut tag = vec![0u8; 12];
    put_u32(&mut tag, 0, u32::from_be_bytes(*b"curv"));
    tag
}

fn identity_rgb_profile() -> Vec<u8> {
    let mut profile = vec![0u8; 132 + 6 * 12];
    profile[8..12].copy_from_slice(&[4, 0, 0, 0]);
    profile[12..16].copy_from_slice(b"mntr");
    put_u32(&mut profile, 16, u32::from_be_bytes(*b"RGB "));
    put_u32(&mut profile, 20, u32::from_be_bytes(*b"XYZ "));
    put_u32(&mut profile, 36, u32::from_be_bytes(*b"acsp"));
    put_u32(&mut profile, 64, 1);
    put_u32(&mut profile, 128, 6);
    let tags: [([u8; 4], Vec<u8>); 6] = [
        (*b"rXYZ", xyz(1.0, 0.0, 0.0)),
        (*b"gXYZ", xyz(0.0, 1.0, 0.0)),
        (*b"bXYZ", xyz(0.0, 0.0, 1.0)),
        (*b"rTRC", identity_curve()),
        (*b"gTRC", identity_curve()),
        (*b"bTRC", identity_curve()),
    ];
    let mut offset = profile.len();
    for (index, (signature, tag)) in tags.into_iter().enumerate() {
        let entry = 132 + index * 12;
        profile.resize(offset + tag.len(), 0);
        profile[offset..offset + tag.len()].copy_from_slice(&tag);
        put_u32(&mut profile, entry, u32::from_be_bytes(signature));
        put_u32(&mut profile, entry + 4, offset as u32);
        put_u32(&mut profile, entry + 8, tag.len() as u32);
        offset = (offset + tag.len() + 3) & !3;
        profile.resize(offset, 0);
    }
    let profile_length = profile.len() as u32;
    put_u32(&mut profile, 0, profile_length);
    profile
}

#[test]
fn explicit_rgb_u8_transform_keeps_source_and_destination_signalling_separate() {
    let descriptor = ImageDescriptor::rgb(2, 1, 8).unwrap();
    let planes = descriptor
        .planes()
        .iter()
        .enumerate()
        .map(|(index, plane)| {
            Plane::new(
                plane.layout().clone(),
                vec![index as u8 * 32, 255 - index as u8 * 32],
            )
        })
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let frame = ImageFrame::new(descriptor, PixelBuffer::u8(planes).unwrap()).unwrap();
    let profile = identity_rgb_profile();
    let output = transform_frame(&frame, &profile, &profile).unwrap();

    assert_eq!(
        output.metadata().source_color(),
        &ColorInformationSet::default()
    );
    assert_eq!(
        output.descriptor().color_information().icc_profile(),
        Some(&profile[..])
    );
    assert!(matches!(output.pixels(), PixelBuffer::U8(_)));
}

#[test]
fn explicit_rgb_u16_transform_does_not_send_alpha_to_the_cms() {
    let layout = PlaneLayout::interleaved(2, 1, 4, Subsampling::FULL).unwrap();
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
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![
            Plane::new(layout, vec![1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let profile = identity_rgb_profile();
    let output = transform_frame(&frame, &profile, &profile).unwrap();
    assert_eq!(output.descriptor().alpha(), AlphaAssociation::Straight);
    let samples = output.pixels().u16_planes().unwrap()[0].samples();
    assert_eq!([samples[3], samples[7]], [4000, 8000]);
}

fn gamma_two_profile() -> Vec<u8> {
    let mut profile = identity_rgb_profile();
    for index in 3..6 {
        let offset = profile.len();
        let mut tag = vec![0; 16];
        tag[..4].copy_from_slice(b"curv");
        put_u32(&mut tag, 8, 1);
        tag[12..14].copy_from_slice(&512u16.to_be_bytes());
        profile.extend_from_slice(&tag);
        put_u32(&mut profile, 132 + index * 12 + 4, offset as u32);
        put_u32(&mut profile, 132 + index * 12 + 8, 16);
    }
    let length = profile.len() as u32;
    put_u32(&mut profile, 0, length);
    profile
}

#[test]
fn nonidentity_icc_respects_precision_alpha_timing_and_reuse() {
    let source = gamma_two_profile();
    let destination = identity_rgb_profile();
    let transform =
        wml2::highres::icc::FrameTransform::new(&source, &destination, Default::default()).unwrap();
    for bits in [8, 10, 12, 16] {
        let max = (1u32 << bits) - 1;
        let layout = PlaneLayout::interleaved(513, 2, 4, Subsampling::FULL).unwrap();
        let descriptor = ImageDescriptor::new(
            513,
            2,
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
                    bits,
                )
                .unwrap(),
            ],
        )
        .unwrap()
        .with_alpha(AlphaAssociation::Straight)
        .unwrap();
        let samples: Vec<u16> = (0..1026)
            .flat_map(|i| {
                let v = match i % 3 {
                    0 => 0,
                    1 => max / 2,
                    _ => max,
                };
                [v as u16, v as u16, v as u16, (i as u32 % max) as u16]
            })
            .collect();
        let pixels = if bits == 8 {
            PixelBuffer::u8(vec![
                Plane::new(layout, samples.iter().map(|v| *v as u8).collect()).unwrap(),
            ])
            .unwrap()
        } else {
            PixelBuffer::u16(vec![Plane::new(layout, samples.clone()).unwrap()]).unwrap()
        };
        let timing = wml2::highres::FrameTiming::new(1000, 123, 45).unwrap();
        let frame = ImageFrame::new(descriptor, pixels)
            .unwrap()
            .with_timing(timing);
        let output = transform.transform(&frame).unwrap();
        assert_eq!(output.timing(), Some(timing));
        let values: Vec<u16> = match output.pixels() {
            PixelBuffer::U8(p) => p.as_slice()[0]
                .samples()
                .iter()
                .map(|v| *v as u16)
                .collect(),
            PixelBuffer::U16(p) => p.as_slice()[0].samples().to_vec(),
            _ => unreachable!(),
        };
        for (input, out) in samples.chunks_exact(4).zip(values.chunks_exact(4)) {
            let expected = ((input[0] as f64 / max as f64).powi(2) * max as f64).round() as i32;
            for &v in &out[..3] {
                assert!(
                    (i32::from(v) - expected).abs() <= 2,
                    "bits={bits}: {v} != {expected}"
                );
            }
            assert_eq!(input[3], out[3]);
        }
        assert_eq!(output.descriptor().planes()[0].meaningful_bits(), bits);
    }
}

#[test]
fn nonidentity_icc_rejects_premultiplied_alpha() {
    for alpha in [0u16, 512, 1023] {
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
                    10,
                )
                .unwrap(),
            ],
        )
        .unwrap()
        .with_alpha(AlphaAssociation::Premultiplied)
        .unwrap();
        let frame = ImageFrame::new(
            descriptor,
            PixelBuffer::u16(vec![Plane::new(layout, vec![alpha; 4]).unwrap()]).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            transform_frame(&frame, &gamma_two_profile(), &identity_rgb_profile()),
            Err(wml2::highres::icc::Error::Frame(
                wml2::highres::HighresError::Unsupported(_)
            ))
        ));
    }
}

#[test]
fn nonidentity_rgb_matches_littlecms_reference_at_each_integer_depth() {
    let reference: Vec<Vec<u16>> = include_str!("fixtures/review/lcms-gamma-rgb.csv")
        .lines()
        .skip(1)
        .map(|line| {
            line.split(',')
                .map(|value| value.parse().unwrap())
                .collect()
        })
        .collect();
    let transform = wml2::highres::icc::FrameTransform::new(
        include_bytes!("fixtures/review/gamma2-rgb.icc"),
        include_bytes!("fixtures/review/linear-rgb.icc"),
        Default::default(),
    )
    .unwrap();
    for bits in [8, 10, 12, 16] {
        let maximum = (1u32 << bits) - 1;
        let descriptor = ImageDescriptor::rgb(reference.len() as u32, 1, bits).unwrap();
        let planes = descriptor
            .planes()
            .iter()
            .enumerate()
            .map(|(channel, plane)| {
                Plane::new(
                    plane.layout().clone(),
                    reference
                        .iter()
                        .map(|row| ((u32::from(row[channel]) * maximum + 127) / 255) as u16)
                        .collect(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let pixels = if bits == 8 {
            PixelBuffer::u8(
                planes
                    .iter()
                    .map(|plane| {
                        Plane::new(
                            plane.layout().clone(),
                            plane.samples().iter().map(|value| *value as u8).collect(),
                        )
                        .unwrap()
                    })
                    .collect(),
            )
            .unwrap()
        } else {
            PixelBuffer::u16(planes).unwrap()
        };
        let output = transform
            .transform(&ImageFrame::new(descriptor, pixels).unwrap())
            .unwrap();
        for channel in 0..3 {
            for (x, row) in reference.iter().enumerate() {
                let sample = match output.pixels() {
                    PixelBuffer::U8(planes) => u32::from(planes.as_slice()[channel].samples()[x]),
                    PixelBuffer::U16(planes) => u32::from(planes.as_slice()[channel].samples()[x]),
                    _ => unreachable!(),
                };
                let reduced = (sample * 255 + maximum / 2) / maximum;
                // LittleCMS's RGB8 output quantization and input rescaling to
                // 10/12 bits can differ by one 8-bit code value.
                assert!(
                    reduced.abs_diff(u32::from(row[channel + 3])) <= 1,
                    "bits={bits}, pixel={x}, channel={channel}: {reduced} vs {}",
                    row[channel + 3]
                );
            }
        }
    }
}

#[test]
fn padded_f32_matches_cms_without_adapter_clipping() {
    let layout = PlaneLayout::new(3, 2, 20, 6, vec![0, 1, 2, 3], Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        3,
        2,
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
    let mut samples = vec![-3.0f32; 36];
    for y in 0..2 {
        for x in 0..3 {
            let i = y * 20 + x * 6;
            samples[i..i + 4].copy_from_slice(&[-0.1, 0.5, 1.5, 0.25]);
        }
    }
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::f32(vec![Plane::new(layout, samples.clone()).unwrap()]).unwrap(),
    )
    .unwrap();
    let profile = identity_rgb_profile();
    let cms_profile = icc_profile::Profile::from_bytes(&profile).unwrap();
    let cms = icc_profile::Transform::new(&cms_profile, &cms_profile, Default::default()).unwrap();
    let mut expected = [0.0; 3];
    cms.transform_f32(&[-0.1, 0.5, 1.5], &mut expected).unwrap();
    let output = transform_frame(&frame, &profile, &profile).unwrap();
    let actual = output.pixels().f32_planes().unwrap()[0].samples();
    for y in 0..2 {
        for x in 0..3 {
            let i = y * 20 + x * 6;
            samples[i..i + 3].copy_from_slice(&expected);
        }
    }
    assert_eq!(actual, samples);
    assert_eq!(output.timing(), None);
}

#[test]
#[ignore = "manual performance comparison"]
fn benchmark_reusable_icc_transform() {
    use std::time::Instant;
    let descriptor = ImageDescriptor::rgb(1, 1, 8).unwrap();
    let planes = descriptor
        .planes()
        .iter()
        .map(|p| Plane::new(p.layout().clone(), vec![128u8]).unwrap())
        .collect();
    let frame = ImageFrame::new(descriptor, PixelBuffer::u8(planes).unwrap()).unwrap();
    let source = gamma_two_profile();
    let destination = identity_rgb_profile();
    let transform =
        wml2::highres::icc::FrameTransform::new(&source, &destination, Default::default()).unwrap();
    for run in 0..8 {
        for reuse in if run % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let now = Instant::now();
            for _ in 0..1000 {
                let result = if reuse {
                    transform.transform(&frame)
                } else {
                    transform_frame(&frame, &source, &destination)
                }
                .unwrap();
                std::hint::black_box(result);
            }
            println!("reuse={reuse},run={run},ns={}", now.elapsed().as_nanos());
        }
    }
}
