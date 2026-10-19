#![cfg(feature = "color-management")]
use wml2::highres::icc::FrameTransform;
use wml2::highres::*;
use wml2::metadata::DataMap;
const RGB: &[u8] = include_bytes!("fixtures/review/avif-icc-linear.icc");
const GRAY: &[u8] = include_bytes!("fixtures/review/avif-icc-gray-linear.icc");
#[derive(Debug)]
struct Case {
    input: [f32; 3],
    output: [f32; 3],
}
fn cases(route: &str, bits: u8) -> Vec<Case> {
    include_str!("fixtures/review/lcms-cross-model.csv")
        .lines()
        .skip(1)
        .filter_map(|line| {
            let f: Vec<_> = line.split(',').collect();
            if f[0] != route || f[1].parse::<u8>().unwrap() != bits {
                return None;
            }
            let v: Vec<f32> = f[2..].iter().map(|x| x.parse().unwrap()).collect();
            Some(Case {
                input: v[..3].try_into().unwrap(),
                output: v[3..].try_into().unwrap(),
            })
        })
        .collect()
}
fn frame(route: &str, bits: u8) -> ImageFrame {
    let (model, roles, offsets, stride, source) = if route == "gray-rgb" {
        (
            ChannelModel::Gray,
            vec![ChannelRole::Alpha, ChannelRole::Gray],
            vec![0, 2],
            3,
            GRAY,
        )
    } else {
        (
            ChannelModel::RGB,
            vec![
                ChannelRole::Blue,
                ChannelRole::Alpha,
                ChannelRole::Red,
                ChannelRole::Green,
            ],
            vec![0, 1, 3, 5],
            6,
            RGB,
        )
    };
    let (width, height) = (513, 2);
    let row = width as usize * stride + 7;
    let layout = PlaneLayout::new(width, height, row, stride, offsets, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        width,
        height,
        model,
        vec![PlaneDescriptor::new(layout.clone(), roles.clone(), bits).unwrap()],
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap()
    .with_color_information(
        ColorInformationSet::new()
            .with_icc_profile(source.to_vec())
            .unwrap(),
    );
    let refs = cases(route, bits);
    let maximum = if bits == 32 {
        1.0
    } else {
        ((1u32 << bits) - 1) as f32
    };
    let mut values = vec![-10.0; row * height as usize + 11];
    for i in 0..width as usize * height as usize {
        let reference = &refs[i % refs.len()];
        let alpha = if bits == 32 {
            (i % 17) as f32 / 16.0
        } else {
            (i as f32 * 11.0) % (maximum + 1.0)
        };
        let at = (i / width as usize) * row + (i % width as usize) * stride;
        for (role, offset) in roles.iter().zip(layout.channel_offsets()) {
            values[at + offset] = match role {
                ChannelRole::Gray | ChannelRole::Red => reference.input[0],
                ChannelRole::Green => reference.input[1],
                ChannelRole::Blue => reference.input[2],
                ChannelRole::Alpha => alpha,
                _ => unreachable!(),
            };
        }
    }
    let pixels = if bits == 8 {
        PixelBuffer::u8(vec![
            Plane::new(
                layout,
                values
                    .iter()
                    .map(|&v| if v < 0.0 { 255 } else { v as u8 })
                    .collect(),
            )
            .unwrap(),
        ])
        .unwrap()
    } else if bits == 32 {
        PixelBuffer::f32(vec![Plane::new(layout, values).unwrap()]).unwrap()
    } else {
        PixelBuffer::u16(vec![
            Plane::new(
                layout,
                values
                    .iter()
                    .map(|&v| if v < 0.0 { 65535 } else { v as u16 })
                    .collect(),
            )
            .unwrap(),
        ])
        .unwrap()
    };
    let mut frame = ImageFrame::new(descriptor, pixels)
        .unwrap()
        .with_timing(FrameTiming::new(1000, 123, 45).unwrap());
    frame
        .metadata_mut()
        .tags_mut()
        .insert("ICC Profile".into(), DataMap::ICCProfile(source.to_vec()));
    frame.metadata_mut().tags_mut().insert(
        "Source ICC Profile".into(),
        DataMap::ICCProfile(source.to_vec()),
    );
    frame
        .metadata_mut()
        .tags_mut()
        .insert("Caller provenance".into(), DataMap::UInt(17));
    frame
}
fn samples(frame: &ImageFrame, plane: usize) -> Vec<f32> {
    match frame.pixels() {
        PixelBuffer::U8(p) => p.as_slice()[plane]
            .samples()
            .iter()
            .map(|&x| f32::from(x))
            .collect(),
        PixelBuffer::U16(p) => p.as_slice()[plane]
            .samples()
            .iter()
            .map(|&x| f32::from(x))
            .collect(),
        PixelBuffer::F32(p) => p.as_slice()[plane].samples().to_vec(),
    }
}
#[test]
fn separate_layouts_match_independent_cms_for_all_storage_types_and_alpha() {
    for route in ["gray-rgb", "rgb-gray"] {
        let (source, destination, model, channels) = if route == "gray-rgb" {
            (GRAY, RGB, ChannelModel::RGB, 3)
        } else {
            (RGB, GRAY, ChannelModel::Gray, 1)
        };
        let transform = FrameTransform::new(source, destination, Default::default()).unwrap();
        for bits in [8, 10, 12, 16, 32] {
            let input = frame(route, bits);
            let before = input.clone();
            let refs = cases(route, bits);
            let bytes = 513
                * 2
                * (channels + 1)
                * if bits == 8 {
                    1
                } else if bits == 32 {
                    4
                } else {
                    2
                };
            assert!(
                transform
                    .transform_with_output_limit(&input, bytes - 1)
                    .is_err()
            );
            let out = transform
                .transform_with_output_limit(&input, bytes)
                .unwrap();
            out.validate().unwrap();
            assert_eq!(input, before);
            assert_eq!(out.descriptor().model(), model);
            assert_eq!(out.descriptor().alpha(), AlphaAssociation::Straight);
            assert_eq!(out.timing(), input.timing());
            assert_eq!(out.pixels().format(), input.pixels().format());
            assert_eq!(
                out.metadata().source_color(),
                input.metadata().source_color()
            );
            assert_eq!(
                out.metadata().tags().get("Source ICC Profile"),
                input.metadata().tags().get("Source ICC Profile")
            );
            assert!(
                matches!(out.metadata().tags().get("ICC Profile"),Some(DataMap::ICCProfile(p)) if p==destination)
            );
            assert_eq!(
                out.descriptor().color_information().icc_profile(),
                Some(destination)
            );
            assert_eq!(
                out.metadata().tags().get("Caller provenance"),
                Some(&DataMap::UInt(17))
            );
            for c in 0..channels {
                let values = samples(&out, c);
                assert_eq!(values.len(), 1026);
                for (i, v) in values.iter().enumerate() {
                    let expected = refs[i % refs.len()].output[c];
                    assert!(
                        (v - expected).abs() <= if bits == 32 { 0.00005 } else { 2.0 },
                        "{route} bits={bits} c={c} i={i}: {v} vs {expected}"
                    );
                }
                let desc = &out.descriptor().planes()[c];
                assert_eq!(desc.layout().row_stride(), 513);
                assert_eq!(desc.layout().pixel_stride(), 1);
                assert_eq!(desc.meaningful_bits(), bits);
            }
            let alpha = samples(&out, channels);
            let source = samples(&input, 0);
            let desc = &input.descriptor().planes()[0];
            let role = desc
                .roles()
                .iter()
                .position(|r| *r == ChannelRole::Alpha)
                .unwrap();
            let offset = desc.layout().channel_offsets()[role];
            for (i, v) in alpha.iter().enumerate() {
                assert_eq!(
                    *v,
                    source[(i / 513) * desc.layout().row_stride()
                        + (i % 513) * desc.layout().pixel_stride()
                        + offset]
                );
            }
        }
    }
}
#[test]
fn mixed_precision_alpha_and_same_model_output_budget_remain_explicit() {
    let layout = PlaneLayout::planar(2, 1, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        2,
        1,
        ChannelModel::Gray,
        vec![
            PlaneDescriptor::planar(layout.clone(), ChannelRole::Gray, 12).unwrap(),
            PlaneDescriptor::planar(layout.clone(), ChannelRole::Alpha, 4).unwrap(),
        ],
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap();
    let input = ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![
            Plane::new(layout.clone(), vec![123, 3999]).unwrap(),
            Plane::new(layout, vec![0, 15]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let out = FrameTransform::new(GRAY, RGB, Default::default())
        .unwrap()
        .transform(&input)
        .unwrap();
    assert_eq!(out.descriptor().planes()[3].meaningful_bits(), 4);
    assert_eq!(out.pixels().u16_planes().unwrap()[3].samples(), [0, 15]);
    assert!(
        out.descriptor().planes()[..3]
            .iter()
            .all(|d| d.meaningful_bits() == 12)
    );
    let same = FrameTransform::new(GRAY, GRAY, Default::default()).unwrap();
    assert!(same.transform_with_output_limit(&input, 7).is_err());
    assert_eq!(
        same.transform_with_output_limit(&input, 8)
            .unwrap()
            .pixels(),
        input.pixels()
    );
    assert!(
        FrameTransform::new(RGB, GRAY, Default::default())
            .unwrap()
            .transform(&input)
            .is_err()
    );
    let layout = PlaneLayout::planar(2, 1, Subsampling::FULL).unwrap();
    let roles = [
        ChannelRole::Red,
        ChannelRole::Green,
        ChannelRole::Blue,
        ChannelRole::Alpha,
    ];
    let descriptor = ImageDescriptor::new(
        2,
        1,
        ChannelModel::RGB,
        roles
            .into_iter()
            .zip([8, 10, 12, 4])
            .map(|(role, bits)| PlaneDescriptor::planar(layout.clone(), role, bits).unwrap())
            .collect(),
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap();
    let source = ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![
            Plane::new(layout.clone(), vec![255, 0]).unwrap(),
            Plane::new(layout.clone(), vec![0, 1023]).unwrap(),
            Plane::new(layout.clone(), vec![0, 0]).unwrap(),
            Plane::new(layout, vec![7, 3]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let gray = FrameTransform::new(RGB, GRAY, Default::default())
        .unwrap()
        .transform(&source)
        .unwrap();
    assert_eq!(gray.descriptor().planes()[0].meaningful_bits(), 12);
    assert_eq!(gray.descriptor().planes()[1].meaningful_bits(), 4);
    let refs = cases("rgb-gray", 12);
    let values = samples(&gray, 0);
    assert!((values[0] - refs[1].output[0]).abs() <= 2.0);
    assert!((values[1] - refs[2].output[0]).abs() <= 2.0);
    assert_eq!(gray.pixels().u16_planes().unwrap()[1].samples(), [7, 3]);
}

#[test]
fn model_changes_require_unassociated_full_range_full_resolution_coordinates() {
    let transform = FrameTransform::new(GRAY, RGB, Default::default()).unwrap();
    let input = frame("gray-rgb", 12);
    let premult = ImageFrame::new(
        input
            .descriptor()
            .clone()
            .with_alpha(AlphaAssociation::Premultiplied)
            .unwrap(),
        input.pixels().clone(),
    )
    .unwrap();
    assert!(transform.transform(&premult).is_err());
    let limited = ImageFrame::new(
        input.descriptor().clone().with_color_information(
            ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 0, false)),
        ),
        input.pixels().clone(),
    )
    .unwrap();
    assert!(transform.transform(&limited).is_err());
    let layout = PlaneLayout::planar(257, 2, Subsampling::new(2, 1).unwrap()).unwrap();
    let descriptor = ImageDescriptor::new(
        513,
        2,
        ChannelModel::Gray,
        vec![PlaneDescriptor::planar(layout.clone(), ChannelRole::Gray, 12).unwrap()],
    )
    .unwrap();
    let subsampled = ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![Plane::new(layout, vec![123; 514]).unwrap()]).unwrap(),
    )
    .unwrap();
    assert!(transform.transform(&subsampled).is_err());
}

#[test]
fn three_arbitrary_colorants_do_not_acquire_an_rgb_sample_declaration() {
    let mut profile = include_bytes!("fixtures/review/lab-pcs-rgb.icc").to_vec();
    profile[16..20].copy_from_slice(b"3CLR");
    assert_eq!(
        icc_profile::Profile::parse(&profile).unwrap().color_space(),
        icc_profile::ColorSpace::NColor(3)
    );
    assert!(FrameTransform::new(&profile, RGB, Default::default()).is_err());
    assert!(FrameTransform::new(RGB, &profile, Default::default()).is_err());
}
