use crate::error::{ImgError, ImgErrorKind};

type Error = Box<dyn std::error::Error>;

fn err(message: impl Into<String>) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::IllegalData,
        message.into(),
    ))
}

pub(crate) fn base_channels(mode: u16) -> Result<usize, Error> {
    match mode {
        1 | 2 => Ok(1),
        3 => Ok(3),
        4 => Ok(4),
        _ => Err(Box::new(ImgError::new_const(
            ImgErrorKind::UnsupportedFeature,
            format!("PSD color mode {mode} is not supported"),
        ))),
    }
}

pub(crate) struct RasterFormat<'a> {
    pub width: usize,
    pub height: usize,
    pub depth: u16,
    pub mode: u16,
    pub palette: Option<&'a [u8]>,
}

fn sample(plane: &[u8], index: usize, depth: u16) -> Result<u8, Error> {
    match depth {
        8 => plane
            .get(index)
            .copied()
            .ok_or_else(|| err("PSD sample is outside its channel")),
        16 => {
            let offset = index
                .checked_mul(2)
                .ok_or_else(|| err("PSD 16-bit sample offset overflow"))?;
            let bytes = plane
                .get(offset..offset + 2)
                .ok_or_else(|| err("PSD 16-bit sample is truncated"))?;
            let value = u16::from_be_bytes([bytes[0], bytes[1]]) as u32;
            Ok(((value + 128) / 257) as u8)
        }
        _ => Err(err("unsupported PSD sample depth")),
    }
}

pub(crate) fn planes_to_rgba(
    format: RasterFormat<'_>,
    planes: &[Option<Vec<u8>>],
    alpha: Option<&[u8]>,
    opacity: u8,
) -> Result<Vec<u8>, Error> {
    let RasterFormat {
        width,
        height,
        depth,
        mode,
        palette,
    } = format;
    let base = base_channels(mode)?;
    if planes.len() < base || planes[..base].iter().any(Option::is_none) {
        return Err(err("PSD raster layer is missing a color channel"));
    }
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| err("PSD pixel count overflow"))?;
    let length = pixels
        .checked_mul(4)
        .ok_or_else(|| err("PSD RGBA size overflow"))?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(length).map_err(|_| {
        Box::new(ImgError::new_const(
            ImgErrorKind::OutOfMemory,
            "cannot allocate PSD RGBA pixels".to_string(),
        )) as Error
    })?;
    let palette = if mode == 2 {
        let palette = palette.ok_or_else(|| err("PSD indexed image has no palette"))?;
        if palette.len() != 768 {
            return Err(err("PSD indexed palette must contain 768 bytes"));
        }
        Some(palette)
    } else {
        None
    };

    for index in 0..pixels {
        let (red, green, blue) = match mode {
            1 => {
                let gray = sample(planes[0].as_deref().unwrap_or_default(), index, depth)?;
                (gray, gray, gray)
            }
            2 => {
                let color =
                    sample(planes[0].as_deref().unwrap_or_default(), index, depth)? as usize;
                let palette = palette.unwrap_or_default();
                (palette[color], palette[color + 256], palette[color + 512])
            }
            3 => (
                sample(planes[0].as_deref().unwrap_or_default(), index, depth)?,
                sample(planes[1].as_deref().unwrap_or_default(), index, depth)?,
                sample(planes[2].as_deref().unwrap_or_default(), index, depth)?,
            ),
            4 => {
                // PSD stores CMYK components inverted: 255 means no ink.
                let cyan = sample(planes[0].as_deref().unwrap_or_default(), index, depth)? as u16;
                let magenta =
                    sample(planes[1].as_deref().unwrap_or_default(), index, depth)? as u16;
                let yellow = sample(planes[2].as_deref().unwrap_or_default(), index, depth)? as u16;
                let black = sample(planes[3].as_deref().unwrap_or_default(), index, depth)? as u16;
                (
                    ((cyan * black + 127) / 255) as u8,
                    ((magenta * black + 127) / 255) as u8,
                    ((yellow * black + 127) / 255) as u8,
                )
            }
            _ => unreachable!(),
        };
        let source_alpha = match alpha {
            Some(alpha) => sample(alpha, index, depth)?,
            None => 255,
        };
        let alpha = ((source_alpha as u16 * opacity as u16 + 127) / 255) as u8;
        rgba.extend_from_slice(&[red, green, blue, alpha]);
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::{RasterFormat, planes_to_rgba};

    #[test]
    fn converts_inverted_cmyk() {
        let planes = [
            Some(vec![255]),
            Some(vec![0]),
            Some(vec![0]),
            Some(vec![255]),
        ];
        assert_eq!(
            planes_to_rgba(
                RasterFormat {
                    width: 1,
                    height: 1,
                    depth: 8,
                    mode: 4,
                    palette: None,
                },
                &planes,
                None,
                255,
            )
            .unwrap(),
            [255, 0, 0, 255]
        );
    }
}
