//! Static unsigned TIFF pixels through the existing native block reader.
use super::*;

fn unsupported(message: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::NoSupportFormat,
        message.into(),
    ))
}

pub(crate) fn decode_ex_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    // The common page parser checks SampleFormat (including every channel),
    // defaults an absent tag to unsigned and rejects signed/float/undefined
    // declarations before this EX initializer is reached.
    let document = Tiff::new(reader)?;
    let mut pages = std::iter::once(&document)
        .chain(document.multi_page.iter())
        .filter(|page| is_display_page(page));
    let page = pages
        .next()
        .ok_or_else(|| unsupported("TIFF contains no display page"))?;
    if pages.next().is_some() {
        return Err(unsupported("EX multi-page TIFF is unsupported"));
    }
    let channels = usize::from(page.samples_per_pixel);
    let colors = match page.photometric_interpretation {
        0 | 1 => 1,
        2 => 3,
        _ => return Err(unsupported("EX TIFF supports Gray/RGB only")),
    };
    let alpha = if channels == colors + 1 {
        if page.extra_samples.as_slice() != [2] {
            return Err(unsupported("EX TIFF requires explicitly straight alpha"));
        }
        true
    } else if channels == colors && page.extra_samples.is_empty() {
        false
    } else {
        return Err(unsupported("EX TIFF has unsupported extra samples"));
    };
    let depth = *page
        .bitspersamples
        .first()
        .ok_or_else(|| unsupported("TIFF sample depth missing"))?;
    if !matches!(depth, 8 | 16)
        || page.bitspersamples.len() != channels
        || page.bitspersamples.iter().any(|bits| *bits != depth)
    {
        return Err(unsupported(
            "EX TIFF requires uniform unsigned 8/16-bit samples",
        ));
    }
    if !matches!(page.predictor, 1 | 2) {
        return Err(unsupported("EX TIFF supports Predictor 1/2 only"));
    }
    let gray = colors == 1 && !alpha;
    let format = match (gray, depth) {
        (true, 8) => PixelFormatEx::Gray8,
        (true, _) => PixelFormatEx::Gray16,
        (false, 8) => PixelFormatEx::Rgba8,
        (false, _) => PixelFormatEx::Rgba16,
    };
    let width = usize::try_from(page.width)?;
    let height = usize::try_from(page.height)?;
    option.drawer.init_ex(
        width,
        height,
        Some(InitOptionsEx {
            source_format: format,
            source_stride: None,
            legacy: Some(InitOptions {
                loop_count: 0,
                background: None,
                animation: false,
            }),
        }),
    )?;
    let samples = decode_unsigned_samples(reader, page)?;
    let row_length = format.row_bytes(width)?;
    crate::limits::check(
        row_length,
        crate::limits::current().expanded_bytes,
        "EX TIFF output row",
    )?;
    let mut row = crate::draw::zeroed_bytes(row_length)?;
    let maximum = if depth == 16 { 65535 } else { 255 };
    for y in 0..height {
        for x in 0..width {
            let input = &samples[(y * width + x) * channels..(y * width + x + 1) * channels];
            for channel in 0..format.channels() {
                let source_channel = if colors == 1 { 0 } else { channel };
                let value = if channel == 3 {
                    if alpha { input[channels - 1] } else { maximum }
                } else {
                    let value = input[source_channel];
                    if page.photometric_interpretation == 0 {
                        maximum - value
                    } else {
                        value
                    }
                };
                let offset = x * format.bytes_per_pixel() + channel * format.bytes_per_sample();
                if depth == 16 {
                    row[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
                } else {
                    row[offset] = value as u8;
                }
            }
        }
        option.drawer.draw(0, y, width, 1, &row, None)?;
    }
    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("Tiff".into()))?;
    option
        .drawer
        .set_metadata("Orientation", DataMap::UInt(u64::from(page.orientation)))?;
    option
        .drawer
        .set_metadata("Tiff headers", DataMap::Exif(page.tiff_headers.clone()))?;
    if let Some(icc) = &page.icc_profile {
        option
            .drawer
            .set_metadata("Source ICC Profile", DataMap::ICCProfile(icc.clone()))?;
    }
    option.drawer.terminate(None)?;
    Ok(None)
}
