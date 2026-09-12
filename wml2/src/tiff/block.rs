//! Common strip/tile descriptors used by the TIFF decoder.

use super::header::Tiff;
use std::io;

type Error = Box<dyn std::error::Error>;

fn allocate_blocks(count: usize) -> Result<Vec<TiffBlock>, Error> {
    let bytes = count
        .checked_mul(std::mem::size_of::<TiffBlock>())
        .ok_or_else(|| io::Error::other("TIFF block descriptors overflow"))?;
    crate::limits::check(
        bytes,
        crate::limits::current().metadata_bytes,
        "TIFF block descriptors",
    )?;
    let mut result = Vec::new();
    result.try_reserve_exact(count)?;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TiffBlockKind {
    Strip,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TiffBlock {
    pub kind: TiffBlockKind,
    pub offset: u64,
    pub compressed_len: u64,
    pub x: usize,
    pub y: usize,
    pub stored_width: usize,
    pub stored_height: usize,
    pub draw_width: usize,
    pub draw_height: usize,
    pub plane: usize,
}

impl TiffBlock {
    pub(crate) fn validate_range(&self, input_len: u64) -> Result<(), Error> {
        let end = self
            .offset
            .checked_add(self.compressed_len)
            .ok_or_else(|| io::Error::other("TIFF block range overflows"))?;
        if end > input_len {
            return Err(io::Error::other("TIFF block is truncated").into());
        }
        Ok(())
    }
}

fn checked_u64<T: Into<u64>>(value: T) -> u64 {
    value.into()
}

/// Generate a validated block list. Strip and tile arrays are deliberately not
/// merged: a file containing both layouts is ambiguous and is rejected.
pub(crate) fn blocks(header: &Tiff) -> Result<Vec<TiffBlock>, Error> {
    let has_tiles = header.tile_width != 0
        || header.tile_length != 0
        || !header.tile_offsets.is_empty()
        || !header.tile_byte_counts.is_empty();
    let has_strips = !header.strip_offsets.is_empty() || !header.strip_byte_counts.is_empty();

    if has_tiles && has_strips {
        return Err(io::Error::other("TIFF contains both strip and tile storage").into());
    }
    if has_tiles {
        if header.tile_width == 0 || header.tile_length == 0 {
            return Err(io::Error::other("TIFF tile dimensions must be non-zero").into());
        }
        if header.tile_offsets.len() != header.tile_byte_counts.len() {
            return Err(io::Error::other("TIFF tile offset/count lengths differ").into());
        }
        if header.width == 0 || header.height == 0 {
            return Err(io::Error::other("TIFF image dimensions must be non-zero").into());
        }
        let tile_width = usize::try_from(header.tile_width)?;
        let tile_height = usize::try_from(header.tile_length)?;
        let image_width = usize::try_from(header.width)?;
        let image_height = usize::try_from(header.height)?;
        let across = (image_width / tile_width)
            .checked_add(usize::from(image_width % tile_width != 0))
            .ok_or_else(|| io::Error::other("TIFF tile count overflows"))?;
        let down = (image_height / tile_height)
            .checked_add(usize::from(image_height % tile_height != 0))
            .ok_or_else(|| io::Error::other("TIFF tile count overflows"))?;
        let per_plane = across
            .checked_mul(down)
            .ok_or_else(|| io::Error::other("TIFF tile count overflows"))?;
        let planes = if header.planar_config == 2 {
            usize::from(header.samples_per_pixel)
        } else {
            1
        };
        let expected = per_plane
            .checked_mul(planes)
            .ok_or_else(|| io::Error::other("TIFF tile count overflows"))?;
        if header.tile_offsets.len() != expected {
            return Err(io::Error::other(format!(
                "TIFF tile count mismatch: expected {expected}, got {}",
                header.tile_offsets.len()
            ))
            .into());
        }
        let mut result = allocate_blocks(expected)?;
        for index in 0..expected {
            let plane = index / per_plane;
            let local = index % per_plane;
            let tx = local % across;
            let ty = local / across;
            let x = tx
                .checked_mul(tile_width)
                .ok_or_else(|| io::Error::other("TIFF tile coordinate overflows"))?;
            let y = ty
                .checked_mul(tile_height)
                .ok_or_else(|| io::Error::other("TIFF tile coordinate overflows"))?;
            result.push(TiffBlock {
                kind: TiffBlockKind::Tile,
                offset: checked_u64(header.tile_offsets[index]),
                compressed_len: checked_u64(header.tile_byte_counts[index]),
                x,
                y,
                stored_width: tile_width,
                stored_height: tile_height,
                draw_width: image_width.saturating_sub(x).min(tile_width),
                draw_height: image_height.saturating_sub(y).min(tile_height),
                plane,
            });
        }
        return Ok(result);
    }

    if header.strip_offsets.len() != header.strip_byte_counts.len() {
        return Err(io::Error::other("TIFF strip offset/count lengths differ").into());
    }
    if header.strip_offsets.is_empty() {
        return Err(io::Error::other("TIFF has no image blocks").into());
    }
    if header.width == 0 || header.height == 0 {
        return Err(io::Error::other("TIFF strip dimensions must be non-zero").into());
    }
    let width = usize::try_from(header.width)?;
    let height = usize::try_from(header.height)?;
    let rows = if header.rows_per_strip == 0 {
        height
    } else {
        usize::try_from(header.rows_per_strip)?
    };
    let strips_per_plane = (height / rows)
        .checked_add(usize::from(height % rows != 0))
        .ok_or_else(|| io::Error::other("TIFF strip count overflows"))?;
    let planes = if header.planar_config == 2 {
        usize::from(header.samples_per_pixel)
    } else {
        1
    };
    let expected = strips_per_plane
        .checked_mul(planes)
        .ok_or_else(|| io::Error::other("TIFF strip count overflows"))?;
    if header.strip_offsets.len() != expected {
        return Err(io::Error::other(format!(
            "TIFF strip count mismatch: expected {expected}, got {}",
            header.strip_offsets.len()
        ))
        .into());
    }
    let mut result = allocate_blocks(expected)?;
    for index in 0..expected {
        let plane = index / strips_per_plane;
        let local = index % strips_per_plane;
        let y = local
            .checked_mul(rows)
            .ok_or_else(|| io::Error::other("TIFF strip coordinate overflows"))?;
        let draw_height = height.saturating_sub(y).min(rows);
        result.push(TiffBlock {
            kind: TiffBlockKind::Strip,
            offset: checked_u64(header.strip_offsets[index]),
            compressed_len: checked_u64(header.strip_byte_counts[index]),
            x: 0,
            y,
            stored_width: width,
            // The final strip commonly contains only its visible rows; using
            // RowsPerStrip here would reject valid short compressed strips.
            stored_height: draw_height,
            draw_width: width,
            draw_height,
            plane,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_mixed_storage() {
        let mut tiff = Tiff::empty();
        tiff.width = 1;
        tiff.height = 1;
        tiff.rows_per_strip = 1;
        tiff.strip_offsets = vec![1];
        tiff.strip_byte_counts = vec![1];
        tiff.tile_width = 1;
        assert!(blocks(&tiff).is_err());
    }
}
