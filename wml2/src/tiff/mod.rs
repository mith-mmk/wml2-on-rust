//! TIFF format support, including EXIF-oriented metadata parsing.

pub mod decoder;
pub mod encoder;
pub mod header;
pub mod tags;
pub(crate) mod util;
pub mod warning;
