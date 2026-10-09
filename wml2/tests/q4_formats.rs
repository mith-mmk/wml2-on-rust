#![cfg(not(feature = "noretoro"))]

use std::env;
use std::fs;
use std::path::Path;

use wml2::draw::image_load;
use wml2::util::{ImageFormat, format_check};

fn q4_header() -> Vec<u8> {
    let mut header = vec![0u8; 22];
    header[0..2].copy_from_slice(&0x001au16.to_le_bytes());
    header[2] = 0x12;
    header[3] = 1;
    header[4] = 1;
    header[11..16].copy_from_slice(b"MAJYO");
    header
}

#[test]
fn detects_q4_header() {
    assert!(matches!(format_check(&q4_header()), ImageFormat::Q4));
    assert!(
        wml2::get_decoder_extentions()
            .iter()
            .any(|extension| extension == "q4")
    );
}

#[test]
fn rejects_truncated_q4() {
    assert!(image_load(&q4_header()).is_err());
    assert!(image_load(b"Q4").is_err());
}

// Hand-derived literal-only stream: 001 raises width 3 -> 4, 0001 raises
// 4 -> 5, then every five-bit value is a literal byte + 2, ending in 00000.
// No decoder output is used to construct either this stream or its expected
// pixels. This tests the current analysis model, not independent Q4 compatibility.
fn literal_block(bytes: &[u8]) -> Vec<u8> {
    assert!(bytes.iter().all(|&byte| byte <= 16));
    let mut bits = vec![false, false, true, false, false, false, true];
    for &byte in bytes {
        let value = byte + 2;
        bits.extend((0..5).rev().map(|bit| value & (1 << bit) != 0));
    }
    bits.extend([false; 5]);
    let mut packed = vec![0; bits.len().div_ceil(8)];
    for (index, bit) in bits.into_iter().enumerate() {
        packed[index / 8] |= u8::from(bit) << (7 - index % 8);
    }
    packed
}

fn striped_image() -> (Vec<u8>, Vec<u8>, Vec<usize>) {
    let mut palette = Vec::new();
    for color in 0u8..16 {
        palette.extend_from_slice(&[0, color, 0, 15 - color, 0, (color * 7) % 16]);
    }
    let compressed_palette = literal_block(&palette);
    let mut file = q4_header();
    file[16..18].copy_from_slice(&(compressed_palette.len() as u16).to_le_bytes());
    file.extend_from_slice(&compressed_palette);
    let mut expected = Vec::new();
    let mut block_offsets = Vec::new();
    for block in 0..4 {
        let mut tokens = Vec::new();
        for row in block * 100..(block + 1) * 100 {
            let token = (row % 16) as u8;
            // Three coloured runs: 16*17+8 = 280, 280, 4*17+12 = 80.
            for (high, low) in [(16, 8), (16, 8), (4, 12)] {
                tokens.extend_from_slice(&[16, 0, token, high, low]);
            }
            // The palette index permutation rotates the low three bits left.
            let color = (token & 8) | ((token & 3) << 1) | ((token & 4) >> 2);
            let pixel = [color * 17, (15 - color) * 17, ((color * 7) % 16) * 17, 255];
            for _ in 0..640 {
                expected.extend_from_slice(&pixel);
            }
        }
        // Split the first row after its first 280-pixel run. A block boundary
        // must not restart the row; the corpus only has row-aligned blocks.
        let segments: Vec<(&[u8], u16)> = if block == 0 {
            vec![(&tokens[..5], 140), (&tokens[5..], 31860)]
        } else {
            vec![(&tokens, 32000)]
        };
        for (segment, original_bytes) in segments {
            let compressed = literal_block(segment);
            block_offsets.push(file.len());
            file.extend_from_slice(&(compressed.len() as u16).to_le_bytes());
            file.extend_from_slice(&[0, 0]);
            file.extend_from_slice(&original_bytes.to_le_bytes());
            file.extend_from_slice(&compressed);
        }
    }
    let length = file.len() as u32;
    file[8..11].copy_from_slice(&length.to_le_bytes()[..3]);
    (file, expected, block_offsets)
}

#[test]
fn literal_stream_preserves_palette_rows_and_image_block_boundaries() {
    let (file, expected, _) = striped_image();
    let image = image_load(&file).unwrap();
    assert_eq!((image.width, image.height), (640, 400));
    assert_eq!(image.buffer.unwrap(), expected);
}

#[test]
fn literal_stream_rejects_incomplete_or_overfull_canvas() {
    let (mut file, _, offsets) = striped_image();
    // Disable only the declared-file-length guard to reach block validation.
    file[8..11].fill(0);
    let last = *offsets.last().unwrap();
    assert!(
        image_load(&file[..last])
            .err()
            .expect("malformed image should fail")
            .to_string()
            .contains("do not fill")
    );
    for header_bytes in 1..6 {
        assert!(
            image_load(&file[..last + header_bytes])
                .err()
                .expect("malformed image should fail")
                .to_string()
                .contains("header is truncated")
        );
    }
    assert!(
        image_load(&file[..file.len() - 1])
            .err()
            .expect("malformed image should fail")
            .to_string()
            .contains("block is truncated")
    );
    let extra = file[last..].to_vec();
    file.extend_from_slice(&extra);
    assert!(
        image_load(&file)
            .err()
            .expect("malformed image should fail")
            .to_string()
            .contains("exceed the canvas")
    );
}

#[test]
#[ignore = "requires local Q4 samples; does not establish independent compatibility"]
fn decodes_collected_q4_samples() {
    let root = env::var_os("WML2_Q4_SAMPLE_ROOT")
        .expect("set WML2_Q4_SAMPLE_ROOT to a directory containing samples/*.Q4");
    let root = Path::new(&root).join("samples");
    let mut count = 0;
    for entry in fs::read_dir(&root).expect("Q4 sample directory should be readable") {
        let path = entry.expect("Q4 sample entry should be readable").path();
        if !path.is_file()
            || !path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("q4"))
        {
            continue;
        }
        let data = fs::read(&path).expect("Q4 sample should be readable");
        let image = image_load(&data)
            .unwrap_or_else(|error| panic!("{} should decode: {error}", path.display()));
        assert_eq!((image.width, image.height), (640, 400));
        let buffer = image.buffer.as_ref().expect("Q4 image should have pixels");
        assert_eq!(buffer.len(), 640 * 400 * 4);
        if path.file_name().is_some_and(|name| name == "30000.Q4") {
            for (pixel, rgb) in [
                (0usize, [255, 255, 255]),
                (639, [221, 255, 238]),
                (640, [255, 255, 255]),
                (1000, [221, 255, 238]),
            ] {
                let offset = pixel * 4;
                assert_eq!(&buffer[offset..offset + 4], &[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        count += 1;
    }
    assert!(count >= 1, "at least one Q4 sample is required");
    eprintln!("decoded {count} local Q4 samples; independent oracle is unverified");
}
