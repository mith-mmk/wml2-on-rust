use wml2::highres::avif::{NativeDecodeLimits, decode_native};
use wml2::highres::avif_encode::{
    EncodeError, NativeEncodeOptions, NativeSubsampling, Quantization, encode_native,
    encode_native_with_quantization,
};
use wml2::highres::{
    AlphaAssociation, ChannelRole, ImageDescriptor, ImageFrame, PixelBuffer, Plane,
    PlaneDescriptor, PlaneLayout, Subsampling,
};

fn limits() -> NativeDecodeLimits {
    NativeDecodeLimits::new(
        64 * 1024 * 1024,
        16 * 1024,
        16 * 1024,
        16 * 1024 * 1024,
        64 * 1024 * 1024,
        8 * 1024 * 1024,
        8 * 1024 * 1024,
        128,
        512,
        128,
        16,
        128,
    )
}

fn options(bit_depth: u8) -> NativeEncodeOptions {
    NativeEncodeOptions {
        bit_depth,
        subsampling: NativeSubsampling::Cs444,
        encoder: avifenc_codec::EncoderOptions {
            color_information: None,
            ..avifenc_codec::EncoderOptions::default()
        },
        color_information: avifenc_codec::NativeColorInformation::from_nclx(
            avifenc_codec::NclxColorInformation {
                color_primaries: 1,
                transfer_characteristics: 13,
                matrix_coefficients: 0,
                full_range_flag: true,
            },
        ),
    }
}

fn rgb_frame_u8() -> ImageFrame {
    let descriptor = ImageDescriptor::rgb(2, 2, 8).unwrap();
    let planes = (0..3)
        .map(|channel| {
            Plane::new(
                descriptor.planes()[channel].layout().clone(),
                vec![channel as u8 * 32, 64, 128, 255 - channel as u8 * 32],
            )
            .unwrap()
        })
        .collect();
    ImageFrame::new(descriptor, PixelBuffer::u8(planes).unwrap()).unwrap()
}

fn rgb_frame_u16(meaningful_bits: u8) -> ImageFrame {
    let descriptor = ImageDescriptor::rgb(2, 2, meaningful_bits).unwrap();
    let maximum = if meaningful_bits == 16 {
        u16::MAX
    } else {
        (1u16 << meaningful_bits) - 1
    };
    let planes = (0..3)
        .map(|channel| {
            Plane::new(
                descriptor.planes()[channel].layout().clone(),
                vec![channel as u16 * 100, 257, maximum / 2, maximum],
            )
            .unwrap()
        })
        .collect();
    ImageFrame::new(descriptor, PixelBuffer::u16(planes).unwrap()).unwrap()
}

#[test]
fn explicit_8_10_12_bit_outputs_use_the_requested_depth() {
    for (bit_depth, frame) in [
        (8, rgb_frame_u8()),
        (10, rgb_frame_u16(10)),
        (12, rgb_frame_u16(12)),
    ] {
        let encoded = encode_native(&frame, &options(bit_depth)).expect("AVIF encode");
        let decoded = decode_native(&encoded, &limits()).expect("native AVIF decode");
        assert_eq!(
            decoded.descriptor().planes()[0].meaningful_bits(),
            bit_depth
        );
    }
}

#[test]
fn implicit_precision_expansion_is_rejected() {
    for (frame, target) in [
        (rgb_frame_u8(), 10),
        (rgb_frame_u8(), 12),
        (rgb_frame_u16(10), 12),
    ] {
        assert!(matches!(
            encode_native(&frame, &options(target)),
            Err(EncodeError::Frame(
                wml2::highres::HighresError::Unsupported(_)
            ))
        ));
        assert!(
            encode_native_with_quantization(&frame, &options(target), Quantization::RightShift)
                .is_err()
        );
    }
}

#[test]
fn sixteen_bit_downshift_requires_explicit_quantization() {
    let frame = rgb_frame_u16(16);
    let native = options(12);
    assert!(matches!(
        encode_native(&frame, &native),
        Err(EncodeError::Frame(
            wml2::highres::HighresError::Unsupported(_)
        ))
    ));
    let encoded = encode_native_with_quantization(&frame, &native, Quantization::RightShift)
        .expect("explicit integer quantization should enable 16-to-12-bit encoding");
    let decoded = decode_native(&encoded, &limits()).expect("native AVIF decode");
    assert_eq!(decoded.descriptor().planes()[0].meaningful_bits(), 12);
}

#[test]
fn interleaved_straight_alpha_is_encoded_as_an_independent_plane() {
    let layout = PlaneLayout::interleaved(2, 2, 4, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        2,
        2,
        wml2::highres::ChannelModel::RGB,
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
    .with_alpha(AlphaAssociation::Straight)
    .unwrap();
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![
            Plane::new(
                layout,
                vec![
                    100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 101, 202, 303, 404, 505, 606,
                ],
            )
            .unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let encoded = encode_native(&frame, &options(10)).expect("AVIF encode");
    let decoded = decode_native(&encoded, &limits()).expect("native AVIF decode");
    assert_eq!(decoded.descriptor().alpha(), AlphaAssociation::Straight);
    assert!(
        decoded
            .descriptor()
            .planes()
            .iter()
            .any(|plane| plane.roles() == [ChannelRole::Alpha].as_slice())
    );
}

#[test]
fn explicit_twelve_to_ten_bit_shift_preserves_exact_native_samples() {
    let frame = rgb_frame_u16(12);
    let mut native = options(10);
    native.encoder.lossless = true;
    let bytes = encode_native_with_quantization(&frame, &native, Quantization::RightShift).unwrap();
    let decoded = decode_native(&bytes, &limits()).unwrap();
    for role in [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue] {
        let source_index = frame
            .descriptor()
            .planes()
            .iter()
            .position(|plane| plane.roles().contains(&role))
            .unwrap();
        let output_index = decoded
            .descriptor()
            .planes()
            .iter()
            .position(|plane| plane.roles().contains(&role))
            .unwrap();
        let source = &frame.pixels().u16_planes().unwrap()[source_index];
        let output = &decoded.pixels().u16_planes().unwrap()[output_index];
        assert_eq!(
            output.samples(),
            source
                .samples()
                .iter()
                .map(|sample| sample >> 2)
                .collect::<Vec<_>>()
        );
    }
}
