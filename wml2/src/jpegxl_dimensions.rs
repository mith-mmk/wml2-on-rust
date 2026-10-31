//! Dimension-only projection: stop before sample/channel/profile descriptions.
//! Use the pinned codec's validated Size/Preview readers; never initialize a
//! decoder, ICC reader, frame/TOC parser, or output buffer on this route.
use crate::image_info::{ImageDimensions, ImageSize};
use jxl_codec::{
    bit_reader::BitReader,
    error::Error,
    headers::{JxlHeader, Orientation, Signature, Size, size::Preview},
};

pub(crate) fn size(s: &Size) -> ImageSize {
    ImageSize {
        width: s.xsize(),
        height: s.ysize(),
    }
}
pub(crate) fn mapped(
    coded: &Size,
    orientation: Orientation,
    intrinsic: Option<&Size>,
) -> ImageDimensions {
    let orient = |s: ImageSize| {
        if orientation.is_transposing() {
            ImageSize {
                width: s.height,
                height: s.width,
            }
        } else {
            s
        }
    };
    ImageDimensions {
        coded: size(coded),
        output: Some(orient(size(coded))),
        intrinsic_display: intrinsic.map(size).map(orient),
    }
}
pub(crate) struct Prefix {
    pub dimensions: ImageDimensions,
    pub orientation: u16,
    pub animated: bool,
}
pub(crate) fn prefix(bytes: &[u8]) -> Result<Prefix, Error> {
    let mut br = BitReader::new(bytes);
    Signature::read(&mut br)?;
    let coded = Size::read(&mut br)?;
    let all_default = br.read(1)? != 0;
    let extra = !all_default && br.read(1)? != 0;
    let mut orientation = Orientation::Identity;
    let mut intrinsic = None;
    let mut animated = false;
    if extra {
        orientation = match br.read(3)? {
            0 => Orientation::Identity,
            1 => Orientation::FlipHorizontal,
            2 => Orientation::Rotate180,
            3 => Orientation::FlipVertical,
            4 => Orientation::Transpose,
            5 => Orientation::Rotate90Cw,
            6 => Orientation::AntiTranspose,
            _ => Orientation::Rotate90Ccw,
        };
        if br.read(1)? != 0 {
            intrinsic = Some(Size::read(&mut br)?);
        }
        if br.read(1)? != 0 {
            Preview::read(&mut br)?;
        }
        animated = br.read(1)? != 0;
    }
    Ok(Prefix {
        dimensions: mapped(&coded, orientation, intrinsic.as_ref()),
        orientation: orientation as u16,
        animated,
    })
}
