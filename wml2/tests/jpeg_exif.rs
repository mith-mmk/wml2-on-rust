#![cfg(feature = "jpeg")]

use bin_rs::reader::BytesReader;
use wml2::draw::{
    DecodeOptions, EncodeOptions, ImageBuffer, image_decoder_with_limits, image_encoder,
    image_loader,
};
use wml2::limits::DecodeLimits;
use wml2::metadata::{
    DataMap,
    exif::{ExifIfd, get_tag, parse_exif},
};
use wml2::tiff::header::DataPack;
use wml2::util::ImageFormat;

fn put16(b: &mut [u8], o: usize, n: u16, le: bool) {
    b[o..o + 2].copy_from_slice(&if le { n.to_le_bytes() } else { n.to_be_bytes() });
}
fn put32(b: &mut [u8], o: usize, n: u32, le: bool) {
    b[o..o + 4].copy_from_slice(&if le { n.to_le_bytes() } else { n.to_be_bytes() });
}
fn entry(b: &mut [u8], o: usize, tag: u16, kind: u16, count: u32, value: u32, le: bool) {
    put16(b, o, tag, le);
    put16(b, o + 2, kind, le);
    put32(b, o + 4, count, le);
    put32(b, o + 8, value, le);
}
// Synthetic EXIF only. The short metadata directory ends where IFD0's
// XResolution value begins; its bytes must not become a next-IFD pointer.
fn exif(le: bool, overlap: bool, orientation: Option<u16>) -> Vec<u8> {
    let count = 4 + usize::from(orientation.is_some());
    let sub = 8 + 2 + count * 12 + 4;
    let values = sub + 2 + 12 + if overlap { 0 } else { 4 };
    let mut b = vec![0; values + 16 + 17];
    b[..2].copy_from_slice(if le { b"II" } else { b"MM" });
    put16(&mut b, 2, 42, le);
    put32(&mut b, 4, 8, le);
    put16(&mut b, 8, count as u16, le);
    entry(&mut b, 10, 0x11a, 5, 1, values as u32, le);
    entry(&mut b, 22, 0x11b, 5, 1, (values + 8) as u32, le);
    entry(&mut b, 34, 0x128, 3, 1, 0, le);
    put16(&mut b, 42, 2, le);
    entry(&mut b, 46, 0x8769, 4, 1, sub as u32, le);
    if let Some(o) = orientation {
        entry(&mut b, 58, 0x112, 3, 1, 0, le);
        put16(&mut b, 66, o, le);
    }
    put16(&mut b, sub, 1, le);
    entry(&mut b, sub + 2, 0x927c, 7, 17, (values + 16) as u32, le);
    put32(&mut b, values, 300, le);
    put32(&mut b, values + 4, 1, le);
    put32(&mut b, values + 8, 300, le);
    put32(&mut b, values + 12, 1, le);
    b
}
fn jpeg() -> Vec<u8> {
    let pixels = (0..128)
        .flat_map(|i| [i as u8, (i * 2) as u8, 17, 255])
        .collect();
    let mut image = ImageBuffer::from_buffer(16, 8, pixels);
    image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut image,
            options: None,
        },
        ImageFormat::Jpeg,
    )
    .unwrap()
}
fn app1(jpeg: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut b = jpeg[..2].to_vec();
    b.extend([0xff, 0xe1]);
    b.extend(((payload.len() + 2) as u16).to_be_bytes());
    b.extend(payload);
    b.extend(&jpeg[2..]);
    b
}
fn with_exif(jpeg: &[u8], tiff: &[u8]) -> Vec<u8> {
    let mut payload = b"Exif\0\0".to_vec();
    payload.extend(tiff);
    app1(jpeg, &payload)
}
fn decode(b: &[u8]) -> (ImageBuffer, Option<String>) {
    let mut image = ImageBuffer::new();
    let warnings = image_loader(
        b,
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
    )
    .unwrap()
    .map(|w| w.to_string());
    (image, warnings)
}
#[test]
fn exif_value_overlapping_subifd_footer_preserves_metadata_and_pixels() {
    let plain = jpeg();
    let (reference, _) = decode(&plain);
    for le in [false, true] {
        let tiff = exif(le, true, None);
        assert_eq!(tiff.len(), 109);
        let parsed = parse_exif(&tiff).unwrap();
        assert_eq!(parsed.exif.as_ref().unwrap().len(), 1);
        // The general TIFF image parser remains strict.
        assert!(wml2::tiff::header::read_tags(&mut BytesReader::new(&tiff)).is_err());
        let (image, warnings) = decode(&with_exif(&plain, &tiff));
        assert_eq!(image.buffer, reference.buffer);
        assert!(warnings.is_none());
        assert!(matches!(
            image.metadata.unwrap().get("EXIF"),
            Some(DataMap::Exif(_))
        ));
    }
}
#[test]
fn orientation_one_through_eight_survives_normal_and_short_metadata_ifds() {
    let plain = jpeg();
    let (reference, _) = decode(&plain);
    for le in [false, true] {
        for overlap in [false, true] {
            for orientation in 1..=8 {
                let (image, warnings) =
                    decode(&with_exif(&plain, &exif(le, overlap, Some(orientation))));
                assert!(warnings.is_none());
                assert_eq!(image.buffer, reference.buffer);
                assert_eq!((image.width, image.height), (16, 8));
                let metadata = image.metadata.unwrap();
                let Some(DataMap::Exif(headers)) = metadata.get("EXIF") else {
                    panic!("missing EXIF")
                };
                assert!(
                    matches!(&get_tag(headers,ExifIfd::Primary,0x112).unwrap().data,DataPack::Short(v) if v==&[orientation])
                );
            }
        }
    }
}
#[test]
fn invalid_exif_is_warned_and_retained_without_losing_pixels() {
    let plain = jpeg();
    let (reference, _) = decode(&plain);
    let mut cycle = exif(false, false, None);
    put32(&mut cycle, 54, 8, false);
    let mut outside = exif(false, false, None);
    put32(&mut outside, 54, u32::MAX, false);
    let mut nested = exif(false, false, None);
    put16(&mut nested, 64, 0x8769, false);
    let mut image_cycle = exif(false, false, None);
    put32(&mut image_cycle, 58, 8, false);
    for tiff in [
        vec![],
        b"MM\0*\0\0\0\x08".to_vec(),
        b"MM\0+\0\x08\0\0".to_vec(),
        cycle,
        outside,
        nested,
        image_cycle,
    ] {
        assert!(parse_exif(&tiff).is_err());
        let (image, warnings) = decode(&with_exif(&plain, &tiff));
        assert_eq!(image.buffer, reference.buffer);
        assert!(warnings.unwrap().contains("EXIF"));
        assert!(matches!(
            image.metadata.unwrap().get("EXIF Raw"),
            Some(DataMap::Raw(_))
        ));
    }
    for payload in [b"Exif\0".as_slice(), b"Exif\0X"] {
        assert!(decode(&app1(&plain, payload)).1.unwrap().contains("EXIF"));
    }
}
#[test]
fn exif_fallback_does_not_swallow_limits_or_jpeg_segment_errors() {
    let data = with_exif(&jpeg(), b"bad");
    let mut image = ImageBuffer::new();
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: &mut image,
    };
    let limits = DecodeLimits {
        metadata_bytes: 1,
        ..DecodeLimits::unlimited()
    };
    let error = image_decoder_with_limits(&mut BytesReader::new(&data), &mut options, limits)
        .err()
        .unwrap();
    assert!(error.to_string().contains("limit"));
    // The APP1 bytes fit, but the parsed directory allocations do not. That
    // resource failure must not be downgraded to an optional-metadata warning.
    let data = with_exif(&jpeg(), &exif(false, true, None));
    let limits = DecodeLimits {
        metadata_bytes: 200,
        ..DecodeLimits::unlimited()
    };
    let error = image_decoder_with_limits(&mut BytesReader::new(&data), &mut options, limits)
        .err()
        .unwrap();
    assert!(error.to_string().contains("limit"));
    for b in [
        vec![0xff, 0xd8, 0xff, 0xe1, 0, 1],
        vec![0xff, 0xd8, 0xff, 0xe1, 0, 20, b'E'],
    ] {
        assert!(image_loader(&b, &mut options).is_err());
    }
    let data = with_exif(&[0xff, 0xd8, 0xff, 0xd9], b"bad");
    assert!(image_loader(&data, &mut options).is_err());
}
#[test]
fn valid_nonzero_metadata_chain_is_not_silently_followed() {
    let mut b = exif(false, false, None);
    put32(&mut b, 76, 8, false);
    assert!(parse_exif(&b).err().unwrap().to_string().contains("Nested"));
}
