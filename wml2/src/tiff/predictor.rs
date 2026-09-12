//! TIFF Predictor restoration.

use bin_rs::Endian;
use std::io;
type Error = Box<dyn std::error::Error>;

/// Restore horizontal differencing in-place. `channels` is the number of
/// samples in one pixel in this stream; use one for a single planar block.
pub(crate) fn apply_predictor(
    data: &mut [u8],
    row_bytes: usize,
    rows: usize,
    bits: u16,
    channels: usize,
    endian: Endian,
) -> Result<(), Error> {
    if bits != 8 && bits != 16 && bits != 32 {
        return Err(io::Error::other(format!(
            "TIFF Predictor 2 does not support {bits}-bit samples"
        ))
        .into());
    }
    if channels == 0 || row_bytes == 0 {
        return Err(io::Error::other("invalid TIFF predictor row layout").into());
    }
    let required = row_bytes
        .checked_mul(rows)
        .ok_or_else(|| io::Error::other("TIFF predictor size overflows"))?;
    if required > data.len() {
        return Err(io::Error::other("TIFF predictor data is truncated").into());
    }
    let sample_bytes = usize::from(bits / 8);
    if row_bytes % sample_bytes != 0 || row_bytes / sample_bytes % channels != 0 {
        return Err(io::Error::other("TIFF predictor row is not sample aligned").into());
    }
    let samples_per_row = row_bytes / sample_bytes;
    for row in 0..rows {
        let start = row * row_bytes;
        for sample in channels..samples_per_row {
            let current = start + sample * sample_bytes;
            let previous = current - channels * sample_bytes;
            match bits {
                8 => data[current] = data[current].wrapping_add(data[previous]),
                16 => {
                    let c = read_u16(&data[current..current + 2], endian);
                    let p = read_u16(&data[previous..previous + 2], endian);
                    write_u16(&mut data[current..current + 2], c.wrapping_add(p), endian);
                }
                32 => {
                    let c = read_u32(&data[current..current + 4], endian);
                    let p = read_u32(&data[previous..previous + 4], endian);
                    write_u32(&mut data[current..current + 4], c.wrapping_add(p), endian);
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}

fn read_u16(data: &[u8], endian: Endian) -> u16 {
    if endian == Endian::LittleEndian {
        u16::from_le_bytes([data[0], data[1]])
    } else {
        u16::from_be_bytes([data[0], data[1]])
    }
}
fn write_u16(data: &mut [u8], value: u16, endian: Endian) {
    let bytes = if endian == Endian::LittleEndian {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    };
    data[..2].copy_from_slice(&bytes);
}
fn read_u32(data: &[u8], endian: Endian) -> u32 {
    if endian == Endian::LittleEndian {
        u32::from_le_bytes([data[0], data[1], data[2], data[3]])
    } else {
        u32::from_be_bytes([data[0], data[1], data[2], data[3]])
    }
}
fn write_u32(data: &mut [u8], value: u32, endian: Endian) {
    let bytes = if endian == Endian::LittleEndian {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    };
    data[..4].copy_from_slice(&bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_16_bit_carry_before_quantization() {
        let mut data = [0xff, 0x00, 0x00, 0x01];
        apply_predictor(&mut data, 4, 1, 16, 1, Endian::BigEndian).unwrap();
        assert_eq!(data, [0xff, 0x00, 0xff, 0x01]);
    }
}
