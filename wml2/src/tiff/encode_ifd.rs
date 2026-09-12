//! TIFF IFD serialization used by the encoder.
//!
//! The legacy serializer is deliberately kept for Classic TIFF.  This module
//! contains the BigTIFF layout, whose 64-bit entry/count fields and 8-byte
//! alignment are sufficiently different that sharing the byte writer tends to
//! hide overflow bugs.

use super::header::{DataPack, EncodedTag, TiffHeader, TiffHeaders, encode_tag_data};
use crate::error::{ImgError, ImgErrorKind};
use bin_rs::Endian;
use bin_rs::io::*;

type Error = Box<dyn std::error::Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TiffOutputVariant {
    Classic,
    BigTiff,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StripLayout {
    pub(crate) offset: u64,
    pub(crate) byte_count: u64,
}

fn invalid(message: impl Into<String>) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::InvalidParameter,
        message.into(),
    ))
}

fn align8(value: usize) -> Result<usize, Error> {
    value
        .checked_add(7)
        .map(|v| v & !7)
        .ok_or_else(|| invalid("TIFF IFD alignment overflow"))
}

fn padded8(value: usize) -> Result<usize, Error> {
    align8(value)
}

fn is_wide_tag(tagid: u16) -> bool {
    matches!(tagid, 0x0111 | 0x0117 | 0x0144 | 0x0145 | 0x014a)
}

fn unsigned_values(tag: &TiffHeader) -> Option<Vec<u64>> {
    match &tag.data {
        DataPack::Short(v) => Some(v.iter().map(|v| u64::from(*v)).collect()),
        DataPack::Long(v) => Some(v.iter().map(|v| u64::from(*v)).collect()),
        _ => None,
    }
}

fn encode_big_tag(
    tag: &TiffHeader,
    endian: Endian,
    layout: Option<StripLayout>,
) -> Result<EncodedTag, Error> {
    let mut encoded = encode_tag_data(tag, endian)?;
    if is_wide_tag(encoded.tagid) {
        let values = match (encoded.tagid, layout) {
            (0x0111, Some(layout)) => vec![layout.offset],
            (0x0117, Some(layout)) => vec![layout.byte_count],
            _ => unsigned_values(tag).ok_or_else(|| {
                invalid(format!(
                    "TIFF offset tag 0x{:04x} is not an integer",
                    encoded.tagid
                ))
            })?,
        };
        let mut payload = Vec::with_capacity(
            values
                .len()
                .checked_mul(8)
                .ok_or_else(|| invalid("TIFF offset payload overflow"))?,
        );
        for value in values {
            write_u64(value, &mut payload, endian);
        }
        encoded.type_id = 16; // LONG8
        encoded.count = u32::try_from(payload.len() / 8)
            .map_err(|_| invalid("TIFF LONG8 count is too large"))?;
        encoded.payload = payload;
    }
    Ok(encoded)
}

fn base_tags(tags: &[TiffHeader]) -> Vec<TiffHeader> {
    let mut result = tags.to_vec();
    // Pointers are structural fields generated below.  Any source pointer is
    // stale once the child IFD is laid out at a new offset.
    result.retain(|tag| tag.tagid != 0x8769 && tag.tagid != 0x8825);
    result.sort_by_key(|tag| tag.tagid);
    result
}

fn pointer_tag(tagid: u16, offset: u64, endian: Endian) -> EncodedTag {
    let mut payload = Vec::with_capacity(8);
    write_u64(offset, &mut payload, endian);
    EncodedTag {
        tagid,
        type_id: 18,
        count: 1,
        payload,
    }
}

/// Serialize one BigTIFF IFD and its optional EXIF/GPS child IFDs.
fn serialize_ifd(
    tags: &[TiffHeader],
    exif: Option<&Vec<TiffHeader>>,
    gps: Option<&Vec<TiffHeader>>,
    absolute_offset: usize,
    next_ifd_offset: u64,
    endian: Endian,
    layout: Option<StripLayout>,
) -> Result<Vec<u8>, Error> {
    let has_exif = exif.is_some_and(|v| !v.is_empty());
    let has_gps = gps.is_some_and(|v| !v.is_empty());
    let source = base_tags(tags);
    let mut encoded = source
        .iter()
        .map(|tag| encode_big_tag(tag, endian, layout))
        .collect::<Result<Vec<_>, _>>()?;
    if has_exif {
        encoded.push(pointer_tag(0x8769, 0, endian));
    }
    if has_gps {
        encoded.push(pointer_tag(0x8825, 0, endian));
    }
    encoded.sort_by_key(|tag| tag.tagid);

    let entry_count = encoded.len();
    let body = 8usize
        .checked_add(
            entry_count
                .checked_mul(20)
                .ok_or_else(|| invalid("BigTIFF IFD size overflow"))?,
        )
        .and_then(|v| v.checked_add(8))
        .ok_or_else(|| invalid("BigTIFF IFD size overflow"))?;
    let data_start = align8(
        absolute_offset
            .checked_add(body)
            .ok_or_else(|| invalid("BigTIFF IFD offset overflow"))?,
    )?;
    let mut local_len = data_start
        .checked_sub(absolute_offset)
        .ok_or_else(|| invalid("BigTIFF IFD offset underflow"))?;
    for tag in &encoded {
        if tag.payload.len() > 8 {
            local_len = local_len
                .checked_add(padded8(tag.payload.len())?)
                .ok_or_else(|| invalid("BigTIFF payload size overflow"))?;
        }
    }
    let nested_start = align8(
        absolute_offset
            .checked_add(local_len)
            .ok_or_else(|| invalid("BigTIFF nested IFD overflow"))?,
    )?;
    let exif_offset = has_exif.then_some(nested_start as u64);
    let exif_bytes = if let (Some(exif), Some(offset)) = (exif, exif_offset) {
        Some(serialize_ifd(
            exif,
            None,
            None,
            usize::try_from(offset).map_err(|_| invalid("BigTIFF EXIF offset is too large"))?,
            0,
            endian,
            None,
        )?)
    } else {
        None
    };
    let gps_offset = has_gps.then_some(align8(
        nested_start
            .checked_add(exif_bytes.as_ref().map_or(0, Vec::len))
            .ok_or_else(|| invalid("BigTIFF GPS offset overflow"))?,
    )? as u64);
    let gps_bytes = if let (Some(gps), Some(offset)) = (gps, gps_offset) {
        Some(serialize_ifd(
            gps,
            None,
            None,
            usize::try_from(offset).map_err(|_| invalid("BigTIFF GPS offset is too large"))?,
            0,
            endian,
            None,
        )?)
    } else {
        None
    };

    let mut directory = Vec::with_capacity(body);
    let mut payload = Vec::with_capacity(local_len.saturating_sub(body));
    write_u64(entry_count as u64, &mut directory, endian);
    let payload_base = data_start;
    let mut payload_offset = payload_base;
    for tag in &encoded {
        write_u16(tag.tagid, &mut directory, endian);
        write_u16(
            if tag.tagid == 0x8769 || tag.tagid == 0x8825 {
                18
            } else {
                tag.type_id
            },
            &mut directory,
            endian,
        );
        write_u64(u64::from(tag.count), &mut directory, endian);
        if tag.tagid == 0x8769 {
            write_u64(
                exif_offset.ok_or_else(|| invalid("BigTIFF EXIF offset missing"))?,
                &mut directory,
                endian,
            );
        } else if tag.tagid == 0x8825 {
            write_u64(
                gps_offset.ok_or_else(|| invalid("BigTIFF GPS offset missing"))?,
                &mut directory,
                endian,
            );
        } else if tag.payload.len() <= 8 {
            write_bytes(&tag.payload, &mut directory);
            for _ in tag.payload.len()..8 {
                write_byte(0, &mut directory);
            }
        } else {
            write_u64(payload_offset as u64, &mut directory, endian);
            payload.extend_from_slice(&tag.payload);
            while payload.len() % 8 != 0 {
                payload.push(0);
            }
            payload_offset = payload_base
                .checked_add(payload.len())
                .ok_or_else(|| invalid("BigTIFF payload offset overflow"))?;
        }
    }
    write_u64(next_ifd_offset, &mut directory, endian);
    let mut output = Vec::with_capacity(
        local_len
            .checked_add(exif_bytes.as_ref().map_or(0, Vec::len))
            .and_then(|v| v.checked_add(gps_bytes.as_ref().map_or(0, Vec::len)))
            .ok_or_else(|| invalid("BigTIFF IFD output size overflow"))?,
    );
    output.extend_from_slice(&directory);
    while output.len() < payload_base.saturating_sub(absolute_offset) {
        output.push(0);
    }
    output.extend_from_slice(&payload);
    while output.len() < nested_start.saturating_sub(absolute_offset) {
        output.push(0);
    }
    if let Some(bytes) = exif_bytes {
        output.extend_from_slice(&bytes);
    }
    if let Some(offset) = gps_offset {
        let target = usize::try_from(offset)
            .map_err(|_| invalid("BigTIFF GPS offset is too large"))?
            .checked_sub(absolute_offset)
            .ok_or_else(|| invalid("BigTIFF GPS offset underflow"))?;
        while output.len() < target {
            output.push(0);
        }
    }
    if let Some(bytes) = gps_bytes {
        output.extend_from_slice(&bytes);
    }
    Ok(output)
}

/// Serialize encoder page headers using the requested output variant.
pub(crate) fn encode_pages(
    pages: &[TiffHeaders],
    variant: TiffOutputVariant,
) -> Result<Vec<u8>, Error> {
    encode_pages_with_layouts(pages, variant, None)
}

pub(crate) fn encode_pages_with_layouts(
    pages: &[TiffHeaders],
    variant: TiffOutputVariant,
    layouts: Option<&[StripLayout]>,
) -> Result<Vec<u8>, Error> {
    if variant == TiffOutputVariant::Classic {
        return super::header::tiff_pages_to_bytes(pages);
    }
    let first = pages
        .first()
        .ok_or_else(|| invalid("TIFF requires at least one page"))?;
    let endian = first.endian;
    if pages.iter().any(|page| page.endian != endian) {
        return Err(invalid("all TIFF pages must use the same endian"));
    }
    if layouts.is_some_and(|values| values.len() != pages.len()) {
        return Err(invalid(
            "BigTIFF strip layout count does not match page count",
        ));
    }
    let mut offsets = Vec::with_capacity(pages.len());
    let mut next = 16usize;
    for (index, page) in pages.iter().enumerate() {
        next = align8(next)?;
        offsets.push(next);
        let bytes = serialize_ifd(
            &page.headers,
            page.exif.as_ref(),
            page.gps.as_ref(),
            next,
            0,
            endian,
            layouts.and_then(|values| values.get(index).copied()),
        )?;
        next = next
            .checked_add(bytes.len())
            .ok_or_else(|| invalid("BigTIFF header size overflow"))?;
    }
    let mut output = Vec::with_capacity(next);
    match endian {
        Endian::LittleEndian => write_bytes(b"II", &mut output),
        Endian::BigEndian => write_bytes(b"MM", &mut output),
    }
    write_u16(43, &mut output, endian);
    write_u16(8, &mut output, endian);
    write_u16(0, &mut output, endian);
    write_u64(offsets[0] as u64, &mut output, endian);
    for (index, page) in pages.iter().enumerate() {
        while output.len() < offsets[index] {
            output.push(0);
        }
        let next_ifd = offsets.get(index + 1).copied().unwrap_or(0) as u64;
        let bytes = serialize_ifd(
            &page.headers,
            page.exif.as_ref(),
            page.gps.as_ref(),
            offsets[index],
            next_ifd,
            endian,
            layouts.and_then(|values| values.get(index).copied()),
        )?;
        output.extend_from_slice(&bytes);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> TiffHeaders {
        TiffHeaders {
            version: 43,
            endian: Endian::LittleEndian,
            headers: vec![
                TiffHeader {
                    tagid: 0x0100,
                    data: DataPack::Long(vec![1]),
                    length: 1,
                },
                TiffHeader {
                    tagid: 0x0101,
                    data: DataPack::Long(vec![1]),
                    length: 1,
                },
                TiffHeader {
                    tagid: 0x0102,
                    data: DataPack::Short(vec![8, 8, 8]),
                    length: 3,
                },
                TiffHeader {
                    tagid: 0x0103,
                    data: DataPack::Short(vec![1]),
                    length: 1,
                },
                TiffHeader {
                    tagid: 0x0106,
                    data: DataPack::Short(vec![2]),
                    length: 1,
                },
                TiffHeader {
                    tagid: 0x0111,
                    data: DataPack::Long(vec![0]),
                    length: 1,
                },
                TiffHeader {
                    tagid: 0x0115,
                    data: DataPack::Short(vec![3]),
                    length: 1,
                },
                TiffHeader {
                    tagid: 0x0116,
                    data: DataPack::Long(vec![1]),
                    length: 1,
                },
                TiffHeader {
                    tagid: 0x0117,
                    data: DataPack::Long(vec![0]),
                    length: 1,
                },
                TiffHeader {
                    tagid: 0x011c,
                    data: DataPack::Short(vec![1]),
                    length: 1,
                },
            ],
            exif: None,
            gps: None,
        }
    }

    #[test]
    fn big_layout_keeps_offsets_and_counts_above_classic_range() {
        let offset = u64::from(u32::MAX) + 1;
        let count = u64::from(u32::MAX) + 9;
        let bytes = encode_pages_with_layouts(
            &[page()],
            TiffOutputVariant::BigTiff,
            Some(&[StripLayout {
                offset,
                byte_count: count,
            }]),
        )
        .unwrap();
        assert_eq!(&bytes[..4], b"II+\0");
        assert!(
            bytes
                .windows(8)
                .any(|window| window == offset.to_le_bytes())
        );
        assert!(bytes.windows(8).any(|window| window == count.to_le_bytes()));
    }
}
