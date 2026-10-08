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
    option: Option<InitOptions>,
}
impl DrawCallback for OldCallback {
    fn init(&mut self, _: usize, _: usize, option: Option<InitOptions>) -> Response {
        self.initializations += 1;
        self.option = option;
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
    let mut old = OldCallback {
        initializations: 0,
        option: None,
    };
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

#[test]
fn public_converter_six_format_matrix_exact_round_with_padding() {
    let formats = [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Rgba16,
    ];
    for source in formats {
        for target in formats {
            let source_max = (1u32 << source.bits()) - 1;
            let target_max = (1u32 << target.bits()) - 1;
            let source_stride = 2 * source.bytes_per_pixel() + 1;
            let target_stride = 2 * target.bytes_per_pixel() + 1;
            let mut input = vec![255; source_stride * 2];
            let mut values = Vec::new();
            for y in 0..2 {
                for x in 0..2 {
                    let mut pixel = Vec::new();
                    for channel in 0..source.channels() {
                        let value = [0, 1, source_max / 2, source_max][(y * 2 + x + channel) % 4];
                        pixel.push(value as u16);
                    }
                    let offset = y * source_stride + x * source.bytes_per_pixel();
                    input[offset..offset + source.bytes_per_pixel()]
                        .copy_from_slice(&samples(source, &pixel));
                    values.push(pixel);
                }
            }
            for policy in [PrecisionConversion::Exact, PrecisionConversion::Round] {
                let request = PixelOutputEx {
                    format: target,
                    stride: Some(target_stride),
                    conversion: policy,
                };
                let result = convert_pixels_ex(&input, 2, 2, source, Some(source_stride), request);
                if source.channels() == 4 && target.channels() == 1 {
                    assert!(result.is_err(), "{source:?} -> {target:?}");
                    continue;
                }
                let mut expected = vec![0; target_stride * 2];
                let mut exact = true;
                for y in 0..2 {
                    for x in 0..2 {
                        let mut pixel = Vec::new();
                        for channel in 0..target.channels() {
                            let value = if source.channels() == 1 && channel == 3 {
                                source_max
                            } else {
                                u32::from(
                                    values[y * 2 + x]
                                        [if source.channels() == 1 { 0 } else { channel }],
                                )
                            };
                            let numerator = u64::from(value) * u64::from(target_max);
                            let denominator = u64::from(source_max);
                            exact &= numerator % denominator == 0;
                            let rounded = (numerator
                                + if policy == PrecisionConversion::Round {
                                    denominator / 2
                                } else {
                                    0
                                })
                                / denominator;
                            pixel.push(rounded as u16);
                        }
                        let offset = y * target_stride + x * target.bytes_per_pixel();
                        expected[offset..offset + target.bytes_per_pixel()]
                            .copy_from_slice(&samples(target, &pixel));
                    }
                }
                if policy == PrecisionConversion::Exact && !exact {
                    assert!(
                        result.is_err(),
                        "{source:?} -> {target:?} must reject lossy scaling"
                    );
                } else {
                    let result = result.unwrap();
                    assert_eq!(
                        result.bytes(),
                        expected,
                        "{source:?} -> {target:?} {policy:?}"
                    );
                    assert_eq!(
                        (
                            result.width(),
                            result.height(),
                            result.format(),
                            result.stride()
                        ),
                        (2, 2, target, target_stride)
                    );
                }
            }
        }
    }
}

#[test]
fn public_converter_rejects_bad_source_output_and_overflow() {
    let output = PixelOutputEx::new(PixelFormatEx::Gray16, PrecisionConversion::Exact);
    for (width, height, stride, bytes) in [
        (0, 1, None, vec![]),
        (1, 0, None, vec![]),
        (1, 1, Some(1), vec![0]),
        (1, 1, None, vec![0]),
        (1, 1, None, vec![0, 16]),
        (usize::MAX, 1, None, vec![]),
        (1, 2, Some(usize::MAX), vec![]),
    ] {
        assert!(
            convert_pixels_ex(&bytes, width, height, PixelFormatEx::Gray12, stride, output)
                .is_err()
        );
    }
    for stride in [0, 1, usize::MAX] {
        assert!(
            convert_pixels_ex(
                &[0; 4],
                1,
                2,
                PixelFormatEx::Gray12,
                None,
                PixelOutputEx {
                    stride: Some(stride),
                    ..output
                }
            )
            .is_err()
        );
    }
    // Padding is excluded from sample validation and regenerated as zero.
    let result = convert_pixels_ex(
        &[1, 0, 255, 2, 0, 255],
        1,
        2,
        PixelFormatEx::Gray12,
        Some(3),
        PixelOutputEx {
            format: PixelFormatEx::Gray12,
            stride: Some(5),
            conversion: PrecisionConversion::Exact,
        },
    )
    .unwrap();
    assert_eq!(result.bytes(), [1, 0, 0, 0, 0, 2, 0, 0, 0, 0]);
}

#[test]
fn picker_reads_padded_canvas_tiles_and_roundtrips_through_draw() {
    let mut source = ImageBufferEx::from_bytes(
        2,
        2,
        PixelFormatEx::Gray12,
        Some(5),
        vec![1, 0, 255, 15, 255, 2, 0, 255, 15, 255],
    )
    .unwrap();
    let request = PixelOutputEx {
        format: PixelFormatEx::Rgba12,
        stride: Some(9),
        conversion: PrecisionConversion::Exact,
    };
    let tile = source
        .encode_pick_ex(1, 0, 1, 2, request, Some(PickOptions {}))
        .unwrap()
        .unwrap();
    assert_eq!(
        tile.bytes(),
        [
            255, 15, 255, 15, 255, 15, 255, 15, 0, 255, 15, 255, 15, 255, 15, 255, 15, 0
        ]
    );
    let mut target = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    target
        .init_ex(1, 2, options(tile.format(), Some(tile.stride())))
        .unwrap();
    target.draw(0, 0, 1, 2, tile.bytes(), None).unwrap();
    let back = target
        .encode_pick_ex(
            0,
            0,
            1,
            2,
            PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact),
            None,
        )
        .unwrap()
        .unwrap();
    assert_eq!(back.bytes(), [255, 15].repeat(8));
}

#[test]
fn picker_bounds_stride_precision_errors_leave_storage_unchanged() {
    let mut image = ImageBufferEx::from_bytes(
        2,
        2,
        PixelFormatEx::Rgba16,
        None,
        samples(PixelFormatEx::Rgba16, &[1; 16]),
    )
    .unwrap();
    let before = image.bytes().unwrap().to_vec();
    let output = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    for (x, y, width, height) in [
        (0, 0, 0, 1),
        (0, 0, 1, 0),
        (2, 0, 1, 1),
        (0, 2, 1, 1),
        (1, 1, 2, 1),
        (1, 1, 1, 2),
        (usize::MAX, 0, 1, 1),
        (0, usize::MAX, 1, 1),
    ] {
        assert!(
            image
                .encode_pick_ex(x, y, width, height, output, None)
                .is_err()
        );
    }
    for stride in [0, 7, usize::MAX] {
        assert!(
            image
                .encode_pick_ex(
                    0,
                    0,
                    1,
                    2,
                    PixelOutputEx {
                        stride: Some(stride),
                        ..output
                    },
                    None
                )
                .is_err()
        );
    }
    assert!(
        image
            .encode_pick_ex(
                0,
                0,
                1,
                1,
                PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact),
                None
            )
            .is_err()
    );
    let rounded = image
        .encode_pick_ex(
            0,
            0,
            1,
            1,
            PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Round),
            None,
        )
        .unwrap()
        .unwrap();
    assert_eq!(rounded.bytes(), [0; 4]);
    assert_eq!(image.bytes().unwrap(), before);
    assert!(ImageBufferEx::new().encode_start_ex(None).is_err());
    assert!(
        ImageBufferEx::new()
            .encode_pick_ex(0, 0, 1, 1, output, None)
            .is_err()
    );
}

#[test]
fn ex_picker_profiles_preserve_background_metadata_and_reinitialization() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    image
        .init(
            1,
            1,
            Some(InitOptions {
                loop_count: 4,
                animation: false,
                background: Some(RGBA {
                    red: 17,
                    green: 34,
                    blue: 51,
                    alpha: 68,
                }),
            }),
        )
        .unwrap();
    image
        .set_metadata("example", wml2::metadata::DataMap::UInt(19))
        .unwrap();
    let profile = image
        .encode_start_ex(Some(EncoderOptions {}))
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            profile.legacy.width,
            profile.legacy.height,
            profile.format,
            profile.stride
        ),
        (1, 1, PixelFormatEx::Rgba16, 8)
    );
    let background = profile.legacy.background.unwrap();
    assert_eq!(
        (
            background.red,
            background.green,
            background.blue,
            background.alpha
        ),
        (17, 34, 51, 68)
    );
    assert!(profile.legacy.metadata.unwrap().contains_key("example"));
    assert!(
        image
            .metadata_ex()
            .unwrap()
            .unwrap()
            .contains_key("example")
    );
    assert!(
        image
            .to_rgba8(PrecisionConversion::Exact)
            .unwrap()
            .background_color
            .is_some()
    );
    image.encode_end_ex(Some(EndOptions {})).unwrap();
    image.init_ex(1, 1, None).unwrap();
    assert!(
        image
            .encode_start_ex(None)
            .unwrap()
            .unwrap()
            .legacy
            .background
            .is_none()
    );
    assert!(image.init_ex(0, 1, None).is_err());
    assert!(image.encode_start_ex(None).is_err());
}

#[test]
fn legacy_buffer_ex_picker_revalidates_mutable_fields_and_keeps_legacy_api() {
    let mut legacy = ImageBuffer::from_buffer(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    let profile = legacy
        .encode_start_ex(Some(EncoderOptions {}))
        .unwrap()
        .unwrap();
    assert_eq!((profile.format, profile.stride), (PixelFormatEx::Rgba8, 8));
    let requested = legacy
        .encode_pick_ex(
            1,
            0,
            1,
            1,
            PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact),
            Some(PickOptions {}),
        )
        .unwrap()
        .unwrap();
    assert_eq!(requested.bytes(), [5, 5, 6, 6, 7, 7, 8, 8]);
    assert_eq!(
        legacy.encode_pick(1, 0, 1, 1, None).unwrap().unwrap(),
        [5, 6, 7, 8]
    );
    legacy.encode_end_ex(Some(EndOptions {})).unwrap();
    legacy.width = 3;
    assert!(legacy.encode_start_ex(None).is_err());
    assert!(
        legacy
            .encode_pick_ex(
                0,
                0,
                1,
                1,
                PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact),
                None
            )
            .is_err()
    );
    legacy.width = 1;
    legacy.buffer = Some(vec![1, 2, 3, 4]);
    assert_eq!(
        legacy
            .encode_pick_ex(
                0,
                0,
                1,
                1,
                PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact),
                None
            )
            .unwrap()
            .unwrap()
            .bytes(),
        [1, 2, 3, 4]
    );
}

#[test]
fn provided_legacy_initializer_preserves_all_declared_legacy_options() {
    let mut callback = OldCallback {
        initializations: 0,
        option: None,
    };
    let option = InitOptions {
        loop_count: 23,
        animation: true,
        background: Some(RGBA {
            red: 11,
            green: 22,
            blue: 33,
            alpha: 44,
        }),
    };
    callback
        .init_ex(1, 1, Some(option.with_format(PixelFormatEx::Rgba8)))
        .unwrap();
    let received = callback.option.take().unwrap();
    assert_eq!(received.loop_count, 23);
    assert!(received.animation);
    let background = received.background.unwrap();
    assert_eq!(
        (
            background.red,
            background.green,
            background.blue,
            background.alpha
        ),
        (11, 22, 33, 44)
    );
    assert_eq!(callback.initializations, 1);
}
