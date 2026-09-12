//! PackBits decoder for TIFF.

type Error = Box<dyn std::error::Error>;

/// Decode PackBits while bounding the expanded output. This prevents a small
/// repeated run sequence from bypassing TIFF's expanded-byte budget.
pub fn decode_bounded(data: &[u8], max_output: usize) -> Result<Vec<u8>, Error> {
    let mut buf = vec![];
    let mut i = 0;
    while i < data.len() {
        let run = data[i] as i8;
        i += 1;
        if run == -128 {
            // TIFF PackBits reserves -128 as a no-op.
        } else if run < 0 {
            let count = usize::try_from(1i16 - i16::from(run))?;
            let byte = *data
                .get(i)
                .ok_or_else(|| std::io::Error::other("PackBits repeat run is truncated"))?;
            let next = buf
                .len()
                .checked_add(count)
                .ok_or_else(|| std::io::Error::other("PackBits output size overflows"))?;
            if next > max_output {
                return Err(std::io::Error::other("PackBits output exceeds decode limit").into());
            }
            buf.try_reserve(count)?;
            for _ in 0..count {
                buf.push(byte);
            }
            i += 1;
        } else {
            let count = usize::try_from(i16::from(run) + 1)?;
            let end = i
                .checked_add(count)
                .ok_or_else(|| std::io::Error::other("PackBits literal run overflows"))?;
            if end > data.len() {
                return Err(std::io::Error::other("PackBits literal run is truncated").into());
            }
            let next = buf
                .len()
                .checked_add(count)
                .ok_or_else(|| std::io::Error::other("PackBits output size overflows"))?;
            if next > max_output {
                return Err(std::io::Error::other("PackBits output exceeds decode limit").into());
            }
            buf.try_reserve(count)?;
            buf.extend_from_slice(&data[i..end]);
            i = end;
        }
    }
    Ok(buf)
}
