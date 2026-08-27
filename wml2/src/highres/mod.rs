//! Checked native and high-precision image representations.
//!
//! The types in this module are deliberately separate from the historical
//! [`crate::draw::ImageBuffer`]. They retain native planes and metadata and do
//! not perform implicit colour conversion, geometry, or precision reduction.

mod hdr;
mod metadata;
mod types;

pub use hdr::{HlgDisplayConditions, hlg_oetf, hlg_scene_from_signal, pq_eotf, pq_oetf};
pub use metadata::{
    Av1ColorInformation, ColorInformationSet, ColorProvenance, FrameMetadata, NclxColorInformation,
    PixelAspectRatio, Rect, Rotation,
};
pub use types::{
    AlphaAssociation, ChannelModel, ChannelRole, FrameTiming, HighresError, ImageDescriptor,
    ImageFrame, PixelBuffer, PixelFormat, Plane, PlaneDescriptor, PlaneLayout, Planes, Result,
    Subsampling,
};
