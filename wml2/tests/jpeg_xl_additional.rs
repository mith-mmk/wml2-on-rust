#![cfg(feature = "jpeg-xl")]
use wml2::draw::*;
use wml2::metadata::DataMap;
macro_rules! orientation {
    ($n:literal) => {
        (
            include_bytes!(concat!("fixtures/review/jxl-orientation", $n, ".jxl")).as_slice(),
            include_bytes!(concat!("fixtures/review/jxl-orientation", $n, ".rgba")).as_slice(),
            $n,
        )
    };
}
#[test]
fn all_eight_orientations_match_independent_djxl_after_one_reorientation() {
    for (input, expected, orientation) in [
        orientation!(1),
        orientation!(2),
        orientation!(3),
        orientation!(4),
        orientation!(5),
        orientation!(6),
        orientation!(7),
        orientation!(8),
    ] {
        let size = if orientation <= 4 { (2, 3) } else { (3, 2) };
        let image = image_from(input).unwrap();
        let metadata = image.metadata.as_ref().unwrap();
        assert!(matches!(
            metadata["orientation_applied"],
            DataMap::Bool(true)
        ));
        assert!(
            matches!(metadata["JPEG XL Source Orientation"], DataMap::UInt(v) if v == orientation)
        );
        assert_eq!(
            (image.width, image.height),
            size,
            "orientation {orientation}"
        );
        assert_eq!(
            image.buffer.as_deref().unwrap(),
            expected,
            "orientation {orientation}"
        );
        #[cfg(feature = "image-buffer-ex")]
        {
            let image =
                image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Exact).unwrap();
            assert!(matches!(
                image.metadata.as_ref().unwrap()["orientation_applied"],
                DataMap::Bool(true)
            ));
            assert!(
                matches!(image.metadata.as_ref().unwrap()["JPEG XL Source Orientation"], DataMap::UInt(v) if v == orientation)
            );
            assert_eq!((image.width(), image.height()), size);
            assert_eq!(
                image.bytes().unwrap(),
                expected,
                "EX orientation {orientation}"
            );
        }
    }
}
#[test]
fn nine_and_ten_bit_sources_promote_to_normalized_sixteen_without_losing_codes() {
    for (input, expected, source, bits) in [
        (
            include_bytes!("fixtures/review/jxl-rgba9.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-rgba9.rgba16le").as_slice(),
            include_bytes!("fixtures/review/jxl-rgba9.source16le").as_slice(),
            9,
        ),
        (
            include_bytes!("fixtures/review/jxl-rgba10.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-rgba10.rgba16le").as_slice(),
            include_bytes!("fixtures/review/jxl-rgba10.source16le").as_slice(),
            10,
        ),
    ] {
        let image = image_from(input).unwrap();
        let metadata = image.metadata.unwrap();
        assert!(matches!(metadata["JPEG XL Source Bit Depth"],DataMap::UInt(v) if v==bits));
        assert!(matches!(
            metadata["JPEG XL Output Bit Depth"],
            DataMap::UInt(8)
        ));
        let rgba8: Vec<u8> = expected
            .chunks_exact(2)
            .map(|b| ((u32::from(u16::from_le_bytes([b[0], b[1]])) * 255 + 32767) / 65535) as u8)
            .collect();
        assert_eq!(image.buffer.unwrap(), rgba8);
        #[cfg(feature = "image-buffer-ex")]
        {
            let image =
                image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba16));
            let actual = image.bytes().unwrap();
            assert_eq!(actual.len(), expected.len());
            for (a, b) in actual.chunks_exact(2).zip(expected.chunks_exact(2)) {
                assert!(
                    u16::from_le_bytes([a[0], a[1]]).abs_diff(u16::from_le_bytes([b[0], b[1]]))
                        <= 1
                );
            }
            let max = (1u32 << bits) - 1;
            for (output, input) in actual.chunks_exact(2).zip(source.chunks_exact(2)) {
                let promoted = u32::from(u16::from_le_bytes([output[0], output[1]]));
                let original = u32::from(u16::from_le_bytes([input[0], input[1]]));
                assert_eq!((promoted * max + 32767) / 65535, original);
            }
            assert!(matches!(
                image.metadata.as_ref().unwrap()["JPEG XL Output Bit Depth"],
                DataMap::UInt(16)
            ));
        }
        #[cfg(not(feature = "image-buffer-ex"))]
        let _ = source;
    }
}
#[cfg(feature = "color-management")]
#[test]
fn explicit_srgb_display_uses_the_actual_reconstruction_profile_once() {
    let bench = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "fixtures/review/jxl-jbrd-bench.srgb.rgba.zlib"
    ))
    .unwrap();
    let gray = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "fixtures/review/jxl-jbrd-gray.srgb.rgba.zlib"
    ))
    .unwrap();
    for (input, expected, original, size) in [
        (
            include_bytes!("fixtures/review/jxl-icc-gray16.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-icc-gray16.srgb.rgba").as_slice(),
            include_bytes!("fixtures/review/jxl-gray-gamma2.icc").as_slice(),
            (7, 5),
        ),
        (
            include_bytes!("fixtures/review/jxl-icc-lossless.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-icc-lossless.srgb.rgba").as_slice(),
            include_bytes!("fixtures/review/jxl-gamma2.icc").as_slice(),
            (7, 5),
        ),
        (
            include_bytes!("fixtures/review/jxl-icc-lossy.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-icc-lossy.srgb.rgba").as_slice(),
            include_bytes!("fixtures/review/jxl-gamma2.icc").as_slice(),
            (7, 5),
        ),
        (
            include_bytes!("fixtures/review/jxl-jbrd-bench.jxl").as_slice(),
            bench.as_slice(),
            include_bytes!("fixtures/review/jxl-jbrd-bench.icc").as_slice(),
            (606, 500),
        ),
        (
            include_bytes!("fixtures/review/jxl-jbrd-gray.jxl").as_slice(),
            gray.as_slice(),
            include_bytes!("fixtures/review/jxl-jbrd-gray.icc").as_slice(),
            (200, 200),
        ),
    ] {
        let image = wml2::jpegxl::image_from_srgb(input).unwrap();
        assert_eq!((image.width, image.height), size);
        let pixels = image.buffer.unwrap();
        assert_eq!(pixels.len(), expected.len());
        for (actual, reference) in pixels.chunks_exact(4).zip(expected.chunks_exact(4)) {
            for c in 0..3 {
                assert!(
                    actual[c].abs_diff(reference[c]) <= 2,
                    "actual={actual:?}, reference={reference:?}"
                );
            }
            assert_eq!(actual[3], reference[3]);
        }
        let meta = image.metadata.unwrap();
        assert!(
            matches!(&meta["ICC Profile"],DataMap::ICCProfile(p) if p==include_bytes!("../src/srgb.icc"))
        );
        assert!(
            matches!(&meta["JPEG XL Source ICC Profile"],DataMap::ICCProfile(p) if p==original)
        );
        assert!(
            matches!(&meta["JPEG XL Reconstruction ICC Profile"],DataMap::ICCProfile(p) if p.len()>=128)
        );
        assert!(
            wml2::jpegxl::image_from_srgb_with_limits(
                input,
                wml2::limits::DecodeLimits {
                    metadata_bytes: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            wml2::jpegxl::image_from_srgb_with_limits(
                input,
                wml2::limits::DecodeLimits {
                    expanded_bytes: 1024,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}

// Standalone trusted-core evaluation, kept separate from the shipping adapter.
#[cfg(feature = "jpeg-xl")]
#[path = "common/jpegxl_progressive.rs"]
mod progressive_core_evaluation;

#[cfg(all(feature = "jpeg-xl", feature = "color-management"))]
#[test]
fn progressive_whole_input_cms_matches_independent_c_reference() {
    for (data, reference) in [
        (
            include_bytes!("fixtures/review/jxl-progressive-ac.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-progressive-ac.rgba16le").as_slice(),
        ),
        (
            include_bytes!("fixtures/review/jxl-progressive-dc.jxl").as_slice(),
            include_bytes!("fixtures/review/jxl-progressive-dc.rgba16le").as_slice(),
        ),
    ] {
        let image = wml2::jpegxl::image_from_srgb(data).unwrap();
        assert_eq!((image.width, image.height), (32, 24));
        let actual = image.buffer.unwrap();
        assert_eq!(actual.len() * 2, reference.len());
        for (i, (a, b)) in actual.iter().zip(reference.chunks_exact(2)).enumerate() {
            let expected =
                ((u32::from(u16::from_le_bytes([b[0], b[1]])) * 255 + 32767) / 65535) as u8;
            if i % 4 == 3 {
                assert_eq!(*a, expected);
            } else {
                assert!(a.abs_diff(expected) <= 2);
            }
        }
    }
}

#[cfg(feature = "jpeg-xl")]
#[test]
fn progressive_invalid_envelopes_and_limits_publish_no_canvas() {
    use bin_rs::reader::BytesReader;
    use wml2::draw::{DecodeOptions, ImageBuffer, image_decoder_with_limits};
    use wml2::limits::DecodeLimits;
    let raw = include_bytes!("fixtures/review/jxl-progressive-dc.jxl");
    let container = include_bytes!("fixtures/review/jxl-progressive-dc-jxlc.jxl");
    let mut bad_tail = container.to_vec();
    bad_tail.extend_from_slice(b"\0\0\0\x10xml");
    let mut junk = raw.to_vec();
    junk.push(255);
    let defaults = DecodeLimits::default();
    for (data, limits) in [
        (&raw[..raw.len() - 1], defaults),
        (junk.as_slice(), defaults),
        (&container[..container.len() - 1], defaults),
        (bad_tail.as_slice(), defaults),
        (
            raw.as_slice(),
            DecodeLimits {
                input_bytes: raw.len() - 1,
                ..defaults
            },
        ),
        (
            raw.as_slice(),
            DecodeLimits {
                pixels: 32 * 24 - 1,
                ..defaults
            },
        ),
        (
            raw.as_slice(),
            DecodeLimits {
                expanded_bytes: 0,
                ..defaults
            },
        ),
        (
            raw.as_slice(),
            DecodeLimits {
                metadata_bytes: 0,
                ..defaults
            },
        ),
        (
            raw.as_slice(),
            DecodeLimits {
                frames: 0,
                ..defaults
            },
        ),
    ] {
        let mut output = ImageBuffer::new();
        let mut reader = BytesReader::new(data);
        assert!(
            image_decoder_with_limits(
                &mut reader,
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut output
                },
                limits
            )
            .is_err()
        );
        assert_eq!((output.width, output.height), (0, 0));
        assert!(output.buffer.is_none());
        assert!(output.metadata.is_none());
    }
}
