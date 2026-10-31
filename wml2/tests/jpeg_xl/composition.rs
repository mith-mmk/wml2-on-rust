use super::*;
const COMBINED: &[u8] = include_bytes!("../fixtures/review/jxl-composition-preview-intrinsic.jxl");
fn boxed_metadata(raw: &[u8]) -> Vec<u8> {
    let frozen = include_bytes!("../fixtures/review/jxl-meta-brob.jxl");
    let mut out = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    out.extend(box_bytes(b"ftyp", b"jxl \0\0\0\0jxl "));
    let mut codestream = Vec::new();
    if raw.starts_with(&[255, 10]) {
        codestream.extend_from_slice(raw);
    } else {
        let mut at = 12;
        while at < raw.len() {
            let n = u32::from_be_bytes(raw[at..at + 4].try_into().unwrap()) as usize;
            let kind = &raw[at + 4..at + 8];
            if kind == b"jxlc" {
                codestream.extend_from_slice(&raw[at + 8..at + n]);
            } else if kind == b"jxlp" {
                codestream.extend_from_slice(&raw[at + 12..at + n]);
            }
            at += n;
        }
    }
    out.extend(box_bytes(b"jxlc", &codestream));
    let mut at = 12;
    while at < frozen.len() {
        let n = u32::from_be_bytes(frozen[at..at + 4].try_into().unwrap()) as usize;
        if &frozen[at + 4..at + 8] == b"brob" {
            out.extend_from_slice(&frozen[at..at + n]);
        }
        at += n;
    }
    out
}
#[test]
fn preview_intrinsic_orientation_and_compressed_structured_metadata_compose() {
    let boxed = boxed_metadata(COMBINED);
    let expected = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "../fixtures/review/jxl-preview-oriented-icc16.rgba.zlib"
    ))
    .unwrap();
    let native = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "../fixtures/review/jxl-preview-oriented-icc16.rgba16le.zlib"
    ))
    .unwrap();
    for data in [COMBINED, boxed.as_slice()] {
        let info = wml2::image_info::image_info(data).unwrap();
        assert_eq!(
            (info.dimensions.coded.width, info.dimensions.coded.height),
            (7, 5)
        );
        let output = info.dimensions.output.unwrap();
        assert_eq!((output.width, output.height), (5, 7));
        let display = info.dimensions.intrinsic_display.unwrap();
        assert_eq!((display.width, display.height), (11, 19));
        let image = image_from(data).unwrap();
        assert_eq!((image.width, image.height), (5, 7));
        assert_eq!(image.buffer.unwrap(), expected);
        let meta = image.metadata.unwrap();
        assert!(matches!(meta["orientation_applied"], DataMap::Bool(true)));
        assert!(matches!(
            meta["JPEG XL Source Orientation"],
            DataMap::UInt(6)
        ));
        assert!(matches!(meta["intrinsic_display_width"], DataMap::UInt(11)));
        #[cfg(feature = "image-buffer-ex")]
        assert_eq!(
            image_from_ex(data, PixelFormatEx::Rgba16, PrecisionConversion::Exact)
                .unwrap()
                .bytes()
                .unwrap(),
            native
        );
        if data.len() == boxed.len() {
            assert!(matches!(meta["XMP"], DataMap::Ascii(_)));
            #[cfg(feature = "exif")]
            {
                use wml2::{
                    metadata::exif::{ExifIfd, get_tag},
                    tiff::header::DataPack,
                };
                let DataMap::Exif(h) = &meta["EXIF"] else {
                    panic!("missing typed EXIF")
                };
                assert!(
                    matches!(&get_tag(h,ExifIfd::Primary,0x112).unwrap().data,DataPack::Short(v)if v==&[1])
                );
            }
        }
    }
    // Dimension query stops before frame payload. It does not certify decodability.
    assert!(wml2::image_info::image_info(&COMBINED[..32]).is_ok());
    assert!(image_from(&COMBINED[..32]).is_err());
}
fn threshold(data: &[u8], workspace: bool) -> usize {
    let (mut lo, mut hi) = (0, 128 << 20);
    image_from_with_limits(
        data,
        DecodeLimits {
            metadata_bytes: hi,
            expanded_bytes: hi,
            ..Default::default()
        },
    )
    .unwrap();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let limits = if workspace {
            DecodeLimits {
                expanded_bytes: mid,
                ..Default::default()
            }
        } else {
            DecodeLimits {
                metadata_bytes: mid,
                ..Default::default()
            }
        };
        if image_from_with_limits(data, limits).is_ok() {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    lo
}
#[test]
fn dimension_alpha_and_brob_structured_metadata_budgets_reject_before_publication() {
    let alpha = include_bytes!("../fixtures/review/jxl-alpha-3c8a16o5.jxl").as_slice();
    for raw in [COMBINED, alpha] {
        let boxed = boxed_metadata(raw);
        let meta = threshold(&boxed, false);
        let work = threshold(&boxed, true);
        assert!(meta > threshold(raw, false));
        assert!(work > threshold(raw, true));
        eprintln!(
            "composition {}: metadata={meta} workspace={work}",
            if raw == COMBINED {
                "preview/intrinsic"
            } else {
                "mixed alpha"
            }
        );
        for limits in [
            DecodeLimits {
                metadata_bytes: meta - 1,
                ..Default::default()
            },
            DecodeLimits {
                expanded_bytes: work - 1,
                ..Default::default()
            },
            DecodeLimits {
                animation_bytes: 279,
                ..Default::default()
            },
        ] {
            for native in [false, true] {
                let mut p = Probe::default();
                assert!(probe(&boxed, &mut p, limits, native).is_err());
                assert!(p.events.is_empty(), "{:?}", p.events);
            }
        }
        let image = image_from(&boxed).unwrap();
        let m = image.metadata.unwrap();
        assert!(matches!(m["XMP"], DataMap::Ascii(_)));
        assert!(m.keys().any(|k| k.starts_with("JPEG XL Decoded Box ")));
        #[cfg(feature = "image-buffer-ex")]
        if raw == alpha {
            let ex =
                image_from_ex(&boxed, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert_eq!(
                ex.bytes().unwrap(),
                image_from_ex(raw, PixelFormatEx::Rgba16, PrecisionConversion::Exact)
                    .unwrap()
                    .bytes()
                    .unwrap()
            );
            assert!(matches!(
                ex.metadata.as_ref().unwrap()["JPEG XL Source Alpha Bit Depth"],
                DataMap::UInt(16)
            ));
            assert!(
                image_from_ex(&boxed, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_err()
            );
        }
    }
}
#[cfg(feature = "color-management")]
#[test]
fn mixed_alpha_and_preview_intrinsic_cancel_at_every_managed_checkpoint() {
    use std::cell::Cell;
    for data in [
        boxed_metadata(COMBINED),
        boxed_metadata(include_bytes!(
            "../fixtures/review/jxl-alpha-icc-c8a16o6.jxl"
        )),
    ] {
        let count = Cell::new(0);
        let probe = || {
            count.set(count.get() + 1);
            false
        };
        let baseline = wml2::jpegxl::image_from_srgb_with_limits_and_cancel(
            &data,
            DecodeLimits::default(),
            &probe,
        )
        .unwrap();
        let expected = wml2::jpegxl::image_from_srgb(&data)
            .unwrap()
            .buffer
            .unwrap();
        assert_eq!(baseline.buffer.unwrap(), expected);
        let checkpoints = count.get();
        assert!(checkpoints > 3);
        for stop in 1..=checkpoints {
            let n = Cell::new(0);
            let cancel = || {
                n.set(n.get() + 1);
                n.get() >= stop
            };
            let err = wml2::jpegxl::image_from_srgb_with_limits_and_cancel(
                &data,
                DecodeLimits::default(),
                &cancel,
            )
            .err()
            .expect("cancelled image must stay private");
            assert!(err.is::<wml2::jpegxl::DecodeCancelled>());
        }
        eprintln!("composition managed: {checkpoints} cancellation points");
    }
}
