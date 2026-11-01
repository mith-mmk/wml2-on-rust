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
    for (data, make) in [
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-meta-little.jxl").as_slice(), "LittleMaker"),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-meta-big-offset.jxl").as_slice(), "BigMaker"),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-meta-brob.jxl").as_slice(), "LittleMaker"),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-meta-multiple.jxl").as_slice(), "LittleMaker"),
    ] {
        use wml2::metadata::DataMap;
        let image = image_from(data).unwrap();
        assert_eq!(image.buffer.as_deref().unwrap(), expected);
        let metadata = image.metadata.as_ref().unwrap();
        assert!(matches!(metadata.get("XMP"), Some(DataMap::Ascii(_))));
        assert!(metadata.iter().any(|(k,v)| k.starts_with("JPEG XL Source Box ") && matches!(v,DataMap::Raw(_))));
        #[cfg(feature = "exif")]
        {
            use wml2::{metadata::exif::{ExifIfd,get_tag}, tiff::header::DataPack};
            let Some(DataMap::Exif(headers)) = metadata.get("EXIF") else { panic!("no EXIF") };
            assert!(matches!(&get_tag(headers,ExifIfd::Primary,0x10f).unwrap().data, DataPack::Ascii(value) if value == make));
            assert!(matches!(&get_tag(headers,ExifIfd::Primary,0x112).unwrap().data, DataPack::Short(value) if value == &[1]));
        }
        #[cfg(not(feature = "exif"))]
        { let _ = make; assert!(!metadata.contains_key("EXIF")); }
        count += 1;
    }
    // C libjxl independently decoded both distinct preview and main images.
    for (input, compressed, native_compressed, size, bits, tolerance) in [
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-rgba8.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-rgba8.rgba.zlib").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-rgba8.rgba16le.zlib").as_slice(),(7,5),8,0),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-icc16.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-icc16.rgba.zlib").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-icc16.rgba16le.zlib").as_slice(),(7,5),16,0),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-oriented-icc16.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-oriented-icc16.rgba.zlib").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-oriented-icc16.rgba16le.zlib").as_slice(),(5,7),16,0),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-grayalpha8.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-grayalpha8.rgba.zlib").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-grayalpha8.rgba16le.zlib").as_slice(),(7,5),8,0),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-larger.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-larger.rgba.zlib").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-larger.rgba16le.zlib").as_slice(),(2,3),8,0),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-lossy.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-lossy.rgba.zlib").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-lossy.rgba16le.zlib").as_slice(),(64,48),8,2),
    ] {
        let expected=miniz_oxide::inflate::decompress_to_vec_zlib(compressed).unwrap();
        let image=image_from(input).unwrap();assert_eq!((image.width,image.height),size);
        for(a,b)in image.buffer.unwrap().chunks_exact(4).zip(expected.chunks_exact(4)){
            for c in 0..3 {assert!(a[c].abs_diff(b[c])<=tolerance);}assert_eq!(a[3],b[3]);
        }
        count+=1;
        let format=if bits==16{PixelFormatEx::Rgba16}else{PixelFormatEx::Rgba8};
        let image=image_from_ex(input,format,PrecisionConversion::Exact).unwrap();
        assert_eq!((image.width(),image.height()),size);assert_eq!(image.source_format(),Some(format));
        if bits==16 {assert_eq!(image.bytes().unwrap(),miniz_oxide::inflate::decompress_to_vec_zlib(native_compressed).unwrap());}
        else {for(a,b)in image.bytes().unwrap().chunks_exact(4).zip(expected.chunks_exact(4)){for c in 0..3{assert!(a[c].abs_diff(b[c])<=tolerance);}assert_eq!(a[3],b[3]);}}
        count+=1;
    }
    let input=include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-larger.jxl");
    let image=image_from_with_limits(input,DecodeLimits{pixels:6,frames:1,animation_bytes:24,expanded_bytes:1_120_000,..Default::default()}).unwrap();
    assert_eq!((image.width,image.height),(2,3));count+=1;
    assert!(image_from_with_limits(include_bytes!("../../wml2/tests/fixtures/review/jxl-preview-oversized-header.jxl"),DecodeLimits::unlimited()).is_err());count+=1;
    assert!(image_from(&input[..784]).is_err());count+=1;
    count + intrinsic_cases() + composition_cases() + associated_cases() + associated_depth_cases() + subsampled_cases() + associated_subsampling_cases() + extra_plane_cases() + selection_plane_cases()
}

fn intrinsic_cases() -> u32 {
    use wml2::image_info::{ImageSize, image_info, image_info_with_limits};
    macro_rules! intrinsic {
        ($n:literal) => {
            (
                include_bytes!(concat!(
                    "../../wml2/tests/fixtures/review/jxl-intrinsic-orientation",
                    $n,
                    ".jxl"
                ))
                .as_slice(),
                include_bytes!(concat!(
                    "../../wml2/tests/fixtures/review/jxl-orientation",
                    $n,
                    ".rgba"
                ))
                .as_slice(),
                $n,
            )
        };
    }
    let mut count = 0;
    for (input, expected, orientation) in [
        intrinsic!(1),
        intrinsic!(2),
        intrinsic!(3),
        intrinsic!(4),
        intrinsic!(5),
        intrinsic!(6),
        intrinsic!(7),
        intrinsic!(8),
    ] {
        let info = image_info(input).unwrap();
        let output = if orientation >= 5 {
            ImageSize {
                width: 3,
                height: 2,
            }
        } else {
            ImageSize {
                width: 2,
                height: 3,
            }
        };
        let display = if orientation >= 5 {
            ImageSize {
                width: 11,
                height: 19,
            }
        } else {
            ImageSize {
                width: 19,
                height: 11,
            }
        };
        assert_eq!(
            info.dimensions.coded,
            ImageSize {
                width: 2,
                height: 3
            }
        );
        assert_eq!(info.dimensions.output, Some(output));
        assert_eq!(info.dimensions.intrinsic_display, Some(display));
        assert_eq!(info.orientation, Some(orientation));
        count += 1;
        let image = image_from(input).unwrap();
        assert_eq!(
            (image.width, image.height),
            (output.width as usize, output.height as usize)
        );
        assert_eq!(image.buffer.as_deref().unwrap(), expected);
        count += 1;
        assert_eq!(
            image_from_ex(input, PixelFormatEx::Rgba8, PrecisionConversion::Exact)
                .unwrap()
                .bytes()
                .unwrap(),
            expected
        );
        count += 1;
    }
    let query_limits = DecodeLimits {
        pixels: 0,
        animation_bytes: 0,
        expanded_bytes: 8192,
        metadata_bytes: 4096,
        ..Default::default()
    };
    let huge =
        include_bytes!("../../wml2/tests/fixtures/review/jxl-intrinsic-huge-coded-header.jxl");
    let info = image_info_with_limits(huge, query_limits).unwrap();
    assert_eq!(
        info.dimensions.coded,
        ImageSize {
            width: 1 << 20,
            height: 1 << 20
        }
    );
    assert!(image_from(huge).is_err());
    count += 1;
    let large = include_bytes!("../../wml2/tests/fixtures/review/jxl-intrinsic-large-display.jxl");
    let info = image_info_with_limits(large, query_limits).unwrap();
    assert_eq!(
        info.dimensions.intrinsic_display,
        Some(ImageSize {
            width: 1 << 20,
            height: 1 << 20
        })
    );
    assert_eq!(
        image_from_with_limits(
            large,
            DecodeLimits {
                pixels: 6,
                animation_bytes: 24,
                ..Default::default()
            }
        )
        .unwrap()
        .buffer
        .as_deref()
        .unwrap(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-orientation1.rgba")
    );
    count += 1;
    for input in [
        include_bytes!("../../wml2/tests/fixtures/review/jxl-intrinsic-oversized-area.jxl")
            .as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-intrinsic-oversized-axis.jxl"),
    ] {
        assert!(image_info_with_limits(input, DecodeLimits::unlimited()).is_err());
        assert!(image_from_with_limits(input, DecodeLimits::unlimited()).is_err());
        count += 1;
    }
    assert_eq!(
        image_info(include_bytes!(
            "../../wml2/tests/fixtures/review/jxl-orientation6.jxl"
        ))
        .unwrap()
        .dimensions
        .intrinsic_display,
        None
    );
    count += 1;
    count + alpha_cases()
}

fn alpha_cases() -> u32 {
    let mut count=0;
    for (input,oracle,color_bits,alpha_bits,orientation) in include!("../../wml2/tests/jpeg_xl/alpha_cases.rs") {
        let expected=words(oracle,16);
        let native_bits=if color_bits!=alpha_bits || matches!(color_bits,9|10) {16} else {color_bits};
        let format=match native_bits {8=>PixelFormatEx::Rgba8,12=>PixelFormatEx::Rgba12,_=>PixelFormatEx::Rgba16};
        let image=image_from_ex(input,format,PrecisionConversion::Exact).unwrap();
        assert_eq!((image.width(),image.height()),if orientation>=5 {(5,7)} else {(7,5)});
        for (i,(actual,expected)) in words(image.bytes().unwrap(),native_bits).iter().zip(&expected).enumerate() {
            let normalized=if native_bits==16 {u32::from(*expected)} else {(u32::from(*expected)*((1<<native_bits)-1)+32767)/65535};
            assert!(u32::from(*actual).abs_diff(normalized)<=1);
            let depth=if i%4==3 {alpha_bits} else {color_bits};let max=(1u32<<depth)-1;let storage=(1u32<<native_bits)-1;
            assert_eq!((u32::from(*actual)*max+storage/2)/storage,(u32::from(*expected)*max+32767)/65535);
        }
        count+=1;
        for (actual,expected) in image_from(input).unwrap().buffer.unwrap().iter().zip(&expected) {assert!(actual.abs_diff(((u32::from(*expected)*255+32767)/65535) as u8)<=1);}
        count+=1;
    }
    for input in [
        include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-depth.jxl").as_slice(),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-spot.jxl"),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-multiple.jxl"),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-multiple-alpha.jxl"),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-float-alpha.jxl"),
        include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-alpha4.jxl"),
    ] {assert!(image_from(input).is_err());assert!(image_from_ex(input,PixelFormatEx::Rgba16,PrecisionConversion::Exact).is_err());count+=1;}
    let input=include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-3c8a16o5.jxl");
    assert!(image_from_with_limits(input,DecodeLimits {animation_bytes:279,..Default::default()}).is_err());count+=1;
    count
}

fn composition_cases() -> u32 {
    let input =
        include_bytes!("../../wml2/tests/fixtures/review/jxl-composition-preview-intrinsic.jxl");
    let info = wml2::image_info::image_info(input).unwrap();
    assert_eq!(
        (info.dimensions.coded.width, info.dimensions.coded.height),
        (7, 5)
    );
    let display = info.dimensions.intrinsic_display.unwrap();
    assert_eq!((display.width, display.height), (11, 19));
    let expected = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "../../wml2/tests/fixtures/review/jxl-preview-oriented-icc16.rgba16le.zlib"
    ))
    .unwrap();
    assert_eq!(
        image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact)
            .unwrap()
            .bytes()
            .unwrap(),
        expected
    );
    let meta = include_bytes!("../../wml2/tests/fixtures/review/jxl-meta-brob.jxl");
    let mut out = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    out.extend_from_slice(b"\0\0\0\x14ftypjxl \0\0\0\0jxl ");
    out.extend_from_slice(&((input.len() + 8) as u32).to_be_bytes());
    out.extend_from_slice(b"jxlc");
    out.extend_from_slice(input);
    let mut at = 12;
    while at < meta.len() {
        let n = u32::from_be_bytes(meta[at..at + 4].try_into().unwrap()) as usize;
        if &meta[at + 4..at + 8] == b"brob" {
            out.extend_from_slice(&meta[at..at + n]);
        }
        at += n;
    }
    let image = image_from(&out).unwrap();
    assert_eq!((image.width, image.height), (5, 7));
    let metadata = image.metadata.unwrap();
    assert!(metadata.contains_key("XMP"));
    #[cfg(feature = "exif")]
    assert!(metadata.contains_key("EXIF"));
    assert!(
        image_from_with_limits(
            &out,
            DecodeLimits {
                metadata_bytes: 18328,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        image_from_with_limits(
            &out,
            DecodeLimits {
                animation_bytes: 279,
                ..Default::default()
            }
        )
        .is_err()
    );
    3
}

fn associated_cases() -> u32 {
    let mut count = 0;
    for (input, expected, bits) in [
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c8o1.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c8o1.rgba16le")
                .as_slice(),
            8,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c8o6.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c8o6.rgba16le")
                .as_slice(),
            8,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c8o8.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c8o8.rgba16le")
                .as_slice(),
            8,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c16o1.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c16o1.rgba16le")
                .as_slice(),
            16,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c16o6.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c16o6.rgba16le")
                .as_slice(),
            16,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c16o8.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-1c16o8.rgba16le")
                .as_slice(),
            16,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c8o1.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c8o1.rgba16le")
                .as_slice(),
            8,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c8o6.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c8o6.rgba16le")
                .as_slice(),
            8,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c8o8.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c8o8.rgba16le")
                .as_slice(),
            8,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c16o1.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c16o1.rgba16le")
                .as_slice(),
            16,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c16o6.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c16o6.rgba16le")
                .as_slice(),
            16,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c16o8.jxl").as_slice(),
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-3c16o8.rgba16le")
                .as_slice(),
            16,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-colored-zero-8.jxl")
                .as_slice(),
            include_bytes!(
                "../../wml2/tests/fixtures/review/jxl-associated-colored-zero-8.rgba16le"
            )
            .as_slice(),
            8,
        ),
        (
            include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-colored-zero-16.jxl")
                .as_slice(),
            include_bytes!(
                "../../wml2/tests/fixtures/review/jxl-associated-colored-zero-16.rgba16le"
            )
            .as_slice(),
            16,
        ),
        (include_bytes!("../../wml2/tests/fixtures/review/jxl-premult.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-original-premult.rgba16le").as_slice(),16),
    ] {
        let im = image_from(input).unwrap();
        let actual = im.buffer.unwrap();
        let expected = words(expected, 16);
        assert_eq!(actual.len(), expected.len());
        assert!(matches!(
            im.metadata.as_ref().unwrap()["JPEG XL Source Alpha Associated"],
            wml2::metadata::DataMap::Bool(true)
        ));
        for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
            let b = ((u32::from(*b) * 255 + 32767) / 65535) as u8;
            if i % 4 == 3 {
                assert_eq!(*a, b)
            } else {
                assert!(a.abs_diff(b) <= 1)
            }
        }
        count += 1;
        let im=image_from_ex(input,PixelFormatEx::Rgba16,PrecisionConversion::Exact).unwrap();
        let values=words(im.bytes().unwrap(),16);
        let _=bits;
        assert_eq!(values.len(), expected.len());
        for (i, (a, b)) in values.iter().zip(&expected).enumerate() {
            let a = *a;
            if i % 4 == 3 {
                assert_eq!(a, *b)
            } else {
                assert!(a.abs_diff(*b) <= 1)
            }
        }
        count += 1;
    }
    assert!(
        image_from(include_bytes!(
            "../../wml2/tests/fixtures/review/jxl-associated-layers-refused.jxl"
        ))
        .is_err()
    );
    count + 1
}

fn associated_depth_cases()->u32 {
 let mut count=0;
 for (input,expected,color,alpha,orientation) in include!("../../wml2/tests/jpeg_xl/associated_depth_cases.rs") {
  let expected=words(expected,16);
  let im=image_from_ex(input,PixelFormatEx::Rgba16,PrecisionConversion::Exact).unwrap();
  assert_eq!((im.width(),im.height()),if orientation>=5{(5,7)}else{(7,5)});
  let values=words(im.bytes().unwrap(),16);assert_eq!(values.len(),expected.len());
  for (i,(a,b)) in values.iter().zip(&expected).enumerate(){if i%4==3{assert_eq!(a,b)}else{assert!(a.abs_diff(*b)<=1)}}
  let metadata=im.metadata.as_ref().unwrap();
  assert!(matches!(metadata["JPEG XL Source Bit Depth"],wml2::metadata::DataMap::UInt(v) if v==u64::from(color)));
  if color!=alpha{assert!(matches!(metadata["JPEG XL Source Alpha Bit Depth"],wml2::metadata::DataMap::UInt(v) if v==u64::from(alpha)));}
  count+=1;
  let raw=image_from(input).unwrap().buffer.unwrap();assert_eq!(raw.len(),expected.len());
  for (i,(a,b)) in raw.iter().zip(&expected).enumerate(){let b=((u32::from(*b)*255+32767)/65535)as u8;if i%4==3{assert_eq!(*a,b)}else{assert!(a.abs_diff(b)<=1)}}
  assert!(image_from_with_limits(input,DecodeLimits{pixels:34,..Default::default()}).is_err());
  assert!(image_from_ex(input,PixelFormatEx::Rgba8,PrecisionConversion::Exact).is_err());
  count+=1;
 }
 count
}

fn subsampled_cases()->u32 {
 let mut count=0;
 for (input,expected,_color,_alpha,shift,w,h,orientation) in include!("../../wml2/tests/jpeg_xl/subsampled_cases.rs") {
  let expected=words(expected,16);let im=image_from_ex(input,PixelFormatEx::Rgba16,PrecisionConversion::Exact).unwrap();
  assert_eq!((im.width(),im.height()),if orientation>=5{(h,w)}else{(w,h)});
  let actual=words(im.bytes().unwrap(),16);assert_eq!(actual.len(),expected.len());
  for (a,b) in actual.iter().zip(&expected){assert!(a.abs_diff(*b)<=1)}
  let metadata=im.metadata.as_ref().unwrap();
  assert!(matches!(metadata["JPEG XL Source Alpha Dim Shift"],wml2::metadata::DataMap::UInt(v)if v==u64::from(shift)));
  for (key,value) in [("JPEG XL Source Alpha Width",w.div_ceil(1usize<<shift)),("JPEG XL Source Alpha Height",h.div_ceil(1usize<<shift))]{assert!(matches!(metadata[key],wml2::metadata::DataMap::UInt(v) if v==value as u64));}
  count+=1;
  let raw=image_from(input).unwrap().buffer.unwrap();assert_eq!(raw.len(),expected.len());for (a,b) in raw.iter().zip(&expected){assert!(a.abs_diff(((u32::from(*b)*255+32767)/65535)as u8)<=1)}
  assert!(image_from_with_limits(input,DecodeLimits{pixels:w*h-1,..Default::default()}).is_err());assert!(image_from_ex(input,PixelFormatEx::Rgba8,PrecisionConversion::Exact).is_err());count+=1;
 }
 for input in [include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-subsampled-layers-refused.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-subsampled-refused-subsampled-layers.jxl").as_slice()]{assert!(image_from(input).is_err());count+=1;}
 let input=include_bytes!("../../wml2/tests/fixtures/review/jxl-subsampled-3c16a8s3o5.jxl");
 for (cap,accepted) in [(959,false),(960,true)] {assert_eq!(wml2::jpegxl::image_from_with_resource_limits(input,DecodeLimits::default(),wml2::jpegxl::JxlResourceLimits{internal_pixels:cap,..Default::default()}).is_ok(),accepted);}
 let (mut lo,mut hi)=(0,64<<20);
 while lo<hi {let mid=lo+(hi-lo)/2;if wml2::jpegxl::image_from_with_resource_limits(input,DecodeLimits::default(),wml2::jpegxl::JxlResourceLimits{workspace_bytes:mid,..Default::default()}).is_ok(){hi=mid}else{lo=mid+1}}
 assert!(wml2::jpegxl::image_from_with_resource_limits(input,DecodeLimits::default(),wml2::jpegxl::JxlResourceLimits{workspace_bytes:lo,..Default::default()}).is_ok());
 assert!(wml2::jpegxl::image_from_with_resource_limits(input,DecodeLimits::default(),wml2::jpegxl::JxlResourceLimits{workspace_bytes:lo-1,..Default::default()}).is_err());
 count+1
}

fn associated_subsampling_cases()->u32 {
 let mut count=0;
 for (input,expected,_,_,shift,w,h,orientation) in include!("../../wml2/tests/jpeg_xl/associated_subsampling_cases.rs") {
  let expected=words(expected,16);let im=image_from_ex(input,PixelFormatEx::Rgba16,PrecisionConversion::Exact).unwrap();
  assert_eq!((im.width(),im.height()),if orientation>=5{(h,w)}else{(w,h)});
  let actual=words(im.bytes().unwrap(),16);assert_eq!(actual.len(),expected.len());for(a,b)in actual.iter().zip(&expected){assert!(a.abs_diff(*b)<=1)}
  let m=im.metadata.as_ref().unwrap();assert!(matches!(m["JPEG XL Source Alpha Associated"],wml2::metadata::DataMap::Bool(true)));
  assert!(matches!(m["JPEG XL Source Alpha Dim Shift"],wml2::metadata::DataMap::UInt(v)if v==u64::from(shift)));
  for(key,value)in[("JPEG XL Source Alpha Width",w.div_ceil(1usize<<shift)),("JPEG XL Source Alpha Height",h.div_ceil(1usize<<shift))]{assert!(matches!(m[key],wml2::metadata::DataMap::UInt(v)if v==value as u64));}
  assert_eq!(image_from_ex(input,PixelFormatEx::Rgba8,PrecisionConversion::Exact).is_ok(),actual.iter().all(|v|v%257==0));count+=1;
  let raw=image_from(input).unwrap().buffer.unwrap();assert_eq!(raw.len(),expected.len());for(a,b)in raw.iter().zip(&expected){assert!(a.abs_diff(((u32::from(*b)*255+32767)/65535)as u8)<=1)}
  assert_eq!(image_from_ex(input,PixelFormatEx::Rgba8,PrecisionConversion::Round).unwrap().bytes().unwrap(),raw);
  assert!(image_from_with_limits(input,DecodeLimits{pixels:w*h-1,..Default::default()}).is_err());count+=1;
 }
 let input=include_bytes!("../../wml2/tests/fixtures/review/jxl-associated-subsampled-3c16a8s3o5.jxl");
 for(cap,accepted)in[(959,false),(960,true)]{assert_eq!(wml2::jpegxl::image_from_with_resource_limits(input,DecodeLimits::default(),wml2::jpegxl::JxlResourceLimits{internal_pixels:cap,..Default::default()}).is_ok(),accepted);}
 let(mut lo,mut hi)=(0,64<<20);while lo<hi{let mid=lo+(hi-lo)/2;if wml2::jpegxl::image_from_with_resource_limits(input,DecodeLimits::default(),wml2::jpegxl::JxlResourceLimits{workspace_bytes:mid,..Default::default()}).is_ok(){hi=mid}else{lo=mid+1}}
 assert!(wml2::jpegxl::image_from_with_resource_limits(input,DecodeLimits::default(),wml2::jpegxl::JxlResourceLimits{workspace_bytes:lo,..Default::default()}).is_ok());assert!(wml2::jpegxl::image_from_with_resource_limits(input,DecodeLimits::default(),wml2::jpegxl::JxlResourceLimits{workspace_bytes:lo-1,..Default::default()}).is_err());
 count+1
}

fn extra_plane_cases() -> u32 {
    use wml2::jpegxl::{image_with_extra_channels_from_with_resource_limits_and_cancel as extra, JxlResourceLimits, DecodeCancelled};
    let mut count = 0;
    for (input, color, depth, cb, db, o) in include!("../../wml2/tests/jpeg_xl/depth_cases.rs") {
        let im = extra(input, DecodeLimits::default(), JxlResourceLimits::default(), &|| false).unwrap();
        assert_eq!(im.color_bits_per_sample, cb);
        assert_eq!(im.source_orientation, o);
        assert_eq!(im.extra_planes.len(), 1);
        let p = &im.extra_planes[0];
        assert_eq!(p.bits_per_sample, db);
        assert_eq!(p.source_index, 0);
        assert_eq!(p.samples, words(depth, 16));
        let expected = words(color, 16);
        for (i, (a,b)) in im.rgba16.iter().zip(&expected).enumerate() {
            assert!(a.abs_diff(*b) <= 1);
            if i % 4 == 3 {assert_eq!(*a, 65535);}
        }
        assert!(input.windows(im.source_header.len()).any(|b| b == im.source_header));
        assert!(image_from(input).is_err());
        count += 1;
    }
    for input in [include_bytes!("../../wml2/tests/fixtures/review/jxl-gray16.jxl").as_slice(), include_bytes!("../../wml2/tests/fixtures/review/jxl-rgb16.jxl").as_slice()] {
        assert!(extra(input, DecodeLimits::default(), JxlResourceLimits::default(), &|| false).unwrap().extra_planes.is_empty()); count += 1;
    }
    let input = include_bytes!("../../wml2/tests/fixtures/review/jxl-depth-3c8d16o5.jxl");
    for (cap, accepted) in [(349, false), (350, true)] {
        assert_eq!(extra(input, DecodeLimits {animation_bytes:cap, ..Default::default()}, JxlResourceLimits::default(), &||false).is_ok(),accepted);
    }
    let (mut lo, mut hi) = (0, 64 << 20);
    while lo < hi {
        let mid = lo + (hi-lo)/2;
        if extra(input,DecodeLimits::default(),JxlResourceLimits{workspace_bytes:mid,..Default::default()},&||false).is_ok(){hi=mid}else{lo=mid+1}
    }
    for (cap, accepted) in [(lo-1, false), (lo, true)] {
        assert_eq!(extra(input, DecodeLimits::default(), JxlResourceLimits{workspace_bytes:cap,..Default::default()}, &||false).is_ok(),accepted);
    }
    count += 1;
    let probes = std::cell::Cell::new(0);
    drop(extra(input,DecodeLimits::default(),JxlResourceLimits::default(),&||{probes.set(probes.get()+1);false}).unwrap());
    assert!(probes.get()>10);
    for stop in 1..=probes.get() {
        let n=std::cell::Cell::new(0);
        let e=extra(input,DecodeLimits::default(),JxlResourceLimits::default(),&||{n.set(n.get()+1);n.get()==stop}).unwrap_err();
        assert!(e.is::<DecodeCancelled>());
    }
    count += 1;
    for input in [include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-spot.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-multiple.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-depth-refused-depth-float.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-depth-refused-depth-subsampled.jxl").as_slice()] {
        assert!(extra(input,DecodeLimits::default(),JxlResourceLimits::default(),&||false).is_err());
    }
    count + 1
}

fn selection_plane_cases() -> u32 {
    use wml2::jpegxl::{image_with_extra_channels_from_with_resource_limits_and_cancel as extra, JxlResourceLimits, DecodeCancelled};
    let mut count = 0;
    for (input, color, selection, cb, db, o) in include!("../../wml2/tests/jpeg_xl/selection_cases.rs") {
        let im = extra(input, DecodeLimits::default(), JxlResourceLimits::default(), &|| false).unwrap();
        assert_eq!(im.color_bits_per_sample, cb);
        assert_eq!(im.source_orientation, o);
        assert_eq!(im.extra_planes.len(), 1);
        let p = &im.extra_planes[0];
        assert_eq!(p.bits_per_sample, db);
        assert_eq!(p.kind, wml2::jpegxl::JxlExtraChannelKind::SelectionMask);
        assert_eq!(p.source_index, 0);
        assert_eq!(p.samples, words(selection, 16));
        let expected = words(color, 16);
        for (i, (a,b)) in im.rgba16.iter().zip(&expected).enumerate() {
            assert!(a.abs_diff(*b) <= 1);
            if i % 4 == 3 {assert_eq!(*a, 65535);}
        }
        assert!(input.windows(im.source_header.len()).any(|b| b == im.source_header));
        assert!(image_from(input).is_err());
        count += 1;
    }
    for input in [include_bytes!("../../wml2/tests/fixtures/review/jxl-gray16.jxl").as_slice(), include_bytes!("../../wml2/tests/fixtures/review/jxl-rgb16.jxl").as_slice()] {
        assert!(extra(input, DecodeLimits::default(), JxlResourceLimits::default(), &|| false).unwrap().extra_planes.is_empty()); count += 1;
    }
    let input = include_bytes!("../../wml2/tests/fixtures/review/jxl-selection-3c8d16o5.jxl");
    for (cap, accepted) in [(349, false), (350, true)] {
        assert_eq!(extra(input, DecodeLimits {animation_bytes:cap, ..Default::default()}, JxlResourceLimits::default(), &||false).is_ok(),accepted);
    }
    let (mut lo, mut hi) = (0, 64 << 20);
    while lo < hi {
        let mid = lo + (hi-lo)/2;
        if extra(input,DecodeLimits::default(),JxlResourceLimits{workspace_bytes:mid,..Default::default()},&||false).is_ok(){hi=mid}else{lo=mid+1}
    }
    for (cap, accepted) in [(lo-1, false), (lo, true)] {
        assert_eq!(extra(input, DecodeLimits::default(), JxlResourceLimits{workspace_bytes:cap,..Default::default()}, &||false).is_ok(),accepted);
    }
    count += 1;
    let probes = std::cell::Cell::new(0);
    drop(extra(input,DecodeLimits::default(),JxlResourceLimits::default(),&||{probes.set(probes.get()+1);false}).unwrap());
    assert!(probes.get()>10);
    for stop in 1..=probes.get() {
        let n=std::cell::Cell::new(0);
        let e=extra(input,DecodeLimits::default(),JxlResourceLimits::default(),&||{n.set(n.get()+1);n.get()==stop}).unwrap_err();
        assert!(e.is::<DecodeCancelled>());
    }
    count += 1;
    for input in [include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-spot.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-alpha-refused-multiple.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-selection-refused-selection-multiple.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-selection-refused-selection-float.jxl").as_slice(),include_bytes!("../../wml2/tests/fixtures/review/jxl-selection-refused-selection-subsampled.jxl").as_slice()] {
        assert!(extra(input,DecodeLimits::default(),JxlResourceLimits::default(),&||false).is_err());
    }
    count + 1
}
