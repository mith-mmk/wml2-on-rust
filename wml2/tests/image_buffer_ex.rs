#![cfg(feature = "image-buffer-ex")]
use wml2::color::RGBA;
use wml2::draw::*;

fn options(format: PixelFormatEx, stride: Option<usize>) -> Option<InitOptionsEx> {
    Some(InitOptionsEx {
        source_format: format,
        source_stride: stride,
        legacy: None,
    })
}
fn samples(format: PixelFormatEx, values: &[u16]) -> Vec<u8> {
    if format.bits() == 8 {
        values.iter().map(|v| *v as u8).collect()
    } else {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }
}
#[test]
fn six_formats_le_samples_and_odd_byte_stride() {
    for format in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Rgba16,
    ] {
        let max = ((1u32 << format.bits()) - 1) as u16;
        let values = [0, 1, max / 2, max];
        let row: Vec<_> = (0..format.channels()).map(|i| values[i]).collect();
        let mut bytes = samples(format, &row);
        let row_length = bytes.len();
        bytes.push(0xff); // Odd stride is safe even for u16 samples.
        let stride = bytes.len();
        let second = samples(format, &vec![max; format.channels()]);
        bytes.extend_from_slice(&second);
        bytes.push(0xff);
        let input = ImageBufferEx::from_bytes(1, 2, format, Some(stride), bytes.clone()).unwrap();
        assert_eq!(input.bytes().unwrap(), bytes);
        let mut target = ImageBufferEx::with_storage(format, PrecisionConversion::Exact);
        target.init_ex(1, 2, options(format, Some(stride))).unwrap();
        target.draw(0, 0, 1, 2, &bytes, None).unwrap();
        assert_eq!(
            target.bytes().unwrap(),
            [&bytes[..row_length], &second].concat()
        );
        assert_eq!(target.format(), format);
        assert_eq!(target.stride(), row_length);
    }
}
#[test]
fn malformed_dimensions_strides_lengths_samples_and_overflow() {
    for (w, h, stride, data) in [
        (0, 1, None, vec![]),
        (1, 0, None, vec![]),
        (2, 1, Some(1), vec![0]),
        (1, 1, Some(0), vec![]),
        (1, 1, None, vec![0]),
        (1, 1, None, vec![0, 0x10]),
        (usize::MAX, 2, None, vec![]),
        (1, 2, Some(usize::MAX), vec![]),
    ] {
        assert!(ImageBufferEx::from_bytes(w, h, PixelFormatEx::Gray12, stride, data).is_err());
    }
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Gray12, PrecisionConversion::Exact);
    image
        .init_ex(1, 1, options(PixelFormatEx::Gray12, None))
        .unwrap();
    for (w, h, data) in [
        (1, 1, vec![1]),
        (1, 1, vec![0, 16]),
        (0, 1, vec![]),
        (2, 1, vec![0, 0]),
    ] {
        assert!(image.draw(0, 0, w, h, &data, None).is_err());
        assert_eq!(image.bytes().unwrap(), [0, 0]);
    }
    assert!(image.draw(usize::MAX, 0, 1, 1, &[0, 0], None).is_err());
}
#[test]
fn clipping_keeps_source_stride_and_validates_clipped_samples() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Gray12, PrecisionConversion::Exact);
    image
        .init_ex(2, 2, options(PixelFormatEx::Gray12, Some(7)))
        .unwrap();
    let data = [1, 0, 2, 0, 3, 0, 255, 4, 0, 5, 0, 6, 0, 255];
    image.draw(1, 0, 3, 2, &data, None).unwrap();
    assert_eq!(image.bytes().unwrap(), [0, 0, 1, 0, 0, 0, 4, 0]);
    let before = image.bytes().unwrap().to_vec();
    let mut invalid = data;
    invalid[5] = 16;
    assert!(image.draw(1, 0, 3, 2, &invalid, None).is_err());
    assert_eq!(image.bytes().unwrap(), before);
    image.draw(10, 10, 3, 2, &data, None).unwrap();
}
#[test]
fn separate_source_storage_and_normalized_exact_round_alpha() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    image
        .init_ex(1, 1, options(PixelFormatEx::Gray8, None))
        .unwrap();
    image.draw(0, 0, 1, 1, &[128], None).unwrap();
    assert_eq!(
        image.bytes().unwrap(),
        samples(PixelFormatEx::Rgba16, &[32896, 32896, 32896, 65535])
    );
    assert_eq!(
        image
            .to_rgba8(PrecisionConversion::Exact)
            .unwrap()
            .buffer
            .unwrap(),
        [128, 128, 128, 255]
    );
    let mut twelve = ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Exact);
    twelve.init(1, 1, None).unwrap();
    assert!(twelve.draw(0, 0, 1, 1, &[1, 2, 3, 128], None).is_err());
    assert_eq!(twelve.bytes().unwrap(), [0; 8]);
    let mut rounded =
        ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Round);
    rounded.init(1, 1, None).unwrap();
    rounded.draw(0, 0, 1, 1, &[1, 2, 3, 128], None).unwrap();
    assert_eq!(
        rounded.bytes().unwrap(),
        samples(PixelFormatEx::Rgba12, &[16, 32, 48, 2056])
    );
    assert!(rounded.to_rgba8(PrecisionConversion::Exact).is_err());
    assert_eq!(
        rounded
            .to_rgba8(PrecisionConversion::Round)
            .unwrap()
            .buffer
            .unwrap(),
        [1, 2, 3, 128]
    );
    let mut gray = ImageBufferEx::with_storage(PixelFormatEx::Gray8, PrecisionConversion::Round);
    assert!(gray.init(1, 1, None).is_err());
    gray.init_ex(1, 1, options(PixelFormatEx::Gray8, None))
        .unwrap();
    assert_eq!(gray.bytes().unwrap(), [0]);
}
#[test]
fn background_scaling_initializers_animation_and_failed_reinit() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    image
        .init(
            1,
            1,
            Some(InitOptions {
                loop_count: 1,
                animation: false,
                background: Some(RGBA {
                    red: 128,
                    green: 0,
                    blue: 255,
                    alpha: 64,
                }),
            }),
        )
        .unwrap();
    assert_eq!(
        image.bytes().unwrap(),
        samples(PixelFormatEx::Rgba16, &[32896, 0, 65535, 16448])
    );
    image.init_ex(1, 1, None).unwrap();
    assert_eq!(image.bytes().unwrap(), [0; 8]);
    assert!(image.next(None).is_err());
    assert!(
        image
            .init(
                1,
                1,
                Some(InitOptions {
                    loop_count: 2,
                    animation: true,
                    background: None
                })
            )
            .is_err()
    );
    assert!(image.draw(0, 0, 1, 1, &[0; 4], None).is_err());
    assert!(image.bytes().is_none());
}
#[test]
fn legacy_fields_and_rgba8_equivalence_remain_mutable() {
    let mut legacy = ImageBuffer::new();
    let mut ex = ImageBufferEx::new();
    legacy.init(3, 2, None).unwrap();
    ex.init(3, 2, None).unwrap();
    let bytes = vec![73; 4 * 4 * 3];
    legacy.draw(1, 1, 4, 3, &bytes, None).unwrap();
    ex.draw(1, 1, 4, 3, &bytes, None).unwrap();
    assert_eq!(legacy.buffer.as_deref().unwrap(), ex.bytes().unwrap());
    // Legacy clipping still permits a partial source; EX has a strict policy.
    legacy.draw(2, 1, 4, 3, &[9; 4], None).unwrap();
    assert!(ex.draw(2, 1, 4, 3, &[9; 4], None).is_err());
    legacy.width = 1;
    legacy.height = 1;
    legacy.buffer = Some(vec![0; 4]);
    legacy.draw(0, 0, 1, 1, &[1, 2, 3, 4], None).unwrap();
    assert_eq!(legacy.buffer.unwrap(), [1, 2, 3, 4]);
    let mut zero = ImageBuffer::new();
    zero.init(0, 0, None).unwrap();
}
// No init_ex method: an unchanged legacy implementation remains source-compatible.
struct OldCallback {
    initializations: usize,
}
impl DrawCallback for OldCallback {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        self.initializations += 1;
        Ok(None)
    }
    fn draw(
        &mut self,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
        _: &[u8],
        _: Option<DrawOptions>,
    ) -> Response {
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        Ok(None)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, _: &str, _: wml2::metadata::DataMap) -> Response {
        Ok(None)
    }
}
#[test]
fn provided_trait_initializer_only_forwards_packed_rgba8() {
    let mut old = OldCallback { initializations: 0 };
    old.init_ex(1, 1, None).unwrap();
    for format in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Rgba16,
    ] {
        assert!(old.init_ex(1, 1, options(format, None)).is_err());
    }
    assert!(
        old.init_ex(1, 1, options(PixelFormatEx::Rgba8, Some(4)))
            .is_err()
    );
    assert_eq!(old.initializations, 1);
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn highres_pixel_only_roundtrips_six_formats_and_rejects_unsupported() {
    use wml2::highres::*;
    for format in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Rgba16,
    ] {
        let bytes = samples(format, &vec![1; format.channels()]);
        let original = ImageBufferEx::from_bytes(1, 1, format, None, bytes.clone()).unwrap();
        let frame = original.to_highres_pixels().unwrap();
        let copy = ImageBufferEx::from_highres_pixels(&frame).unwrap();
        assert_eq!(copy.bytes().unwrap(), bytes);
        assert_eq!(copy.format(), format);
        assert!(frame.timing().is_none());
    }
    for (bits, floating) in [(10, false), (16, true)] {
        let desc = ImageDescriptor::gray(1, 1, bits).unwrap();
        let layout = desc.planes()[0].layout().clone();
        let pixels = if floating {
            PixelBuffer::f32(vec![Plane::new(layout, vec![0.5]).unwrap()]).unwrap()
        } else {
            PixelBuffer::u16(vec![Plane::new(layout, vec![1]).unwrap()]).unwrap()
        };
        let frame = ImageFrame::new(desc, pixels).unwrap();
        assert!(ImageBufferEx::from_highres_pixels(&frame).is_err());
    }
    let ex = ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba8, None, vec![1; 4]).unwrap();
    let frame = ex.to_highres_pixels().unwrap();
    let premult = ImageFrame::new(
        frame
            .descriptor()
            .clone()
            .with_alpha(AlphaAssociation::Premultiplied)
            .unwrap(),
        frame.pixels().clone(),
    )
    .unwrap();
    assert!(ImageBufferEx::from_highres_pixels(&premult).is_err());
    let yuv = ImageDescriptor::ycbcr(1, 1, 8, [Subsampling::FULL; 3]).unwrap();
    let pixels = PixelBuffer::u8(
        yuv.planes()
            .iter()
            .map(|p| Plane::new(p.layout().clone(), vec![1]).unwrap())
            .collect(),
    )
    .unwrap();
    assert!(ImageBufferEx::from_highres_pixels(&ImageFrame::new(yuv, pixels).unwrap()).is_err());
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn highres_padded_planar_u16_at_eight_bits_preserves_roles_only() {
    use wml2::highres::*;
    let roles = [
        ChannelRole::Alpha,
        ChannelRole::Blue,
        ChannelRole::Red,
        ChannelRole::Green,
    ];
    let layout = PlaneLayout::new(1, 2, 3, 1, vec![0], Subsampling::FULL).unwrap();
    let color = ColorInformationSet::new().with_nclx(NclxColorInformation::new(9, 16, 0, true));
    let descriptor = ImageDescriptor::new(
        1,
        2,
        ChannelModel::RGB,
        roles
            .iter()
            .map(|role| PlaneDescriptor::planar(layout.clone(), *role, 8).unwrap())
            .collect(),
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap()
    .with_color_information(color.clone());
    let pixels = PixelBuffer::u16(
        roles
            .iter()
            .enumerate()
            .map(|(i, _)| {
                Plane::new(
                    layout.clone(),
                    vec![(i + 1) as u16, 65535, 65535, (i + 11) as u16],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let timing = FrameTiming::new(1000, 4, 5).unwrap();
    let frame = ImageFrame::new(descriptor, pixels)
        .unwrap()
        .with_timing(timing);
    let bytes = ImageBufferEx::from_highres_pixels(&frame).unwrap();
    assert_eq!(bytes.bytes().unwrap(), [3, 4, 2, 1, 13, 14, 12, 11]);
    assert!(bytes.metadata.is_none());
    assert_eq!(frame.timing(), Some(timing));
    assert_eq!(frame.descriptor().color_information(), &color);
    let exported = bytes.to_highres_pixels().unwrap();
    assert!(exported.timing().is_none());
    assert_eq!(
        exported.descriptor().color_information(),
        &ColorInformationSet::default()
    );
}

#[cfg(feature = "png")]
#[test]
fn unchanged_png_decoder_to_ex_and_explicit_legacy_encoder_adapter() {
    use wml2::util::ImageFormat;
    let pixels = vec![1, 2, 3, 4, 255, 0, 64, 255];
    let mut source = ImageBuffer::from_buffer(2, 1, pixels.clone());
    let encoded = image_to(&mut source, ImageFormat::Png, None).unwrap();
    let mut ex = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    image_loader(
        &encoded,
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut ex,
        },
    )
    .unwrap();
    assert_eq!(ex.format(), PixelFormatEx::Rgba16);
    let mut adapter = ex.to_rgba8(PrecisionConversion::Exact).unwrap();
    assert_eq!(adapter.buffer.as_ref().unwrap(), &pixels);
    let encoded = image_to(&mut adapter, ImageFormat::Png, None).unwrap();
    assert_eq!(image_from(&encoded).unwrap().buffer.unwrap(), pixels);
}
