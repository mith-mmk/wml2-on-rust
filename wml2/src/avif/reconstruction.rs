//! Matrix/range reconstruction without display transfer, ICC or tone mapping.
pub(crate) fn rgba16(
    frame: &avif_codec::DecodedFrame,
) -> Result<avif_codec::Rgba16ImageBuffer, avif_codec::DecoderError> {
    let mut color = frame.color_config;
    if let Some(description) = color.color_description.as_mut() {
        // This suppresses only the pinned converter's display-transfer stage;
        // source primaries/matrix/range metadata remain unchanged in the frame.
        description.transfer_characteristics = 13;
    }
    let mut image = avif_codec::av1::frame_buffers_to_rgba_16(&frame.buffers, &color)?;
    if color.monochrome
        && let Some(alpha) = frame
            .buffers
            .planes
            .iter()
            .find(|plane| plane.layout.plane == 3)
    {
        // Native Gray+alpha has two planes with identities [0, 3]. The pinned
        // converter looks for alpha at vector index 3 and otherwise emits opaque.
        if alpha.layout.width != image.width
            || alpha.layout.height != image.height
            || alpha.layout.subsampling_x != 0
            || alpha.layout.subsampling_y != 0
            || alpha.samples.len() != image.rgba.len() / 4
        {
            return Err(avif_codec::DecoderError::Unsupported(
                "AVIF gray alpha requires a full-resolution plane".into(),
            ));
        }
        let max_source = (1u32 << color.bit_depth) - 1;
        for (pixel, &sample) in image.rgba.chunks_exact_mut(4).zip(&alpha.samples) {
            pixel[3] = ((u32::from(sample) * 65535 + max_source / 2) / max_source) as u16;
        }
    }
    if color.monochrome && matches!(color.color_range, avif_codec::av1::ColorRange::Studio) {
        // The pinned converter's monochrome branch scales storage depth but
        // omits video-range expansion. ICC needs normalized device gray.
        let luma =
            frame.buffers.planes.first().ok_or_else(|| {
                avif_codec::DecoderError::Bitstream("missing AVIF gray plane".into())
            })?;
        if luma.layout.width != image.width
            || luma.layout.height != image.height
            || luma.samples.len() != image.rgba.len() / 4
        {
            return Err(avif_codec::DecoderError::Unsupported(
                "AVIF gray range requires a full-resolution plane".into(),
            ));
        }
        let shift = color.bit_depth.checked_sub(8).ok_or_else(|| {
            avif_codec::DecoderError::Unsupported("AVIF gray depth below8".into())
        })?;
        let black = 16u32 << shift;
        let span = 219u32 << shift;
        for (pixel, &sample) in image.rgba.chunks_exact_mut(4).zip(&luma.samples) {
            let value = ((u32::from(sample).saturating_sub(black) * 65535 + span / 2) / span)
                .min(65535) as u16;
            pixel[..3].fill(value);
        }
    }
    if frame.alpha_premultiplied {
        unpremultiply(&mut image.rgba);
    }
    Ok(image)
}

fn unpremultiply(rgba: &mut [u16]) {
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = u64::from(pixel[3]);
        for sample in &mut pixel[..3] {
            *sample = if alpha == 0 {
                0
            } else {
                ((u64::from(*sample) * 65535 + alpha / 2) / alpha).min(65535) as u16
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gray_alpha_uses_plane_identity_and_unassociates_at_native_precision() {
        let bytes = include_bytes!("../../tests/fixtures/review/avif-icc-gray-linear.avif");
        let limits = avif_codec::NativeDecodeLimits::new(
            64 << 20,
            16384,
            16384,
            16 << 20,
            64 << 20,
            8 << 20,
            8 << 20,
            128,
            512,
            128,
            16,
            128,
        );
        let decoded = avif_codec::decode_frame_bytes_strict_with_limits(bytes, &limits).unwrap();
        let (mut frame, _) = decoded.into_frame_and_rich();
        let mut alpha = frame.buffers.planes[0].clone();
        alpha.layout.plane = 3;
        frame.buffers.planes.push(alpha);
        for depth in [8, 10, 12] {
            frame.bit_depth = depth;
            frame.color_config.bit_depth = depth;
            let max = (1u32 << depth) - 1;
            let gray = 1u16 << (depth - 2);
            let alpha = 1u16 << (depth - 1);
            frame.buffers.planes[0].samples.fill(gray);
            frame.buffers.planes[1].samples.fill(alpha);
            frame.buffers.planes[1].samples[0] = 0;
            let normalized = |sample: u16| ((u32::from(sample) * 65535 + max / 2) / max) as u16;
            for prem in [false, true] {
                frame.alpha_premultiplied = prem;
                let image = rgba16(&frame).unwrap();
                let mut expected = [normalized(gray); 4];
                expected[3] = normalized(alpha);
                if prem {
                    unpremultiply(&mut expected);
                }
                assert_eq!(&image.rgba[4..8], &expected, "{depth}-bit prem={prem}");
                assert_eq!(image.rgba[3], 0);
                assert_eq!(image.rgba[0], if prem { 0 } else { normalized(gray) });
            }
        }
        frame.buffers.planes[1].layout.width -= 1;
        assert!(rgba16(&frame).is_err());
    }

    #[test]
    fn encoded_rgb_is_unassociated_before_any_nonlinear_processing() {
        let mut data = [
            16384, 8192, 32768, 32768, 65535, 1, 12, 0, 1, 22222, 65535, 65535,
        ];
        unpremultiply(&mut data);
        assert_eq!(
            data,
            [
                32768, 16384, 65535, 32768, 0, 0, 0, 0, 1, 22222, 65535, 65535
            ]
        );
    }
}
