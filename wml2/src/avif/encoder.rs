//! WML2 draw-side adapter for the standalone avifenc-rust encoder.

use std::collections::HashMap;

use crate::draw::{EncodeOptions, EncoderOptions, PickOptions};
use crate::error::{ImgError, ImgErrorKind};
use crate::metadata::DataMap;

type Error = Box<dyn std::error::Error>;

fn icc_color_model(value: Option<&DataMap>) -> Result<Option<icc_profile::ColorSpace>, Error> {
    let Some(DataMap::ICCProfile(bytes)) = value else {
        return Ok(None);
    };
    crate::limits::check(
        bytes.len(),
        crate::limits::current().metadata_bytes,
        "AVIF ICC metadata",
    )?;
    // Container parsing identifies channels without compiling a CMS route.
    // Opaque explicitly supplied profiles remain compatible with the old API.
    Ok(icc_profile::Profile::parse(bytes)
        .ok()
        .map(|profile| profile.color_space()))
}

/// Encode an explicitly requested EX layout. The callback converts storage
/// with Exact/Round; this adapter borrows those bytes as the shared codec's
/// packed view. It never interprets EX bytes as YUV or changes stored precision.
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
        let gray_icc_expansion = profile.format.channels() == 1
            && format.channels() == 4
            && matches!(metadata.get("ICC Profile"), Some(DataMap::ICCProfile(_)));
        if let Some(options) = &option.options {
            metadata.extend(options.clone());
        }
        validate_output_icc(
            &metadata,
            option.options.as_ref(),
            format,
            gray_icc_expansion,
        )?;
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
        let options = avifenc_codec::EncodeOptionsEx {
            native: avifenc_codec::NativeEncodeOptions {
                bit_depth: format.bits(),
                subsampling: if format.channels() == 1 {
                    avifenc_codec::NativeSubsampling::Cs400
                } else {
                    avifenc_codec::NativeSubsampling::Cs444
                },
                encoder,
                color_information: colors,
            },
            // WML2's picker already applied the requested Exact/Round policy,
            // including alpha. Do not introduce a second precision conversion.
            precision: avifenc_codec::buffer::PrecisionPolicy::Exact,
            color: avifenc_codec::ColorPolicyEx::Preserve,
        };
        let nclx = options.native.color_information.nclx.unwrap();
        // These are caller-prepared output samples and explicit output metadata,
        // not the source file's ICC/CICP. No source profile is inferred/applied.
        let view = packed_view_ex(
            &picked,
            avifenc_codec::buffer::ColorMetadataRef {
                cicp: Some(avifenc_codec::buffer::Cicp {
                    primaries: nclx.color_primaries,
                    transfer: nclx.transfer_characteristics,
                    matrix: nclx.matrix_coefficients,
                }),
                icc: options.native.color_information.icc_profile.as_deref(),
            },
        );
        // Validate backend controls before compact native plane allocation.
        avifenc_codec::preflight_ex(view, &options).map_err(map_ex_error)?;
        let encoded = avifenc_codec::encode_ex(view, &options)
            .map_err(map_ex_error)?
            .bytes;
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

#[cfg(feature = "image-buffer-ex")]
fn validate_output_icc(
    metadata: &HashMap<String, DataMap>,
    explicit: Option<&HashMap<String, DataMap>>,
    format: crate::draw::PixelFormatEx,
    gray_icc_expansion: bool,
) -> Result<(), Error> {
    use icc_profile::ColorSpace;
    let expected = if format.channels() == 1 {
        ColorSpace::Gray
    } else {
        ColorSpace::Rgb
    };
    if gray_icc_expansion
        && icc_color_model(explicit.and_then(|options| options.get("ICC Profile")))?
            != Some(ColorSpace::Rgb)
    {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "AVIF EX Gray storage ICC cannot describe expanded RGB samples; explicitly convert and supply a compatible RGB output ICC in encoder options, or request native Gray output".into(),
        )));
    }
    if icc_color_model(metadata.get("ICC Profile"))?.is_some_and(|space| space != expected) {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "AVIF EX output ICC color model differs from the requested samples; explicitly convert and supply a compatible output ICC, or request native Gray output".into(),
        )));
    }
    if icc_color_model(metadata.get("Source ICC Profile"))?.is_some_and(|space| space != expected)
        && icc_color_model(explicit.and_then(|options| options.get("ICC Profile")))?
            != Some(expected)
    {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "AVIF EX source ICC color model differs from the requested samples; explicitly convert and supply a compatible output ICC in encoder options, or request native Gray output".into(),
        )));
    }
    Ok(())
}

fn validate_ordinary_output_icc(metadata: &HashMap<String, DataMap>) -> Result<(), Error> {
    for key in ["ICC Profile", "Source ICC Profile"] {
        if icc_color_model(metadata.get(key))?
            .is_some_and(|space| space != icc_profile::ColorSpace::Rgb)
        {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::UnsupportedFeature,
                "ordinary AVIF cannot encode a known non-RGB ICC source as RGB; use the EX encoder for native Gray output, or explicitly convert and supply a compatible EX output ICC".into(),
            )));
        }
    }
    Ok(())
}

// WML2 ImageBufferEx and the shared codec model have different contracts.
// Keep this bridge private: no owned shared buffer or intermediate U16 copy.
#[cfg(feature = "image-buffer-ex")]
fn packed_view_ex<'a>(
    picked: &'a crate::draw::PixelBytesEx,
    color: avifenc_codec::buffer::ColorMetadataRef<'a>,
) -> avifenc_codec::buffer::ImageViewEx<'a> {
    use avifenc_codec::buffer::*;
    let gray = picked.format().channels() == 1;
    ImageViewEx::Packed(PackedImageViewEx {
        plane: PlaneViewEx {
            samples: if picked.format().bits() == 8 {
                SamplesRef::U8(picked.bytes())
            } else {
                // Explicit LE bytes also support unaligned/odd byte strides.
                SamplesRef::U16LeBytes(picked.bytes())
            },
            layout: PlaneLayoutEx {
                width: picked.width(),
                height: picked.height(),
                valid_bits: picked.format().bits(),
                offset_bytes: 0,
                row_stride_bytes: picked.stride(),
            },
        },
        channels: if gray {
            PackedChannels::Gray
        } else {
            PackedChannels::Rgba
        },
        alpha: if gray {
            AlphaMode::Absent
        } else {
            AlphaMode::Straight
        },
        color,
    })
}

#[cfg(feature = "image-buffer-ex")]
fn map_ex_error(error: avifenc_codec::EncodeErrorEx) -> Error {
    use avifenc_codec::EncodeErrorEx;
    let message = error.to_string();
    let kind = match error {
        EncodeErrorEx::Buffer(_) | EncodeErrorEx::ColorConflict(_) => {
            ImgErrorKind::InvalidParameter
        }
        EncodeErrorEx::ConversionRequired(_) | EncodeErrorEx::Unsupported(_) => {
            ImgErrorKind::UnsupportedFeature
        }
        EncodeErrorEx::Codec(error) => return map_error(error),
    };
    Box::new(ImgError::new_const(kind, message))
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
        let mut metadata = profile.metadata.unwrap_or_default();
        validate_ordinary_output_icc(&metadata)?;
        if let Some(encode_options) = option.options.as_ref() {
            // This legacy encoder does not signal ICC. An option must not
            // hide an incompatible profile already attached to its input.
            validate_ordinary_output_icc(encode_options)?;
            metadata.extend(encode_options.clone());
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
        let options = convert_options((!metadata.is_empty()).then_some(&metadata))?;
        let encoded = avifenc_codec::encode_bytes(
            &avifenc_codec::ImageBuffer {
                width: profile.width,
                height: profile.height,
                rgba: data,
            },
            &options,
        )
        .map_err(map_error)?;
        Ok(encoded)
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

#[cfg(all(test, feature = "image-buffer-ex"))]
mod shared_view_tests {
    use super::packed_view_ex;
    use crate::draw::*;
    use avifenc_codec::buffer::*;

    #[test]
    fn all_wml2_formats_borrow_bytes_with_depth_stride_and_alpha_intact() {
        for format in [
            PixelFormatEx::Gray8,
            PixelFormatEx::Rgba8,
            PixelFormatEx::Gray12,
            PixelFormatEx::Rgba12,
            PixelFormatEx::Gray16,
            PixelFormatEx::Rgba16,
        ] {
            let (w, h) = (2, 2);
            let stride = w * format.bytes_per_pixel() + 1;
            let mut raw = vec![0xee; stride * h];
            for y in 0..h {
                for x in 0..w {
                    for c in 0..format.channels() {
                        let value = (1 + y * 100 + x * 10 + c) as u16;
                        let at =
                            y * stride + (x * format.channels() + c) * format.bytes_per_sample();
                        raw[at..at + format.bytes_per_sample()]
                            .copy_from_slice(&value.to_le_bytes()[..format.bytes_per_sample()]);
                    }
                }
            }
            let mut image =
                crate::draw::ImageBufferEx::from_bytes(w, h, format, Some(stride), raw.clone())
                    .unwrap();
            let mut output = PixelOutputEx::new(format, PrecisionConversion::Exact);
            output.stride = Some(stride);
            let picked = image
                .encode_pick_ex(0, 0, w, h, output, None)
                .unwrap()
                .unwrap();
            let ImageViewEx::Packed(view) = packed_view_ex(&picked, ColorMetadataRef::default())
            else {
                panic!("packed input")
            };
            // Type-check the decoder and encoder re-exports against each other.
            let shared: avif_codec::buffer::ImageViewEx<'_> = ImageViewEx::Packed(view);
            shared.validate().unwrap();
            let borrowed = match view.plane.samples {
                SamplesRef::U8(b) => {
                    assert_eq!(format.bits(), 8);
                    b
                }
                SamplesRef::U16LeBytes(b) => {
                    assert!(format.bits() > 8);
                    b
                }
                _ => panic!("must borrow bytes, never cast to numeric U16"),
            };
            assert_eq!(borrowed.as_ptr(), picked.bytes().as_ptr());
            assert_eq!(view.plane.layout.row_stride_bytes, stride);
            assert_eq!(view.plane.layout.valid_bits, format.bits());
            assert_eq!(
                view.alpha,
                if format.channels() == 1 {
                    AlphaMode::Absent
                } else {
                    AlphaMode::Straight
                }
            );
            assert_eq!(view.plane.sample(1, 1, 0, format.channels()).unwrap(), 111);
            assert_eq!(image.bytes().unwrap(), raw);
        }
    }
}
