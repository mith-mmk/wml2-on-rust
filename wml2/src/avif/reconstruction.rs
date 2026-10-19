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
