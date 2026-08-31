//! Checked typed image representations for explicit high-bit-depth processing.
//!
//! This namespace is opt-in and deliberately separate from the historical
//! RGBA8 drawing API. It stores native samples and signaling without implicit
//! colour conversion, geometry application, or precision reduction.

mod metadata;
mod types;

pub use metadata::{
    Av1ColorInformation, ColorInformationSet, ColorProvenance, FrameMetadata, NclxColorInformation,
    PixelAspectRatio, Rect, Rotation,
};
pub use types::{
    AlphaAssociation, ChannelModel, ChannelRole, FrameTiming, HighresError, ImageDescriptor,
    ImageFrame, PixelBuffer, PixelFormat, Plane, PlaneDescriptor, PlaneLayout, Planes, Result,
    Subsampling,
};
