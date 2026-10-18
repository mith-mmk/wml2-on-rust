#![cfg(all(feature = "tiff", feature = "image-buffer-ex"))]

use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;

// Independent classic-TIFF writer. SampleFormat uses the supplied tag type,
// count and bytes, so malformed cases cannot be corrected by a TIFF encoder.
fn fixture(depth: u16, channels: u16, be: bool, sample: Option<(u16, u32, &[u8])>) -> Vec<u8> {
    let u16_bytes = |v: u16| if be { v.to_be_bytes() } else { v.to_le_bytes() };
    let u32_bytes = |v: u32| if be { v.to_be_bytes() } else { v.to_le_bytes() };
    let short = |v: u16| (3, 1, u16_bytes(v).to_vec());
    let long = |v: u32| (4, 1, u32_bytes(v).to_vec());
    let width = if channels == 1 { 4 } else { 1 };
    let values = if depth == 8 {
        vec![128, 255, 0, 127]
    } else {
        vec![0x8000, 0xffff, 0, 0x7fff]
    };
    let raw: Vec<u8> = (0..width * u32::from(channels))
        .flat_map(|i| {
            let value = values[i as usize % values.len()];
            if depth == 8 {
                vec![value as u8]
            } else if depth == 16 {
                u16_bytes(value).to_vec()
            } else {
                u32_bytes(u32::from(value)).to_vec()
            }
        })
        .collect();
    let mut tags = vec![
        (256, long(width)),
        (257, long(1)),
        (
            258,
            (
                3,
                u32::from(channels),
                (0..channels).flat_map(|_| u16_bytes(depth)).collect(),
            ),
        ),
        (259, short(1)),
        (262, short(if channels == 1 { 1 } else { 2 })),
        (273, long(0)),
        (277, short(channels)),
        (278, long(1)),
        (279, long(raw.len() as u32)),
        (284, short(1)),
    ];
    if let Some((kind, count, bytes)) = sample {
        tags.push((339, (kind, count, bytes.to_vec())));
    }
    tags.sort_by_key(|entry| entry.0);
    let mut out = if be {
        b"MM\0*".to_vec()
    } else {
        b"II*\0".to_vec()
    };
    out.extend(u32_bytes(8));
    out.extend(u16_bytes(tags.len() as u16));
    let extra_start = 8 + 2 + tags.len() * 12 + 4;
    let mut extra: Vec<u8> = Vec::new();
    for (tag, (kind, count, bytes)) in &tags {
        out.extend(u16_bytes(*tag));
        out.extend(u16_bytes(*kind));
        out.extend(u32_bytes(*count));
        if bytes.len() <= 4 {
            out.extend(bytes);
            out.resize(out.len() + 4 - bytes.len(), 0);
        } else {
            out.extend(u32_bytes((extra_start + extra.len()) as u32));
            extra.extend(bytes);
        }
    }
    out.extend([0; 4]);
    out.extend(extra);
    let offset = out.len() as u32;
    let strip = tags.iter().position(|(tag, _)| *tag == 273).unwrap();
    out[10 + strip * 12 + 8..10 + strip * 12 + 12].copy_from_slice(&u32_bytes(offset));
    out.extend(raw);
    out
}

#[derive(Default)]
struct Probe {
    calls: usize,
}
impl DrawCallback for Probe {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        self.calls += 1;
        Ok(None)
    }
    fn init_ex(&mut self, _: usize, _: usize, _: Option<InitOptionsEx>) -> Response {
        self.calls += 1;
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
        self.calls += 1;
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        self.calls += 1;
        Ok(None)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.calls += 1;
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        self.calls += 1;
        Ok(None)
    }
    fn set_metadata(&mut self, _: &str, _: wml2::metadata::DataMap) -> Response {
        self.calls += 1;
        Ok(None)
    }
}

fn reject(data: &[u8], reason: &str) {
    let mut probe = Probe::default();
    let error = image_decoder_ex_with_limits(
        &mut BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut probe,
        },
        DecodeLimits {
            pixels: 4,
            expanded_bytes: 7,
            ..DecodeLimits::default()
        },
    )
    .expect_err("SampleFormat must be rejected before any EX callback");
    assert!(error.to_string().contains(reason), "{error}");
    assert_eq!(probe.calls, 0);
    // Failed decode must restore its tighter limit scope for the next image.
    let valid = fixture(16, 1, false, None);
    assert_eq!(
        image_from_ex(&valid, PixelFormatEx::Gray16, PrecisionConversion::Exact)
            .unwrap()
            .bytes()
            .unwrap(),
        [0, 128, 255, 255, 0, 0, 255, 127]
    );
    #[cfg(feature = "high-bit-depth")]
    for result in [
        wml2::highres::tiff::decode_native(data, &DecodeLimits::default()).map(|_| ()),
        wml2::highres::tiff::decode_native_pages(data, &DecodeLimits::default()).map(|_| ()),
    ] {
        let error = result.expect_err("typed native TIFF must enforce SampleFormat too");
        assert!(error.to_string().contains(reason), "{error}");
    }
}

#[test]
fn signed_and_float_samples_are_rejected_before_output() {
    for be in [false, true] {
        for depth in [8, 16, 32] {
            for format in [2u16, 3] {
                let bytes = if be {
                    format.to_be_bytes()
                } else {
                    format.to_le_bytes()
                };
                reject(
                    &fixture(depth, 1, be, Some((3, 1, &bytes))),
                    "non-unsigned SampleFormat",
                );
            }
        }
    }
}

#[test]
fn libtiff_confirmed_signed_and_float_fixtures_are_not_unsigned() {
    // Independent byte writer + LibTIFF 4.7.0 scanline oracle confirms the
    // actual signed/float samples. WML2 must reject their type, not cast bits.
    for data in [
        include_bytes!("fixtures/ex-review-sampleformat-signed8.tif").as_slice(),
        include_bytes!("fixtures/ex-review-sampleformat-signed16.tif").as_slice(),
        include_bytes!("fixtures/ex-review-sampleformat-float16.tif").as_slice(),
        include_bytes!("fixtures/ex-review-sampleformat-float32.tif").as_slice(),
    ] {
        reject(data, "non-unsigned SampleFormat");
    }
    for data in [
        include_bytes!("fixtures/ex-review-sampleformat-unsigned16le.tif").as_slice(),
        include_bytes!("fixtures/ex-review-sampleformat-unsigned16be.tif").as_slice(),
    ] {
        assert_eq!(
            image_from_ex(data, PixelFormatEx::Gray16, PrecisionConversion::Exact)
                .unwrap()
                .bytes()
                .unwrap(),
            [0, 128, 255, 255, 0, 0, 255, 127]
        );
    }
}

#[test]
fn default_explicit_and_scalar_unsigned_preserve_native_bits() {
    // TIFF6 specifies one value per sample. Like LibTIFF, the existing reader
    // also accepts a single uniform value as a vendor compatibility convention.
    for be in [false, true] {
        for depth in [8, 16] {
            for channels in [1, 3] {
                let one = if be {
                    1u16.to_be_bytes()
                } else {
                    1u16.to_le_bytes()
                };
                let all = one.repeat(usize::from(channels));
                for sample in [
                    None,
                    Some((3, 1, one.as_slice())),
                    Some((3, u32::from(channels), all.as_slice())),
                ] {
                    let format = match (depth, channels) {
                        (8, 1) => PixelFormatEx::Gray8,
                        (16, 1) => PixelFormatEx::Gray16,
                        (8, _) => PixelFormatEx::Rgba8,
                        _ => PixelFormatEx::Rgba16,
                    };
                    let image = image_from_ex(
                        &fixture(depth, channels, be, sample),
                        format,
                        PrecisionConversion::Exact,
                    )
                    .unwrap();
                    let expected = match (depth, channels) {
                        (8, 1) => vec![128, 255, 0, 127],
                        (16, 1) => vec![0, 128, 255, 255, 0, 0, 255, 127],
                        (8, _) => vec![128, 255, 0, 255],
                        _ => vec![0, 128, 255, 255, 0, 0, 255, 255],
                    };
                    assert_eq!(image.bytes().unwrap(), expected);
                    #[cfg(feature = "high-bit-depth")]
                    if depth == 16 {
                        let data = fixture(depth, channels, be, sample);
                        let frame =
                            wml2::highres::tiff::decode_native(&data, &DecodeLimits::default())
                                .unwrap();
                        assert_eq!(
                            frame.pixels().u16_planes().unwrap()[0].samples(),
                            if channels == 1 {
                                &[0x8000, 0xffff, 0, 0x7fff][..]
                            } else {
                                &[0x8000, 0xffff, 0][..]
                            }
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn sample_format_tag_type_count_and_each_channel_are_validated() {
    for be in [false, true] {
        let shorts = |values: &[u16]| {
            values
                .iter()
                .flat_map(|v| if be { v.to_be_bytes() } else { v.to_le_bytes() })
                .collect::<Vec<_>>()
        };
        reject(
            &fixture(16, 1, be, Some((3, 0, &[]))),
            "sample description cannot be empty",
        );
        for kind in [1, 4, 8] {
            let bytes = if kind == 1 {
                vec![1]
            } else if kind == 4 {
                if be {
                    1u32.to_be_bytes().to_vec()
                } else {
                    1u32.to_le_bytes().to_vec()
                }
            } else {
                shorts(&[1])
            };
            reject(&fixture(16, 1, be, Some((kind, 1, &bytes))), "SHORT type");
        }
        for count in [2, 4] {
            reject(
                &fixture(
                    16,
                    3,
                    be,
                    Some((3, count, &shorts(&vec![1; count as usize]))),
                ),
                "SampleFormat count differs",
            );
        }
        for bad in [0, 2, 3, 4, 5, 6, u16::MAX] {
            for channel in 0..3 {
                let mut values = [1; 3];
                values[channel] = bad;
                reject(
                    &fixture(16, 3, be, Some((3, 3, &shorts(&values)))),
                    "non-unsigned SampleFormat",
                );
            }
        }
    }
}
