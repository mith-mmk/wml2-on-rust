mod bitwriter;
mod encoder;
mod fdct;
mod huffman;
mod quantize_table;

use crate::draw::EncodeOptions as DrawEncodeOptions;
use crate::error::{ImgError, ImgErrorKind};
use crate::metadata::DataMap;

type Error = Box<dyn std::error::Error>;

pub use self::encoder::create_qt;

fn jpeg_quality(option: &DrawEncodeOptions<'_>) -> usize {
    option
        .options
        .as_ref()
        .and_then(|map| map.get("quality"))
        .and_then(|value| match value {
            DataMap::UInt(v) => Some(*v as usize),
            DataMap::SInt(v) if *v > 0 => Some(*v as usize),
            _ => None,
        })
        .unwrap_or(80)
        .clamp(1, 100)
}

pub fn encode(image: &mut DrawEncodeOptions<'_>) -> Result<Vec<u8>, Error> {
    let profile = image.drawer.encode_start(None)?;
    let profile = profile.ok_or_else(|| {
        Box::new(ImgError::new_const(
            ImgErrorKind::OutboundIndex,
            "Image profiles nothing".to_string(),
        )) as Error
    })?;

    let rgba = image
        .drawer
        .encode_pick(0, 0, profile.width, profile.height, None)?
        .ok_or_else(|| {
            Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                "Image buffer nothing".to_string(),
            )) as Error
        })?;

    let inner = self::encoder::EncodeOptions::new(
        profile.width,
        profile.height,
        &rgba,
        jpeg_quality(image),
    );
    let data = self::encoder::encode(&inner)?;
    image.drawer.encode_end(None)?;
    Ok(data)
}
