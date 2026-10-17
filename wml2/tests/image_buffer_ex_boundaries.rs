#![cfg(feature = "image-buffer-ex")]

use wml2::draw::*;

fn words(values: impl IntoIterator<Item = u16>) -> Vec<u8> {
    values.into_iter().flat_map(u16::to_le_bytes).collect()
}

fn gray_samples(format: PixelFormatEx, bytes: &[u8]) -> Vec<u32> {
    if format.bits() == 8 {
        bytes.iter().map(|&v| u32::from(v)).collect()
    } else {
        bytes
            .chunks_exact(2)
            .map(|v| u32::from(u16::from_le_bytes([v[0], v[1]])))
            .collect()
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[test]
fn normalized_conversion_exhaustive_nearest_symmetry_and_exact_lattice() {
    // An independent oracle compares distances to adjacent output integers;
    // it does not reproduce the implementation's rounded-division expression.
    let formats = [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Gray16,
    ];
    for source in formats {
        let source_max = (1u32 << source.bits()) - 1;
        let input = if source.bits() == 8 {
            (0..=255).collect::<Vec<u8>>()
        } else {
            words((0..=source_max).map(|v| v as u16))
        };
        for destination in formats {
            let destination_max = (1u32 << destination.bits()) - 1;
            let output = convert_pixels_ex(
                &input,
                source_max as usize + 1,
                1,
                source,
                None,
                PixelOutputEx::new(destination, PrecisionConversion::Round),
            )
            .unwrap();
            let values = gray_samples(destination, output.bytes());
            assert_eq!(values[0], 0);
            assert_eq!(values[source_max as usize], destination_max);
            let lattice = source_max / gcd(source_max, destination_max);
            for (x, &y) in values.iter().enumerate() {
                let exact_position = x as i64 * i64::from(destination_max);
                let error = (i64::from(y) * i64::from(source_max) - exact_position).abs();
                assert!(
                    2 * error < i64::from(source_max),
                    "{source:?} -> {destination:?}, {x}"
                );
                if y > 0 {
                    assert!(
                        error < ((i64::from(y) - 1) * i64::from(source_max) - exact_position).abs()
                    );
                }
                if y < destination_max {
                    assert!(
                        error < ((i64::from(y) + 1) * i64::from(source_max) - exact_position).abs()
                    );
                }
                if x > 0 {
                    assert!(values[x - 1] <= y);
                }
                assert_eq!(y + values[source_max as usize - x], destination_max);
                let start = x * source.bytes_per_sample();
                let exact = convert_pixels_ex(
                    &input[start..start + source.bytes_per_sample()],
                    1,
                    1,
                    source,
                    None,
                    PixelOutputEx::new(destination, PrecisionConversion::Exact),
                );
                assert_eq!(exact.is_ok(), (x as u32).is_multiple_of(lattice));
                if let Ok(exact) = exact {
                    assert_eq!(gray_samples(destination, exact.bytes()), [y]);
                }
            }
        }
    }
}

#[test]
fn rgba16_narrowing_preserves_transparent_rgb_and_scales_alpha_independently() {
    let mut source = Vec::with_capacity(65536 * 8);
    for value in 0..=u16::MAX {
        source.extend(words([
            value,
            u16::MAX - value,
            value.rotate_left(8),
            value,
        ]));
    }
    for format in [PixelFormatEx::Rgba8, PixelFormatEx::Rgba12] {
        let image = convert_pixels_ex(
            &source,
            65536,
            1,
            PixelFormatEx::Rgba16,
            None,
            PixelOutputEx::new(format, PrecisionConversion::Round),
        )
        .unwrap();
        let maximum = (1i64 << format.bits()) - 1;
        let output = gray_samples(format, image.bytes());
        for value in 0..=u16::MAX {
            let originals = [value, u16::MAX - value, value.rotate_left(8), value];
            for (channel, original) in originals.into_iter().enumerate() {
                let sample = i64::from(output[value as usize * 4 + channel]);
                assert!((sample * 65535 - i64::from(original) * maximum).abs() * 2 < 65535);
            }
        }
        assert_eq!(&output[..4], &[0, maximum as u32, 0, 0]);
        assert_eq!(
            &output[65535 * 4..],
            &[maximum as u32, 0, maximum as u32, maximum as u32]
        );
    }
    let transparent = convert_pixels_ex(
        &words([0x1234, 0xabcd, 0xffff, 0]),
        1,
        1,
        PixelFormatEx::Rgba16,
        None,
        PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Round),
    )
    .unwrap();
    assert_eq!(transparent.bytes(), [18, 171, 255, 0]);
}

#[test]
fn conversion_draw_pick_commute_with_odd_padding_and_rectangle_partition() {
    for source_format in [PixelFormatEx::Gray12, PixelFormatEx::Rgba16] {
        for destination in [
            PixelFormatEx::Rgba8,
            PixelFormatEx::Rgba12,
            PixelFormatEx::Rgba16,
        ] {
            let (width, height) = (5, 3);
            let source_stride = width * source_format.bytes_per_pixel() + 3;
            let maximum = (1u32 << source_format.bits()) - 1;
            let mut bytes = vec![255; height * source_stride];
            for y in 0..height {
                for x in 0..width {
                    for channel in 0..source_format.channels() {
                        let value =
                            ((y * 1013 + x * 313 + channel * 1171) as u32 % (maximum + 1)) as u16;
                        let offset =
                            y * source_stride + x * source_format.bytes_per_pixel() + channel * 2;
                        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
                    }
                }
            }
            let packed = convert_pixels_ex(
                &bytes,
                width,
                height,
                source_format,
                Some(source_stride),
                PixelOutputEx::new(destination, PrecisionConversion::Round),
            )
            .unwrap();
            let mut source =
                ImageBufferEx::from_bytes(width, height, source_format, Some(source_stride), bytes)
                    .unwrap();
            let mut drawn = ImageBufferEx::with_storage(destination, PrecisionConversion::Exact);
            drawn
                .init_ex(
                    width,
                    height,
                    Some(InitOptionsEx {
                        source_format: destination,
                        source_stride: Some(3 * destination.bytes_per_pixel() + 1),
                        legacy: None,
                    }),
                )
                .unwrap();
            for (x, tile_width) in [(0, 2), (2, 3)] {
                let tile = source
                    .encode_pick_ex(
                        x,
                        0,
                        tile_width,
                        height,
                        PixelOutputEx {
                            format: destination,
                            stride: Some(3 * destination.bytes_per_pixel() + 1),
                            conversion: PrecisionConversion::Round,
                        },
                        None,
                    )
                    .unwrap()
                    .unwrap();
                for row in tile.bytes().chunks_exact(tile.stride()) {
                    assert!(
                        row[tile_width * destination.bytes_per_pixel()..]
                            .iter()
                            .all(|&v| v == 0)
                    );
                }
                drawn
                    .draw(x, 0, tile_width, height, tile.bytes(), None)
                    .unwrap();
            }
            assert_eq!(drawn.bytes().unwrap(), packed.bytes());
        }
    }
}

#[test]
fn late_precision_or_sample_errors_never_modify_visible_pixels() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba8, PrecisionConversion::Exact);
    image
        .init_ex(
            2,
            1,
            Some(InitOptionsEx {
                source_format: PixelFormatEx::Rgba16,
                ..Default::default()
            }),
        )
        .unwrap();
    image.draw(0, 0, 2, 1, &words([257; 8]), None).unwrap();
    let before = image.bytes().unwrap().to_vec();
    for bad_channel in 0..4 {
        let mut values = [65535; 8];
        values[4 + bad_channel] = 1;
        for x in [0, 1, 3] {
            // second pixel visible, clipped, or entirely off canvas
            assert!(image.draw(x, 0, 2, 1, &words(values), None).is_err());
            assert_eq!(image.bytes().unwrap(), before);
        }
    }
    let mut twelve = ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Exact);
    twelve
        .init_ex(
            1,
            2,
            Some(InitOptionsEx {
                source_format: PixelFormatEx::Rgba12,
                source_stride: Some(9),
                legacy: None,
            }),
        )
        .unwrap();
    let mut data = words([4095; 4]);
    data.push(255);
    data.extend(words([4095; 4]));
    data.push(255);
    twelve.draw(0, 0, 1, 2, &data, None).unwrap();
    let before = twelve.bytes().unwrap().to_vec();
    data[16] = 0x10; // final addressed alpha's high nibble; padding stays ignored
    assert!(twelve.draw(0, 0, 1, 2, &data, None).is_err());
    assert_eq!(twelve.bytes().unwrap(), before);
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn highres_import_revalidates_mutated_last_channel_and_ignores_padding() {
    use wml2::highres::*;
    let layout = PlaneLayout::new(1, 2, 6, 4, vec![0, 1, 2, 3], Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        1,
        2,
        ChannelModel::RGB,
        vec![
            PlaneDescriptor::new(
                layout.clone(),
                vec![
                    ChannelRole::Blue,
                    ChannelRole::Alpha,
                    ChannelRole::Red,
                    ChannelRole::Green,
                ],
                12,
            )
            .unwrap(),
        ],
    )
    .unwrap()
    .with_alpha(AlphaAssociation::Straight)
    .unwrap();
    let mut frame = ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![
            Plane::new(layout, vec![1, 2, 3, 4, 65535, 65535, 5, 6, 7, 8]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let copied = ImageBufferEx::from_highres_pixels(&frame).unwrap();
    assert_eq!(copied.bytes().unwrap(), words([3, 4, 1, 2, 7, 8, 5, 6]));
    if let PixelBuffer::U16(planes) = frame.pixels_mut() {
        planes.as_mut_slice()[0].samples_mut()[7] = 4096;
    } else {
        unreachable!();
    }
    assert!(ImageBufferEx::from_highres_pixels(&frame).is_err());
    assert_eq!(copied.bytes().unwrap(), words([3, 4, 1, 2, 7, 8, 5, 6]));
}
