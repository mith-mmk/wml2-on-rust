//! Explicit full-precision unsigned 16-bit TIFF decoding.
use super::{
    AlphaAssociation, ChannelModel, ChannelRole, ColorInformationSet, FrameMetadata,
    ImageDescriptor, ImageFrame, PixelBuffer, Plane, PlaneDescriptor, PlaneLayout, Subsampling,
};
use crate::{limits::DecodeLimits, metadata::DataMap, tiff::header::Tiff};
use bin_rs::reader::BytesReader;

type Error = Box<dyn std::error::Error>;
fn unsupported(message: &str) -> Error {
    Box::new(crate::error::ImgError::new_const(
        crate::error::ImgErrorKind::NoSupportFormat,
        message.into(),
    ))
}
/// Decode the first full-resolution page into Gray16, GrayAlpha16, RGB16 or RGBA16.
/// Orientation is retained as metadata; samples are not rotated.
pub fn decode_native(data: &[u8], limits: &DecodeLimits) -> Result<ImageFrame, Error> {
    crate::limits::scope(*limits, || {
        let mut reader = BytesReader::new(data);
        let page = Tiff::new(&mut reader)?;
        let display = std::iter::once(&page)
            .chain(page.multi_page.iter())
            .find(|page| crate::tiff::page::is_display_page(page))
            .ok_or_else(|| unsupported("TIFF contains no full-resolution image page"))?;
        decode_page(&mut reader, display)
    })
}
/// Decode every full-resolution image page into an independent full-precision frame.
pub fn decode_native_pages(data: &[u8], limits: &DecodeLimits) -> Result<Vec<ImageFrame>, Error> {
    crate::limits::scope(*limits, || {
        let mut reader = BytesReader::new(data);
        let first = Tiff::new(&mut reader)?;
        let mut frames = Vec::new();
        let mut total = 0usize;
        for page in std::iter::once(&first)
            .chain(first.multi_page.iter())
            .filter(|page| crate::tiff::page::is_display_page(page))
        {
            let bytes = usize::try_from(page.width)?
                .checked_mul(usize::try_from(page.height)?)
                .and_then(|n| n.checked_mul(usize::from(page.samples_per_pixel)))
                .and_then(|n| n.checked_mul(2))
                .ok_or_else(|| crate::tiff::ifd::invalid("TIFF native frame size overflow"))?;
            total = total
                .checked_add(bytes)
                .ok_or_else(|| crate::tiff::ifd::invalid("TIFF native animation size overflow"))?;
            crate::limits::check(total, limits.animation_bytes, "TIFF native pages")?;
            frames.push(decode_page(&mut reader, page)?);
        }
        if frames.is_empty() {
            return Err(unsupported("TIFF contains no full-resolution image page"));
        }
        Ok(frames)
    })
}
fn decode_page(
    reader: &mut dyn bin_rs::reader::BinaryReader,
    page: &Tiff,
) -> Result<ImageFrame, Error> {
    if page.bitspersamples.is_empty() || page.bitspersamples.iter().any(|v| *v != 16) {
        return Err(unsupported(
            "Native TIFF API requires unsigned 16-bit samples",
        ));
    }
    let (model, mut roles) = match page.photometric_interpretation {
        0 | 1 => (ChannelModel::Gray, vec![ChannelRole::Gray]),
        2 => (
            ChannelModel::RGB,
            vec![ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue],
        ),
        _ => return Err(unsupported("Native TIFF API supports Gray and RGB only")),
    };
    let channels = usize::from(page.samples_per_pixel);
    let alpha = if channels == roles.len() + 1 {
        roles.push(ChannelRole::Alpha);
        match page.extra_samples.first() {
            Some(1) => AlphaAssociation::Premultiplied,
            Some(2) => AlphaAssociation::Straight,
            // The typed image model cannot label an unspecified extra channel.
            // Do not turn unknown samples into alpha or silently discard them.
            None | Some(0) => {
                return Err(unsupported(
                    "Native TIFF cannot represent unspecified extra samples",
                ));
            }
            _ => return Err(unsupported("Unknown TIFF alpha association")),
        }
    } else if channels == roles.len() {
        AlphaAssociation::None
    } else {
        return Err(unsupported("Native TIFF has unsupported extra channels"));
    };
    let mut samples = crate::tiff::decoder::decode_samples_u16(reader, page)?;
    if page.photometric_interpretation == 0 {
        for pixel in samples.chunks_exact_mut(channels) {
            pixel[0] = u16::MAX - pixel[0];
        }
    }
    let layout = PlaneLayout::interleaved(page.width, page.height, channels, Subsampling::FULL)?;
    let plane_descriptor = PlaneDescriptor::new(layout.clone(), roles, 16)?;
    let mut color = ColorInformationSet::default();
    if let Some(icc) = &page.icc_profile {
        color.set_icc_profile(icc.clone())?;
    }
    let descriptor = ImageDescriptor::new(page.width, page.height, model, vec![plane_descriptor])?
        .with_alpha(alpha)?
        .with_color_information(color.clone());
    let mut metadata = FrameMetadata::new(color);
    metadata.tags_mut().insert(
        "Tiff headers".into(),
        DataMap::Exif(page.tiff_headers.clone()),
    );
    metadata.tags_mut().insert(
        "Orientation".into(),
        DataMap::UInt(u64::from(page.orientation)),
    );
    if let Some(icc) = &page.icc_profile {
        metadata
            .tags_mut()
            .insert("ICC Profile".into(), DataMap::ICCProfile(icc.clone()));
    }
    Ok(ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![Plane::new(layout, samples)?])?,
    )?
    .with_metadata(metadata))
}
