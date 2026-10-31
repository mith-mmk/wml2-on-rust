//! JPEG format support.

pub mod decoder;
pub mod encoder;
pub mod header;
pub mod progressive;
pub mod util;
pub mod warning;

mod scaled;
pub use scaled::{JpegScale, ScaledJpegImage, image_from_scaled, image_from_scaled_with_limits};
