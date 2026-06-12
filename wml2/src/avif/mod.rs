//! AVIF support backed by the standalone `avif-rust` codec crate.

/// AVIF decoding APIs plus the `wml2` draw adapter.
pub mod decoder;

pub use avif_codec::{
    AvifInfo, ColorInformation, DecoderError, ImageSpatialExtents, PixelInformation,
};
