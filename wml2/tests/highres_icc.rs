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
