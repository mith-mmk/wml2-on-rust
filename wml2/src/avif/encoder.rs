//! WML2 draw-side adapter for the standalone avifenc-rust encoder.

use std::collections::HashMap;

use crate::draw::{EncodeOptions, EncoderOptions, PickOptions};
use crate::error::{ImgError, ImgErrorKind};
use crate::metadata::DataMap;

type Error = Box<dyn std::error::Error>;

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
        avifenc_codec::encode_bytes(
            &avifenc_codec::ImageBuffer {
                width: profile.width,
                height: profile.height,
                rgba: data,
            },
            &options,
        )
        .map_err(map_error)
    })();
    let end_result = option.drawer.encode_end(None::<crate::draw::EndOptions>);
    match (result, end_result) {
        (Ok(data), Ok(())) => Ok(data),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
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
