//! Checked TIFF sample readers and native 16-bit reconstruction helpers.

use bin_rs::Endian;
use std::io;

type Error = Box<dyn std::error::Error>;

fn invalid(message: impl Into<String>) -> Error {
    io::Error::other(message.into()).into()
}

#[cfg(feature = "high-bit-depth")]
pub(crate) fn sample_bytes(bits: u16) -> Result<usize, Error> {
    if bits == 0 || bits > 32 || bits % 8 != 0 {
        return Err(invalid(format!(
            "unsupported byte-aligned TIFF sample depth {bits}"
        )));
    }
    Ok(usize::from(bits / 8))
}

/// Read one TIFF sample. Packed samples are MSB-first as required by TIFF's
/// default FillOrder; callers handling FillOrder=2 should reverse the bit
/// position before calling this helper.
pub(crate) fn read_sample(
    data: &[u8],
    bits: u16,
    endian: Endian,
    index: usize,
) -> Result<u32, Error> {
    if !matches!(bits, 1..=8 | 16 | 32) {
        return Err(invalid(format!("unsupported TIFF sample depth {bits}")));
    }
    if bits < 8 {
        let bit = index
            .checked_mul(usize::from(bits))
            .ok_or_else(|| io::Error::other("TIFF sample bit offset overflows"))?;
        let end_bit = bit
            .checked_add(usize::from(bits))
            .ok_or_else(|| io::Error::other("TIFF sample bit offset overflows"))?;
        let end = end_bit
            .checked_add(7)
            .map(|value| value / 8)
            .ok_or_else(|| io::Error::other("TIFF sample offset overflows"))?;
        if end > data.len() {
            return Err(invalid("TIFF sample is truncated"));
        }
        let mut value = 0u32;
        for bit_index in bit..end_bit {
            value = (value << 1) | u32::from((data[bit_index / 8] >> (7 - (bit_index % 8))) & 1);
        }
        return Ok(value);
    }
    let bytes = usize::from(bits / 8);
    let byte = index
        .checked_mul(bytes)
        .ok_or_else(|| io::Error::other("TIFF sample offset overflows"))?;
    let end = byte
        .checked_add(bytes)
        .ok_or_else(|| io::Error::other("TIFF sample offset overflows"))?;
    if end > data.len() {
        return Err(invalid("TIFF sample is truncated"));
    }
    let mut value = 0u32;
    if endian == Endian::LittleEndian {
        for shift in (0..bytes).map(|i| i * 8) {
            value |= u32::from(data[byte + shift / 8]) << shift;
        }
    } else {
        for i in 0..bytes {
            value = (value << 8) | u32::from(data[byte + i]);
        }
    }
    Ok(value)
}

/// Read a sample while honoring TIFF FillOrder for packed samples.
/// Byte-aligned samples are unaffected by FillOrder.
pub(crate) fn read_sample_with_fill_order(
    data: &[u8],
    bits: u16,
    endian: Endian,
    index: usize,
    fill_order: u16,
) -> Result<u32, Error> {
    if bits < 8 && fill_order == 2 {
        if !(1..8).contains(&bits) {
            return Err(invalid(format!(
                "unsupported packed TIFF sample depth {bits}"
            )));
        }
        let bit = index
            .checked_mul(usize::from(bits))
            .ok_or_else(|| io::Error::other("TIFF sample bit offset overflows"))?;
        let end_bit = bit
            .checked_add(usize::from(bits))
            .ok_or_else(|| io::Error::other("TIFF sample bit offset overflows"))?;
        let end = end_bit
            .checked_add(7)
            .map(|value| value / 8)
            .ok_or_else(|| io::Error::other("TIFF sample offset overflows"))?;
        if end > data.len() {
            return Err(invalid("TIFF sample is truncated"));
        }
        let mut value = 0u32;
        for bit_index in bit..end_bit {
            value = (value << 1) | u32::from((data[bit_index / 8] >> (bit_index % 8)) & 1);
        }
        return Ok(value);
    }
    read_sample(data, bits, endian, index)
}

/// Decode a complete byte-aligned 16-bit page to interleaved native samples.
/// Predictor restoration is performed before conversion and before any
/// 8-bit legacy quantization.
#[cfg(feature = "high-bit-depth")]
pub(crate) fn decode_samples_u16(
    data: &[u8],
    width: usize,
    height: usize,
    bits_per_sample: &[u16],
    samples_per_pixel: usize,
    planar_config: u16,
    endian: Endian,
    predictor: u16,
) -> Result<Vec<u16>, Error> {
    if bits_per_sample.len() != samples_per_pixel || samples_per_pixel == 0 {
        return Err(invalid("TIFF BitsPerSample/SamplesPerPixel mismatch"));
    }
    if bits_per_sample.iter().any(|bits| *bits != 16) {
        return Err(invalid(
            "native TIFF sample output currently requires 16-bit samples",
        ));
    }
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| io::Error::other("TIFF pixel count overflows"))?;
    let values = pixels
        .checked_mul(samples_per_pixel)
        .ok_or_else(|| io::Error::other("TIFF sample count overflows"))?;
    let plane_values = pixels;
    let expected = values
        .checked_mul(2)
        .ok_or_else(|| io::Error::other("TIFF sample byte count overflows"))?;
    if data.len() < expected {
        return Err(invalid("TIFF 16-bit sample data is truncated"));
    }
    let mut output = vec![0u16; values];
    if planar_config == 1 {
        let row_samples = width
            .checked_mul(samples_per_pixel)
            .ok_or_else(|| io::Error::other("TIFF row sample count overflows"))?;
        let row_bytes = row_samples
            .checked_mul(2)
            .ok_or_else(|| io::Error::other("TIFF row byte count overflows"))?;
        let mut restored = data[..expected].to_vec();
        if predictor == 2 {
            crate::tiff::predictor::apply_predictor(
                &mut restored,
                row_bytes,
                height,
                16,
                samples_per_pixel,
                endian,
            )?;
        }
        for i in 0..values {
            output[i] = read_sample(&restored, 16, endian, i)? as u16;
        }
    } else if planar_config == 2 {
        for plane in 0..samples_per_pixel {
            let start = plane
                .checked_mul(plane_values)
                .and_then(|v| v.checked_mul(2))
                .ok_or_else(|| io::Error::other("TIFF plane offset overflows"))?;
            let end = start
                .checked_add(plane_values * 2)
                .ok_or_else(|| io::Error::other("TIFF plane end overflows"))?;
            let mut plane_data = data[start..end].to_vec();
            if predictor == 2 {
                crate::tiff::predictor::apply_predictor(
                    &mut plane_data,
                    width * 2,
                    height,
                    16,
                    1,
                    endian,
                )?;
            }
            for i in 0..plane_values {
                output[i * samples_per_pixel + plane] =
                    read_sample(&plane_data, 16, endian, i)? as u16;
            }
        }
    } else {
        return Err(invalid("invalid TIFF PlanarConfiguration"));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_packed_samples() {
        assert_eq!(
            read_sample(&[0b1011_0000], 4, Endian::BigEndian, 0).unwrap(),
            0xb
        );
        assert_eq!(
            read_sample(&[0b1011_0000], 4, Endian::BigEndian, 1).unwrap(),
            0
        );
    }

    #[test]
    fn reads_samples_that_cross_byte_boundaries() {
        // Four six-bit values: 0, 63, 1, and 62.
        let packed = [0x03, 0xf0, 0x7e];
        assert_eq!(read_sample(&packed, 6, Endian::BigEndian, 0).unwrap(), 0);
        assert_eq!(read_sample(&packed, 6, Endian::BigEndian, 1).unwrap(), 63);
        assert_eq!(read_sample(&packed, 6, Endian::BigEndian, 2).unwrap(), 1);
        assert_eq!(read_sample(&packed, 6, Endian::BigEndian, 3).unwrap(), 62);
    }

    #[test]
    fn reads_lsb_first_packed_samples() {
        // FillOrder=2 stores the first sample in the low bits.
        let packed = [0b00_01_11_10];
        assert_eq!(
            read_sample_with_fill_order(&packed, 2, Endian::BigEndian, 0, 2).unwrap(),
            1
        );
        assert_eq!(
            read_sample_with_fill_order(&packed, 2, Endian::BigEndian, 1, 2).unwrap(),
            3
        );
        assert_eq!(
            read_sample_with_fill_order(&packed, 2, Endian::BigEndian, 2, 2).unwrap(),
            2
        );
        assert_eq!(
            read_sample_with_fill_order(&packed, 2, Endian::BigEndian, 3, 2).unwrap(),
            0
        );
    }
}
