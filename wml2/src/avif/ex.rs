//! Bounded static AVIF → declared RGBA16LE. Codec colour reconstruction stays
//! in avif-rust; EX performs only its usual input-to-storage conversion.
use super::*;
use crate::draw::{InitOptionsEx, PixelFormatEx};
use std::io::SeekFrom;

fn unsupported(message: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::NoSupportFormat,
        message.into(),
    ))
}
fn codec_error(error: avif_codec::DecoderError) -> Error {
    Box::new(ImgError::new_const(
        map_decoder_error(&error),
        error.to_string(),
    ))
}

pub(crate) fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let limits = crate::limits::current();
    crate::limits::check(1, limits.frames, "AVIF static frames")?;
    let offset = reader.offset()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(offset))?;
    let length = usize::try_from(
        end.checked_sub(offset)
            .ok_or("invalid AVIF input position")?,
    )?;
    crate::limits::check(length, limits.input_bytes, "AVIF input")?;
    let input = reader.read_bytes_as_vec(length)?;
    // Bound the supported still profile independently of caller unlimited
    // limits. The dependency's native limits do not promise total process RSS.
    let native_limits = avif_codec::NativeDecodeLimits::new(
        limits.input_bytes,
        16_384,
        16_384,
        limits.pixels,
        limits.expanded_bytes,
        limits.metadata_bytes,
        limits.metadata_bytes,
        128,
        512,
        128,
        16,
        limits.frames.min(1),
    );
    let decoded = avif_codec::decode_frame_bytes_strict_with_limits(&input, &native_limits)
        .map_err(codec_error)?;
    let (mut frame, mut rich) = decoded.into_frame_and_rich();
    crate::avif::signaling::correct_alpha(&input, &mut frame, &mut rich)?;
    if !matches!(frame.bit_depth, 8 | 10 | 12) {
        return Err(unsupported("EX AVIF supports coded8/10/12-bit stills only"));
    }
    let source_premultiplied = frame.alpha_premultiplied;
    if rich.info.clean_aperture.is_some()
        || rich.info.rotation.is_some()
        || rich.info.mirror.is_some()
    {
        return Err(unsupported(
            "EX AVIF geometry transforms require the native API",
        ));
    }
    // Preserve encoded RGB coordinates; display transfer/ICC/tone mapping are
    // distinct operations. Associated samples are unassociated before CMS.
    let width = frame.width;
    let height = frame.height;
    let pixels = width
        .checked_mul(height)
        .ok_or("AVIF pixel count overflow")?;
    crate::limits::check(pixels, limits.pixels, "AVIF pixels")?;
    let bytes = pixels.checked_mul(8).ok_or("AVIF RGBA16 size overflow")?;
    let native_bytes = frame
        .buffers
        .planes
        .iter()
        .try_fold(0usize, |total, plane| {
            plane
                .samples
                .len()
                .checked_mul(2)
                .and_then(|n| total.checked_add(n))
                .ok_or("AVIF native plane size overflow")
        })?;
    crate::limits::check(
        bytes
            .checked_add(bytes.max(native_bytes))
            .ok_or("AVIF workspace overflow")?,
        limits.expanded_bytes,
        "AVIF EX conversion workspace",
    )?;
    let depth = frame.bit_depth;
    let color = frame.color_config;
    let rgba = crate::avif::reconstruction::rgba16(&frame).map_err(codec_error)?;
    if rgba.width != width || rgba.height != height || rgba.rgba.len() != pixels * 4 {
        return Err(unsupported("AVIF RGBA reconstruction layout mismatch"));
    }
    drop(frame);
    let mut data = crate::draw::zeroed_bytes(bytes)?;
    for (destination, sample) in data.chunks_exact_mut(2).zip(&rgba.rgba) {
        destination.copy_from_slice(&sample.to_le_bytes());
    }
    drop(rgba);
    option.drawer.init_ex(
        width,
        height,
        Some(InitOptionsEx {
            source_format: PixelFormatEx::Rgba16,
            source_stride: None,
            legacy: Some(InitOptions {
                loop_count: 0,
                background: None,
                animation: false,
            }),
        }),
    )?;
    option.drawer.draw(0, 0, width, height, &data, None)?;
    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("Avif".into()))?;
    option
        .drawer
        .set_metadata("AVIF Source Bit Depth", DataMap::UInt(u64::from(depth)))?;
    option.drawer.set_metadata(
        "Source AVIF Alpha Premultiplied",
        DataMap::UInt(u64::from(source_premultiplied)),
    )?;
    option.drawer.set_metadata(
        "Source AV1 Full Range",
        DataMap::UInt(u64::from(matches!(
            color.color_range,
            avif_codec::av1::ColorRange::Full
        ))),
    )?;
    if let Some(c) = color.color_description {
        option.drawer.set_metadata(
            "Source AV1 CICP",
            DataMap::UIntAllay(vec![
                u64::from(c.color_primaries),
                u64::from(c.transfer_characteristics),
                u64::from(c.matrix_coefficients),
                u64::from(matches!(
                    color.color_range,
                    avif_codec::av1::ColorRange::Full
                )),
            ]),
        )?;
    }
    if let Some(c) = rich.color_information.nclx {
        option.drawer.set_metadata(
            "Source AVIF nclx",
            DataMap::UIntAllay(vec![
                u64::from(c.color_primaries),
                u64::from(c.transfer_characteristics),
                u64::from(c.matrix_coefficients),
                u64::from(c.full_range_flag),
            ]),
        )?;
    }
    if let Some(icc) = rich.color_information.icc_profile.take() {
        option
            .drawer
            .set_metadata("Source ICC Profile", DataMap::ICCProfile(icc))?;
    }
    option.drawer.terminate(None)?;
    Ok(None)
}
