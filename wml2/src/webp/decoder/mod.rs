//! Lower-level WebP parsing and decoding APIs.

pub mod alpha;
pub mod animation;
pub mod header;
pub mod lossless;
pub mod lossy;
pub mod quant;
pub mod tree;
pub mod vp8;
pub mod vp8i;

type Error = Box<dyn std::error::Error>;

use crate::color::RGBA;
use crate::draw::{
    DecodeOptions, ImageRect, InitOptions, NextBlend, NextDispose, NextOption, NextOptions,
    ResponseCommand,
};
use crate::error::{ImgError, ImgErrorKind};
use crate::warning::ImgWarnings;
use bin_rs::io::read_u32_le;
use bin_rs::reader::BinaryReader;
use std::fmt::{Display, Formatter, Result as FmtResult};

pub use alpha::{apply_alpha_plane, decode_alpha_plane, AlphaHeader};
pub use animation::{decode_animation_webp, DecodedAnimation, DecodedAnimationFrame};
pub use header::{
    get_features, parse_animation_webp, parse_still_webp, AnimationHeader, ChunkHeader,
    ParsedAnimationFrame, ParsedAnimationWebp, ParsedWebp, Vp8xHeader, WebpFeatures,
};
pub use lossless::{decode_lossless_vp8l_to_rgba, decode_lossless_webp_to_rgba};
pub use lossy::{
    decode_lossy_vp8_to_rgba, decode_lossy_vp8_to_yuv, decode_lossy_webp_to_rgba,
    decode_lossy_webp_to_yuv, DecodedImage, DecodedYuvImage,
};
pub use vp8::{
    parse_lossy_headers, parse_macroblock_data, parse_macroblock_headers, LosslessInfo,
    LossyHeader, MacroBlockData, MacroBlockDataFrame, MacroBlockHeaders,
};
pub use vp8i::WebpFormat;

fn argb_to_rgba(argb: u32) -> RGBA {
    RGBA {
        red: ((argb >> 16) & 0xff) as u8,
        green: ((argb >> 8) & 0xff) as u8,
        blue: (argb & 0xff) as u8,
        alpha: (argb >> 24) as u8,
    }
}

fn map_error(error: DecoderError) -> Error {
    let kind = match error {
        DecoderError::InvalidParam(_) => ImgErrorKind::InvalidParameter,
        DecoderError::NotEnoughData(_) => ImgErrorKind::UnexpectedEof,
        DecoderError::Bitstream(_) => ImgErrorKind::IllegalData,
        DecoderError::Unsupported(_) => ImgErrorKind::UnsupportedFeature,
    };
    Box::new(ImgError::new_const(kind, error.to_string()))
}

fn read_container<B: BinaryReader>(reader: &mut B) -> Result<Vec<u8>, Error> {
    let header = reader.read_bytes_no_move(12)?;
    if header.len() < 12 || &header[0..4] != b"RIFF" || &header[8..12] != b"WEBP" {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::IllegalData,
            "not a WebP RIFF container".to_string(),
        )));
    }

    let riff_size = read_u32_le(&header, 4) as usize;
    let total_size = riff_size + 8;
    if total_size < 12 {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::IllegalData,
            "invalid WebP container length".to_string(),
        )));
    }

    Ok(reader.read_bytes_as_vec(total_size)?)
}

fn next_options(frame: &ParsedAnimationFrame<'_>) -> NextOptions {
    NextOptions {
        flag: NextOption::Continue,
        await_time: frame.duration as u64,
        image_rect: Some(ImageRect {
            start_x: frame.x_offset as i32,
            start_y: frame.y_offset as i32,
            width: frame.width,
            height: frame.height,
        }),
        dispose_option: Some(if frame.dispose_to_background {
            NextDispose::Background
        } else {
            NextDispose::None
        }),
        blend: Some(if frame.blend {
            NextBlend::Source
        } else {
            NextBlend::Override
        }),
    }
}

fn decode_frame_rgba(frame: &ParsedAnimationFrame<'_>) -> Result<DecodedImage, DecoderError> {
    let image = match &frame.image_chunk.fourcc {
        b"VP8L" => {
            if frame.alpha_chunk.is_some() {
                return Err(DecoderError::Bitstream(
                    "VP8L animation frame must not carry ALPH chunk",
                ));
            }
            lossless::decode_lossless_vp8l_to_rgba(frame.image_data)?
        }
        b"VP8 " => lossy::decode_lossy_vp8_frame_to_rgba(frame.image_data, frame.alpha_data)?,
        _ => {
            return Err(DecoderError::Bitstream(
                "unsupported animation frame chunk",
            ))
        }
    };

    if image.width != frame.width || image.height != frame.height {
        return Err(DecoderError::Bitstream(
            "animation frame dimensions do not match bitstream",
        ));
    }
    Ok(image)
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let data = read_container(reader)?;
    let (metadata, warnings) = crate::webp::utils::make_metadata(&data).map_err(map_error)?;
    let features = get_features(&data).map_err(map_error)?;

    if features.has_animation {
        let parsed = parse_animation_webp(&data).map_err(map_error)?;
        let init = InitOptions {
            loop_count: parsed.animation.loop_count as u32,
            background: Some(argb_to_rgba(parsed.animation.background_color)),
            animation: true,
        };
        option
            .drawer
            .init(parsed.features.width, parsed.features.height, Some(init))?;

        let mut allow_multi_image = false;
        for (index, frame) in parsed.frames.iter().enumerate() {
            let decoded = decode_frame_rgba(frame).map_err(map_error)?;
            if index == 0 {
                option.drawer.draw(
                    frame.x_offset,
                    frame.y_offset,
                    frame.width,
                    frame.height,
                    &decoded.rgba,
                    None,
                )?;

                let result = option.drawer.next(Some(next_options(frame)))?;
                if let Some(response) = result {
                    if response.response == ResponseCommand::Continue {
                        allow_multi_image = true;
                        option
                            .drawer
                            .draw(0, 0, frame.width, frame.height, &decoded.rgba, None)?;
                    }
                }
                continue;
            }

            if !allow_multi_image {
                continue;
            }

            let result = option.drawer.next(Some(next_options(frame)))?;
            if let Some(response) = result {
                if response.response == ResponseCommand::Abort {
                    break;
                }
            }

            option
                .drawer
                .draw(0, 0, frame.width, frame.height, &decoded.rgba, None)?;
        }
    } else {
        let init = InitOptions {
            loop_count: 0,
            background: None,
            animation: false,
        };
        option
            .drawer
            .init(features.width, features.height, Some(init))?;

        let decoded = match features.format {
            WebpFormat::Lossy => lossy::decode_lossy_webp_to_rgba(&data).map_err(map_error)?,
            WebpFormat::Lossless => {
                lossless::decode_lossless_webp_to_rgba(&data).map_err(map_error)?
            }
            WebpFormat::Undefined => {
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::UnsupportedFeature,
                    "unsupported WebP format".to_string(),
                )))
            }
        };

        option
            .drawer
            .draw(0, 0, decoded.width, decoded.height, &decoded.rgba, None)?;
    }

    for (key, value) in &metadata {
        option.drawer.set_metadata(key, value.clone())?;
    }
    option.drawer.terminate(None)?;

    Ok(warnings)
}

/// Error type used by decoding and parsing entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderError {
    /// A caller-provided buffer size or dimension is invalid.
    InvalidParam(&'static str),
    /// The input ended before a required structure was fully available.
    NotEnoughData(&'static str),
    /// The bitstream violates the WebP container or codec format.
    Bitstream(&'static str),
    /// The input uses a feature that is intentionally not implemented.
    Unsupported(&'static str),
}

impl Display for DecoderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::InvalidParam(msg) => write!(f, "invalid parameter: {msg}"),
            Self::NotEnoughData(msg) => write!(f, "not enough data: {msg}"),
            Self::Bitstream(msg) => write!(f, "bitstream error: {msg}"),
            Self::Unsupported(msg) => write!(f, "unsupported feature: {msg}"),
        }
    }
}

impl std::error::Error for DecoderError {}
