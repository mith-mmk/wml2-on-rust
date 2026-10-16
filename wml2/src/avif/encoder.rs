//! WML2 draw-side adapter for the standalone avifenc-rust encoder.

use std::collections::HashMap;

use crate::draw::{EncodeOptions, EncoderOptions, PickOptions};
use crate::error::{ImgError, ImgErrorKind};
use crate::metadata::DataMap;

type Error = Box<dyn std::error::Error>;

/// Encode an explicitly requested EX layout. The callback converts storage
/// with Exact/Round; this adapter only unpacks8/12-bit Gray or straight RGBA
/// into the codec's native planes. It never interprets EX bytes as YUV.
#[cfg(feature = "image-buffer-ex")]
pub fn encode_ex(option: &mut crate::draw::EncodeOptionsEx) -> Result<Vec<u8>, Error> {
    use crate::draw::{PixelFormatEx, image_buffer_ex::validate_bytes};
    let format = option.output.format;
    if !matches!(
        format,
        PixelFormatEx::Gray8 | PixelFormatEx::Gray12 | PixelFormatEx::Rgba8 | PixelFormatEx::Rgba12
    ) {
        return Err(Box::new(ImgError::new_const(ImgErrorKind::NoSupportFormat,
            "EX AVIF output requires Gray/RGBA8 or12; choose an explicit precision conversion from16-bit storage".into())));
    }
    let profile = option.drawer.encode_start_ex(None)?.ok_or_else(|| {
        Box::new(ImgError::new_const(
            ImgErrorKind::EncodeError,
            "AVIF EX encoder did not receive image profiles".into(),
        )) as Error
    })?;
    let result = (|| {
        let width = profile.legacy.width;
        let height = profile.legacy.height;
        let (stride, length) = format.extent(width, height, option.output.stride)?;
        let limits = crate::limits::current();
        let pixels = width
            .checked_mul(height)
            .ok_or("AVIF pixel count overflow")?;
        crate::limits::check(pixels, limits.pixels, "AVIF encode pixels")?;
        crate::limits::check(1, limits.frames, "AVIF encode frames")?;
        let plane_bytes = pixels
            .checked_mul(format.channels())
            .and_then(|n| n.checked_mul(2))
            .ok_or("AVIF encode plane size overflow")?;
        crate::limits::check(
            length
                .checked_add(plane_bytes)
                .ok_or("AVIF encode workspace overflow")?,
            limits.expanded_bytes,
            "AVIF EX encode workspace",
        )?;
        let mut metadata = profile.legacy.metadata.unwrap_or_default();
        if let Some(tags) = option.drawer.metadata_ex()? {
            metadata.extend(tags);
        }
        if let Some(options) = &option.options {
            metadata.extend(options.clone());
        }
        let mut encoder = convert_options(Some(&metadata))?;
        // NativeColorInformation below is the sole colour authority. The
        // legacy RGBA8 backend's default YUV nclx must not conflict with it.
        encoder.color_information = None;
        let mut colors =
            avifenc_codec::NativeColorInformation::from_nclx(avifenc_codec::NclxColorInformation {
                color_primaries: 1,
                transfer_characteristics: 13,
                matrix_coefficients: if format.channels() == 1 { 1 } else { 0 },
                full_range_flag: true,
            });
        // Only an explicitly declared output profile is signalled. Source ICC
        // metadata is never renamed/applied and CICP is not inferred from it.
        if let Some(value) = metadata.remove("ICC Profile") {
            let DataMap::ICCProfile(profile) = value else {
                return Err(invalid_option("ICC Profile", "ICCProfile output bytes"));
            };
            crate::limits::check(profile.len(), limits.metadata_bytes, "AVIF output ICC")?;
            colors.icc_profile = Some(profile);
            colors.icc_color_type = Some(*b"prof");
        }
        let picked = option
            .drawer
            .encode_pick_ex(0, 0, width, height, option.output, None)?
            .ok_or("AVIF EX encoder did not receive pixels")?;
        if picked.width() != width
            || picked.height() != height
            || picked.format() != format
            || picked.stride() != stride
        {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::InvalidParameter,
                "AVIF EX picker returned a different layout than requested".into(),
            )));
        }
        validate_bytes(picked.bytes(), width, height, format, stride, length)?;
        let channels = format.channels();
        let mut samples = Vec::with_capacity(channels);
        for _ in 0..channels {
            let mut plane = Vec::new();
            plane.try_reserve_exact(pixels)?;
            samples.push(plane);
        }
        for y in 0..height {
            for x in 0..width {
                for (channel, plane) in samples.iter_mut().enumerate() {
                    let at = y * stride
                        + x * format.bytes_per_pixel()
                        + channel * format.bytes_per_sample();
                    plane.push(if format.bits() == 8 {
                        u16::from(picked.bytes()[at])
                    } else {
                        u16::from_le_bytes([picked.bytes()[at], picked.bytes()[at + 1]])
                    });
                }
            }
        }
        drop(picked);
        let mut planes = Vec::with_capacity(channels);
        // AV1 identity matrix uses G/B/R, while EX is always R/G/B/A.
        let order: &[usize] = if channels == 1 { &[0] } else { &[1, 2, 0, 3] };
        for (index, &channel) in order.iter().enumerate() {
            planes.push(avifenc_codec::PlaneBuffer {
                layout: avifenc_codec::PlaneLayout {
                    plane: if index == 3 { 3 } else { index as u8 },
                    width,
                    height,
                    subsampling_x: 0,
                    subsampling_y: 0,
                    sample_count: pixels,
                },
                samples: std::mem::take(&mut samples[channel]),
            });
        }
        let mut encoded = avifenc_codec::encode_native_bytes(
            &avifenc_codec::FrameBuffers {
                width,
                height,
                planes,
            },
            &avifenc_codec::NativeEncodeOptions {
                bit_depth: format.bits(),
                subsampling: if channels == 1 {
                    avifenc_codec::NativeSubsampling::Cs400
                } else {
                    avifenc_codec::NativeSubsampling::Cs444
                },
                encoder,
                color_information: colors,
            },
        )
        .map_err(map_error)?;
        normalize_alpha_reference(&mut encoded, channels == 4)?;
        crate::limits::check(encoded.len(), limits.expanded_bytes, "AVIF encoded output")?;
        Ok(encoded)
    })();
    let ended = option.drawer.encode_end_ex(None);
    match (result, ended) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(data), Ok(())) => Ok(data),
    }
}

pub fn encode(option: &mut EncodeOptions) -> Result<Vec<u8>, Error> {
    let profile = option
        .drawer
        .encode_start(None::<EncoderOptions>)?
        .ok_or_else(|| {
            Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                "AVIF encoder did not receive image profiles".to_string(),
            )) as Error
        })?;
    let result = (|| {
        if profile.width == 0 || profile.height == 0 {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::InvalidParameter,
                "AVIF image dimensions must be non-zero".to_string(),
            )) as Error);
        }
        let data = option
            .drawer
            .encode_pick(0, 0, profile.width, profile.height, None::<PickOptions>)?
            .ok_or_else(|| {
                Box::new(ImgError::new_const(
                    ImgErrorKind::EncodeError,
                    "AVIF encoder did not receive RGBA pixels".to_string(),
                )) as Error
            })?;
        let mut metadata = profile.metadata.unwrap_or_default();
        if let Some(encode_options) = option.options.as_ref() {
            metadata.extend(encode_options.clone());
        }
        let options = convert_options((!metadata.is_empty()).then_some(&metadata))?;
        let has_alpha = data.chunks_exact(4).any(|pixel| pixel[3] != 255);
        let mut encoded = avifenc_codec::encode_bytes(
            &avifenc_codec::ImageBuffer {
                width: profile.width,
                height: profile.height,
                rgba: data,
            },
            &options,
        )
        .map_err(map_error)?;
        normalize_alpha_reference(&mut encoded, has_alpha)?;
        Ok(encoded)
    })();
    let end_result = option.drawer.encode_end(None::<crate::draw::EndOptions>);
    match (result, end_result) {
        (Ok(data), Ok(())) => Ok(data),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

// avifenc-rust0.0.7 writes primary1 -> alpha2 auxl. Readers such as libavif
// require auxiliary2 -> primary1. Fix only the known producer schema in place,
// without searching ICC/AV1 payload bytes or changing offsets/codec samples.
// Already-correct output is accepted so an upstream fix stays compatible.
pub(crate) fn normalize_alpha_reference(data: &mut [u8], required: bool) -> Result<(), Error> {
    fn invalid() -> Error {
        Box::new(ImgError::new_const(
            ImgErrorKind::EncodeError,
            "unexpected AVIF encoder alpha-reference layout".into(),
        ))
    }
    fn find(
        data: &[u8],
        mut at: usize,
        end: usize,
        kind: &[u8; 4],
    ) -> Result<Option<std::ops::Range<usize>>, Error> {
        let mut found = None;
        while at < end {
            if end - at < 8 {
                return Err(invalid());
            }
            let size = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
            if size < 8 || size > end - at {
                return Err(invalid());
            }
            if &data[at + 4..at + 8] == kind {
                if found.is_some() {
                    return Err(invalid());
                }
                found = Some(at + 8..at + size);
            }
            at += size;
        }
        Ok(found)
    }
    let meta = find(data, 0, data.len(), b"meta")?.ok_or_else(invalid)?;
    if meta.len() < 4 || data[meta.start..meta.start + 4] != [0; 4] {
        return Err(invalid());
    }
    let Some(iref) = find(data, meta.start + 4, meta.end, b"iref")? else {
        return if required { Err(invalid()) } else { Ok(()) };
    };
    if iref.len() < 4 || data[iref.start..iref.start + 4] != [0; 4] {
        return Err(invalid());
    }
    let reference = find(data, iref.start + 4, iref.end, b"auxl")?.ok_or_else(invalid)?;
    if reference.len() != 6 {
        return Err(invalid());
    }
    let payload = &mut data[reference];
    match payload {
        [0, 1, 0, 1, 0, 2] => {
            payload[1] = 2;
            payload[5] = 1;
            Ok(())
        }
        [0, 2, 0, 1, 0, 1] => Ok(()),
        _ => Err(invalid()),
    }
}

fn convert_options(
    metadata: Option<&HashMap<String, DataMap>>,
) -> Result<avifenc_codec::EncoderOptions, Error> {
    let mut options = avifenc_codec::EncoderOptions::default();
    let Some(metadata) = metadata else {
        return Ok(options);
    };
    for (key, value) in metadata {
        match key.as_str() {
            "quality" | "qcolor" | "min" | "max" => options.quality = as_u8(value, key)?,
            "quality_alpha" | "qalpha" | "minalpha" | "maxalpha" => {
                options.quality_alpha = as_u8(value, key)?
            }
            "speed" => options.speed = as_speed(value, key)?,
            "jobs" => options.jobs = as_jobs(value, key)?,
            "lossless" => options.lossless = as_bool(value, key)?,
            "tilerowslog2" | "tile_rows_log2" => options.tile_rows_log2 = as_u8(value, key)?,
            "tilecolslog2" | "tile_cols_log2" => options.tile_cols_log2 = as_u8(value, key)?,
            "advanced" => {
                let DataMap::Ascii(value) = value else {
                    return Err(invalid_option(key, "ASCII comma-separated key=value list"));
                };
                for entry in value.split([',', ';']) {
                    let Some((name, setting)) = entry.split_once('=') else {
                        return Err(invalid_option(key, "key=value entries"));
                    };
                    options.advanced.insert(
                        name.trim().to_string(),
                        avifenc_codec::DataMap::Ascii(setting.trim().to_string()),
                    );
                }
            }
            "icc" | "icc_profile" | "cicp" => {
                return Err(non_srgb_option(key));
            }
            _ => {}
        }
    }
    Ok(options)
}

fn as_u8(value: &DataMap, key: &str) -> Result<u8, Error> {
    match value {
        DataMap::UInt(value) => Ok(u8::try_from(*value)?),
        _ => Err(invalid_option(key, "UInt")),
    }
}

fn as_speed(value: &DataMap, key: &str) -> Result<u8, Error> {
    match value {
        DataMap::UInt(value) => Ok(u8::try_from(*value)?),
        DataMap::Ascii(value) if value.eq_ignore_ascii_case("default") || value == "d" => Ok(6),
        _ => Err(invalid_option(key, "numeric speed or default")),
    }
}

fn as_jobs(value: &DataMap, key: &str) -> Result<usize, Error> {
    match value {
        DataMap::UInt(value) => Ok(usize::try_from(*value)?),
        DataMap::Ascii(value) if value.eq_ignore_ascii_case("all") => Ok(0),
        _ => Err(invalid_option(key, "numeric worker count or all")),
    }
}

fn as_bool(value: &DataMap, key: &str) -> Result<bool, Error> {
    match value {
        DataMap::UInt(value) => Ok(*value != 0),
        DataMap::Ascii(value) if value.eq_ignore_ascii_case("true") => Ok(true),
        DataMap::Ascii(value) if value.eq_ignore_ascii_case("false") => Ok(false),
        _ => Err(invalid_option(key, "boolean")),
    }
}

fn invalid_option(key: &str, expected: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::InvalidParameter,
        format!("AVIF option {key} must be {expected}"),
    ))
}

fn non_srgb_option(key: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::UnsupportedFeature,
        format!("wml2 AVIF encoding is fixed to standard sRGB; option {key} is not supported"),
    ))
}

fn map_error(error: avifenc_codec::EncoderError) -> Error {
    let kind = match error {
        avifenc_codec::EncoderError::InvalidParam(_) => ImgErrorKind::InvalidParameter,
        avifenc_codec::EncoderError::NotEnoughData(_) => ImgErrorKind::UnexpectedEof,
        avifenc_codec::EncoderError::Bitstream(_) => ImgErrorKind::IllegalData,
        avifenc_codec::EncoderError::Unsupported(_) => ImgErrorKind::UnsupportedFeature,
        avifenc_codec::EncoderError::Io(_) => ImgErrorKind::IOError,
        _ => ImgErrorKind::EncodeError,
    };
    Box::new(ImgError::new_const(kind, error.to_string()))
}

#[cfg(test)]
mod alpha_reference_tests {
    use super::normalize_alpha_reference;
    fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        [
            ((payload.len() + 8) as u32).to_be_bytes().as_slice(),
            kind,
            payload,
        ]
        .concat()
    }
    #[test]
    fn producer_reference_is_corrected_without_touching_other_payloads_and_is_idempotent() {
        let reference = boxed(b"auxl", &[0, 1, 0, 1, 0, 2]);
        let iref = boxed(b"iref", &[[0; 4].as_slice(), &reference].concat());
        let unrelated = boxed(b"free", b"auxl\0\x01\0\x01\0\x02");
        let mut data = boxed(b"meta", &[[0; 4].as_slice(), &unrelated, &iref].concat());
        let original = data.clone();
        normalize_alpha_reference(&mut data, true).unwrap();
        assert_eq!(data.len(), original.len());
        assert_eq!(
            data.iter().zip(&original).filter(|(a, b)| a != b).count(),
            2
        );
        assert!(
            data.windows(unrelated.len())
                .any(|bytes| bytes == unrelated)
        );
        let corrected = data.clone();
        normalize_alpha_reference(&mut data, true).unwrap();
        assert_eq!(data, corrected);
        for end in 0..data.len() {
            assert!(normalize_alpha_reference(&mut data[..end].to_vec(), true).is_err());
        }
    }
    #[test]
    fn no_alpha_and_malformed_producer_shapes_are_explicit() {
        let mut data = boxed(b"meta", &[0; 4]);
        normalize_alpha_reference(&mut data, false).unwrap();
        assert!(normalize_alpha_reference(&mut data, true).is_err());
        assert!(normalize_alpha_reference(&mut [0; 16], false).is_err());
    }
}
