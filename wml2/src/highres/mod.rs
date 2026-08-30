//! Checked native and high-precision image representations.
//!
//! The types in this module are deliberately separate from the historical
//! [`crate::draw::ImageBuffer`]. They retain native planes and metadata and do
//! not perform implicit colour conversion, geometry, or precision reduction.

mod allocation;
#[cfg(feature = "avif")]
pub mod avif;
pub mod icc;
mod convert;
mod domain;
mod hdr;
mod limits;
mod metadata;
mod output_plan;
mod ownership;
mod processing;
mod types;

pub use convert::convert_frame;
pub use icc::{IccConversionOptions, convert_frame_with_icc};
pub use convert::{
    AlphaPolicy, ChromaLocation, ChromaPhase, ColorConvertOptions, Destination, MatrixCoefficients,
    NativeSampleEncoding, RenderingIntent, SampleRange, SourceInterpretation, WhiteAdaptation,
    ZeroAlphaPolicy,
};
pub use domain::{RgbPrimaries, SampleDomain};
pub use hdr::{HlgDisplayConditions, hlg_oetf, hlg_scene_from_signal, pq_eotf, pq_oetf};
pub use limits::{ResourceLimits, ResourceLimitsBuilder};
pub use metadata::{
    Av1ColorInformation, Av1Description, CleanAperture, ColorInformationSet, ColorProvenance,
    ConversionAlpha, ConversionDestination, ConversionIntent, ConversionPrimaries, ConversionSource,
    ConversionWhiteAdaptation, FrameMetadata, GeometryOperation, IccColorType,
    IccConversionAuthority, LastConversion, NativeInterpretation, NclxColorInformation,
    PixelAspectRatio, PixelChannelInformation,
    PixelInformation, PixelSubsampling, RawRational, Rect, RepetitionCount, Rotation,
    SequenceInformation, UnknownColorInformation,
};
pub use processing::ProcessingError;
pub use types::{
    AlphaAssociation, ChannelModel, ChannelRole, FrameTiming, HighresError, ImageDescriptor,
    ImageFrame, PixelBuffer, PixelFormat, Plane, PlaneDescriptor, PlaneLayout, Planes, Result,
    Subsampling,
};

#[cfg(feature = "avif")]
#[allow(unused_imports)]
pub(crate) use avif::consume_native_frame;
#[cfg(feature = "avif")]
pub(crate) use metadata::AV1_COLOR_INFORMATION_BYTES;
