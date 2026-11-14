//! Private, block-oriented color conversion shared by decoders.
//!
//! Decoders own encoded sample interpretation and output buffers. A converter
//! owns only its per-image prepared transform and bounded reusable scratch.
#[cfg(any(feature = "jpeg", feature = "tiff", feature = "psd"))]
#[path = "cmyk.rs"]
pub(crate) mod cmyk;

#[cfg(any(
    feature = "avif",
    all(feature = "jpeg-xl", feature = "color-management")
))]
#[path = "color_convert_icc.rs"]
mod rgb;
#[cfg(any(
    feature = "avif",
    all(feature = "jpeg-xl", feature = "color-management")
))]
pub(crate) use rgb::*;
