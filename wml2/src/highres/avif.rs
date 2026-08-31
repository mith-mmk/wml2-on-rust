//! Thin adapter from the standalone AVIF native decoder to typed WML2 frames.

use avif_codec::av1::ColorRange;
use avif_codec::{DecodedFrame, RichAvifInfo};

pub use avif_codec::NativeDecodeLimits;

use super::{
    AlphaAssociation, Av1ColorInformation, ChannelModel, ChannelRole, CleanAperture,
    ColorInformationSet, FrameMetadata, HighresError, ImageDescriptor, ImageFrame, PixelBuffer,
    Plane, PlaneDescriptor, PlaneLayout, Rotation, Subsampling,
};

/// Errors raised while mapping one native AVIF still image.
#[derive(Debug)]
pub enum DecodeError {
    Codec(avif_codec::DecoderError),
    Frame(HighresError),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codec(error) => error.fmt(formatter),
            Self::Frame(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<avif_codec::DecoderError> for DecodeError {
    fn from(error: avif_codec::DecoderError) -> Self {
        Self::Codec(error)
    }
}

impl From<HighresError> for DecodeError {
    fn from(error: HighresError) -> Self {
        Self::Frame(error)
    }
}

/// Decode one ordinary AVIF still image into native typed planes.
///
/// The standalone decoder's strict native entry point is used exactly once;
/// it retains its input, dimension, plane, and malformed-container limits.
/// Derived Sample Transform items remain an explicit unsupported result until
/// the codec exposes a bounded native derived-image entry point.
pub fn decode_native(data: &[u8], limits: &NativeDecodeLimits) -> Result<ImageFrame, DecodeError> {
    let decoded = avif_codec::decode_frame_bytes_strict_with_limits(data, limits)?;
    let pixel_aspect_ratio = decoded.information().pixel_aspect_ratio();
    let (frame, rich) = decoded.into_frame_and_rich();
    map_frame(frame, &rich, pixel_aspect_ratio)
}

fn map_frame(
    frame: DecodedFrame,
    rich: &RichAvifInfo,
    pixel_aspect_ratio: Option<(u32, u32)>,
) -> Result<ImageFrame, DecodeError> {
    let width = u32::try_from(frame.width).map_err(|_| {
        HighresError::InvalidDimensions("native AVIF width exceeds WML2 dimensions".into())
    })?;
    let height = u32::try_from(frame.height).map_err(|_| {
        HighresError::InvalidDimensions("native AVIF height exceeds WML2 dimensions".into())
    })?;
    if frame.buffers.width != frame.width || frame.buffers.height != frame.height {
        return Err(HighresError::InvalidLayout(
            "native AVIF buffer dimensions differ from frame".into(),
        )
        .into());
    }
    if !(8..=16).contains(&frame.bit_depth) {
        return Err(HighresError::Unsupported(
            "native AVIF bit depth is outside the typed WML2 range".into(),
        )
        .into());
    }
    let colors = color_information(&frame, rich)?;
    let monochrome = frame.color_config.monochrome;
    let identity = frame
        .color_config
        .color_description
        .is_some_and(|description| description.matrix_coefficients == 0);
    let mut descriptors = Vec::with_capacity(frame.buffers.planes.len());
    let mut planes = Vec::with_capacity(frame.buffers.planes.len());
    let mut has_alpha = false;
    for native_plane in frame.buffers.planes {
        let role = native_role(native_plane.layout.plane, monochrome, identity)?;
        if role == ChannelRole::Alpha {
            has_alpha = true;
        }
        let x_factor = 1u8
            .checked_shl(u32::from(native_plane.layout.subsampling_x))
            .ok_or_else(|| {
                HighresError::InvalidLayout("native AVIF X subsampling overflows".into())
            })?;
        let y_factor = 1u8
            .checked_shl(u32::from(native_plane.layout.subsampling_y))
            .ok_or_else(|| {
                HighresError::InvalidLayout("native AVIF Y subsampling overflows".into())
            })?;
        let subsampling = if role == ChannelRole::Alpha {
            Subsampling::FULL
        } else {
            Subsampling::new(x_factor, y_factor)?
        };
        let plane_width = u32::try_from(native_plane.layout.width).map_err(|_| {
            HighresError::InvalidDimensions(
                "native AVIF plane width exceeds WML2 dimensions".into(),
            )
        })?;
        let plane_height = u32::try_from(native_plane.layout.height).map_err(|_| {
            HighresError::InvalidDimensions(
                "native AVIF plane height exceeds WML2 dimensions".into(),
            )
        })?;
        if native_plane.layout.sample_count != native_plane.samples.len() {
            return Err(HighresError::InvalidLayout(
                "native AVIF plane sample count does not match storage".into(),
            )
            .into());
        }
        let layout = PlaneLayout::planar(plane_width, plane_height, subsampling);
        let layout = layout?;
        let descriptor = PlaneDescriptor::planar(layout.clone(), role, frame.bit_depth)?;
        descriptors.push(descriptor);
        planes.push(Plane::new(layout, native_plane.samples)?);
    }
    let model = if monochrome {
        ChannelModel::Gray
    } else if identity {
        ChannelModel::RGB
    } else {
        ChannelModel::YCbCr
    };
    let mut descriptor = ImageDescriptor::new(width, height, model, descriptors)?
        .with_color_information(colors.clone());
    if has_alpha {
        descriptor = descriptor.with_alpha(if frame.alpha_premultiplied {
            AlphaAssociation::Premultiplied
        } else {
            AlphaAssociation::Straight
        })?;
    }
    let mut metadata = FrameMetadata::new(colors);
    metadata.set_coded_dimensions(Some((width, height)));
    metadata.set_render_dimensions(Some((
        u32::try_from(frame.render_width).map_err(|_| {
            HighresError::InvalidDimensions("native AVIF render width overflows".into())
        })?,
        u32::try_from(frame.render_height).map_err(|_| {
            HighresError::InvalidDimensions("native AVIF render height overflows".into())
        })?,
    )));
    if let Some(aperture) = rich.info.clean_aperture {
        metadata.set_clean_aperture(Some(CleanAperture::new(
            aperture.width_n,
            aperture.width_d,
            aperture.height_n,
            aperture.height_d,
            aperture.horizontal_offset_n,
            aperture.horizontal_offset_d,
            aperture.vertical_offset_n,
            aperture.vertical_offset_d,
        )?));
    }
    if let Some(rotation) = rich.info.rotation {
        metadata.set_rotation(match rotation.angle {
            0 => Rotation::None,
            1 => Rotation::Degrees90,
            2 => Rotation::Degrees180,
            _ => Rotation::Degrees270,
        });
    }
    if let Some(mirror) = rich.info.mirror {
        metadata.set_mirror(mirror.axis == 0, mirror.axis != 0);
    }
    if let Some((horizontal, vertical)) = pixel_aspect_ratio {
        metadata.set_pixel_aspect_ratio(Some(super::PixelAspectRatio::new(horizontal, vertical)?));
    }
    Ok(ImageFrame::new(descriptor, PixelBuffer::u16(planes)?)?.with_metadata(metadata))
}

fn native_role(
    plane_id: u8,
    monochrome: bool,
    identity: bool,
) -> Result<ChannelRole, HighresError> {
    Ok(match plane_id {
        0 if monochrome => ChannelRole::Gray,
        0 if identity => ChannelRole::Green,
        1 if identity => ChannelRole::Blue,
        2 if identity => ChannelRole::Red,
        0 => ChannelRole::Y,
        1 => ChannelRole::Cb,
        2 => ChannelRole::Cr,
        3 => ChannelRole::Alpha,
        _ => {
            return Err(HighresError::Unsupported(
                "native AVIF plane id is unsupported".into(),
            ));
        }
    })
}

fn color_information(
    frame: &DecodedFrame,
    rich: &RichAvifInfo,
) -> Result<ColorInformationSet, HighresError> {
    let mut colors = ColorInformationSet::new();
    if let Some(profile) = &rich.color_information.icc_profile {
        colors.set_icc_profile(profile.clone())?;
    }
    if let Some(nclx) = rich.color_information.nclx {
        colors.set_nclx(super::NclxColorInformation::new(
            nclx.color_primaries,
            nclx.transfer_characteristics,
            nclx.matrix_coefficients,
            nclx.full_range_flag,
        ));
    }
    if let Some(description) = frame.color_config.color_description {
        colors.set_av1(Av1ColorInformation::new(
            u16::from(description.color_primaries),
            u16::from(description.transfer_characteristics),
            u16::from(description.matrix_coefficients),
            matches!(frame.color_config.color_range, ColorRange::Full),
        ));
    }
    Ok(colors)
}
