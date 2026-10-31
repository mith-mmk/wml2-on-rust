#![cfg(all(feature = "png", feature = "image-buffer-ex"))]

use std::collections::HashMap;
use std::io::Cursor;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

type Error = Box<dyn std::error::Error>;
const RGB: &[u8] = include_bytes!("fixtures/review/linear-rgb.icc");
const GRAY: &[u8] = include_bytes!("fixtures/review/avif-icc-gray-linear.icc");

fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn encode(image: &mut dyn PickCallbackEx, output: PixelOutputEx) -> Result<Vec<u8>, Error> {
    image_encoder_ex(
        &mut EncodeOptionsEx {
            debug_flag: 0,
            drawer: image,
            output,
            options: None,
        },
        ImageFormat::Png,
    )
}
fn decode(bytes: &[u8]) -> (png::OutputInfo, Vec<u8>, Option<Vec<u8>>) {
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let icc = reader.info().icc_profile.as_ref().map(|v| v.to_vec());
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    pixels.truncate(info.buffer_size());
    (info, pixels, icc)
}

#[test]
fn independent_decoder_all_layouts_with_odd_padding_and_two_row_paeth_boundaries() {
    for format in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Rgba16,
    ] {
        let width = 3;
        let height = 2;
        let row = width * format.bytes_per_pixel();
        let stride = row + 1;
        let mut data = vec![0xa5; stride * height];
        let mut expected = Vec::new();
        for y in 0..height {
            for x in 0..width * format.channels() {
                // Includes zero alpha and half alpha with nonzero RGB, plus
                // values whose lower bytes differ (endianness cannot pass by chance).
                let sample = match x % 4 {
                    3 => {
                        if y == 0 {
                            0
                        } else {
                            32768
                        }
                    }
                    _ => (0x1234 + y * 0x2345 + x * 0x111) as u16,
                };
                let start = y * stride + x * format.bytes_per_sample();
                if format.bits() == 16 {
                    data[start..start + 2].copy_from_slice(&sample.to_le_bytes());
                    expected.extend(sample.to_be_bytes());
                } else {
                    data[start] = (sample >> 8) as u8;
                    expected.push((sample >> 8) as u8);
                }
            }
        }
        let mut image =
            ImageBufferEx::from_bytes(width, height, format, Some(stride), data.clone()).unwrap();
        let mut output = PixelOutputEx::new(format, PrecisionConversion::Exact);
        output.stride = Some(stride + 2);
        let encoded = encode(&mut image, output).unwrap();
        let (info, actual, _) = decode(&encoded);
        assert_eq!((info.width, info.height), (width as u32, height as u32));
        assert_eq!(
            info.bit_depth,
            if format.bits() == 16 {
                png::BitDepth::Sixteen
            } else {
                png::BitDepth::Eight
            }
        );
        assert_eq!(
            info.color_type,
            if format.channels() == 1 {
                png::ColorType::Grayscale
            } else {
                png::ColorType::Rgba
            }
        );
        assert_eq!(actual, expected, "{format:?}");
        assert_eq!(image.bytes().unwrap(), data);
    }
}

#[test]
fn twelve_to_sixteen_uses_normalization_for_rgb_and_alpha() {
    for format in [PixelFormatEx::Gray12, PixelFormatEx::Rgba12] {
        let destination = if format.channels() == 1 {
            PixelFormatEx::Gray16
        } else {
            PixelFormatEx::Rgba16
        };
        let source: Vec<u16> = if format.channels() == 1 {
            vec![273, 4095]
        } else {
            vec![0, 273, 1365, 4095, 4095, 1365, 273, 0]
        };
        let mut image = ImageBufferEx::from_bytes(2, 1, format, None, words(&source)).unwrap();
        let (_, actual, _) = decode(
            &encode(
                &mut image,
                PixelOutputEx::new(destination, PrecisionConversion::Exact),
            )
            .unwrap(),
        );
        let expected: Vec<u8> = source
            .iter()
            .flat_map(|&s| ((u32::from(s) * 65535 / 4095) as u16).to_be_bytes())
            .collect();
        assert_eq!(actual, expected);
        for channel in 0..format.channels() {
            let mut values = vec![4095; format.channels()];
            values[channel] = 1;
            let bytes = words(&values);
            let mut image = ImageBufferEx::from_bytes(1, 1, format, None, bytes.clone()).unwrap();
            assert!(
                encode(
                    &mut image,
                    PixelOutputEx::new(destination, PrecisionConversion::Exact)
                )
                .is_err()
            );
            let (_, actual, _) = decode(
                &encode(
                    &mut image,
                    PixelOutputEx::new(destination, PrecisionConversion::Round),
                )
                .unwrap(),
            );
            let expected: Vec<u8> = values
                .iter()
                .map(|&s| ((u32::from(s) * 65535 + 2047) / 4095) as u16)
                .flat_map(u16::to_be_bytes)
                .collect();
            assert_eq!(actual, expected);
            assert_eq!(image.bytes().unwrap(), bytes);
        }
    }
}

#[test]
fn sixteen_to_eight_exact_rejects_each_color_and_alpha_rounds_without_compositing() {
    for channel in 0..4 {
        let mut source = vec![257, 514, 771, 0];
        source[channel] = 32768;
        let mut image =
            ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba16, None, words(&source)).unwrap();
        assert!(
            encode(
                &mut image,
                PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact)
            )
            .is_err()
        );
        let (_, bytes, _) = decode(
            &encode(
                &mut image,
                PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Round),
            )
            .unwrap(),
        );
        let expected: Vec<u8> = source
            .iter()
            .map(|&s| ((u32::from(s) * 255 + 32767) / 65535) as u8)
            .collect();
        assert_eq!(bytes, expected);
    }
}

#[test]
fn icc_output_model_is_consistent_and_source_is_never_blindly_embedded() {
    for (format, icc) in [(PixelFormatEx::Gray16, GRAY), (PixelFormatEx::Rgba16, RGB)] {
        let mut image =
            ImageBufferEx::from_bytes(1, 1, format, None, vec![0; format.bytes_per_pixel()])
                .unwrap();
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(icc.to_vec()))
            .unwrap();
        let output = PixelOutputEx::new(format, PrecisionConversion::Exact);
        assert_eq!(
            decode(&encode(&mut image, output).unwrap()).2.as_deref(),
            Some(icc)
        );
        let wrong = if format.channels() == 1 { RGB } else { GRAY };
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(wrong.to_vec()))
            .unwrap();
        assert!(encode(&mut image, output).is_err());
    }
    let mut image =
        ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Gray16, None, words(&[12345])).unwrap();
    image
        .set_metadata("Source ICC Profile", DataMap::ICCProfile(GRAY.to_vec()))
        .unwrap();
    let gray = PixelOutputEx::new(PixelFormatEx::Gray16, PrecisionConversion::Exact);
    assert!(decode(&encode(&mut image, gray).unwrap()).2.is_none());
    let rgb = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    assert!(encode(&mut image, rgb).is_err());
    let options = Some(HashMap::from([(
        "ICC Profile".into(),
        DataMap::ICCProfile(RGB.to_vec()),
    )]));
    let encoded = image_encoder_ex(
        &mut EncodeOptionsEx {
            debug_flag: 0,
            drawer: &mut image,
            output: rgb,
            options,
        },
        ImageFormat::Png,
    )
    .unwrap();
    assert_eq!(decode(&encoded).2.as_deref(), Some(RGB));
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(GRAY.to_vec()))
        .unwrap();
    assert!(encode(&mut image, rgb).is_err());
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(b"invalid".to_vec()))
        .unwrap();
    assert!(encode(&mut image, gray).is_err());
}

struct Picker {
    image: ImageBufferEx,
    wrong: u8,
    fail_end: bool,
    picks: usize,
    ends: usize,
    dimensions: Option<(usize, usize)>,
    start: u8,
}
impl Picker {
    fn new() -> Self {
        Self {
            image: ImageBufferEx::from_bytes(2, 2, PixelFormatEx::Rgba16, None, vec![0; 32])
                .unwrap(),
            wrong: 0,
            fail_end: false,
            picks: 0,
            ends: 0,
            dimensions: None,
            start: 0,
        }
    }
}
impl PickCallbackEx for Picker {
    fn encode_start_ex(
        &mut self,
        o: Option<EncoderOptions>,
    ) -> Result<Option<ImageProfilesEx>, Error> {
        match self.start {
            1 => return Ok(None),
            2 => return Err("start failed".into()),
            _ => {}
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
            1 => out.stride = Some(2 * out.format.bytes_per_pixel() + 1),
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
fn callback_layout_dimensions_limits_and_end_lifecycle_are_checked() {
    let output = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    let mut picker = Picker::new();
    for wrong in 1..=5 {
        picker.wrong = wrong;
        let error = encode(&mut picker, output).unwrap_err().to_string();
        assert!(
            error.contains(if wrong == 5 {
                "pixels missing"
            } else {
                "different layout"
            }),
            "{error}"
        );
        assert_eq!(picker.ends, wrong as usize);
    }
    picker.wrong = 0;
    picker.fail_end = true;
    assert_eq!(
        encode(&mut picker, output).unwrap_err().to_string(),
        "end failed"
    );
    picker.fail_end = false;
    for dimensions in [(0, 2), (2, 0), (usize::MAX, 2)] {
        picker.dimensions = Some(dimensions);
        assert!(encode(&mut picker, output).is_err());
    }
    picker.dimensions = None;
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
        let previous = picker.picks;
        assert!(
            image_encoder_ex_with_limits(
                &mut EncodeOptionsEx {
                    debug_flag: 0,
                    drawer: &mut picker,
                    output,
                    options: None
                },
                ImageFormat::Png,
                limits
            )
            .is_err()
        );
        assert_eq!(picker.picks, previous);
    }
    for stride in [0, 1, usize::MAX] {
        assert!(
            encode(
                &mut picker,
                PixelOutputEx {
                    stride: Some(stride),
                    ..output
                }
            )
            .is_err()
        );
    }
    let limits = DecodeLimits {
        metadata_bytes: 1,
        ..DecodeLimits::unlimited()
    };
    picker
        .image
        .set_metadata("ICC Profile", DataMap::ICCProfile(RGB.to_vec()))
        .unwrap();
    assert!(
        image_encoder_ex_with_limits(
            &mut EncodeOptionsEx {
                debug_flag: 0,
                drawer: &mut picker,
                output,
                options: None
            },
            ImageFormat::Png,
            limits
        )
        .is_err()
    );
}

/// Manual image artifact and reproducible local timing; ordinary tests do not
/// depend on an output directory or a timing threshold.
#[test]
#[ignore = "manual display artifact and release benchmark"]
fn display_and_benchmark() {
    let (w, h) = (512, 384);
    let mut samples = Vec::new();
    for y in 0..h {
        for x in 0..w {
            samples.extend_from_slice(&[
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
            ]);
        }
    }
    let data = words(&samples);
    let mut image = ImageBufferEx::from_bytes(w, h, PixelFormatEx::Rgba16, None, data).unwrap();
    let output = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    let mut times = Vec::new();
    let mut encoded = Vec::new();
    for i in 0..23 {
        let start = std::time::Instant::now();
        encoded = encode(&mut image, output).unwrap();
        if i >= 3 {
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    times.sort_by(f64::total_cmp);
    let (_, actual, _) = decode(&encoded);
    assert_eq!(
        actual,
        samples
            .iter()
            .flat_map(|s| s.to_be_bytes())
            .collect::<Vec<_>>()
    );
    println!(
        "PNG EX RGBA16 {w}x{h}: median {:.3} ms, {} bytes (20 runs, 3 warmup)",
        times[times.len() / 2],
        encoded.len()
    );
    if let Some(path) = std::env::var_os("WML2_PNG_EX_DISPLAY") {
        std::fs::write(path, encoded).unwrap();
    }
}

#[test]
fn static_only_rejects_animation_and_twelve_bit_output_and_preserves_exif() {
    let mut picker = Picker::new();
    assert!(
        encode(
            &mut picker,
            PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Round)
        )
        .is_err()
    );
    assert_eq!((picker.picks, picker.ends), (0, 0));
    let output = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    picker
        .image
        .set_metadata("wml2.animation.frames", DataMap::UInt(2))
        .unwrap();
    assert!(
        encode(&mut picker, output)
            .unwrap_err()
            .to_string()
            .contains("static images only")
    );
    assert_eq!((picker.picks, picker.ends), (0, 1));
    picker.image.metadata = None;
    let exif = b"II\x2a\0\x08\0\0\0\0\0\0\0\0\0".to_vec();
    let options = Some(HashMap::from([("exif".into(), DataMap::Raw(exif.clone()))]));
    let bytes = image_encoder_ex(
        &mut EncodeOptionsEx {
            debug_flag: 0,
            drawer: &mut picker,
            output,
            options,
        },
        ImageFormat::Png,
    )
    .unwrap();
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    assert_eq!(
        reader.info().exif_metadata.as_deref(),
        Some(exif.as_slice())
    );
}

#[test]
fn successful_start_without_profiles_still_ends_but_failed_start_does_not() {
    let output = PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
    let mut picker = Picker::new();
    picker.start = 1;
    picker.fail_end = true;
    assert_eq!(
        encode(&mut picker, output).unwrap_err().to_string(),
        "PNG EX image profiles missing"
    );
    assert_eq!((picker.picks, picker.ends), (0, 1));
    picker.start = 2;
    assert_eq!(
        encode(&mut picker, output).unwrap_err().to_string(),
        "start failed"
    );
    assert_eq!((picker.picks, picker.ends), (0, 1));
    picker.start = 0;
    picker.fail_end = false;
    assert!(encode(&mut picker, output).is_ok());
    assert_eq!((picker.picks, picker.ends), (1, 2));
}
