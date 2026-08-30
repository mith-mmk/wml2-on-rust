use super::super::super::{
    AlphaAssociation, ChannelModel, ChannelRole, ColorConvertOptions, ColorInformationSet,
    Destination, HighresError, ImageDescriptor, ImageFrame, MatrixCoefficients,
    NativeSampleEncoding, NclxColorInformation, PixelBuffer, PixelFormat, Plane, PlaneDescriptor,
    PlaneLayout, ProcessingError, ResourceLimits, RgbPrimaries, SampleDomain, SampleRange,
    Subsampling,
};
use super::{NativeColor, NativePixelReader};

fn limits() -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(4096)
        .max_height(4096)
        .max_pixels(16_777_216)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 24)
        .max_frame_bytes(1 << 26)
        .max_total_live_decoded_bytes(1 << 27)
        .max_references(64)
        .max_frame_count(64)
        .max_metadata_bytes(1 << 20)
        .max_icc_bytes(1 << 20)
        .max_clut_bytes(1 << 24)
        .max_parser_entries(1 << 16)
        .max_parser_depth(64)
        .max_grid_cells(1 << 16)
        .max_derived_work(1 << 20)
        .max_derived_depth(32)
        .build()
        .unwrap()
}

fn encoded_color(matrix: u16, full_range: bool) -> ColorInformationSet {
    ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, matrix, full_range))
}

fn destination() -> Destination<'static> {
    Destination::linear_rgb(SampleDomain::LinearRelative, RgbPrimaries::srgb())
}

fn integer_options(
    range: SampleRange,
    matrix: MatrixCoefficients,
    chroma_location: Option<super::super::super::ChromaLocation>,
) -> ColorConvertOptions<'static> {
    ColorConvertOptions::new(destination()).with_native_sample_encoding(NativeSampleEncoding::new(
        range,
        matrix,
        chroma_location,
    ))
}

#[test]
fn interleaved_rgb_role_offsets_and_separate_alpha_are_read_without_reordering() {
    let rgb_layout = PlaneLayout::interleaved(2, 1, 3, Subsampling::FULL).unwrap();
    let alpha_layout = PlaneLayout::planar(2, 1, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        2,
        1,
        ChannelModel::RGB,
        vec![
            PlaneDescriptor::new(
                rgb_layout,
                vec![ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue],
                8,
            )
            .unwrap(),
            PlaneDescriptor::planar(alpha_layout.clone(), ChannelRole::Alpha, 8).unwrap(),
        ],
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap()
    .with_color_information(encoded_color(0, true))
    .with_domain(SampleDomain::Encoded);
    let pixels = PixelBuffer::u8(vec![
        Plane::new(
            PlaneLayout::interleaved(2, 1, 3, Subsampling::FULL).unwrap(),
            vec![10, 20, 30, 40, 50, 60],
        )
        .unwrap(),
        Plane::new(alpha_layout, vec![128, 255]).unwrap(),
    ])
    .unwrap();
    let frame = ImageFrame::new(descriptor, pixels).unwrap();
    let reader = NativePixelReader::inspect(
        &frame,
        &integer_options(SampleRange::Full, MatrixCoefficients::Identity, None),
        &limits(),
    )
    .unwrap();

    assert_eq!(frame.pixels().format(), PixelFormat::U8);
    assert_eq!(
        reader.pixel(0, 0).unwrap(),
        super::NativePixel {
            color: NativeColor::Rgb([10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0,]),
            alpha: Some(128.0 / 255.0),
        }
    );
    assert_eq!(
        reader.pixel(1, 0).unwrap().alpha,
        Some(1.0),
        "alpha remains an independent full-range sample"
    );
}

#[test]
fn planar_offsets_padding_invalid_coordinates_and_source_storage_are_stable() {
    let layout = PlaneLayout::new(2, 1, 4, 2, vec![1], Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        2,
        1,
        ChannelModel::RGB,
        [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
            .into_iter()
            .map(|role| PlaneDescriptor::planar(layout.clone(), role, 8).unwrap())
            .collect(),
    )
    .unwrap()
    .with_color_information(encoded_color(0, true))
    .with_domain(SampleDomain::Encoded);
    let pixels = PixelBuffer::u8(
        [10u8, 20, 30]
            .into_iter()
            .map(|value| Plane::new(layout.clone(), vec![77, value, 88, value + 1]).unwrap())
            .collect(),
    )
    .unwrap();
    let frame = ImageFrame::new(descriptor, pixels).unwrap();
    let before = frame
        .pixels()
        .u8_planes()
        .unwrap()
        .iter()
        .map(|plane| (plane.samples().as_ptr(), plane.samples().to_vec()))
        .collect::<Vec<_>>();
    let reader = NativePixelReader::inspect(
        &frame,
        &integer_options(SampleRange::Full, MatrixCoefficients::Identity, None),
        &limits(),
    )
    .unwrap();
    assert_eq!(
        reader.pixel(0, 0).unwrap().color,
        NativeColor::Rgb([10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0,])
    );
    assert!(matches!(
        reader.pixel(2, 0),
        Err(super::super::super::HighresError::InvalidDimensions(_))
    ));
    for (plane, (pointer, samples)) in frame.pixels().u8_planes().unwrap().iter().zip(before) {
        assert_eq!(plane.samples().as_ptr(), pointer);
        assert_eq!(plane.samples(), samples.as_slice());
    }
}

#[test]
fn integer_depths_ranges_and_alpha_are_expanded_independently() {
    for bits in [8u8, 10, 12] {
        let max = (1u16 << bits) - 1;
        for range in [SampleRange::Full, SampleRange::Limited] {
            let full_range = range == SampleRange::Full;
            let color = encoded_color(0, full_range);
            let descriptor = ImageDescriptor::new(
                3,
                1,
                ChannelModel::RGB,
                [
                    ChannelRole::Red,
                    ChannelRole::Green,
                    ChannelRole::Blue,
                    ChannelRole::Alpha,
                ]
                .into_iter()
                .map(|role| {
                    PlaneDescriptor::planar(
                        PlaneLayout::planar(3, 1, Subsampling::FULL).unwrap(),
                        role,
                        bits,
                    )
                    .unwrap()
                })
                .collect(),
            )
            .unwrap()
            .with_alpha(AlphaAssociation::Straight)
            .unwrap()
            .with_color_information(color)
            .with_domain(SampleDomain::Encoded);
            let color_samples = if full_range {
                vec![0, max / 2, max]
            } else {
                let scale = 1u16 << (bits - 8);
                vec![16 * scale, 128 * scale, 235 * scale]
            };
            let alpha_samples = vec![0, max / 2, max];
            let planes = [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
                .into_iter()
                .map(|_| {
                    Plane::new(
                        PlaneLayout::planar(3, 1, Subsampling::FULL).unwrap(),
                        color_samples.clone(),
                    )
                    .unwrap()
                })
                .chain(std::iter::once(
                    Plane::new(
                        PlaneLayout::planar(3, 1, Subsampling::FULL).unwrap(),
                        alpha_samples,
                    )
                    .unwrap(),
                ))
                .collect();
            let frame = ImageFrame::new(descriptor, PixelBuffer::u16(planes).unwrap()).unwrap();
            let reader = NativePixelReader::inspect(
                &frame,
                &integer_options(range, MatrixCoefficients::Identity, None),
                &limits(),
            )
            .unwrap();
            let middle = reader.pixel(1, 0).unwrap();
            let expected_color = if full_range {
                f32::from(max / 2) / f32::from(max)
            } else {
                (128.0 - 16.0) / 219.0
            };
            let expected_alpha = f32::from(max / 2) / f32::from(max);
            assert!(
                (middle_color(middle) - expected_color).abs() < 1e-6,
                "bits={bits}, range={range:?}, color={:?}, expected={expected_color}",
                middle_color(middle)
            );
            assert!(
                (middle.alpha.unwrap() - expected_alpha).abs() < 1e-6,
                "bits={bits}, range={range:?}, alpha={}, expected={expected_alpha}",
                middle.alpha.unwrap()
            );
            assert_eq!(reader.pixel(0, 0).unwrap().alpha, Some(0.0));
            assert_eq!(reader.pixel(2, 0).unwrap().alpha, Some(1.0));
        }
    }
}

fn middle_color(pixel: super::NativePixel) -> f32 {
    match pixel.color {
        NativeColor::Rgb(rgb) => rgb[0],
        NativeColor::Gray(value) => value,
    }
}

#[test]
fn limited_range_u16_gray_uses_bit_depth_scaled_code_values() {
    let descriptor = ImageDescriptor::gray(2, 1, 10)
        .unwrap()
        .with_color_information(encoded_color(0, false))
        .with_domain(SampleDomain::Encoded);
    let layout = PlaneLayout::planar(2, 1, Subsampling::FULL).unwrap();
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![Plane::new(layout, vec![64, 940]).unwrap()]).unwrap(),
    )
    .unwrap();
    let reader = NativePixelReader::inspect(
        &frame,
        &integer_options(SampleRange::Limited, MatrixCoefficients::Identity, None),
        &limits(),
    )
    .unwrap();
    assert_eq!(reader.pixel(0, 0).unwrap().color, NativeColor::Gray(0.0));
    assert_eq!(reader.pixel(1, 0).unwrap().color, NativeColor::Gray(1.0));
}

#[test]
fn ycbcr_420_interpolates_before_matrix() {
    let subsampling = [
        Subsampling::FULL,
        Subsampling::new(2, 2).unwrap(),
        Subsampling::new(2, 2).unwrap(),
    ];
    let descriptor = ImageDescriptor::ycbcr(3, 2, 8, subsampling)
        .unwrap()
        .with_color_information(encoded_color(1, true))
        .with_domain(SampleDomain::Encoded);
    let y_layout = PlaneLayout::planar(3, 2, Subsampling::FULL).unwrap();
    let c_layout = PlaneLayout::planar(2, 1, Subsampling::new(2, 2).unwrap()).unwrap();
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u8(vec![
            Plane::new(y_layout, vec![128; 6]).unwrap(),
            Plane::new(c_layout.clone(), vec![128, 255]).unwrap(),
            Plane::new(c_layout, vec![128, 0]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let location = super::super::super::ChromaLocation::new(
        super::super::super::ChromaPhase::Zero,
        super::super::super::ChromaPhase::Half,
        0,
    )
    .unwrap();
    let reader = NativePixelReader::inspect(
        &frame,
        &integer_options(SampleRange::Full, MatrixCoefficients::Bt709, Some(location)),
        &limits(),
    )
    .unwrap();
    let pixel = reader.pixel(1, 0).unwrap();
    let NativeColor::Rgb(rgb) = pixel.color else {
        panic!("YCbCr reader must produce RGB");
    };
    let cb = (128.0 - 128.0) / 255.0 * 0.5 + (255.0 - 128.0) / 255.0 * 0.5;
    let cr = (128.0 - 128.0) / 255.0 * 0.5 + (0.0 - 128.0) / 255.0 * 0.5;
    let y = 128.0 / 255.0;
    let red = y + 2.0 * (1.0 - 0.2126) * cr;
    let blue = y + 2.0 * (1.0 - 0.0722) * cb;
    let green = (y - 0.2126 * red - 0.0722 * blue) / (1.0 - 0.2126 - 0.0722);
    assert!((f64::from(rgb[0]) - red).abs() < 1e-6);
    assert!((f64::from(rgb[1]) - green).abs() < 1e-6);
    assert!((f64::from(rgb[2]) - blue).abs() < 1e-6);
}

#[test]
fn ycbcr_matrix_codes_keep_neutral_and_colored_excursions_distinct() {
    for (code, matrix, kr, kb) in [
        (1u16, MatrixCoefficients::Bt709, 0.2126, 0.0722),
        (5, MatrixCoefficients::Bt601, 0.299, 0.114),
        (6, MatrixCoefficients::Bt601, 0.299, 0.114),
        (9, MatrixCoefficients::Bt2020, 0.2627, 0.0593),
    ] {
        let descriptor = ImageDescriptor::ycbcr(
            1,
            1,
            8,
            [Subsampling::FULL, Subsampling::FULL, Subsampling::FULL],
        )
        .unwrap()
        .with_color_information(encoded_color(code, true))
        .with_domain(SampleDomain::Encoded);
        let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
        let frame = ImageFrame::new(
            descriptor,
            PixelBuffer::u8(vec![
                Plane::new(layout.clone(), vec![128]).unwrap(),
                Plane::new(layout.clone(), vec![200]).unwrap(),
                Plane::new(layout, vec![100]).unwrap(),
            ])
            .unwrap(),
        )
        .unwrap();
        let reader = NativePixelReader::inspect(
            &frame,
            &integer_options(SampleRange::Full, matrix, None),
            &limits(),
        )
        .unwrap();
        let pixel = reader.pixel(0, 0).unwrap();
        let NativeColor::Rgb(rgb) = pixel.color else {
            panic!("YCbCr reader must produce RGB");
        };
        let y = 128.0 / 255.0;
        let cb = (200.0 - 128.0) / 255.0;
        let cr = (100.0 - 128.0) / 255.0;
        let red = y + 2.0 * (1.0 - kr) * cr;
        let blue = y + 2.0 * (1.0 - kb) * cb;
        let green = (y - kr * red - kb * blue) / (1.0 - kr - kb);
        assert!((f64::from(rgb[0]) - red).abs() < 1e-6);
        assert!((f64::from(rgb[1]) - green).abs() < 1e-6);
        assert!((f64::from(rgb[2]) - blue).abs() < 1e-6);
    }
}

#[test]
fn five_by_three_420_422_and_444_cover_all_h273_chroma_phases() {
    let phases = (0..=5)
        .map(|code| super::super::super::ChromaLocation::from_h273_code(code).unwrap())
        .collect::<Vec<_>>();
    for (cb_subsampling, cr_subsampling) in [
        (
            Subsampling::new(2, 2).unwrap(),
            Subsampling::new(2, 2).unwrap(),
        ),
        (
            Subsampling::new(2, 1).unwrap(),
            Subsampling::new(2, 1).unwrap(),
        ),
        (Subsampling::FULL, Subsampling::FULL),
    ] {
        let (cb_width, cb_height) = cb_subsampling.dimensions(5, 3).unwrap();
        let (cr_width, cr_height) = cr_subsampling.dimensions(5, 3).unwrap();
        for location in phases.iter().copied() {
            let descriptor = ImageDescriptor::ycbcr(
                5,
                3,
                8,
                [Subsampling::FULL, cb_subsampling, cr_subsampling],
            )
            .unwrap()
            .with_color_information(encoded_color(1, true))
            .with_domain(SampleDomain::Encoded);
            let y_layout = PlaneLayout::planar(5, 3, Subsampling::FULL).unwrap();
            let cb_layout = PlaneLayout::planar(cb_width, cb_height, cb_subsampling).unwrap();
            let cr_layout = PlaneLayout::planar(cr_width, cr_height, cr_subsampling).unwrap();
            let cb_values = (0..(cb_width * cb_height))
                .map(|index| 80u8 + (index as u8) * 5)
                .collect::<Vec<_>>();
            let cr_values = (0..(cr_width * cr_height))
                .map(|index| 220u16.saturating_sub((index as u16) * 11) as u8)
                .collect::<Vec<_>>();
            let frame = ImageFrame::new(
                descriptor,
                PixelBuffer::u8(vec![
                    Plane::new(y_layout, vec![128; 15]).unwrap(),
                    Plane::new(cb_layout, cb_values.clone()).unwrap(),
                    Plane::new(cr_layout, cr_values.clone()).unwrap(),
                ])
                .unwrap(),
            )
            .unwrap();
            let reader = NativePixelReader::inspect(
                &frame,
                &integer_options(SampleRange::Full, MatrixCoefficients::Bt709, Some(location)),
                &limits(),
            )
            .unwrap();
            let pixel = reader.pixel(2, 1).unwrap();
            let NativeColor::Rgb(rgb) = pixel.color else {
                panic!("YCbCr reader must produce RGB");
            };
            let cb = interpolate_expected(
                &cb_values,
                cb_width,
                cb_height,
                cb_subsampling,
                location,
                2,
                1,
            );
            let cr = interpolate_expected(
                &cr_values,
                cr_width,
                cr_height,
                cr_subsampling,
                location,
                2,
                1,
            );
            let y = 128.0 / 255.0;
            let red = y + 2.0 * (1.0 - 0.2126) * cr;
            let blue = y + 2.0 * (1.0 - 0.0722) * cb;
            let green = (y - 0.2126 * red - 0.0722 * blue) / (1.0 - 0.2126 - 0.0722);
            assert!((f64::from(rgb[0]) - red).abs() < 1e-6);
            assert!((f64::from(rgb[1]) - green).abs() < 1e-6);
            assert!((f64::from(rgb[2]) - blue).abs() < 1e-6);
        }
    }
}

#[test]
fn one_pixel_and_one_column_subsampled_edges_are_read_for_every_phase() {
    for (width, height, chroma) in [
        (1, 1, Subsampling::new(2, 2).unwrap()),
        (1, 3, Subsampling::new(2, 1).unwrap()),
    ] {
        let (chroma_width, chroma_height) = chroma.dimensions(width, height).unwrap();
        let location_codes = 0..=5;
        for code in location_codes {
            let location = super::super::super::ChromaLocation::from_h273_code(code).unwrap();
            let descriptor =
                ImageDescriptor::ycbcr(width, height, 8, [Subsampling::FULL, chroma, chroma])
                    .unwrap()
                    .with_color_information(encoded_color(1, true))
                    .with_domain(SampleDomain::Encoded);
            let y_layout = PlaneLayout::planar(width, height, Subsampling::FULL).unwrap();
            let c_layout = PlaneLayout::planar(chroma_width, chroma_height, chroma).unwrap();
            let y_samples = vec![128; (width * height) as usize];
            let c_samples = vec![128; (chroma_width * chroma_height) as usize];
            let frame = ImageFrame::new(
                descriptor,
                PixelBuffer::u8(vec![
                    Plane::new(y_layout, y_samples).unwrap(),
                    Plane::new(c_layout.clone(), c_samples.clone()).unwrap(),
                    Plane::new(c_layout, c_samples).unwrap(),
                ])
                .unwrap(),
            )
            .unwrap();
            let reader = NativePixelReader::inspect(
                &frame,
                &integer_options(SampleRange::Full, MatrixCoefficients::Bt709, Some(location)),
                &limits(),
            )
            .unwrap();
            assert!(matches!(
                reader.pixel(width - 1, height - 1).unwrap().color,
                NativeColor::Rgb(_)
            ));
        }
    }
}

fn interpolate_expected(
    values: &[u8],
    width: u32,
    height: u32,
    subsampling: Subsampling,
    location: super::super::super::ChromaLocation,
    x: u32,
    y: u32,
) -> f64 {
    let (x0, x1, wx) = expected_axis(x, subsampling.x(), location.x_phase(), width);
    let (y0, y1, wy) = expected_axis(y, subsampling.y(), location.y_phase(), height);
    let at = |x: usize, y: usize| (f64::from(values[y * width as usize + x]) - 128.0) / 255.0;
    at(x0, y0) * (1.0 - wx) * (1.0 - wy)
        + at(x1, y0) * wx * (1.0 - wy)
        + at(x0, y1) * (1.0 - wx) * wy
        + at(x1, y1) * wx * wy
}

fn expected_axis(
    index: u32,
    factor: u8,
    phase: super::super::super::ChromaPhase,
    extent: u32,
) -> (usize, usize, f64) {
    if factor == 1 {
        return (index as usize, index as usize, 0.0);
    }
    let phase = match phase {
        super::super::super::ChromaPhase::Zero => 0.0,
        super::super::super::ChromaPhase::Half => 0.5,
        super::super::super::ChromaPhase::One => 1.0,
    };
    let coordinate = (f64::from(index) - phase) / f64::from(factor);
    let base = coordinate.floor();
    let max = f64::from(extent - 1);
    (
        base.clamp(0.0, max) as usize,
        (base + 1.0).clamp(0.0, max) as usize,
        coordinate - base,
    )
}

#[test]
fn f32_gray_and_rgb_are_passthrough_but_f32_ycbcr_is_explicitly_unsupported() {
    let gray = ImageDescriptor::gray(1, 1, 32)
        .unwrap()
        .with_domain(SampleDomain::LinearRelative)
        .with_primaries(RgbPrimaries::srgb());
    let gray_frame = ImageFrame::new(
        gray,
        PixelBuffer::f32(vec![
            Plane::new(
                PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap(),
                vec![0.375],
            )
            .unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let gray_options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    let gray_reader = NativePixelReader::inspect(&gray_frame, &gray_options, &limits()).unwrap();
    assert_eq!(
        gray_reader.pixel(0, 0).unwrap().color,
        NativeColor::Gray(0.375)
    );

    let rgb_descriptor = ImageDescriptor::rgb(1, 1, 32)
        .unwrap()
        .with_domain(SampleDomain::LinearRelative)
        .with_primaries(RgbPrimaries::srgb());
    let rgb_planes = [0.125f32, 0.5, 0.875]
        .into_iter()
        .map(|value| {
            Plane::new(
                PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap(),
                vec![value],
            )
            .unwrap()
        })
        .collect();
    let rgb_frame = ImageFrame::new(rgb_descriptor, PixelBuffer::f32(rgb_planes).unwrap()).unwrap();
    let rgb_reader = NativePixelReader::inspect(&rgb_frame, &gray_options, &limits()).unwrap();
    assert_eq!(
        rgb_reader.pixel(0, 0).unwrap().color,
        NativeColor::Rgb([0.125, 0.5, 0.875])
    );

    let alpha_layout = PlaneLayout::interleaved(3, 1, 4, Subsampling::FULL).unwrap();
    let alpha_descriptor = ImageDescriptor::new(
        3,
        1,
        ChannelModel::RGB,
        vec![
            PlaneDescriptor::new(
                alpha_layout.clone(),
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
    .unwrap()
    .with_domain(SampleDomain::LinearRelative)
    .with_primaries(RgbPrimaries::srgb());
    let alpha_frame = ImageFrame::new(
        alpha_descriptor,
        PixelBuffer::f32(vec![
            Plane::new(
                alpha_layout,
                vec![
                    -0.5, 1.5, 0.0, 0.0, 2.0, -1.0, 0.25, 0.5, 0.1, 0.2, 0.3, 1.0,
                ],
            )
            .unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let alpha_reader = NativePixelReader::inspect(&alpha_frame, &gray_options, &limits()).unwrap();
    assert_eq!(
        alpha_reader.pixel(0, 0).unwrap(),
        super::NativePixel {
            color: NativeColor::Rgb([-0.5, 1.5, 0.0]),
            alpha: Some(0.0),
        }
    );
    assert_eq!(alpha_reader.pixel(2, 0).unwrap().alpha, Some(1.0));

    let bad_descriptor = ImageDescriptor::rgb(1, 1, 32)
        .unwrap()
        .with_domain(SampleDomain::LinearRelative)
        .with_primaries(RgbPrimaries::srgb());
    let bad_layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let bad_frame = ImageFrame::new(
        bad_descriptor,
        PixelBuffer::f32(
            [f32::NAN, 0.0, 0.0]
                .into_iter()
                .map(|sample| Plane::new(bad_layout.clone(), vec![sample]).unwrap())
                .collect(),
        )
        .unwrap(),
    );
    assert!(matches!(bad_frame, Err(HighresError::InvalidSamples(_))));

    let ycbcr_descriptor = ImageDescriptor::ycbcr(
        1,
        1,
        32,
        [Subsampling::FULL, Subsampling::FULL, Subsampling::FULL],
    )
    .unwrap()
    .with_domain(SampleDomain::LinearRelative)
    .with_primaries(RgbPrimaries::srgb());
    let planes = (0..3)
        .map(|_| {
            Plane::new(
                PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap(),
                vec![0.25],
            )
            .unwrap()
        })
        .collect();
    let ycbcr_frame = ImageFrame::new(ycbcr_descriptor, PixelBuffer::f32(planes).unwrap()).unwrap();
    gray_options.validate_for(&ycbcr_frame, &limits()).unwrap();
    let result = NativePixelReader::inspect(&ycbcr_frame, &gray_options, &limits());
    assert!(matches!(result, Err(ProcessingError::Unsupported(_))));
}
