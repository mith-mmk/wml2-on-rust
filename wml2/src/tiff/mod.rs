//! TIFF format support, including EXIF-oriented metadata parsing.

#[cfg(feature = "tiff")]
pub(crate) mod block;
#[cfg(feature = "tiff")]
pub(crate) mod color;
#[cfg(feature = "exif")]
pub(crate) mod ifd;
#[cfg(feature = "exif")]
pub(crate) mod page;
#[cfg(feature = "tiff")]
pub(crate) mod predictor;
#[cfg(feature = "tiff")]
pub(crate) mod sample;

#[cfg(feature = "tiff")]
pub mod decoder;
#[cfg(feature = "tiff")]
pub(crate) mod encode_ifd;
#[cfg(feature = "tiff")]
pub mod encoder;
#[cfg(feature = "exif")]
pub mod header;
#[cfg(feature = "exif")]
pub mod tags;
#[cfg(feature = "exif")]
pub(crate) mod util;
#[cfg(feature = "tiff")]
pub mod warning;
