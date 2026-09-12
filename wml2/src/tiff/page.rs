//! Convert explicit directory boundaries into image page descriptors.
use super::{
    header::Tiff,
    ifd::{TiffDocument, invalid},
};
use bin_rs::reader::BinaryReader;
type Error = Box<dyn std::error::Error>;
pub(crate) fn read_pages(reader: &mut dyn BinaryReader) -> Result<Tiff, Error> {
    let doc = TiffDocument::read(reader)?;
    let mut pages = Vec::new();
    for ifd in &doc.ifds {
        for entry in &ifd.entries {
            if matches!(entry.tag, 0x102 | 0x153) && entry.count == 0 {
                return Err(invalid("TIFF sample description cannot be empty"));
            }
            if matches!(
                entry.tag,
                0x100
                    | 0x101
                    | 0x103
                    | 0x106
                    | 0x10a
                    | 0x112
                    | 0x115
                    | 0x116
                    | 0x11c
                    | 0x13d
                    | 0x142
                    | 0x143
            ) && entry.count != 1
            {
                return Err(invalid("TIFF scalar tag must contain one value"));
            }
            if matches!(
                entry.tag,
                0x102 | 0x103 | 0x106 | 0x10a | 0x115 | 0x11c | 0x13d | 0x140 | 0x152 | 0x153
            ) && entry.type_id != 3
            {
                return Err(invalid("TIFF sample description tag must have SHORT type"));
            }
            if matches!(entry.tag, 0x100 | 0x101 | 0x116 | 0x142 | 0x143)
                && !matches!(entry.type_id, 3 | 4 | 16)
            {
                return Err(invalid("Invalid TIFF dimension field type"));
            }
        }
        let mut page = Tiff::from_headers(ifd.headers(doc.variant, doc.endian)?)?;
        for entry in &ifd.entries {
            match entry.tag {
                0xfe | 0xff => {
                    let values = entry.unsigned(doc.endian)?;
                    if values.len() != 1 {
                        return Err(invalid("Invalid TIFF subfile type count"));
                    }
                    let value = u32::try_from(values[0])?;
                    if entry.tag == 0xfe {
                        page.newsubfiletype = value
                    } else {
                        page.subfiletype = value
                    }
                }
                0x111 => page.strip_offsets = entry.unsigned(doc.endian)?,
                0x117 => page.strip_byte_counts = entry.unsigned(doc.endian)?,
                0x144 => page.tile_offsets = entry.unsigned(doc.endian)?,
                0x145 => page.tile_byte_counts = entry.unsigned(doc.endian)?,
                0x100 | 0x101 | 0x116 | 0x142 | 0x143 if matches!(entry.type_id, 16 | 18) => {
                    let values = entry.unsigned(doc.endian)?;
                    if values.len() != 1 {
                        return Err(invalid("Invalid TIFF dimension count"));
                    }
                    let v = u32::try_from(values[0])
                        .map_err(|_| invalid("TIFF dimensions exceed supported range"))?;
                    match entry.tag {
                        0x100 => page.width = v,
                        0x101 => page.height = v,
                        0x116 => page.rows_per_strip = v,
                        0x142 => page.tile_width = v,
                        _ => page.tile_length = v,
                    }
                }
                0x112 => {
                    let values = entry.unsigned(doc.endian)?;
                    if values.len() != 1 {
                        return Err(invalid("Invalid orientation count"));
                    }
                    page.orientation = u32::try_from(values[0])?;
                }
                0x153 => {
                    let v = entry.unsigned(doc.endian)?;
                    if v.len() != 1 && v.len() != usize::from(page.samples_per_pixel) {
                        return Err(invalid(
                            "TIFF SampleFormat count differs from SamplesPerPixel",
                        ));
                    }
                    if v.iter().any(|v| *v != 1) {
                        return Err(Box::new(crate::error::ImgError::new_const(
                            crate::error::ImgErrorKind::NoSupportFormat,
                            "TIFF non-unsigned SampleFormat is unsupported".into(),
                        )));
                    }
                }
                _ => {}
            }
        }
        if page.bitspersamples.len() == 1 && page.samples_per_pixel > 1 {
            page.bitspersamples = vec![page.bitspersamples[0]; usize::from(page.samples_per_pixel)];
            page.bitspersample = page.bitspersamples[0]
                .checked_mul(page.samples_per_pixel)
                .ok_or_else(|| invalid("TIFF BitsPerSample overflow"))?;
        }
        if page.width == 0 || page.height == 0 {
            return Err(invalid("TIFF image dimensions must be nonzero"));
        }
        let pixels = usize::try_from(page.width)?
            .checked_mul(usize::try_from(page.height)?)
            .ok_or_else(|| invalid("TIFF image dimensions overflow"))?;
        crate::limits::check(pixels, crate::limits::current().pixels, "TIFF pixels")?;
        if page.samples_per_pixel == 0
            || page.bitspersamples.len() != usize::from(page.samples_per_pixel)
            || page.bitspersamples.contains(&0)
        {
            return Err(invalid("Invalid TIFF sample count or bit depth"));
        }
        if !matches!(page.planar_config, 1 | 2) {
            return Err(invalid("Invalid TIFF PlanarConfiguration"));
        }
        if !matches!(page.fill_order, 1 | 2) {
            return Err(invalid("Invalid TIFF FillOrder"));
        }
        let base_channels = match page.photometric_interpretation {
            0 | 1 | 3 => Some(1),
            2 | 6 => Some(3),
            5 => Some(4),
            _ => None,
        };
        if let Some(base) = base_channels {
            let channels = usize::from(page.samples_per_pixel);
            if channels < base {
                return Err(invalid(
                    "TIFF has fewer samples than its color model requires",
                ));
            }
            if channels > base + 1 {
                return Err(Box::new(crate::error::ImgError::new_const(
                    crate::error::ImgErrorKind::NoSupportFormat,
                    "TIFF extra channels are unsupported".into(),
                )));
            }
            if ifd.entries.iter().any(|e| e.tag == 0x152)
                && page.extra_samples.len() != channels - base
            {
                return Err(invalid(
                    "TIFF ExtraSamples count differs from color channels",
                ));
            }
            if page.extra_samples.iter().any(|v| *v > 2) {
                return Err(invalid("Invalid TIFF ExtraSamples value"));
            }
        }
        if page
            .bitspersamples
            .iter()
            .any(|b| *b != page.bitspersamples[0])
        {
            return Err(Box::new(crate::error::ImgError::new_const(
                crate::error::ImgErrorKind::NoSupportFormat,
                "TIFF mixed sample bit depths are unsupported".into(),
            )));
        }
        if !matches!(page.predictor, 1 | 2) {
            return Err(Box::new(crate::error::ImgError::new_const(
                crate::error::ImgErrorKind::NoSupportFormat,
                "Unsupported TIFF Predictor".into(),
            )));
        }
        if let Some(entry) = ifd.entries.iter().find(|e| e.tag == 0x140) {
            let depth = page.bitspersamples[0];
            let expected = 1u64
                .checked_shl(u32::from(depth))
                .and_then(|v| v.checked_mul(3));
            if expected != Some(entry.count) {
                return Err(invalid("Invalid TIFF ColorMap length"));
            }
        }
        if page.photometric_interpretation == 3 && page.color_table.is_none() {
            return Err(invalid("Palette TIFF requires ColorMap"));
        }
        pages.push(page);
    }
    let mut iter = pages.into_iter();
    let mut first = iter
        .next()
        .ok_or_else(|| invalid("TIFF contains no image IFD"))?;
    first.multi_page = Box::new(iter.collect());
    Ok(first)
}
