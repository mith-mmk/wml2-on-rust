#![cfg(feature = "jpeg-xl")]
use bin_rs::reader::BytesReader;
use wml2::draw::{
    DecodeOptions, ImageBuffer, image_decoder_with_limits, image_from, image_from_with_limits,
};
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;
const RAW: &[u8] = include_bytes!("fixtures/review/jxl-rgba8.jxl");
const EXPECTED: &[u8] = include_bytes!("fixtures/review/jxl-rgba8-djxl.bin");
const XML: &[u8] = b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><metadata>JPEG XL bounded metadata</metadata></x:xmpmeta>";
const XML_BR: &[u8] = include_bytes!("fixtures/review/jxl-brob-xml.br");
const EMPTY_BR: &[u8] = include_bytes!("fixtures/review/jxl-brob-empty.br");
const REPEAT_BR: &[u8] = include_bytes!("fixtures/review/jxl-brob-repeat.br");
fn box_bytes(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut result = ((data.len() + 8) as u32).to_be_bytes().to_vec();
    result.extend(kind);
    result.extend(data);
    result
}
fn brob(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut payload = kind.to_vec();
    payload.extend(data);
    box_bytes(b"brob", &payload)
}
fn container(boxes: Vec<Vec<u8>>) -> Vec<u8> {
    let mut result = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    result.extend(box_bytes(b"ftyp", b"jxl \0\0\0\0jxl "));
    for b in boxes {
        result.extend(b);
    }
    result
}
fn rejected(data: &[u8], limits: DecodeLimits) -> String {
    let mut image = ImageBuffer::new();
    let result = image_decoder_with_limits(
        &mut BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        limits,
    );
    let error = result.expect_err("input must be rejected").to_string();
    assert_eq!((image.width, image.height), (0, 0), "{error}");
    assert!(image.buffer.is_none(), "{error}");
    assert!(
        image
            .metadata
            .as_ref()
            .is_none_or(|metadata| metadata.is_empty()),
        "{error}"
    );
    error
}
fn values<'a>(image: &'a ImageBuffer, stage: &str) -> Vec<(&'a str, &'a [u8])> {
    image
        .metadata
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|(key, value)| {
            if key.starts_with(&format!("JPEG XL {stage} Box "))
                && let DataMap::Raw(data) = value
            {
                Some((key.as_str(), data.as_slice()))
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn independent_compressed_and_plain_containers_preserve_pixels_and_indexed_metadata() {
    let compressed = include_bytes!("fixtures/review/jxl-brob-metadata.jxl");
    let plain = include_bytes!("fixtures/review/jxl-brob-metadata-plain.jxl");
    // C libbrotli generated these streams; C libjxl independently verified
    // decompressed boxes AND pixels. See jxl-brob-fixtures.json and generator.
    let image = image_from(compressed).unwrap();
    let reference = image_from(plain).unwrap();
    assert_eq!((image.width, image.height), (7, 5));
    assert_eq!(image.buffer.as_deref().unwrap(), EXPECTED);
    assert_eq!(reference.buffer.as_deref().unwrap(), EXPECTED);
    let decoded = values(&image, "Decoded");
    assert_eq!(decoded.len(), 4);
    for (key, data) in decoded {
        let source = key.replacen("Decoded", "Source", 1);
        assert!(
            matches!(&reference.metadata.as_ref().unwrap()[&source],DataMap::Raw(expected) if expected==data)
        );
    }
    let raw = values(&image, "Source");
    assert_eq!(raw.len(), 4);
    let mut at = 12;
    let mut index = 0;
    while at < compressed.len() {
        let size = u32::from_be_bytes(compressed[at..at + 4].try_into().unwrap()) as usize;
        if &compressed[at + 4..at + 8] == b"brob" {
            let key = format!("JPEG XL Source Box {index} 62726f62");
            assert!(
                matches!(&image.metadata.as_ref().unwrap()[&key],DataMap::Raw(v) if v==&compressed[at+8..at+size])
            );
        }
        at += size;
        index += 1;
    }
    let metadata = image.metadata.as_ref().unwrap();
    assert!(matches!(
        metadata["orientation_applied"],
        DataMap::Bool(true)
    ));
    assert!(matches!(
        metadata["JPEG XL Source Orientation"],
        DataMap::UInt(1)
    ));
    #[cfg(not(feature = "exif"))]
    assert!(!metadata.contains_key("EXIF")); // raw Exif says 6, codestream remains authoritative
    #[cfg(feature = "image-buffer-ex")]
    {
        use wml2::draw::{PixelFormatEx, PrecisionConversion, image_from_ex};
        let ex =
            image_from_ex(compressed, PixelFormatEx::Rgba8, PrecisionConversion::Exact).unwrap();
        assert_eq!(ex.bytes().unwrap(), EXPECTED);
    }
    #[cfg(feature = "color-management")]
    {
        let display = wml2::jpegxl::image_from_srgb(compressed).unwrap();
        for (actual, expected) in display
            .buffer
            .unwrap()
            .chunks_exact(4)
            .zip(EXPECTED.chunks_exact(4))
        {
            for c in 0..3 {
                assert!(actual[c].abs_diff(expected[c]) <= 2);
            }
            assert_eq!(actual[3], expected[3]);
        }
    }
}
#[test]
fn placement_chunking_duplicates_extended_and_to_eof_boxes_keep_effective_contents() {
    let chunk = |index: u32, data: &[u8]| {
        let mut payload = index.to_be_bytes().to_vec();
        payload.extend(data);
        box_bytes(b"jxlp", &payload)
    };
    let mut extended = 1u32.to_be_bytes().to_vec();
    extended.extend(b"brob");
    extended.extend(((16 + 4 + XML_BR.len()) as u64).to_be_bytes());
    extended.extend(b"xml ");
    extended.extend(XML_BR);
    let mut to_eof = vec![0; 4];
    to_eof.extend(b"brobxml ");
    to_eof.extend(XML_BR);
    for boxes in [
        vec![
            brob(b"xml ", XML_BR),
            box_bytes(b"jxlc", RAW),
            brob(b"xml ", XML_BR),
        ],
        vec![
            chunk(0, &RAW[..RAW.len() / 2]),
            extended,
            chunk(0x80000001, &RAW[RAW.len() / 2..]),
            to_eof,
        ],
    ] {
        let image = image_from(&container(boxes)).unwrap();
        assert_eq!(image.buffer.as_deref().unwrap(), EXPECTED);
        let decoded = values(&image, "Decoded");
        assert_eq!(decoded.len(), 2);
        assert_ne!(decoded[0].0, decoded[1].0);
        assert!(decoded.iter().all(|(_, value)| *value == XML));
    }
    // Opaque uncompressed jbrd retention stays independent of brob support.
    let image = image_from(&container(vec![
        box_bytes(b"jbrd", b"opaque, not a JPEG reconstruction claim"),
        brob(b"test", EMPTY_BR),
        box_bytes(b"jxlc", RAW),
    ]))
    .unwrap();
    assert_eq!(values(&image, "Decoded")[0].1, b"");
}
#[test]
fn malformed_truncated_trailing_and_concatenated_streams_fail_before_publication() {
    let mut payloads: Vec<Vec<u8>> = (0..4).map(|n| b"xml "[..n].to_vec()).collect();
    for n in 0..XML_BR.len() {
        let mut data = b"xml ".to_vec();
        data.extend(&XML_BR[..n]);
        payloads.push(data);
    }
    for stream in [
        b"invalid brotli".as_slice(),
        include_bytes!("fixtures/review/jxl-brob-large-window-invalid.br").as_slice(),
        &[0xff; 16],
        &[0x11, 0x00],
    ] {
        let mut data = b"xml ".to_vec();
        data.extend(stream);
        payloads.push(data);
    }
    for (first, second) in [
        (XML_BR, b"\0".as_slice()),
        (XML_BR, EMPTY_BR),
        (EMPTY_BR, XML_BR),
    ] {
        let mut data = b"xml ".to_vec();
        data.extend(first);
        data.extend(second);
        payloads.push(data);
    }
    for payload in payloads {
        for after in [false, true] {
            let mut boxes = vec![box_bytes(b"brob", &payload), box_bytes(b"jxlc", RAW)];
            if after {
                boxes.reverse();
            }
            rejected(&container(boxes), DecodeLimits::default());
        }
    }
    assert!(image_from(RAW).is_ok());
}
#[test]
fn forbidden_and_control_types_never_enter_the_metadata_decoder() {
    for kind in [
        b"jxlc", b"jxlp", b"jxll", b"jxli", b"jxl?", b"jbrd", b"brob", b"JXL ", b"JXL?", b"ftyp",
    ] {
        let input = container(vec![box_bytes(b"jxlc", RAW), brob(kind, EMPTY_BR)]);
        let message = rejected(
            &input,
            DecodeLimits {
                metadata_bytes: 0,
                ..Default::default()
            },
        );
        assert!(!message.contains("decode limit"), "{message}");
    }
    // Wrapped control data cannot replace the required initial ftyp.
    let mut input = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    input.extend(brob(b"ftyp", EMPTY_BR));
    input.extend(box_bytes(b"jxlc", RAW));
    rejected(&input, DecodeLimits::default());
}
#[test]
fn compressed_input_expanded_metadata_aggregate_workspace_and_existing_limits_are_independent() {
    let input = container(vec![brob(b"xml ", REPEAT_BR), box_bytes(b"jxlc", RAW)]);
    assert_eq!(
        values(&image_from(&input).unwrap(), "Decoded")[0].1,
        vec![b'A'; 65536 + 123]
    );
    for limits in [
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 65536,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1024,
            ..Default::default()
        },
        DecodeLimits {
            pixels: 34,
            ..Default::default()
        },
        DecodeLimits {
            frames: 0,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(rejected(&input, limits).contains("decode limit"));
    }
    let single = container(vec![brob(b"xml ", REPEAT_BR), box_bytes(b"jxlc", RAW)]);
    let multiple = container(vec![
        brob(b"xml ", REPEAT_BR),
        brob(b"xml ", REPEAT_BR),
        box_bytes(b"jxlc", RAW),
    ]);
    let limits = DecodeLimits {
        metadata_bytes: 100000,
        ..Default::default()
    };
    assert!(image_from_with_limits(&single, limits).is_ok());
    assert!(rejected(&multiple, limits).contains("decode limit"));
    let mut boxes = vec![brob(b"test", EMPTY_BR); 1025];
    boxes.push(box_bytes(b"jxlc", RAW));
    assert!(rejected(&container(boxes), DecodeLimits::unlimited()).contains("box count"));
}
#[test]
fn caller_unlimited_does_not_disable_the_cumulative_expanded_brob_cap() {
    // 1023 copies expand beyond 64 MiB, with a tiny valid compressed input.
    // Each insertion still has an admitted box index (1..=1023).
    let mut boxes = vec![brob(b"test", REPEAT_BR); 1023];
    boxes.push(box_bytes(b"jxlc", RAW));
    assert!(
        rejected(&container(boxes), DecodeLimits::unlimited()).contains("brob expanded metadata")
    );
}
