//! AVIF support backed by the standalone `avif-rust` and `avifenc-rust` crates.

/// AVIF decoding APIs plus the `wml2` draw adapter.
pub mod decoder;
#[cfg(feature = "avifenc")]
pub mod encoder;

pub use avif_codec::{
    AvifInfo, ColorInformation, DecoderError, ImageSpatialExtents, PixelInformation,
};
