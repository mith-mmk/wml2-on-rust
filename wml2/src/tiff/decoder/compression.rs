//! Block-local TIFF compression adapters.

use super::packbits;
use crate::decoder::lzw::Lzwdecode;
use crate::tiff::header::Compression;
use std::io;
type Error = Box<dyn std::error::Error>;

pub(crate) fn decompress_block(
    compression: &Compression,
    compressed: &[u8],
    expected_len: Option<usize>,
    fill_order_lsb: bool,
) -> Result<Vec<u8>, Error> {
    let output_limit = expected_len.unwrap_or(crate::limits::current().expanded_bytes);
    if let Some(expected) = expected_len {
        crate::limits::check(
            expected,
            crate::limits::current().expanded_bytes,
            "expanded TIFF block",
        )?;
    }
    let decoded = match compression {
        Compression::NoneCompression => {
            if compressed.len() > output_limit {
                return Err(
                    io::Error::other("uncompressed TIFF block exceeds decode limit").into(),
                );
            }
            compressed.to_vec()
        }
        Compression::LZW => {
            // Old libtiff streams use LSB codes and late code-width changes.
            // Their initial Clear code has the signature used by LibTIFF's
            // LZWDecodeCompat; modern FillOrder=2 remains early-change.
            let old_style = !fill_order_lsb
                && compressed.first() == Some(&0)
                && compressed.get(1).is_some_and(|byte| byte & 1 != 0);
            let mut decoder = if old_style {
                Lzwdecode::new(8, true, false)
            } else {
                Lzwdecode::tiff(fill_order_lsb)
            };
            decoder.decode_with_limit(compressed, output_limit)?
        }
        Compression::Packbits => packbits::decode_bounded(compressed, output_limit)?,
        Compression::AdobeDeflate | Compression::DEFLATE => {
            let expected =
                expected_len.ok_or_else(|| io::Error::other("Deflate block size is unknown"))?;
            crate::limits::inflate_image(compressed, expected)?
        }
        _ => {
            return Err(io::Error::other(
                "compression is not handled by the generic TIFF block adapter",
            )
            .into());
        }
    };
    if let Some(expected) = expected_len {
        if decoded.len() < expected {
            return Err(io::Error::other(format!(
                "TIFF block is truncated after decompression: expected {expected}, got {}",
                decoded.len()
            ))
            .into());
        }
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_libtiff_lzw_uses_lsb_late_code_width_changes() {
        let raw: Vec<u8> = (0..4096)
            .map(|i| ((i * 97 + i / 251) % 256) as u8)
            .collect();
        let stream = crate::encoder::lzw::encode_gif(&raw, 8).unwrap();
        assert_eq!(
            decompress_block(&Compression::LZW, &stream, Some(raw.len()), false).unwrap(),
            raw
        );
    }
}
