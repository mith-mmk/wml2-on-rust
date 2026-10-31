use wml2::draw::*;
use wml2::limits::DecodeLimits;
fn words(bytes: &[u8], bits: u8) -> Vec<u16> {
    if bits == 8 {
        bytes.iter().map(|&v| v.into()).collect()
    } else {
        bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect()
    }
}
macro_rules! case {
    ($name:literal,$channels:literal,$bits:literal) => {
        (
            include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-", $name, ".jxl")).as_slice(),
            include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-", $name, ".bin")).as_slice(),
            $channels,
            $bits,
        )
    };
}
#[unsafe(no_mangle)]
pub extern "C" fn verify_jpegxl() -> u32 {
    let mut count = 0;
    for (input, expected, channels, bits) in [
        case!("gray8", 1, 8),
        case!("gray12", 1, 12),
        case!("gray16", 1, 16),
        case!("grayalpha8", 2, 8),
        case!("grayalpha12", 2, 12),
        case!("grayalpha16", 2, 16),
        case!("rgb8", 3, 8),
        case!("rgb12", 3, 12),
        case!("rgb16", 3, 16),
        case!("rgba8", 4, 8),
        case!("rgba12", 4, 12),
        case!("rgba16", 4, 16),
    ] {
        let format = match (channels == 1, bits) {
            (true, 8) => PixelFormatEx::Gray8,
            (true, 12) => PixelFormatEx::Gray12,
            (true, 16) => PixelFormatEx::Gray16,
            (false, 8) => PixelFormatEx::Rgba8,
            (false, 12) => PixelFormatEx::Rgba12,
            _ => PixelFormatEx::Rgba16,
        };
        let values = words(expected, bits);
        let max = ((1u32 << bits) - 1) as u16;
        let rgba: Vec<u16> = values
            .chunks_exact(channels)
            .flat_map(|p| match channels {
                1 => [p[0], p[0], p[0], max],
                2 => [p[0], p[0], p[0], p[1]],
                3 => [p[0], p[1], p[2], max],
                _ => [p[0], p[1], p[2], p[3]],
            })
            .collect();
        let image = image_from_ex(input, format, PrecisionConversion::Exact).unwrap();
        assert_eq!(image.source_format(), Some(format));
        assert_eq!(
            words(image.bytes().unwrap(), bits),
            if channels == 1 { values } else { rgba.clone() }
        );
        count += 1;
        let old = image_from(input).unwrap();
        let expected: Vec<u8> = rgba
            .iter()
            .map(|&v| ((u32::from(v) * 255 + u32::from(max) / 2) / u32::from(max)) as u8)
            .collect();
        assert_eq!(old.buffer.unwrap(), expected);
        count += 1;
        assert!(
            image_from_ex_with_limits(
                input,
                format,
                PrecisionConversion::Exact,
                DecodeLimits {
                    pixels: 34,
                    ..Default::default()
                }
            )
            .is_err()
        );
        count += 1;
    }
    let input = include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-icc-lossless.jxl"));
    let image = image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
    assert_eq!(
        image.bytes().unwrap(),
        include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-rgba16.bin"))
    );
    count += 1;
    assert!(
        image_from_with_limits(
            input,
            DecodeLimits {
                metadata_bytes: 100,
                ..Default::default()
            }
        )
        .is_err()
    );
    count += 1;
    let input = include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-lossy.jxl"));
    let image = image_from(input).unwrap();
    let pixels = image.buffer.unwrap();
    let expected = include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-lossy-djxl.bin"));
    for (actual, expected) in pixels.chunks_exact(4).zip(expected.chunks_exact(3)) {
        for c in 0..3 {
            assert!(actual[c].abs_diff(expected[c]) <= 2);
        }
        assert_eq!(actual[3], 255);
    }
    count += 1;
    let input = include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-icc-gray16.jxl"));
    let image = image_from_ex(input, PixelFormatEx::Gray16, PrecisionConversion::Exact).unwrap();
    assert_eq!(
        image.bytes().unwrap(),
        include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-gray16.bin"))
    );
    count += 1;
    assert!(image_from(input).is_err());
    count += 1;
    let raw = include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-rgba8.jxl"));
    for level in [5, 10] {
        for partial in [false, true] {
            let mut boxed = b"\0\0\0\x0cJXL \r\n\x87\n\0\0\0\x14ftypjxl \0\0\0\0jxl ".to_vec();
            boxed.extend(9u32.to_be_bytes());
            boxed.extend(b"jxll");
            boxed.push(level);
            boxed.extend(9u32.to_be_bytes());
            boxed.extend(b"jbrd");
            boxed.push(255); // Deliberately uninterpreted reconstruction data.
            boxed.extend((raw.len() as u32 + if partial { 12 } else { 8 }).to_be_bytes());
            boxed.extend(if partial { b"jxlp" } else { b"jxlc" });
            if partial {
                boxed.extend(0x8000_0000u32.to_be_bytes());
            }
            boxed.extend(raw);
            boxed.extend(8u32.to_be_bytes());
            boxed.extend(b"jbrd"); // Retain duplicate/empty payload distinctly.
            let image = image_from(&boxed).unwrap();
            assert_eq!(
                image
                    .metadata
                    .as_ref()
                    .unwrap()
                    .keys()
                    .filter(|k| k.ends_with("6a627264"))
                    .count(),
                2
            );
            assert_eq!(
                image.buffer.unwrap(),
                include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-rgba8.bin"))
            );
            count += 1;
        }
    }
    for (input, icc, probes, width, height, source_channels, format) in [
        (
            include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-jbrd-bench.jxl")).as_slice(),
            include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-jbrd-bench.icc")).as_slice(),
            include_bytes!(concat!(
                "../../wml2/tests/fixtures/review",
                "/jxl-jbrd-bench-probes.bin"
            ))
            .as_slice(),
            606,
            500,
            3,
            PixelFormatEx::Rgba8,
        ),
        (
            include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-jbrd-gray.jxl")).as_slice(),
            include_bytes!(concat!("../../wml2/tests/fixtures/review", "/jxl-jbrd-gray.icc")).as_slice(),
            include_bytes!(concat!(
                "../../wml2/tests/fixtures/review",
                "/jxl-jbrd-gray-probes.bin"
            ))
            .as_slice(),
            200,
            200,
            1,
            PixelFormatEx::Gray8,
        ),
    ] {
        let image = image_from_ex(input, format, PrecisionConversion::Exact).unwrap();
        assert_eq!((image.width(), image.height()), (width, height));
        assert_eq!(image.source_format(), Some(format));
        for record in probes.chunks_exact(4 + source_channels) {
            let x = u16::from_le_bytes([record[0], record[1]]) as usize;
            let y = u16::from_le_bytes([record[2], record[3]]) as usize;
            let pixel = &image.bytes().unwrap()[(y * width + x) * format.channels()..];
            for c in 0..source_channels {
                assert!(pixel[c].abs_diff(record[4 + c]) <= 2);
            }
        }
        for key in ["ICC Profile", "JPEG XL Source ICC Profile"] {
            assert!(
                matches!(&image.metadata.as_ref().unwrap()[key],wml2::metadata::DataMap::ICCProfile(v) if v == icc)
            );
        }
        count += 1;
        if source_channels == 3 {
            assert_eq!(
                image_from(input).unwrap().buffer.unwrap(),
                image.bytes().unwrap()
            );
        } else {
            assert!(
                image_from(input)
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("Gray ICC expansion")
            );
        }
        count += 1;
        for limits in [
            DecodeLimits {
                input_bytes: input.len() - 1,
                ..Default::default()
            },
            DecodeLimits {
                pixels: width * height - 1,
                ..Default::default()
            },
            DecodeLimits {
                metadata_bytes: 100,
                ..Default::default()
            },
        ] {
            assert!(
                image_from_ex_with_limits(input, format, PrecisionConversion::Exact, limits)
                    .is_err()
            );
            count += 1;
        }
    }
    for (input, expected, coded, bits) in [
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba9.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba9.rgba16le").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba9.source16le").as_slice(),9),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba10.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba10.rgba16le").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba10.source16le").as_slice(),10),
    ] {
        let image=image_from_ex(input,PixelFormatEx::Rgba16,PrecisionConversion::Exact).unwrap();
        for ((actual,oracle),source) in words(image.bytes().unwrap(),16).iter().zip(words(expected,16)).zip(words(coded,16)) {
            assert!(actual.abs_diff(oracle)<=1);
            assert_eq!((u32::from(*actual)*((1<<bits)-1)+32767)/65535,u32::from(source));
        }
        count+=1;
        let expected8:Vec<u8>=words(expected,16).iter().map(|&v|((u32::from(v)*255+32767)/65535) as u8).collect();
        assert_eq!(image_from(input).unwrap().buffer.unwrap(),expected8);count+=1;
        assert!(image_from_with_limits(input,DecodeLimits {pixels:34,..Default::default()}).is_err());count+=1;
    }
    // Independent C libbrotli streams, with C libjxl box/pixel verification.
    let brob = include_bytes!("../../wml2/tests/fixtures/review/jxl-brob-metadata.jxl");
    let expected = include_bytes!("../../wml2/tests/fixtures/review/jxl-rgba8-djxl.bin");
    let image = image_from(brob).unwrap();
    assert_eq!(image.buffer.as_deref().unwrap(), expected);
    assert!(
        matches!(&image.metadata.as_ref().unwrap()["JPEG XL Decoded Box 3 786d6c20"], wml2::metadata::DataMap::Raw(v) if v.starts_with(b"<x:xmpmeta"))
    );
    assert!(
        matches!(&image.metadata.as_ref().unwrap()["JPEG XL Source Box 3 62726f62"], wml2::metadata::DataMap::Raw(v) if v.starts_with(b"xml "))
    );
    count += 1;
    let native = image_from_ex(brob, PixelFormatEx::Rgba8, PrecisionConversion::Exact).unwrap();
    assert_eq!(native.bytes().unwrap(), expected);
    count += 1;
    for limits in [
        DecodeLimits {
            input_bytes: brob.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 1,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1024,
            ..Default::default()
        },
    ] {
        assert!(image_from_with_limits(brob, limits).is_err());
        count += 1;
    }
    assert!(image_from(&brob[..brob.len() - 1]).is_err());
    count += 1;
    let mut forbidden = brob.to_vec();
    assert_eq!(&forbidden[40..44], b"Exif");
    forbidden[40..44].copy_from_slice(b"jbrd");
    assert!(image_from(&forbidden).is_err());
    count += 1;
    let mut trailing = brob.to_vec(); // Add a byte to the last bounded brob stream.
    let mut position = 12;
    while position < trailing.len() {
        let length =
            u32::from_be_bytes(trailing[position..position + 4].try_into().unwrap()) as usize;
        if position + length == trailing.len() {
            trailing[position..position + 4].copy_from_slice(&((length + 1) as u32).to_be_bytes());
            break;
        }
        position += length;
    }
    trailing.push(0);
    assert!(image_from(&trailing).is_err());
    count += 1;
    count
}
