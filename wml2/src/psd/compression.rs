use crate::error::{ImgError, ImgErrorKind};

type Error = Box<dyn std::error::Error>;

fn err(kind: ImgErrorKind, message: impl Into<String>) -> Error {
    Box::new(ImgError::new_const(kind, message.into()))
}

fn checked_row_bytes(width: usize, depth: u16) -> Result<usize, Error> {
    let bytes = match depth {
        8 => 1,
        16 => 2,
        _ => {
            return Err(err(
                ImgErrorKind::UnsupportedFeature,
                format!("PSD depth {depth} is not supported"),
            ));
        }
    };
    width
        .checked_mul(bytes)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PSD row size overflow"))
}

fn expected_len(
    width: usize,
    height: usize,
    channels: usize,
    depth: u16,
) -> Result<(usize, usize), Error> {
    let row_bytes = checked_row_bytes(width, depth)?;
    let length = row_bytes
        .checked_mul(height)
        .and_then(|value| value.checked_mul(channels))
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PSD decoded size overflow"))?;
    Ok((row_bytes, length))
}

fn allocated(length: usize) -> Result<Vec<u8>, Error> {
    crate::limits::check(
        length,
        crate::limits::current().expanded_bytes,
        "PSD pixels",
    )?;
    let mut output = Vec::new();
    output.try_reserve_exact(length).map_err(|_| {
        err(
            ImgErrorKind::OutOfMemory,
            format!("cannot allocate {length} bytes for PSD pixels"),
        )
    })?;
    Ok(output)
}

fn decode_packbits_row(data: &[u8], expected: usize) -> Result<Vec<u8>, Error> {
    let mut output = allocated(expected)?;
    let mut offset = 0usize;
    while offset < data.len() {
        let control = data[offset] as i8;
        offset += 1;
        match control {
            0..=127 => {
                let count = control as usize + 1;
                let end = offset.checked_add(count).ok_or_else(|| {
                    err(ImgErrorKind::IllegalData, "PSD PackBits offset overflow")
                })?;
                if end > data.len() || output.len() + count > expected {
                    return Err(err(
                        ImgErrorKind::IllegalData,
                        "PSD PackBits literal exceeds its scanline",
                    ));
                }
                output.extend_from_slice(&data[offset..end]);
                offset = end;
            }
            -127..=-1 => {
                if offset >= data.len() {
                    return Err(err(
                        ImgErrorKind::UnexpectedEof,
                        "PSD PackBits repeat value is missing",
                    ));
                }
                let count = (1i16 - control as i16) as usize;
                if output.len() + count > expected {
                    return Err(err(
                        ImgErrorKind::IllegalData,
                        "PSD PackBits repeat exceeds its scanline",
                    ));
                }
                output.extend(std::iter::repeat_n(data[offset], count));
                offset += 1;
            }
            -128 => {}
        }
    }
    if output.len() != expected {
        return Err(err(
            ImgErrorKind::IllegalData,
            format!(
                "PSD PackBits scanline decoded to {} bytes, expected {expected}",
                output.len()
            ),
        ));
    }
    Ok(output)
}

fn undo_prediction(
    data: &mut [u8],
    width: usize,
    height: usize,
    channels: usize,
    depth: u16,
) -> Result<(), Error> {
    let row_bytes = checked_row_bytes(width, depth)?;
    let rows = height.checked_mul(channels).ok_or_else(|| {
        err(
            ImgErrorKind::IllegalData,
            "PSD prediction row count overflow",
        )
    })?;
    for row_index in 0..rows {
        let start = row_index
            .checked_mul(row_bytes)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "PSD prediction offset overflow"))?;
        let end = start + row_bytes;
        let row = &mut data[start..end];
        for index in 1..row.len() {
            row[index] = row[index].wrapping_add(row[index - 1]);
        }
        if depth == 16 {
            let shuffled = row.to_vec();
            for x in 0..width {
                row[x * 2] = shuffled[x];
                row[x * 2 + 1] = shuffled[x + width];
            }
        }
    }
    Ok(())
}

pub(crate) fn decompress_planar(
    compression: u16,
    payload: &[u8],
    width: usize,
    height: usize,
    channels: usize,
    depth: u16,
) -> Result<Vec<u8>, Error> {
    let (row_bytes, expected) = expected_len(width, height, channels, depth)?;
    match compression {
        0 => {
            if payload.len() != expected {
                return Err(err(
                    ImgErrorKind::UnexpectedEof,
                    format!(
                        "PSD raw data has {} bytes, expected {expected}",
                        payload.len()
                    ),
                ));
            }
            Ok(payload.to_vec())
        }
        1 => {
            let row_count = height
                .checked_mul(channels)
                .ok_or_else(|| err(ImgErrorKind::IllegalData, "PSD RLE row count overflow"))?;
            let table_len = row_count
                .checked_mul(2)
                .ok_or_else(|| err(ImgErrorKind::IllegalData, "PSD RLE table size overflow"))?;
            if payload.len() < table_len {
                return Err(err(
                    ImgErrorKind::UnexpectedEof,
                    "PSD RLE byte-count table is truncated",
                ));
            }
            let mut counts = Vec::new();
            counts.try_reserve_exact(row_count).map_err(|_| {
                err(
                    ImgErrorKind::OutOfMemory,
                    "cannot allocate PSD RLE row table",
                )
            })?;
            for pair in payload[..table_len].chunks_exact(2) {
                counts.push(u16::from_be_bytes([pair[0], pair[1]]) as usize);
            }
            let mut output = allocated(expected)?;
            let mut offset = table_len;
            for count in counts {
                let end = offset.checked_add(count).ok_or_else(|| {
                    err(ImgErrorKind::IllegalData, "PSD RLE data offset overflow")
                })?;
                if end > payload.len() {
                    return Err(err(
                        ImgErrorKind::UnexpectedEof,
                        "PSD RLE scanline is truncated",
                    ));
                }
                output.extend_from_slice(&decode_packbits_row(&payload[offset..end], row_bytes)?);
                offset = end;
            }
            if payload[offset..].iter().any(|byte| *byte != 0) {
                return Err(err(
                    ImgErrorKind::IllegalData,
                    "PSD RLE data contains unexpected trailing bytes",
                ));
            }
            Ok(output)
        }
        2 | 3 => {
            let mut output = crate::limits::inflate_image(payload, expected).map_err(|_| {
                err(
                    ImgErrorKind::DecodeError,
                    "PSD ZIP stream could not be decompressed",
                )
            })?;
            if output.len() != expected {
                return Err(err(
                    ImgErrorKind::IllegalData,
                    format!(
                        "PSD ZIP stream decoded to {} bytes, expected {expected}",
                        output.len()
                    ),
                ));
            }
            if compression == 3 {
                undo_prediction(&mut output, width, height, channels, depth)?;
            }
            Ok(output)
        }
        _ => Err(err(
            ImgErrorKind::UnsupportedFeature,
            format!("PSD compression {compression} is not supported"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_packbits_row, undo_prediction};

    #[test]
    fn packbits_handles_literals_repeats_and_noop() {
        assert_eq!(
            decode_packbits_row(&[1, 1, 2, 254, 3, 128], 5).unwrap(),
            [1, 2, 3, 3, 3]
        );
    }

    #[test]
    fn prediction_16_unshuffles_high_and_low_bytes() {
        let mut encoded = vec![0x12, 0x22, 0x44, 0x66];
        undo_prediction(&mut encoded, 2, 1, 1, 16).unwrap();
        assert_eq!(encoded, [0x12, 0x78, 0x34, 0xde]);
    }
}
