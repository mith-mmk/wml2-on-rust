//! Correct item alpha association missing from the pinned codec projection.
use avif_codec::{AvifInfo, DecodedFrame, RichAvifInfo};
use bin_rs::reader::{BinaryReader, BytesReader};
use std::io::{self, SeekFrom};

fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}

fn boxes(
    data: &[u8],
    count: &mut usize,
    mut visit: impl FnMut([u8; 4], &[u8], &mut usize) -> io::Result<()>,
) -> io::Result<()> {
    let mut reader = BytesReader::new(data);
    while reader.offset()? < data.len() as u64 {
        let at = reader.offset()?;
        if data.len() as u64 - at < 8 {
            return Err(invalid("truncated AVIF box"));
        }
        *count += 1;
        if *count > 16384 {
            return Err(invalid("AVIF signaling box count exceeds limit"));
        }
        let length = reader.read_u32_be()?;
        let mut kind = [0; 4];
        reader.read_exact(&mut kind)?;
        let (length, header) = match length {
            0 => (data.len() as u64 - at, 8),
            1 => (reader.read_u64_be()?, 16),
            n => (u64::from(n), 8),
        };
        let end = at
            .checked_add(length)
            .ok_or_else(|| invalid("AVIF box extent overflow"))?;
        if length < header || end > data.len() as u64 {
            return Err(invalid("AVIF box extent outside input"));
        }
        let start =
            usize::try_from(reader.offset()?).map_err(|_| invalid("AVIF offset exceeds usize"))?;
        visit(
            kind,
            &data[start..usize::try_from(end).map_err(|_| invalid("AVIF extent exceeds usize"))?],
            count,
        )?;
        reader.seek(SeekFrom::Start(end))?;
    }
    Ok(())
}

fn find_prem(
    data: &[u8],
    primary: Option<u32>,
    valid_alpha: impl Fn(u32) -> bool,
) -> io::Result<bool> {
    let mut found = false;
    boxes(data, &mut 0, |kind, meta, count| {
        if &kind != b"meta" {
            return Ok(());
        }
        let meta = meta
            .get(4..)
            .ok_or_else(|| invalid("truncated AVIF meta fullbox"))?;
        boxes(meta, count, |kind, iref, count| {
            if &kind != b"iref" {
                return Ok(());
            }
            let version = *iref.first().ok_or_else(|| invalid("truncated AVIF iref"))?;
            if version > 1 {
                return Err(invalid("unsupported AVIF iref version"));
            }
            let refs = iref
                .get(4..)
                .ok_or_else(|| invalid("truncated AVIF iref fullbox"))?;
            boxes(refs, count, |kind, payload, _| {
                if &kind != b"prem" {
                    return Ok(());
                }
                let mut reader = BytesReader::new(payload);
                let from = if version == 0 {
                    u32::from(reader.read_u16_be()?)
                } else {
                    reader.read_u32_be()?
                };
                let n = usize::from(reader.read_u16_be()?);
                let id_size = if version == 0 { 2 } else { 4 };
                if payload.len() != id_size + 2 + n * id_size {
                    return Err(invalid("invalid AVIF prem reference extent"));
                }
                if Some(from) != primary {
                    return Ok(());
                }
                if n != 1 || found {
                    return Err(invalid(
                        "AVIF primary prem must identify one alpha item once",
                    ));
                }
                let target = if version == 0 {
                    u32::from(reader.read_u16_be()?)
                } else {
                    reader.read_u32_be()?
                };
                if !valid_alpha(target) {
                    return Err(invalid(
                        "AVIF prem does not reference the decoded alpha item",
                    ));
                }
                found = true;
                Ok(())
            })
        })
    })?;
    Ok(found)
}

pub(crate) fn alpha_premultiplied(data: &[u8], info: &AvifInfo) -> io::Result<bool> {
    // Match the pinned strict decoder's selection: an alpha grid takes
    // precedence; otherwise only the first auxiliary item is decoded.
    let decoded_alpha = info
        .alpha_grid
        .as_ref()
        .map(|grid| grid.item_id)
        .or_else(|| info.alpha_auxiliary_items.first().map(|item| item.item_id));
    find_prem(data, info.primary_item_id, |id| decoded_alpha == Some(id))
}

pub(crate) fn is_sequence(data: &[u8]) -> io::Result<bool> {
    let mut sequence = false;
    boxes(data, &mut 0, |kind, body, _| {
        if &kind == b"ftyp" {
            if body.len() < 8 || body.len() % 4 != 0 {
                return Err(invalid("invalid AVIF ftyp extent"));
            }
            sequence = body[..4] == *b"avis" || body[8..].chunks_exact(4).any(|b| b == b"avis");
        }
        Ok(())
    })?;
    Ok(sequence)
}

pub(crate) fn has_profile_or_prem(data: &[u8]) -> io::Result<bool> {
    let mut found = false;
    boxes(data, &mut 0, |kind, meta, count| {
        if &kind != b"meta" {
            return Ok(());
        }
        let meta = meta
            .get(4..)
            .ok_or_else(|| invalid("truncated AVIF meta"))?;
        boxes(meta, count, |kind, body, count| {
            if &kind == b"iref" {
                let refs = body
                    .get(4..)
                    .ok_or_else(|| invalid("truncated AVIF iref"))?;
                boxes(refs, count, |kind, _, _| {
                    found |= &kind == b"prem";
                    Ok(())
                })?;
            } else if &kind == b"iprp" {
                boxes(body, count, |kind, body, count| {
                    if &kind == b"ipco" {
                        boxes(body, count, |kind, body, _| {
                            if &kind == b"colr" {
                                found |= matches!(body.get(..4), Some(b"prof" | b"rICC"));
                            }
                            Ok(())
                        })?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })
    })?;
    Ok(found)
}

pub(crate) fn correct_alpha(
    data: &[u8],
    frame: &mut DecodedFrame,
    rich: &mut RichAvifInfo,
) -> io::Result<()> {
    // `prem` is an iref relation, not an ipco property. Preserve native samples
    // and correct both public projections to describe their actual association.
    let prem = alpha_premultiplied(data, &rich.info)?;
    if prem
        && !frame
            .buffers
            .planes
            .iter()
            .any(|plane| plane.layout.plane == 3)
    {
        return Err(invalid("AVIF prem target has no decoded alpha plane"));
    }
    frame.alpha_premultiplied = prem;
    rich.info.alpha_premultiplied = prem;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend(kind);
        out.extend(body);
        out
    }
    fn carrier(version: u8, from: u32, to: u32) -> Vec<u8> {
        let mut p = Vec::new();
        if version == 0 {
            p.extend((from as u16).to_be_bytes())
        } else {
            p.extend(from.to_be_bytes())
        }
        p.extend(1u16.to_be_bytes());
        if version == 0 {
            p.extend((to as u16).to_be_bytes())
        } else {
            p.extend(to.to_be_bytes())
        }
        let mut refs = vec![version, 0, 0, 0];
        refs.extend(boxed(b"prem", &p));
        let mut meta = vec![0; 4];
        meta.extend(boxed(b"iref", &refs));
        boxed(b"meta", &meta)
    }
    #[test]
    fn prem_is_a_bounded_directional_item_reference() {
        for (version, primary, alpha) in [(0, 1, 2), (1, 70000, 90000)] {
            let data = carrier(version, primary, alpha);
            assert!(find_prem(&data, Some(primary), |id| id == alpha).unwrap());
            assert!(!find_prem(&data, Some(alpha), |id| id == primary).unwrap());
            assert!(find_prem(&data, Some(primary), |_| false).is_err());
            for n in 1..data.len() {
                assert!(find_prem(&data[..n], Some(primary), |_| true).is_err());
            }
        }
        let fake = boxed(
            b"meta",
            &[
                vec![0; 4],
                boxed(b"iprp", &boxed(b"ipco", &boxed(b"prem", &[]))),
            ]
            .concat(),
        );
        assert!(!find_prem(&fake, Some(1), |_| true).unwrap());
    }
}
