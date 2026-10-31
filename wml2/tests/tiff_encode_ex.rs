#![cfg(all(feature = "tiff", feature = "image-buffer-ex"))]
use std::collections::HashMap;
use std::io::Cursor;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;
type Error = Box<dyn std::error::Error>;
const RGB: &[u8] = include_bytes!("fixtures/review/linear-rgb.icc");
const GRAY: &[u8] = include_bytes!("fixtures/review/avif-icc-gray-linear.icc");
fn words(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|s| s.to_le_bytes()).collect()
}
fn options(
    compression: &str,
    predictor: bool,
    big: bool,
    endian: &str,
) -> HashMap<String, DataMap> {
    HashMap::from([
        ("compression".into(), DataMap::Ascii(compression.into())),
        (
            "predictor".into(),
            DataMap::UInt(if predictor { 2 } else { 1 }),
        ),
        ("bigtiff".into(), DataMap::UInt(u64::from(big))),
        ("endian".into(), DataMap::Ascii(endian.into())),
    ])
}
fn encode(
    image: &mut dyn PickCallbackEx,
    output: PixelOutputEx,
    options: Option<HashMap<String, DataMap>>,
) -> Result<Vec<u8>, Error> {
    image_encoder_ex(
        &mut EncodeOptionsEx {
            debug_flag: 0,
            drawer: image,
            output,
            options,
        },
        ImageFormat::Tiff,
    )
}
fn samples(bytes: &[u8]) -> Vec<u16> {
    let mut d = Decoder::new(Cursor::new(bytes)).unwrap();
    assert!(!d.more_images());
    match d.read_image().unwrap() {
        DecodingResult::U16(v) => v,
        _ => panic!("expected unsigned16-bit samples"),
    }
}
#[test]
fn independent_tiff_decoder_checks_compression_endian_predictor_strips_icc_and_alpha() {
    for format in [PixelFormatEx::Gray16, PixelFormatEx::Rgba16] {
        let (w, h) = (3, 3);
        let count = w * h * format.channels();
        let values: Vec<u16> = (0..count)
            .map(|i| {
                if format.channels() == 4 && i % 4 == 3 {
                    [0, 32768, 65535][(i / 4) % 3]
                } else {
                    [0x1234, 0x1201, 0xffff, 1, 0x100, 0x8000, 0xff01][i % 7]
                }
            })
            .collect();
        let row = w * format.bytes_per_pixel();
        let stride = row + 1;
        let mut padded = vec![0xa5; stride * h];
        for (y, source) in words(&values).chunks_exact(row).enumerate() {
            padded[y * stride..y * stride + row].copy_from_slice(source);
        }
        let mut image =
            ImageBufferEx::from_bytes(w, h, format, Some(stride), padded.clone()).unwrap();
        let icc = if format.channels() == 1 { GRAY } else { RGB };
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(icc.to_vec()))
            .unwrap();
        for compression in ["none", "lzw", "deflate", "deflate_legacy"] {
            for predictor in [false, true] {
                if compression == "none" && predictor {
                    continue;
                }
                for big in [false, true] {
                    for endian in ["little", "big"] {
                        let out = PixelOutputEx {
                            format,
                            stride: Some(row + 3),
                            conversion: PrecisionConversion::Exact,
                        };
                        let bytes = encode(
                            &mut image,
                            out,
                            Some(options(compression, predictor, big, endian)),
                        )
                        .unwrap();
                        assert_eq!(&bytes[..2], if endian == "big" { b"MM" } else { b"II" });
                        assert_eq!(
                            samples(&bytes),
                            values,
                            "{format:?}/{compression}/{predictor}/{big}/{endian}"
                        );
                        let mut d = Decoder::new(Cursor::new(&bytes)).unwrap();
                        assert_eq!(d.dimensions().unwrap(), (w as u32, h as u32));
                        assert_eq!(
                            d.get_tag_u16_vec(Tag::BitsPerSample).unwrap(),
                            vec![16; format.channels()]
                        );
                        assert_eq!(
                            d.get_tag_u16_vec(Tag::SampleFormat).unwrap(),
                            vec![1; format.channels()]
                        );
                        assert_eq!(
                            d.get_tag_u16_vec(Tag::PhotometricInterpretation).unwrap(),
                            vec![if format.channels() == 1 { 1 } else { 2 }]
                        );
                        assert_eq!(d.get_tag_u8_vec(Tag::IccProfile).unwrap(), icc);
                        if format.channels() == 4 {
                            assert_eq!(d.get_tag_u16_vec(Tag::ExtraSamples).unwrap(), vec![2]);
                        }
                        assert_eq!(
                            d.get_tag_u32_vec(Tag::RowsPerStrip).unwrap(),
                            vec![h as u32]
                        );
                        let offsets = d.get_tag_u64_vec(Tag::StripOffsets).unwrap();
                        let counts = d.get_tag_u64_vec(Tag::StripByteCounts).unwrap();
                        assert_eq!(offsets.len(), 1);
                        assert_eq!(counts.len(), 1);
                        assert_eq!(offsets[0] + counts[0], bytes.len() as u64);
                        if compression == "none" {
                            let expected: Vec<u8> = values
                                .iter()
                                .flat_map(|s| {
                                    if endian == "big" {
                                        s.to_be_bytes()
                                    } else {
                                        s.to_le_bytes()
                                    }
                                })
                                .collect();
                            assert_eq!(&bytes[offsets[0] as usize..], expected);
                        }
                    }
                }
            }
        }
        assert_eq!(image.bytes().unwrap(), padded);
    }
}
#[test]
fn twelve_to_sixteen_normalizes_exact_and_round_including_each_alpha_sample() {
    for format in [PixelFormatEx::Gray12, PixelFormatEx::Rgba12] {
        let dest = if format.channels() == 1 {
            PixelFormatEx::Gray16
        } else {
            PixelFormatEx::Rgba16
        };
        let values: Vec<u16> = (0..format.channels() * 3)
            .map(|i| [0, 273, 4095][i % 3])
            .collect();
        let mut image = ImageBufferEx::from_bytes(3, 1, format, None, words(&values)).unwrap();
        let bytes = encode(
            &mut image,
            PixelOutputEx::new(dest, PrecisionConversion::Exact),
            None,
        )
        .unwrap();
        assert_eq!(
            samples(&bytes),
            values
                .iter()
                .map(|&s| (u32::from(s) * 65535 / 4095) as u16)
                .collect::<Vec<_>>()
        );
        for channel in 0..format.channels() {
            let mut values = vec![4095; format.channels()];
            values[channel] = 1;
            let original = words(&values);
            let mut image =
                ImageBufferEx::from_bytes(1, 1, format, None, original.clone()).unwrap();
            assert!(
                encode(
                    &mut image,
                    PixelOutputEx::new(dest, PrecisionConversion::Exact),
                    None
                )
                .is_err()
            );
            let bytes = encode(
                &mut image,
                PixelOutputEx::new(dest, PrecisionConversion::Round),
                Some(options("lzw", true, false, "big")),
            )
            .unwrap();
            assert_eq!(
                samples(&bytes),
                values
                    .iter()
                    .map(|&s| ((u32::from(s) * 65535 + 2047) / 4095) as u16)
                    .collect::<Vec<_>>()
            );
            assert_eq!(image.bytes().unwrap(), original);
        }
    }
}
#[test]
fn icc_models_and_source_provenance_follow_png_ex_contract() {
    let mut image =
        ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Gray16, None, words(&[0x1234])).unwrap();
    let gray = PixelOutputEx::new(PixelFormatEx::Gray16, PrecisionConversion::Exact);
    let rgba = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    image
        .set_metadata("Source ICC Profile", DataMap::ICCProfile(GRAY.to_vec()))
        .unwrap();
    let bytes = encode(&mut image, gray, None).unwrap();
    assert!(
        Decoder::new(Cursor::new(bytes))
            .unwrap()
            .get_tag_u8_vec(Tag::IccProfile)
            .is_err()
    );
    assert!(encode(&mut image, rgba, None).is_err());
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(GRAY.to_vec()))
        .unwrap();
    assert!(encode(&mut image, rgba, None).is_err());
    let opts = Some(HashMap::from([(
        "ICC Profile".into(),
        DataMap::ICCProfile(RGB.to_vec()),
    )]));
    let bytes = encode(&mut image, rgba, opts).unwrap();
    assert_eq!(samples(&bytes), vec![0x1234, 0x1234, 0x1234, 65535]);
    assert_eq!(
        Decoder::new(Cursor::new(bytes))
            .unwrap()
            .get_tag_u8_vec(Tag::IccProfile)
            .unwrap(),
        RGB
    );
    image.metadata = None;
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(RGB.to_vec()))
        .unwrap();
    assert!(encode(&mut image, gray, None).is_err());
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(b"invalid".to_vec()))
        .unwrap();
    assert!(encode(&mut image, gray, None).is_err());
}
struct Picker {
    image: ImageBufferEx,
    start: u8,
    wrong: u8,
    fail_end: bool,
    dimensions: Option<(usize, usize)>,
    picks: usize,
    ends: usize,
}
impl Picker {
    fn new() -> Self {
        Self {
            image: ImageBufferEx::from_bytes(2, 2, PixelFormatEx::Rgba16, None, vec![0; 32])
                .unwrap(),
            start: 0,
            wrong: 0,
            fail_end: false,
            dimensions: None,
            picks: 0,
            ends: 0,
        }
    }
}
impl PickCallbackEx for Picker {
    fn encode_start_ex(
        &mut self,
        o: Option<EncoderOptions>,
    ) -> Result<Option<ImageProfilesEx>, Error> {
        if self.start == 1 {
            return Ok(None);
        }
        if self.start == 2 {
            return Err("start failed".into());
        }
        let mut p = self.image.encode_start_ex(o)?.unwrap();
        if let Some((w, h)) = self.dimensions {
            p.legacy.width = w;
            p.legacy.height = h;
        }
        Ok(Some(p))
    }
    fn encode_pick_ex(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        mut out: PixelOutputEx,
        o: Option<PickOptions>,
    ) -> Result<Option<PixelBytesEx>, Error> {
        self.picks += 1;
        match self.wrong {
            1 => out.stride = Some(17),
            2 => out.format = PixelFormatEx::Rgba8,
            3 => return self.image.encode_pick_ex(x, y, 1, h, out, o),
            4 => return self.image.encode_pick_ex(x, y, w, 1, out, o),
            5 => return Ok(None),
            _ => {}
        }
        self.image.encode_pick_ex(x, y, w, h, out, o)
    }
    fn encode_end_ex(&mut self, _: Option<EndOptions>) -> Result<(), Error> {
        self.ends += 1;
        if self.fail_end {
            Err("end failed".into())
        } else {
            Ok(())
        }
    }
    fn metadata_ex(&mut self) -> Result<Option<HashMap<String, DataMap>>, Error> {
        self.image.metadata_ex()
    }
}
#[test]
fn callback_errors_layout_limits_and_single_page_options_are_checked_before_picking() {
    let out = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    let mut p = Picker::new();
    p.start = 1;
    p.fail_end = true;
    assert_eq!(
        encode(&mut p, out, None).unwrap_err().to_string(),
        "TIFF EX image profiles missing"
    );
    assert_eq!(p.ends, 1);
    p.start = 2;
    assert_eq!(
        encode(&mut p, out, None).unwrap_err().to_string(),
        "start failed"
    );
    assert_eq!(p.ends, 1);
    p.start = 0;
    p.fail_end = false;
    for wrong in 1..=5 {
        p.wrong = wrong;
        assert!(encode(&mut p, out, None).is_err());
    }
    p.wrong = 0;
    p.fail_end = true;
    assert_eq!(
        encode(&mut p, out, None).unwrap_err().to_string(),
        "end failed"
    );
    p.fail_end = false;
    for (key, value) in [
        ("compression", DataMap::Ascii("lzw_lsb".into())),
        ("compression", DataMap::Ascii("jpeg".into())),
        ("predictor", DataMap::UInt(2)),
        ("endian", DataMap::Ascii("invalid".into())),
    ] {
        let before = p.picks;
        assert!(encode(&mut p, out, Some(HashMap::from([(key.into(), value)]))).is_err());
        assert_eq!(p.picks, before);
    }
    for dimensions in [(0, 2), (2, 0), (usize::MAX, 2)] {
        p.dimensions = Some(dimensions);
        assert!(encode(&mut p, out, None).is_err());
    }
    p.dimensions = None;
    for limits in [
        DecodeLimits {
            pixels: 3,
            ..DecodeLimits::unlimited()
        },
        DecodeLimits {
            frames: 0,
            ..DecodeLimits::unlimited()
        },
        DecodeLimits {
            expanded_bytes: 32,
            ..DecodeLimits::unlimited()
        },
    ] {
        let before = p.picks;
        assert!(
            image_encoder_ex_with_limits(
                &mut EncodeOptionsEx {
                    debug_flag: 0,
                    drawer: &mut p,
                    output: out,
                    options: None
                },
                ImageFormat::Tiff,
                limits
            )
            .is_err()
        );
        assert_eq!(p.picks, before);
    }
    for stride in [0, 1, usize::MAX] {
        assert!(
            encode(
                &mut p,
                PixelOutputEx {
                    stride: Some(stride),
                    ..out
                },
                None
            )
            .is_err()
        );
    }
    for format in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Rgba12,
    ] {
        assert!(
            encode(
                &mut p,
                PixelOutputEx::new(format, PrecisionConversion::Round),
                None
            )
            .is_err()
        );
    }
    p.image
        .set_metadata("wml2.animation.frames", DataMap::UInt(2))
        .unwrap();
    assert!(encode(&mut p, out, None).is_err());
    p.image.metadata = None;
    p.image
        .set_metadata("ICC Profile", DataMap::ICCProfile(RGB.to_vec()))
        .unwrap();
    let before = p.picks;
    assert!(
        image_encoder_ex_with_limits(
            &mut EncodeOptionsEx {
                debug_flag: 0,
                drawer: &mut p,
                output: out,
                options: None
            },
            ImageFormat::Tiff,
            DecodeLimits {
                metadata_bytes: 1,
                ..DecodeLimits::unlimited()
            }
        )
        .is_err()
    );
    assert_eq!(p.picks, before);
}

#[test]
fn explicitly_selected_exif_keeps_descriptive_metadata_and_cannot_poison_layout_or_icc() {
    use bin_rs::Endian;
    use wml2::tiff::header::{DataPack, TiffHeader, TiffHeaders};
    let mut headers = TiffHeaders::empty(Endian::LittleEndian);
    for (id, data) in [
        (258, DataPack::Short(vec![8])),
        (262, DataPack::Short(vec![3])),
        (339, DataPack::Short(vec![3])),
        (270, DataPack::Ascii("EX TIFF description".into())),
        (34675, DataPack::Undef(GRAY.to_vec())),
    ] {
        headers.headers.push(TiffHeader {
            tagid: id,
            length: 1,
            data,
        });
    }
    let mut image = ImageBufferEx::from_bytes(
        1,
        1,
        PixelFormatEx::Rgba16,
        None,
        words(&[1, 256, 0x1234, 32768]),
    )
    .unwrap();
    let output = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    for endian in ["little", "big"] {
        let mut opts = options("deflate", true, false, endian);
        opts.insert("exif".into(), DataMap::Exif(headers.clone()));
        let bytes = encode(&mut image, output, Some(opts)).unwrap();
        assert_eq!(samples(&bytes), vec![1, 256, 0x1234, 32768]);
        let mut d = Decoder::new(Cursor::new(bytes)).unwrap();
        assert!(d.get_tag_u8_vec(Tag::IccProfile).is_err());
        assert_eq!(d.get_tag_u16_vec(Tag::BitsPerSample).unwrap(), vec![16; 4]);
        assert_eq!(
            d.get_tag_u16_vec(Tag::PhotometricInterpretation).unwrap(),
            vec![2]
        );
        assert_eq!(d.get_tag_u16_vec(Tag::SampleFormat).unwrap(), vec![1; 4]);
        assert!(
            d.get_tag(Tag::ImageDescription)
                .unwrap()
                .into_string()
                .unwrap()
                .contains("EX TIFF description")
        );
    }
}

#[test]
#[ignore = "manual display artifact and release benchmark"]
fn display_and_benchmark() {
    let (w, h) = (512, 384);
    let values: Vec<u16> = (0..h)
        .flat_map(|y| {
            (0..w).flat_map(move |x| {
                [
                    (x * 65535 / (w - 1)) as u16,
                    (y * 65535 / (h - 1)) as u16,
                    ((x + y) * 65535 / (w + h - 2)) as u16,
                    if x < w / 4 {
                        0
                    } else if x < w / 2 {
                        32768
                    } else {
                        65535
                    },
                ]
            })
        })
        .collect();
    let mut image =
        ImageBufferEx::from_bytes(w, h, PixelFormatEx::Rgba16, None, words(&values)).unwrap();
    for compression in ["none", "lzw", "deflate"] {
        let mut times = Vec::new();
        let mut bytes = Vec::new();
        for i in 0..23 {
            let start = std::time::Instant::now();
            bytes = encode(
                &mut image,
                PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact),
                Some(options(compression, compression != "none", false, "little")),
            )
            .unwrap();
            if i >= 3 {
                times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        assert_eq!(samples(&bytes), values);
        times.sort_by(f64::total_cmp);
        println!(
            "TIFF EX RGBA16 {w}x{h} {compression}: median {:.3} ms, {} bytes (20 timed, 3 warmup)",
            times[times.len() / 2],
            bytes.len()
        );
        if let Some(path) = std::env::var_os("WML2_TIFF_EX_DISPLAY_PREFIX") {
            std::fs::write(
                format!(
                    "{}-{compression}.tif",
                    std::path::Path::new(&path).display()
                ),
                bytes,
            )
            .unwrap();
        }
    }
}

#[test]
fn eight_bit_storage_expands_exactly_and_opaque_rgba_retains_explicit_straight_alpha() {
    for (source, destination, bytes) in [
        (PixelFormatEx::Gray8, PixelFormatEx::Gray16, vec![1, 2]),
        (
            PixelFormatEx::Rgba8,
            PixelFormatEx::Rgba16,
            vec![1, 2, 3, 255, 10, 20, 30, 255],
        ),
    ] {
        let expected: Vec<u16> = bytes.iter().map(|&v| u16::from(v) * 257).collect();
        let mut image = ImageBufferEx::from_bytes(2, 1, source, None, bytes).unwrap();
        let bytes = encode(
            &mut image,
            PixelOutputEx::new(destination, PrecisionConversion::Exact),
            None,
        )
        .unwrap();
        assert_eq!(samples(&bytes), expected);
        if source.channels() == 4 {
            assert_eq!(
                Decoder::new(Cursor::new(bytes))
                    .unwrap()
                    .get_tag_u16_vec(Tag::ExtraSamples)
                    .unwrap(),
                vec![2]
            );
        }
    }
}
