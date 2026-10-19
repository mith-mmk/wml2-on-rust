#![cfg(feature = "bmp")]
use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;

const ICC: &[u8] = include_bytes!("fixtures/review/linear-rgb.icc");
fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// Independent BMP V5 construction: RGB24 or two-entry indexed8, two rows.
fn bmp(profile: &[u8], palette: bool, before: bool, top_down: bool) -> Vec<u8> {
    let mut dib = vec![0; 124];
    put32(&mut dib, 0, 124);
    put32(&mut dib, 4, 2);
    put32(&mut dib, 8, if top_down { (-2i32) as u32 } else { 2 });
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&(if palette { 8u16 } else { 24 }).to_le_bytes());
    let pixels: &[u8] = if palette {
        &[0, 1, 0, 0, 1, 0, 0, 0]
    } else {
        &[64, 96, 128, 16, 32, 48, 0, 0, 8, 24, 40, 80, 112, 144, 0, 0]
    };
    put32(&mut dib, 20, pixels.len() as u32);
    put32(&mut dib, 32, if palette { 2 } else { 0 });
    put32(&mut dib, 56, 0x4d424544);
    put32(&mut dib, 108, 4);
    let table: &[u8] = if palette {
        &[64, 96, 128, 0, 16, 32, 48, 0]
    } else {
        &[]
    };
    let prefix = 124 + table.len();
    put32(
        &mut dib,
        112,
        (prefix + if before { 0 } else { pixels.len() }) as u32,
    );
    put32(&mut dib, 116, profile.len() as u32);
    let mut bytes = vec![0; 14];
    bytes[..2].copy_from_slice(b"BM");
    put32(
        &mut bytes,
        2,
        (14 + prefix + pixels.len() + profile.len()) as u32,
    );
    put32(
        &mut bytes,
        10,
        (14 + prefix + if before { profile.len() } else { 0 }) as u32,
    );
    bytes.extend(dib);
    bytes.extend_from_slice(table);
    if before {
        bytes.extend_from_slice(profile);
        bytes.extend_from_slice(pixels)
    } else {
        bytes.extend_from_slice(pixels);
        bytes.extend_from_slice(profile)
    }
    bytes
}
fn profile<'a>(image: &'a ImageBuffer, key: &str) -> Option<&'a [u8]> {
    match image.metadata.as_ref().and_then(|m| m.get(key)) {
        Some(DataMap::ICCProfile(p)) => Some(p),
        _ => None,
    }
}
fn plain(bytes: &[u8]) -> Vec<u8> {
    let mut plain = bytes.to_vec();
    put32(&mut plain, 14 + 56, 0x73524742);
    plain
}

#[test]
fn v5_rgb_and_palette_profiles_are_exact_before_or_after_pixels() {
    for palette in [false, true] {
        for before in [false, true] {
            for top_down in [false, true] {
                let bytes = bmp(ICC, palette, before, top_down);
                let image = image_from(&bytes).unwrap();
                let control = image_from(&plain(&bytes)).unwrap();
                assert_eq!(image.buffer, control.buffer);
                assert_eq!((image.width, image.height), (2, 2));
                assert_eq!(profile(&image, "Source ICC Profile"), Some(ICC));
                assert_eq!(profile(&image, "ICC Profile"), Some(ICC));
            }
        }
    }
}

#[test]
fn unsupported_or_short_source_profiles_are_opaque_and_not_active() {
    let mut gray = ICC.to_vec();
    gray[16..20].copy_from_slice(b"GRAY");
    for p in [gray.as_slice(), b"opaque\0\xff", &[]] {
        let image = image_from(&bmp(p, false, false, false)).unwrap();
        assert_eq!(profile(&image, "Source ICC Profile"), Some(p));
        assert_eq!(profile(&image, "ICC Profile"), None);
    }
}

#[test]
fn linked_or_non_v5_fields_are_not_followed_or_used_as_icc() {
    for space in [0x4c494e4b, 0x73524742, 0] {
        let mut bytes = bmp(ICC, false, false, false);
        put32(&mut bytes, 14 + 56, space);
        put32(&mut bytes, 14 + 112, u32::MAX);
        let image = image_from(&bytes).unwrap();
        assert_eq!(profile(&image, "Source ICC Profile"), None);
    }
    let mut bytes = bmp(ICC, false, false, false);
    put32(&mut bytes, 14, 108);
    assert_eq!(
        profile(&image_from(&bytes).unwrap(), "Source ICC Profile"),
        None
    );
}

#[test]
fn profile_extents_reject_headers_palette_pixels_and_truncation() {
    for palette in [false, true] {
        let bytes = bmp(ICC, palette, false, false);
        for offset in [0, 1, 100, 124, u32::MAX] {
            let mut corrupt = bytes.clone();
            put32(&mut corrupt, 14 + 112, offset);
            assert!(
                image_from(&corrupt).is_err(),
                "palette={palette},offset={offset}"
            );
        }
        let mut corrupt = bytes.clone();
        put32(&mut corrupt, 14 + 116, u32::MAX);
        assert!(image_from(&corrupt).is_err());
        for cut in [1, ICC.len() / 2, ICC.len()] {
            assert!(image_from(&bytes[..bytes.len() - cut]).is_err());
        }
    }
}

#[test]
fn profile_copies_and_keys_share_an_exact_metadata_budget() {
    let bytes = bmp(ICC, false, false, false);
    let cost = 2 * ICC.len() + "Source ICC Profile".len() + "ICC Profile".len();
    assert!(
        image_from_with_limits(
            &bytes,
            DecodeLimits {
                metadata_bytes: cost,
                ..Default::default()
            }
        )
        .is_ok()
    );
    let error = image_from_with_limits(
        &bytes,
        DecodeLimits {
            metadata_bytes: cost - 1,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("metadata"));
}

#[derive(Default)]
struct Probe {
    abort: bool,
    fail: bool,
    sources: usize,
    init: usize,
    terms: usize,
}
impl DrawCallback for Probe {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        self.init += 1;
        Ok(None)
    }
    fn draw(
        &mut self,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
        _: &[u8],
        _: Option<DrawOptions>,
    ) -> Response {
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        Ok(None)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.terms += 1;
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, key: &str, _: DataMap) -> Response {
        if key == "Source ICC Profile" {
            self.sources += 1;
            if self.fail {
                return Err(std::io::Error::other("BMP source callback failed").into());
            }
            if self.abort {
                return Ok(Some(CallbackResponse::abort()));
            }
        }
        Ok(None)
    }
}
#[test]
fn new_source_callback_abort_and_errors_stop_before_pixels_in_both_apis() {
    let bytes = bmp(ICC, false, false, false);
    for direct in [false, true] {
        for fail in [false, true] {
            let mut probe = Probe {
                abort: !fail,
                fail,
                ..Default::default()
            };
            let mut options = DecodeOptions {
                debug_flag: 1,
                drawer: &mut probe,
            };
            let mut reader = BytesReader::new(&bytes);
            let result = if direct {
                wml2::bmp::decoder::decode(&mut reader, &mut options)
            } else {
                image_decoder(&mut reader, &mut options)
            };
            if fail {
                assert_eq!(
                    result.unwrap_err().to_string(),
                    "BMP source callback failed"
                )
            } else {
                assert!(result.is_ok());
                assert_eq!(probe.terms, 1)
            }
            assert_eq!((probe.sources, probe.init), (1, 0));
        }
    }
}

#[test]
fn reused_image_does_not_keep_a_previous_bmp_profile() {
    let bytes = bmp(ICC, false, false, false);
    let mut image = ImageBuffer::new();
    for input in [&bytes, &plain(&bytes)] {
        image_decoder(
            &mut BytesReader::new(input),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
        )
        .unwrap();
    }
    assert_eq!(profile(&image, "Source ICC Profile"), None);
    assert_eq!(profile(&image, "ICC Profile"), None);
}

#[cfg(feature = "bmp-png")]
#[test]
fn embedded_png_profile_is_not_replaced_by_the_outer_bmp_profile() {
    let inner = include_bytes!("fixtures/review/gamma2-rgb.icc");
    let mut image = ImageBuffer::from_buffer(1, 1, vec![128, 96, 64, 255]);
    let mut png = image_to(&mut image, wml2::util::ImageFormat::Png, None).unwrap();
    let mut payload = b"inner-profile\0\0".to_vec();
    payload.extend(miniz_oxide::deflate::compress_to_vec_zlib(inner, 6));
    let mut chunk = (payload.len() as u32).to_be_bytes().to_vec();
    chunk.extend_from_slice(b"iCCP");
    chunk.extend(payload);
    let crc = wml2::png::utils::CRC32::new().crc32(&chunk[4..]);
    chunk.extend_from_slice(&crc.to_be_bytes());
    png.splice(33..33, chunk);
    let mut bytes = bmp(ICC, false, false, false);
    bytes.truncate(14 + 124);
    put32(&mut bytes, 14 + 4, 1);
    put32(&mut bytes, 14 + 8, 1);
    put32(&mut bytes, 14 + 16, 5);
    put32(&mut bytes, 14 + 20, png.len() as u32);
    put32(&mut bytes, 14 + 112, (124 + png.len()) as u32);
    put32(&mut bytes, 2, (14 + 124 + png.len() + ICC.len()) as u32);
    bytes.extend(png);
    bytes.extend_from_slice(ICC);
    let decoded = image_from(&bytes).unwrap();
    assert_eq!(decoded.buffer.unwrap(), [128, 96, 64, 255]);
    let metadata = decoded.metadata.unwrap();
    assert_eq!(
        metadata["BMP Source ICC Profile"],
        DataMap::ICCProfile(ICC.to_vec())
    );
    assert_eq!(metadata["ICC Profile"], DataMap::ICCProfile(inner.to_vec()));
    assert!(!metadata.contains_key("Source ICC Profile"));
}
