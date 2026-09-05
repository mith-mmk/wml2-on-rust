use super::*;
use crate::highres::{ImageDescriptor, Plane, PlaneDescriptor, Subsampling};

#[test]
#[ignore = "manual native-plane extraction benchmark"]
fn role_extraction_benchmark() {
    for (name, bits, stride) in [
        ("u8-planar", 8, 1),
        ("u16-planar", 10, 1),
        ("u16-interleaved", 10, 3),
        ("u16-padded", 10, 5),
    ] {
        let width = 257;
        let height = 127;
        let descriptor = if stride == 1 {
            ImageDescriptor::rgb(width, height, bits).unwrap()
        } else {
            let layout = crate::highres::PlaneLayout::new(
                width,
                height,
                width as usize * stride + 7,
                stride,
                vec![0, 1, 2],
                Subsampling::FULL,
            )
            .unwrap();
            ImageDescriptor::new(
                width,
                height,
                ChannelModel::RGB,
                vec![
                    PlaneDescriptor::new(
                        layout,
                        vec![ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue],
                        bits,
                    )
                    .unwrap(),
                ],
            )
            .unwrap()
        };
        let pixels = if bits == 8 {
            PixelBuffer::u8(
                descriptor
                    .planes()
                    .iter()
                    .map(|p| {
                        Plane::new(
                            p.layout().clone(),
                            vec![127u8; p.layout().row_stride() * height as usize],
                        )
                        .unwrap()
                    })
                    .collect(),
            )
            .unwrap()
        } else {
            PixelBuffer::u16(
                descriptor
                    .planes()
                    .iter()
                    .map(|p| {
                        Plane::new(
                            p.layout().clone(),
                            vec![511u16; p.layout().row_stride() * height as usize],
                        )
                        .unwrap()
                    })
                    .collect(),
            )
            .unwrap()
        };
        let frame = ImageFrame::new(descriptor, pixels).unwrap();
        for run in 0..8 {
            let start = std::time::Instant::now();
            for _ in 0..100 {
                std::hint::black_box(
                    encode_role(&frame, ChannelRole::Red, bits, Quantization::None).unwrap(),
                );
            }
            let elapsed = start.elapsed().as_nanos();
            let output = encode_role(&frame, ChannelRole::Red, bits, Quantization::None).unwrap();
            assert!(
                output
                    .samples
                    .iter()
                    .all(|v| *v == if bits == 8 { 127 } else { 511 })
            );
            println!("{name},{run},{elapsed},{}", output.samples.len());
        }
    }
}
