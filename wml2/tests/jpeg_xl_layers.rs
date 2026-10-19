#![cfg(feature = "jpeg-xl")]
use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;

const INPUT: &[u8] = include_bytes!("fixtures/review/jxl-sunset-logo.jxl");
const PIXELS: usize = 924 * 1386;
fn reference(bytes: &[u8]) -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec_zlib(bytes).unwrap()
}

#[test]
fn oversized_negative_origin_layers_crop_blend_and_orient_to_one_canvas() {
    let expected = reference(include_bytes!("fixtures/review/jxl-sunset-logo.rgba.zlib"));
    let limits = DecodeLimits {
        pixels: PIXELS,
        frames: 1,
        ..Default::default()
    };
    let image = image_from_with_limits(INPUT, limits).unwrap();
    assert_eq!((image.width, image.height), (924, 1386));
    assert_eq!(image.buffer.unwrap(), expected);
    #[cfg(feature = "image-buffer-ex")]
    {
        let image = image_from_ex_with_limits(
            INPUT,
            PixelFormatEx::Rgba16,
            PrecisionConversion::Exact,
            limits,
        )
        .unwrap();
        assert_eq!((image.width(), image.height()), (924, 1386));
        let bytes = image.bytes().unwrap();
        let rounded: Vec<u8> = bytes
            .chunks_exact(2)
            .map(|b| ((u32::from(u16::from_le_bytes([b[0], b[1]])) * 255 + 32767) / 65535) as u8)
            .collect();
        assert_eq!(rounded, expected);
        for line in include_str!("fixtures/review/jxl-sunset-logo.probes.txt").lines() {
            let values: Vec<usize> = line
                .split_whitespace()
                .map(|v| v.parse().unwrap())
                .collect();
            for c in 0..4 {
                let at = values[0] * 8 + c * 2;
                let actual = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
                assert!(
                    actual.abs_diff(values[c + 1] as u16) <= 1,
                    "pixel{} channel{c}",
                    values[0]
                );
            }
        }
    }
    #[cfg(feature = "color-management")]
    {
        let expected = reference(include_bytes!(
            "fixtures/review/jxl-sunset-logo.srgb.rgba.zlib"
        ));
        let image = wml2::jpegxl::image_from_srgb_with_limits(INPUT, limits).unwrap();
        assert_eq!((image.width, image.height), (924, 1386));
        for (a, b) in image.buffer.unwrap().iter().zip(&expected) {
            assert!(a.abs_diff(*b) <= 1);
        }
    }
}

fn rejected_before_initialization(data: &[u8], limits: DecodeLimits) {
    let mut image = ImageBuffer::new();
    let mut reader = BytesReader::new(data);
    let result = image_decoder_with_limits(
        &mut reader,
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        limits,
    );
    assert!(result.is_err());
    assert_eq!((image.width, image.height), (0, 0));
    assert!(image.buffer.is_none());
    assert!(image.metadata.is_none());
}

#[test]
fn internal_storage_is_admitted_separately_from_canvas_pixels() {
    // The old canvas-derived estimate fitted256MiB; the two2048x1024 layers
    // and conservative retained planes do not. No visible callback may run.
    for limits in [
        DecodeLimits {
            pixels: PIXELS - 1,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 256 << 20,
            ..Default::default()
        },
        DecodeLimits {
            frames: 0,
            ..Default::default()
        },
    ] {
        rejected_before_initialization(INPUT, limits);
    }
    for n in 0..INPUT.len() {
        rejected_before_initialization(&INPUT[..n], DecodeLimits::default());
    }
}

#[test]
fn internal_layer_count_stays_bounded_with_unlimited_caller_limits() {
    // Repeating the independently coded invisible reference frame preserves
    // its per-frame TOC. The fixed64-layer cap must stop preflight before pixels.
    let mut input = INPUT[..9].to_vec();
    for _ in 0..64 {
        input.extend_from_slice(&INPUT[9..125]);
    }
    input.extend_from_slice(&INPUT[125..]);
    rejected_before_initialization(&input, DecodeLimits::unlimited());
}
