//! Block-local TIFF compression adapters.

use super::packbits;
use crate::decoder::lzw::Lzwdecode;
use crate::tiff::header::Compression;
use std::io;
type Error = Box<dyn std::error::Error>;

// Both LSB TIFF code-width variants share this byte signature.  The
// signature identifies the LSB candidate family; it does not identify the
// LibTIFF late-change variant by itself.
fn lzw_lsb_signature(compressed: &[u8]) -> bool {
    compressed.first() == Some(&0) && compressed.get(1).is_some_and(|byte| byte & 1 != 0)
}

fn lzw_initial_clear(compressed: &[u8], lsb: bool) -> bool {
    let Some((&first, rest)) = compressed.split_first() else {
        return false;
    };
    let Some(&second) = rest.first() else {
        return false;
    };
    let code = if lsb {
        (u16::from(first) | (u16::from(second) << 8)) & 0x01ff
    } else {
        ((u16::from(first) << 1) | (u16::from(second) >> 7)) & 0x01ff
    };
    code == 256
}

fn decode_lzw(
    compressed: &[u8],
    expected_len: Option<usize>,
    output_limit: usize,
) -> Result<Vec<u8>, Error> {
    if !lzw_lsb_signature(compressed) {
        if !lzw_initial_clear(compressed, false) {
            return Err(
                io::Error::other("TIFF LZW stream is missing the initial clear code").into(),
            );
        }
        return Lzwdecode::tiff_standard().decode_with_limit(compressed, output_limit);
    }

    // The old and WML2 LSB streams have the same initial clear-code bytes.
    // Try both strict decoders and require the expected full block length.
    if !lzw_initial_clear(compressed, true) {
        return Err(io::Error::other("TIFF LZW stream is missing the initial clear code").into());
    }

    // Exact comparison requires two expanded candidates. Admit that only
    // when the block limit has room for both; otherwise reject the ambiguous
    // stream before allocating beyond the configured expanded-byte budget.
    let expected = expected_len.ok_or_else(|| {
        io::Error::other("ambiguous TIFF LZW candidates require an expected block length")
    })?;
    let compare_budget = expected.checked_mul(2).ok_or_else(|| {
        crate::error::ImgError::new_const(
            crate::error::ImgErrorKind::OutOfMemory,
            "TIFF LZW candidate size overflows".into(),
        )
    })?;
    crate::limits::check(
        compare_budget,
        crate::limits::current().expanded_bytes,
        "TIFF LZW candidate buffers",
    )?;

    // A bounded decoder may validly stop at EOI before filling the block.
    // For TIFF that is a candidate failure when the block length is known;
    // normalize it before selecting between early and late code-width rules.
    let normalize = |candidate: Result<Vec<u8>, Error>, flavor: &str| match candidate {
        Ok(data) if data.len() == expected => Ok(data),
        Ok(data) => Err(io::Error::other(format!(
            "TIFF LZW {flavor} candidate has invalid length: expected {expected}, got {}",
            data.len()
        ))
        .into()),
        Err(error) => Err(error),
    };
    let early = normalize(
        Lzwdecode::tiff_wml2_lsb().decode_with_limit(compressed, output_limit),
        "early-change",
    );
    let late = normalize(
        Lzwdecode::tiff_libtiff_compat().decode_with_limit(compressed, output_limit),
        "late-change",
    );
    match (early, late) {
        (Ok(early), Ok(late)) => {
            if early == late {
                Ok(early)
            } else {
                Err(io::Error::other(
                    "ambiguous TIFF LZW code order: early and late candidates differ",
                )
                .into())
            }
        }
        (Ok(early), Err(_)) => Ok(early),
        (Err(_), Ok(late)) => Ok(late),
        (Err(error), Err(_)) => Err(error),
    }
}

pub(crate) fn decompress_block(
    compression: &Compression,
    compressed: &[u8],
    expected_len: Option<usize>,
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
        Compression::LZW => decode_lzw(compressed, expected_len, output_limit)?,
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

    fn codes9(codes: &[u16], lsb: bool) -> Vec<u8> {
        let mut data = vec![0u8; (codes.len() * 9).div_ceil(8)];
        for (index, code) in codes.iter().enumerate() {
            for bit in 0..9 {
                let position = index * 9 + bit;
                let source_bit = if lsb { bit } else { 8 - bit };
                let target_bit = if lsb { position % 8 } else { 7 - position % 8 };
                data[position / 8] |= (((code >> source_bit) & 1) as u8) << target_bit;
            }
        }
        data
    }

    #[test]
    fn tiff_lzw_requires_clear_eoi_and_exact_block_length() {
        for lsb in [false, true] {
            let valid = codes9(&[256, 7, 257], lsb);
            assert_eq!(
                decompress_block(&Compression::LZW, &valid, Some(1)).unwrap(),
                [7]
            );
            for (codes, expected) in [
                (&[7, 257][..], 1),         // Missing initial Clear.
                (&[256, 7][..], 1),         // Missing EOI, even if enough pixels exist.
                (&[256, 7, 257][..], 2),    // Premature EOI.
                (&[256, 7, 8, 257][..], 1), // Expanded bytes exceed the block.
            ] {
                assert!(
                    decompress_block(&Compression::LZW, &codes9(codes, lsb), Some(expected))
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn lsb_candidate_comparison_is_charged_before_decoding() {
        let valid = codes9(&[256, 7, 257], true);
        let mut limits = crate::limits::DecodeLimits {
            expanded_bytes: 1,
            ..Default::default()
        };
        crate::limits::scope(limits, || {
            let error = decompress_block(&Compression::LZW, &valid, Some(1)).unwrap_err();
            assert!(error.to_string().contains("TIFF LZW candidate buffers"));
            assert!(error.downcast_ref::<crate::error::ImgError>().is_some());
        });
        limits.expanded_bytes = 2;
        crate::limits::scope(limits, || {
            assert_eq!(
                decompress_block(&Compression::LZW, &valid, Some(1)).unwrap(),
                [7]
            );
        });
    }

    #[test]
    fn lsb_candidate_with_premature_eoi_does_not_hide_the_complete_variant() {
        let mut state = 223u32;
        let raw: Vec<u8> = (0..1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        let stream = crate::encoder::lzw::encode_tiff_wml2_lsb(&raw).unwrap();
        // The wrong width convention reaches a real EOI, but too soon. A
        // successful codec Result alone must not make it a valid TIFF block.
        let short = Lzwdecode::tiff_libtiff_compat()
            .decode_with_limit(&stream, raw.len())
            .unwrap();
        assert_eq!(short.len(), 932);
        assert_eq!(
            decompress_block(&Compression::LZW, &stream, Some(raw.len())).unwrap(),
            raw
        );
    }

    #[test]
    fn standard_tiff_lzw_uses_msb_early_change() {
        let raw: Vec<u8> = (0..4096)
            .map(|i| ((i * 97 + i / 251) % 256) as u8)
            .collect();
        let stream = crate::encoder::lzw::encode_tiff_standard(&raw).unwrap();
        assert_eq!(stream.first(), Some(&0x80));
        assert_eq!(
            decompress_block(&Compression::LZW, &stream, Some(raw.len())).unwrap(),
            raw
        );
    }

    #[test]
    fn old_libtiff_lzw_uses_lsb_late_code_width_changes() {
        let raw: Vec<u8> = (0..4096)
            .map(|i| ((i * 97 + i / 251) % 256) as u8)
            .collect();
        let stream = crate::encoder::lzw::encode_tiff_libtiff_compat(&raw).unwrap();
        assert_eq!(stream.first(), Some(&0));
        assert!(stream.get(1).is_some_and(|byte| byte & 1 != 0));
        assert_eq!(
            decompress_block(&Compression::LZW, &stream, Some(raw.len())).unwrap(),
            raw
        );
    }

    #[test]
    fn wml2_lzw_lsb_is_separate_from_old_libtiff_detection() {
        let raw: Vec<u8> = (0..4096)
            .map(|i| ((i * 37 + i / 127) % 256) as u8)
            .collect();
        let stream = crate::encoder::lzw::encode_tiff_wml2_lsb(&raw).unwrap();
        assert_eq!(stream.first(), Some(&0));
        assert!(stream.get(1).is_some_and(|byte| byte & 1 != 0));
        assert_eq!(
            decompress_block(&Compression::LZW, &stream, Some(raw.len())).unwrap(),
            raw
        );
    }
}
