//! Native AVIF ownership mapping.
//!
//! The native limit and codec error types are re-exported here so a caller
//! using the high-resolution bridge does not need a second direct dependency
//! on the standalone codec crate.

use super::allocation;
mod mapping;
#[cfg(test)]
mod mapping_tests;
#[cfg(test)]
mod sequence_tests;
mod metadata;
pub mod sequence;
pub use sequence::{AvifSequenceFrame, AvifSequenceInformation, AvifSequenceReader};

#[cfg(feature = "avif")]
pub use avif_codec::{DecoderError, NativeDecodeLimits};

pub(crate) use mapping::consume_native_frame;
pub(crate) use mapping::consume_native_frame_with_external_live;

/// Errors from the additive high-resolution AVIF bridge.
///
/// This enum is non-exhaustive because the bridge may expose additional
/// codec and mapping failure classes as high-resolution AVIF support grows.
/// Callers should match the stable `Codec` and `Processing` categories and
/// include a wildcard arm for forward compatibility.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Codec(avif_codec::DecoderError),
    Processing(super::ProcessingError),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codec(error) => error.fmt(formatter),
            Self::Processing(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<avif_codec::DecoderError> for DecodeError {
    fn from(error: avif_codec::DecoderError) -> Self {
        Self::Codec(error)
    }
}

impl From<super::ProcessingError> for DecodeError {
    fn from(error: super::ProcessingError) -> Self {
        Self::Processing(error)
    }
}

impl From<super::HighresError> for DecodeError {
    fn from(error: super::HighresError) -> Self {
        Self::Processing(super::ProcessingError::from(error))
    }
}

/// Decode one ordinary still AVIF into its native high-resolution planes.
///
/// This is an additive API: legacy AVIF decode continues to produce its
/// historical RGBA8 result.  The native decoder is called exactly once and
/// derived images, sequences, and geometry pixel application remain outside
/// this bridge.  Container geometry is retained in
/// [`crate::highres::FrameMetadata`].
#[cfg(feature = "avif")]
pub fn decode_native(
    data: &[u8],
    native_limits: &NativeDecodeLimits,
    resource_limits: &super::ResourceLimits,
) -> Result<super::ImageFrame, DecodeError> {
    if data.len() > resource_limits.max_input_bytes {
        return Err(DecodeError::Processing(
            super::ProcessingError::ResourceLimit(
                "AVIF input exceeds highres resource limits".into(),
            ),
        ));
    }
    let decoded = avif_codec::decode_frame_bytes_strict_with_limits(data, native_limits)?;
    let pixel_aspect_ratio = decoded.information().pixel_aspect_ratio();
    let (frame, rich) = decoded.into_frame_and_rich();
    let mut output = consume_native_frame(frame, &rich, resource_limits, None)?;
    if let Some((horizontal, vertical)) = pixel_aspect_ratio {
        let ratio = super::PixelAspectRatio::new(horizontal, vertical)?;
        output.metadata_mut().set_pixel_aspect_ratio(Some(ratio));
    }
    Ok(output)
}
