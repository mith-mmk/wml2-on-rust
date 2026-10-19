//! Still-image display boundary: reconstruct, unassociate, then own-library ICC.
use super::*;
use crate::draw::{CallbackResponse, ResponseCommand};

fn accepted(response: Option<CallbackResponse>, option: &mut DecodeOptions) -> Result<bool, Error> {
    if matches!(response,Some(r) if r.response==ResponseCommand::Abort) {
        option.drawer.terminate(None)?;
        return Ok(false);
    }
    Ok(true)
}
fn metadata(option: &mut DecodeOptions, key: &str, value: DataMap) -> Result<bool, Error> {
    let response = option.drawer.set_metadata(key, value)?;
    accepted(response, option)
}

pub(super) fn decode(
    input: &[u8],
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let limits = crate::limits::current();
    crate::limits::check(1, limits.frames, "AVIF static frames")?;
    let decoded = avif_codec::decode_frame_bytes_strict_with_limits(input, &native_limits(limits))
        .map_err(|e| map_error(Box::new(e)))?;
    crate::limits::charge_metadata(decoded.information().metadata_bytes())?;
    let (mut frame, mut rich) = decoded.into_frame_and_rich();
    crate::avif::signaling::correct_alpha(input, &mut frame, &mut rich)?;
    if rich.info.clean_aperture.is_some()
        || rich.info.rotation.is_some()
        || rich.info.mirror.is_some()
    {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::NoSupportFormat,
            "AVIF ICC/associated-alpha geometry requires the native API".into(),
        )));
    }
    let (width, height) = (frame.width, frame.height);
    let pixels = width
        .checked_mul(height)
        .ok_or("AVIF pixel count overflow")?;
    crate::limits::check(pixels, limits.pixels, "AVIF pixels")?;
    let rgba16_bytes = pixels.checked_mul(8).ok_or("AVIF RGBA16 overflow")?;
    let rgba8_bytes = pixels.checked_mul(4).ok_or("AVIF RGBA8 overflow")?;
    let output_bytes = rgba16_bytes
        .checked_add(rgba8_bytes)
        .ok_or("AVIF output workspace overflow")?;
    let native_bytes = frame.buffers.planes.iter().try_fold(0usize, |sum, p| {
        p.samples
            .len()
            .checked_mul(2)
            .and_then(|n| sum.checked_add(n))
            .ok_or("AVIF native workspace overflow")
    })?;
    crate::limits::check(
        rgba16_bytes
            .checked_add(native_bytes.max(rgba8_bytes))
            .ok_or("AVIF workspace overflow")?,
        limits.expanded_bytes,
        "AVIF conversion workspace",
    )?;
    let color = frame.color_config;
    let source_gray = color.monochrome;
    let source_premultiplied = frame.alpha_premultiplied;
    let source_depth = frame.bit_depth;
    let source_icc = rich.color_information.icc_profile.take();
    let source_nclx = rich.color_information.nclx;
    drop(rich);
    // The codec's legacy to_rgba16 runs display transfer before its own ICC.
    // This shared primitive performs neither stage, and unassociates first.
    let mut rgba =
        crate::avif::reconstruction::rgba16(&frame).map_err(|e| map_error(Box::new(e)))?;
    if rgba.width != width || rgba.height != height || rgba.rgba.len() != pixels * 4 {
        return Err(std::io::Error::other("AVIF reconstruction layout mismatch").into());
    }
    drop(frame);
    let active_icc = if let Some(profile) = source_icc.as_deref() {
        crate::limits::charge_metadata("Source ICC Profile".len())?;
        crate::limits::charge_metadata(crate::color_convert::SRGB_ICC.len() + "ICC Profile".len())?;
        crate::color_convert::rgba16_to_rgb_profile(
            &mut rgba.rgba,
            profile,
            crate::color_convert::SRGB_ICC,
            source_gray,
            limits.expanded_bytes.saturating_sub(output_bytes),
        )?;
        let mut copy = Vec::new();
        copy.try_reserve_exact(crate::color_convert::SRGB_ICC.len())?;
        copy.extend_from_slice(crate::color_convert::SRGB_ICC);
        Some(copy)
    } else {
        // These transfers already use encoded RGB in the codec's existing
        // ordinary API. Other associated-alpha display routes need an explicit
        // source profile rather than applying transfer before alpha division.
        if color.color_description.is_some_and(|c| {
            !matches!(
                c.transfer_characteristics,
                1 | 2 | 6 | 7 | 11 | 12 | 13 | 14 | 15
            )
        }) {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::NoSupportFormat,
                "AVIF associated-alpha display transfer requires an explicit source ICC".into(),
            )));
        }
        None
    };
    let mut data = crate::draw::zeroed_bytes(rgba8_bytes)?;
    for (out, &sample) in data.iter_mut().zip(&rgba.rgba) {
        *out = ((u32::from(sample) * 255 + 32767) / 65535) as u8;
    }
    drop(rgba);
    if option.debug_flag > 0 {
        let response = option
            .drawer
            .verbose("AVIF bounded ICC/alpha display conversion", None)?;
        if !accepted(response, option)? {
            return Ok(None);
        }
    }
    let response = option.drawer.init(
        width,
        height,
        Some(InitOptions {
            animation: false,
            loop_count: 0,
            background: None,
        }),
    )?;
    if !accepted(response, option)? {
        return Ok(None);
    }
    let response = option.drawer.draw(0, 0, width, height, &data, None)?;
    if !accepted(response, option)? {
        return Ok(None);
    }
    for (key, value) in [
        ("Format", DataMap::Ascii("Avif".into())),
        ("width", DataMap::UInt(width as u64)),
        ("height", DataMap::UInt(height as u64)),
        (
            "AVIF Source Bit Depth",
            DataMap::UInt(u64::from(source_depth)),
        ),
        (
            "Source AVIF Alpha Premultiplied",
            DataMap::UInt(u64::from(source_premultiplied)),
        ),
    ] {
        if !metadata(option, key, value)? {
            return Ok(None);
        }
    }
    if let Some(profile) = source_icc
        && !metadata(option, "Source ICC Profile", DataMap::ICCProfile(profile))?
    {
        return Ok(None);
    }
    if let Some(profile) = active_icc
        && !metadata(option, "ICC Profile", DataMap::ICCProfile(profile))?
    {
        return Ok(None);
    }
    if let Some(nclx) = source_nclx
        && !metadata(
            option,
            "Source AVIF nclx",
            DataMap::UIntAllay(vec![
                u64::from(nclx.color_primaries),
                u64::from(nclx.transfer_characteristics),
                u64::from(nclx.matrix_coefficients),
                u64::from(nclx.full_range_flag),
            ]),
        )?
    {
        return Ok(None);
    }
    if let Some(c) = color.color_description
        && !metadata(
            option,
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
        )?
    {
        return Ok(None);
    }
    option.drawer.terminate(None)?;
    Ok(None)
}
