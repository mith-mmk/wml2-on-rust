use super::*;
use wml2::{metadata::DataMap, util::ImageFormat};
macro_rules! case {
    ($n:literal) => {
        (
            include_bytes!(concat!(
                "../fixtures/review/jxl-intrinsic-orientation",
                $n,
                ".jxl"
            ))
            .as_slice(),
            include_bytes!(concat!("../fixtures/review/jxl-orientation", $n, ".jxl")).as_slice(),
            $n,
        )
    };
}
#[test]
fn queried_dimensions_match_decoded_metadata_with_absence_and_all_orientations() {
    for (input, absent, orientation) in [
        case!(1),
        case!(2),
        case!(3),
        case!(4),
        case!(5),
        case!(6),
        case!(7),
        case!(8),
    ] {
        let info = image_info(input).unwrap();
        assert!(matches!(info.format, ImageFormat::JpegXl));
        assert_eq!(
            info.dimensions.coded,
            ImageSize {
                width: 2,
                height: 3
            }
        );
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
        assert_eq!(info.dimensions.output, Some(output));
        assert_eq!(info.dimensions.intrinsic_display, Some(display));
        assert_eq!(info.orientation, Some(orientation));
        assert_eq!(info.color, EncodedColor::Unknown);
        assert!(info.channels.is_none() && info.bits_per_sample.is_none());
        assert_eq!(info.alpha, AlphaInfo::Unknown);
        assert_eq!(info.sample_kind, SampleKind::Unknown);
        assert_eq!(info.structure, ImageStructure::Still);
        assert_eq!(info.scan, InfoScan::HeaderPrefix);
        let decoded = wml2::draw::image_from(input).unwrap();
        assert!(
            matches!(decoded.metadata.as_ref().unwrap()["intrinsic_display_width"],DataMap::UInt(v)if v==u64::from(display.width))
        );
        let missing = image_info(absent).unwrap();
        assert_eq!(missing.dimensions.intrinsic_display, None);
        assert_eq!(missing.dimensions.output, Some(output));
    }
}
#[test]
fn header_projection_has_no_pixel_icc_or_channel_description_reads_or_canvas_allocation() {
    for (input, cut) in [
        (
            include_bytes!("../fixtures/review/jxl-intrinsic-orientation6.jxl").as_slice(),
            9,
        ),
        (
            include_bytes!("../fixtures/review/jxl-intrinsic-icc16.jxl").as_slice(),
            7,
        ),
        (
            include_bytes!("../fixtures/review/jxl-intrinsic-huge-coded-header.jxl").as_slice(),
            15,
        ),
    ] {
        let data = codestream(input);
        let info = no_expansion(
            data.clone(),
            std::iter::once(cut..data.len() as u64).collect(),
        );
        assert!(info.bytes_read <= 64);
        // The unqueried sample/profile/frame bytes may be absent or malformed.
        let prefix = &data[..cut as usize];
        assert!(image_info_with_limits(prefix, tiny_limits()).is_ok());
        let mut damaged = prefix.to_vec();
        damaged.extend([0xff; 32]);
        assert!(image_info_with_limits(&damaged, tiny_limits()).is_ok());
    }
    let info = image_info_with_limits(
        include_bytes!("../fixtures/review/jxl-intrinsic-huge-coded-header.jxl"),
        tiny_limits(),
    )
    .unwrap();
    assert_eq!(
        info.dimensions.coded,
        ImageSize {
            width: 1 << 20,
            height: 1 << 20
        }
    );
    let huge = image_info_with_limits(
        include_bytes!("../fixtures/review/jxl-intrinsic-large-display.jxl"),
        tiny_limits(),
    )
    .unwrap();
    assert_eq!(
        huge.dimensions.intrinsic_display,
        Some(ImageSize {
            width: 1 << 20,
            height: 1 << 20
        })
    );
}
fn box_bytes(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = ((data.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend(kind);
    out.extend(data);
    out
}
fn container(boxes: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    out.extend(box_bytes(b"ftyp", b"jxl \0\0\0\0jxl "));
    for b in boxes {
        out.extend(b);
    }
    out
}
#[test]
fn bounded_container_prefix_skips_opaque_boxes_and_restores_position_across_partial_chunks() {
    let input = include_bytes!("../fixtures/review/jxl-intrinsic-orientation6.jxl");
    for cut in [0, 1, 2, 4, 8] {
        let mut first = 0u32.to_be_bytes().to_vec();
        first.extend(&input[..cut]);
        let mut last = 0x8000_0001u32.to_be_bytes().to_vec();
        last.extend(&input[cut..]);
        let opaque = vec![0xff; 16384];
        let data = container(vec![
            box_bytes(b"brob", &opaque),
            box_bytes(b"jxlp", &first),
            box_bytes(b"xml ", b"not parsed"),
            box_bytes(b"jxlp", &last),
        ]);
        // Source opaque brob data is skipped without decompression or validation.
        let info = no_expansion(data, std::iter::once(40..40 + 16384).collect());
        assert_eq!(
            info.dimensions.intrinsic_display,
            Some(ImageSize {
                width: 11,
                height: 19
            })
        );
    }
    let boxed = container(vec![box_bytes(b"jxlc", input)]);
    let mut reader = Cursor::new(boxed);
    reader.set_position(5);
    let info = image_info_from_reader(&mut reader).unwrap();
    assert_eq!(reader.position(), 5);
    assert!(info.bytes_read < 128);
}
#[test]
fn malformed_oversized_or_budget_exhausted_prefix_returns_no_partial_information() {
    let input = include_bytes!("../fixtures/review/jxl-intrinsic-orientation1.jxl");
    for n in 0..9 {
        assert!(image_info(&input[..n]).is_err());
    }
    for input in [
        include_bytes!("../fixtures/review/jxl-intrinsic-oversized-area.jxl").as_slice(),
        include_bytes!("../fixtures/review/jxl-intrinsic-oversized-axis.jxl"),
    ] {
        assert!(image_info_with_limits(input, DecodeLimits::unlimited()).is_err());
    }
    for limits in [
        DecodeLimits {
            metadata_bytes: 1,
            ..tiny_limits()
        },
        DecodeLimits {
            expanded_bytes: 4095,
            ..tiny_limits()
        },
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..tiny_limits()
        },
        DecodeLimits {
            frames: 0,
            ..tiny_limits()
        },
    ] {
        let mut reader = Cursor::new(input);
        reader.set_position(4);
        assert!(image_info_from_reader_with_limits(&mut reader, limits).is_err());
        assert_eq!(reader.position(), 4);
    }
    let mut bad = container(vec![box_bytes(b"jxlc", input)]);
    bad[32..36].copy_from_slice(&1u32.to_be_bytes());
    bad[40..48].copy_from_slice(&u64::MAX.to_be_bytes());
    assert!(image_info_with_limits(&bad, DecodeLimits::unlimited()).is_err());
    let empty = 0u32.to_be_bytes();
    let mut boxes = vec![box_bytes(b"jxlp", &empty)];
    for i in 1..1024u32 {
        boxes.push(box_bytes(b"jxlp", &i.to_be_bytes()));
    }
    let many = container(boxes);
    assert!(matches!(
        image_info_with_limits(&many, DecodeLimits::unlimited()),
        Err(ImageInfoError::LimitExceeded("JPEG XL prefix boxes"))
    ));
}

fn codestream(input: &[u8]) -> Vec<u8> {
    if input.starts_with(&[0xff, 0x0a]) {
        return input.to_vec();
    }
    let mut out = Vec::new();
    let mut at = 12;
    while at < input.len() {
        let n = u32::from_be_bytes(input[at..at + 4].try_into().unwrap()) as usize;
        let kind = &input[at + 4..at + 8];
        if kind == b"jxlc" {
            out.extend(&input[at + 8..at + n]);
        } else if kind == b"jxlp" {
            out.extend(&input[at + 12..at + n]);
        }
        at += n;
    }
    out
}

#[test]
fn queried_container_framing_checks_only_the_visited_prefix() {
    let raw = include_bytes!("../fixtures/review/jxl-intrinsic-orientation1.jxl");
    let mut extended = 1u32.to_be_bytes().to_vec();
    extended.extend(b"jxlc");
    extended.extend((raw.len() as u64 + 16).to_be_bytes());
    extended.extend(raw);
    let mut to_end = 0u32.to_be_bytes().to_vec();
    to_end.extend(b"jxlc");
    to_end.extend(raw);
    for b in [extended, to_end] {
        assert!(image_info(&container(vec![b])).is_ok());
    }
    for level in [5, 10] {
        assert!(
            image_info(&container(vec![
                box_bytes(b"jxll", &[level]),
                box_bytes(b"jxlc", raw)
            ]))
            .is_ok()
        );
    }
    let mut first = 0u32.to_be_bytes().to_vec();
    first.push(raw[0]);
    let mut final_one = 0x8000_0001u32.to_be_bytes().to_vec();
    final_one.extend(&raw[1..]);
    for boxes in [
        vec![box_bytes(b"jxlp", &first), box_bytes(b"jxlc", &raw[1..])],
        vec![
            box_bytes(b"jxlp", &0x8000_0001u32.to_be_bytes()),
            box_bytes(b"jxlp", &final_one),
        ],
        vec![
            box_bytes(b"jxlp", &first),
            box_bytes(b"jxll", &[5]),
            box_bytes(b"jxlp", &final_one),
        ],
        vec![
            box_bytes(b"jxll", &[5]),
            box_bytes(b"jxll", &[5]),
            box_bytes(b"jxlc", raw),
        ],
        vec![box_bytes(b"jxll", &[6]), box_bytes(b"jxlc", raw)],
    ] {
        assert!(image_info(&container(boxes)).is_err());
    }
    let mut bad_ftyp = container(vec![box_bytes(b"jxlc", raw)]);
    bad_ftyp[24..28].copy_from_slice(&1u32.to_be_bytes());
    assert!(image_info(&bad_ftyp).is_err());
    // A completed prefix deliberately does not inspect a later duplicate or
    // malformed metadata: this is information, not full container validation.
    assert!(
        image_info(&container(vec![
            box_bytes(b"jxlc", raw),
            box_bytes(b"jxlc", raw)
        ]))
        .is_ok()
    );
}
