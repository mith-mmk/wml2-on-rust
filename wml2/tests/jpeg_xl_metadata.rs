#![cfg(feature = "jpeg-xl")]
use wml2::{
    draw::{DecodeOptions, ImageBuffer, image_decoder_with_limits, image_from},
    limits::DecodeLimits,
    metadata::DataMap,
};
const RAW: &[u8] = include_bytes!("fixtures/review/jxl-rgba8.jxl");
const EXPECTED: &[u8] = include_bytes!("fixtures/review/jxl-rgba8-djxl.bin");
const LITTLE: &[u8] = include_bytes!("fixtures/review/jxl-meta-little.tiff");
const BIG: &[u8] = include_bytes!("fixtures/review/jxl-meta-big.tiff");
const XMP: &[u8] = include_bytes!("fixtures/review/jxl-meta.xmp");
fn box_bytes(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut result = ((data.len() + 8) as u32).to_be_bytes().to_vec();
    result.extend(kind);
    result.extend(data);
    result
}
fn exif(offset: u32, padding: &[u8], tiff: &[u8]) -> Vec<u8> {
    let mut p = offset.to_be_bytes().to_vec();
    p.extend(padding);
    p.extend(tiff);
    p
}
fn container(boxes: Vec<Vec<u8>>) -> Vec<u8> {
    let mut data = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    data.extend(box_bytes(b"ftyp", b"jxl \0\0\0\0jxl "));
    data.extend(box_bytes(b"jxlc", RAW));
    for b in boxes {
        data.extend(b);
    }
    data
}
fn raw_count(image: &ImageBuffer) -> usize {
    image
        .metadata
        .as_ref()
        .unwrap()
        .iter()
        .filter(|(k, v)| k.starts_with("JPEG XL Source Box ") && matches!(v, DataMap::Raw(_)))
        .count()
}
#[cfg(feature = "exif")]
fn common(image: &ImageBuffer, make: &str) {
    use wml2::{
        metadata::exif::{ExifIfd, get_tag},
        tiff::header::DataPack,
    };
    let Some(DataMap::Exif(headers)) = image.metadata.as_ref().unwrap().get("EXIF") else {
        panic!("no typed EXIF")
    };
    assert!(
        matches!(&get_tag(headers,ExifIfd::Primary,0x10f).unwrap().data,DataPack::Ascii(value) if value==make)
    );
    assert!(
        matches!(&get_tag(headers,ExifIfd::Primary,0x112).unwrap().data,DataPack::Short(value) if value==&[1])
    );
}
#[test]
fn independent_endian_offset_brob_and_multiple_fixtures_keep_raw_and_normalize_orientation() {
    for (bytes, make, count) in [
        (
            include_bytes!("fixtures/review/jxl-meta-little.jxl").as_slice(),
            "LittleMaker",
            2,
        ),
        (
            include_bytes!("fixtures/review/jxl-meta-big-offset.jxl").as_slice(),
            "BigMaker",
            2,
        ),
        (
            include_bytes!("fixtures/review/jxl-meta-brob.jxl").as_slice(),
            "LittleMaker",
            2,
        ),
        (
            include_bytes!("fixtures/review/jxl-meta-multiple.jxl").as_slice(),
            "LittleMaker",
            4,
        ),
        (
            include_bytes!("fixtures/review/jxl-meta-brob-invalid.jxl").as_slice(),
            "LittleMaker",
            4,
        ),
    ] {
        let image = image_from(bytes).unwrap();
        assert_eq!(image.buffer.as_deref().unwrap(), EXPECTED);
        assert_eq!(raw_count(&image), count);
        let meta = image.metadata.as_ref().unwrap();
        assert!(matches!(meta["orientation_applied"], DataMap::Bool(true)));
        assert!(matches!(
            meta["JPEG XL Source Orientation"],
            DataMap::UInt(1)
        ));
        let xmp: &[u8] = if make == "BigMaker" {
            include_bytes!("fixtures/review/jxl-meta-second.xmp")
        } else {
            XMP
        };
        assert!(matches!(&meta["XMP"],DataMap::Ascii(value) if value.as_bytes() == xmp));
        let original = if make == "BigMaker" { BIG } else { LITTLE };
        assert!(meta.iter().any(|(key, value)| {
            key.ends_with(" 45786966")
                && matches!(value,DataMap::Raw(raw) if raw.ends_with(original))
        }));
        #[cfg(feature = "exif")]
        common(&image, make);
        #[cfg(not(feature = "exif"))]
        {
            let _ = make;
            assert!(!meta.contains_key("EXIF"));
        }
    }
}
#[test]
fn invalid_optional_metadata_stays_raw_and_first_valid_box_wins() {
    let malformed = [
        b"".as_slice(),
        b"\0\0\0",
        b"\xff\xff\xff\xffjunk",
        b"\0\0\0\x10short",
        b"\0\0\0\0II*\0\xff\xff\xff\xff",
    ];
    for bad in malformed {
        let input = container(vec![
            box_bytes(b"Exif", bad),
            box_bytes(b"xml ", b"<not-closed"),
            box_bytes(b"Exif", &exif(0, b"", LITTLE)),
            box_bytes(b"xml ", XMP),
            box_bytes(b"Exif", &exif(0, b"", BIG)),
        ]);
        let image = image_from(&input).unwrap();
        assert_eq!(image.buffer.as_deref().unwrap(), EXPECTED);
        assert_eq!(raw_count(&image), 5);
        #[cfg(feature = "exif")]
        common(&image, "LittleMaker");
        assert!(
            matches!(&image.metadata.as_ref().unwrap()["XMP"],DataMap::Ascii(v) if v.as_bytes()==XMP)
        );
    }
    for n in 0..LITTLE.len() {
        let input = container(vec![box_bytes(b"Exif", &exif(0, b"", &LITTLE[..n]))]);
        let image = image_from(&input).unwrap();
        assert_eq!(image.buffer.as_deref().unwrap(), EXPECTED);
        assert_eq!(raw_count(&image), 1);
        assert!(!image.metadata.as_ref().unwrap().contains_key("EXIF"));
    }
}
#[test]
fn xmp_is_utf8_packet_text_and_never_resolves_dtd_entities_includes_or_stylesheets() {
    let open = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">";
    let close = "</x:xmpmeta>";
    for text in [
        format!("<!DOCTYPE x SYSTEM 'file:///not-opened'>{open}{close}"),
        format!(
            "<!DOCTYPE x [<!ENTITY e SYSTEM 'https://example.invalid/not-requested'>]>{open}&e;{close}"
        ),
        format!("<?xml-stylesheet href='https://example.invalid/not-requested'?>{open}{close}"),
        format!(
            "{open}<xi:include xmlns:xi='http://www.w3.org/2001/XInclude' href='file:///not-opened'/>{close}"
        ),
        format!("{open}&unknown;{close}"),
        format!("<?xml version='1.0' encoding='UTF-16'?>{open}{close}"),
        format!("<?xml version='1.1'?>{open}{close}"),
        "<ordinary-xml/>".to_owned(),
    ] {
        let image = image_from(&container(vec![box_bytes(b"xml ", text.as_bytes())])).unwrap();
        assert!(
            !image.metadata.as_ref().unwrap().contains_key("XMP"),
            "{text}"
        );
        assert_eq!(raw_count(&image), 1);
        assert_eq!(image.buffer.as_deref().unwrap(), EXPECTED);
    }
    for bad in [&[0xff, 0xfe, 0x00, 0x3c][..], b"<x>\0</x>".as_slice()] {
        let image = image_from(&container(vec![box_bytes(b"xml ", bad)])).unwrap();
        assert!(!image.metadata.as_ref().unwrap().contains_key("XMP"));
    }
    let valid = format!("{open}<v>日本語 &amp; &#65; <![CDATA[<!DOCTYPE harmless>]]></v>{close}");
    let image = image_from(&container(vec![box_bytes(b"xml ", valid.as_bytes())])).unwrap();
    assert!(matches!(&image.metadata.as_ref().unwrap()["XMP"],DataMap::Ascii(v) if v==&valid));
}
#[test]
fn every_coded_orientation_remains_authoritative_over_conflicting_exif() {
    for n in 1..=8 {
        let input = match n {
            1 => include_bytes!("fixtures/review/jxl-orientation1.jxl").as_slice(),
            2 => include_bytes!("fixtures/review/jxl-orientation2.jxl"),
            3 => include_bytes!("fixtures/review/jxl-orientation3.jxl"),
            4 => include_bytes!("fixtures/review/jxl-orientation4.jxl"),
            5 => include_bytes!("fixtures/review/jxl-orientation5.jxl"),
            6 => include_bytes!("fixtures/review/jxl-orientation6.jxl"),
            7 => include_bytes!("fixtures/review/jxl-orientation7.jxl"),
            _ => include_bytes!("fixtures/review/jxl-orientation8.jxl"),
        };
        let mut boxed = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
        boxed.extend(box_bytes(b"ftyp", b"jxl \0\0\0\0jxl "));
        boxed.extend(box_bytes(b"Exif", &exif(0, b"", LITTLE)));
        boxed.extend(box_bytes(b"jxlc", input));
        let reference: &[u8] = match n {
            1 => include_bytes!("fixtures/review/jxl-orientation1.rgba"),
            2 => include_bytes!("fixtures/review/jxl-orientation2.rgba"),
            3 => include_bytes!("fixtures/review/jxl-orientation3.rgba"),
            4 => include_bytes!("fixtures/review/jxl-orientation4.rgba"),
            5 => include_bytes!("fixtures/review/jxl-orientation5.rgba"),
            6 => include_bytes!("fixtures/review/jxl-orientation6.rgba"),
            7 => include_bytes!("fixtures/review/jxl-orientation7.rgba"),
            _ => include_bytes!("fixtures/review/jxl-orientation8.rgba"),
        };
        let actual = image_from(&boxed).unwrap();
        assert_eq!(
            (actual.width, actual.height),
            if n <= 4 { (2, 3) } else { (3, 2) }
        );
        assert_eq!(actual.buffer.as_deref().unwrap(), reference);
        assert!(
            matches!(actual.metadata.as_ref().unwrap()["JPEG XL Source Orientation"],DataMap::UInt(value) if value==n)
        );
        #[cfg(feature = "exif")]
        common(&actual, "LittleMaker");
    }
}
#[test]
fn xml_resource_limits_fail_before_publication_instead_of_becoming_raw_success() {
    let input = include_bytes!("fixtures/review/jxl-meta-little.jxl");
    for limits in [
        DecodeLimits {
            metadata_bytes: 40000,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1200000,
            ..Default::default()
        },
    ] {
        // Use an input whose packet cannot fit the added metadata/workspace budget.
        let text = format!(
            "<x:xmpmeta xmlns:x='adobe:ns:meta/'>{}</x:xmpmeta>",
            "a".repeat(20000)
        );
        let data = container(vec![box_bytes(b"xml ", text.as_bytes())]);
        rejected(&data, limits);
    }
    assert!(image_from(input).is_ok());
}

fn rejected(data: &[u8], limits: DecodeLimits) {
    let mut image = ImageBuffer::new();
    let result = image_decoder_with_limits(
        &mut bin_rs::reader::BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        limits,
    );
    assert!(result.is_err());
    assert_eq!((image.width, image.height), (0, 0));
    assert!(image.buffer.is_none());
    assert!(image.metadata.as_ref().is_none_or(|m| m.is_empty()));
}
#[test]
fn xml_complexity_and_fixed_cap_are_fatal_before_publication() {
    let text = format!(
        "<x:xmpmeta xmlns:x='adobe:ns:meta/'>{}</x:xmpmeta>",
        "<a/>".repeat(65536)
    );
    rejected(
        &container(vec![box_bytes(b"xml ", text.as_bytes())]),
        DecodeLimits::unlimited(),
    );
    let text = format!(
        "<x:xmpmeta xmlns:x='adobe:ns:meta/'>{}</x:xmpmeta>",
        "a".repeat(4 << 20)
    );
    rejected(
        &container(vec![box_bytes(b"xml ", text.as_bytes())]),
        DecodeLimits::unlimited(),
    );
}
#[test]
#[cfg(feature = "exif")]
fn exif_alias_amplification_is_bounded_before_any_output() {
    // Many distinct TIFF fields can reference the same source range. The
    // retained structures and parse copies must be billed, not just input.
    let n = 64u16;
    let offset = 8 + 2 + usize::from(n) * 12 + 4;
    let mut tiff = b"II*\0\x08\0\0\0".to_vec();
    tiff.extend(n.to_le_bytes());
    for tag in 0..n {
        tiff.extend((0xc000 + tag).to_le_bytes());
        tiff.extend(7u16.to_le_bytes());
        tiff.extend(8192u32.to_le_bytes());
        tiff.extend((offset as u32).to_le_bytes());
    }
    tiff.extend(0u32.to_le_bytes());
    tiff.resize(offset + 8192, 42);
    let data = container(vec![box_bytes(b"Exif", &exif(0, b"", &tiff))]);
    rejected(
        &data,
        DecodeLimits {
            metadata_bytes: 50000,
            ..Default::default()
        },
    );
    // Raising the quota actually permits the independently constructed TIFF.
    assert!(matches!(
        image_from(&data)
            .unwrap()
            .metadata
            .as_ref()
            .unwrap()
            .get("EXIF"),
        Some(DataMap::Exif(_))
    ));
}
