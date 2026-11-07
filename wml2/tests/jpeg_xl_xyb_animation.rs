#![cfg(feature = "jpeg-xl")]
//! Independent libjxl animation references and the unchanged bounded API contract.
use wml2::{draw::*, jpegxl::*, limits::DecodeLimits};

struct Case {
    input: &'static [u8],
    reference: &'static [u8],
    size: usize,
    frames: usize,
    numerator: u32,
    ticks: u32,
    alpha: bool,
}
const CASES: &[Case] = &[
    Case {
        input: include_bytes!("fixtures/review/jxl-animation-xyb-icos4d.jxl"),
        reference: include_bytes!("fixtures/review/jxl-animation-xyb-icos4d.rgba8.zlib"),
        size: 128,
        frames: 48,
        numerator: 1000,
        ticks: 50,
        alpha: true,
    },
    Case {
        input: include_bytes!("fixtures/review/jxl-animation-xyb-spline.jxl"),
        reference: include_bytes!("fixtures/review/jxl-animation-xyb-spline.rgba8.zlib"),
        size: 320,
        frames: 60,
        numerator: 100,
        ticks: 2,
        alpha: false,
    },
];
fn reference(c: &Case) -> Vec<u8> {
    let bytes = miniz_oxide::inflate::decompress_to_vec_zlib(c.reference).unwrap();
    assert_eq!(bytes.len(), c.frames * c.size * c.size * 4);
    bytes
}
fn rounded(v: u16) -> u8 {
    ((u32::from(v) * 255 + 32767) / 65535) as u8
}
fn compare(actual: impl Iterator<Item = u8>, gold: &[u8]) {
    let actual: Vec<_> = actual.collect();
    assert_eq!(actual.len(), gold.len());
    for (i, (&a, &b)) in actual.iter().zip(gold).enumerate() {
        // libjxl's 8-bit output may differ by one RGB quantization unit.
        assert!(
            a.abs_diff(b) <= u8::from(i % 4 != 3),
            "sample {i}: {a} != {b}"
        );
    }
}
#[test]
fn xyb_native_all_frames_match_libjxl_with_exact_timing_and_alpha() {
    use jxl_preflight::headers::{FileHeader, JxlHeader};
    for c in CASES {
        let h = FileHeader::read(&mut jxl_preflight::bit_reader::BitReader::new(c.input)).unwrap();
        assert!(h.image_metadata.xyb_encoded);
        let image =
            animation_rgba16_from_with_limits_and_cancel(c.input, Default::default(), &|| false)
                .unwrap();
        assert_eq!(image.frames.len(), c.frames);
        assert_eq!(image.bits_per_sample, 8);
        assert_eq!(image.alpha_bits_per_sample, c.alpha.then_some(8));
        assert_eq!(
            (
                image.ticks_per_second_numerator,
                image.ticks_per_second_denominator,
                image.loop_count
            ),
            (c.numerator, 1, 0)
        );
        assert!(!image.source_icc.is_empty());
        assert!(!image.output_icc.is_empty());
        for f in &image.frames {
            assert_eq!(f.duration_ticks, c.ticks);
        }
        compare(
            image
                .frames
                .iter()
                .flat_map(|f| f.rgba16.iter().copied().map(rounded)),
            &reference(c),
        );
    }
}
#[test]
fn xyb_common_and_ex_publish_every_canvas_and_first_frame_poster() {
    for c in CASES {
        let image = image_from(c.input).unwrap();
        assert_eq!((image.width, image.height), (c.size, c.size));
        assert_eq!(image.loop_count, Some(0));
        let frames = image.animation.as_ref().unwrap();
        assert_eq!(frames.len(), c.frames);
        assert_eq!(image.buffer.as_ref().unwrap(), &frames[0].buffer);
        for f in frames {
            assert_eq!(
                f.control.await_time,
                u64::from(c.ticks) * 1000 / u64::from(c.numerator)
            );
        }
        compare(
            frames.iter().flat_map(|f| f.buffer.iter().copied()),
            &reference(c),
        );
        #[cfg(feature = "image-buffer-ex")]
        {
            let ex =
                image_from_ex(c.input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            let frames = ex.animation().unwrap();
            assert_eq!(frames.len(), c.frames);
            assert_eq!(ex.bytes().unwrap(), frames[0].pixels.bytes());
            compare(
                frames.iter().flat_map(|f| {
                    f.pixels
                        .bytes()
                        .chunks_exact(2)
                        .map(|v| rounded(u16::from_le_bytes(v.try_into().unwrap())))
                }),
                &reference(c),
            );
        }
    }
}
#[cfg(feature = "color-management")]
#[test]
fn xyb_managed_frames_match_libjxl_srgb_without_changing_alpha() {
    for c in CASES {
        let image = image_from_srgb_with_limits(c.input, Default::default()).unwrap();
        let frames = image.animation.as_ref().unwrap();
        assert_eq!(frames.len(), c.frames);
        assert_eq!(image.buffer.as_ref().unwrap(), &frames[0].buffer);
        compare(
            frames.iter().flat_map(|f| f.buffer.iter().copied()),
            &reference(c),
        );
    }
}
#[test]
fn xyb_still_obeys_limits_truncation_and_cooperative_cancellation() {
    let c = &CASES[0];
    for limits in [
        DecodeLimits {
            frames: c.frames - 1,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: c.frames * c.size * c.size * 8 - 1,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1024,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 0,
            ..Default::default()
        },
        DecodeLimits {
            pixels: c.size * c.size - 1,
            ..Default::default()
        },
    ] {
        assert!(animation_rgba16_from_with_limits_and_cancel(c.input, limits, &|| false).is_err());
        assert!(image_from_with_limits(c.input, limits).is_err());
    }
    assert!(image_from(&c.input[..c.input.len() - 1]).is_err());
    let mut trailing = c.input.to_vec();
    trailing.push(0);
    assert!(image_from(&trailing).is_err());
    for stop in [0, 10, 100] {
        let count = std::cell::Cell::new(0);
        let cancelled = || {
            let i = count.get();
            count.set(i + 1);
            i >= stop
        };
        assert!(
            animation_rgba16_from_with_limits_and_cancel(c.input, Default::default(), &cancelled)
                .unwrap_err()
                .is::<DecodeCancelled>()
        );
    }
    assert_eq!(
        image_from(c.input).unwrap().animation.unwrap().len(),
        c.frames
    );
}

const XYB16: &[u8] = include_bytes!("fixtures/review/jxl-animation-xyb-rgba16.jxl");
const XYB16_REFERENCE: &[u8] = include_bytes!("fixtures/review/jxl-animation-xyb-rgba16.rgba16le");

fn compare_16bit(actual: impl Iterator<Item = u16>) {
    let actual: Vec<_> = actual.collect();
    let gold: Vec<_> = XYB16_REFERENCE
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
        .collect();
    assert_eq!(actual.len(), 3 * 32 * 24 * 4);
    assert_eq!(gold.len(), actual.len());
    // Catch an accidentally 8-bit oracle or an 8-bit intermediate in WML2.
    assert_eq!(gold.iter().filter(|&&v| v % 257 != 0).count(), 8979);
    assert!(actual.iter().filter(|&&v| v % 257 != 0).count() > 8900);
    for (frame, (actual, gold)) in actual
        .chunks_exact(32 * 24 * 4)
        .zip(gold.chunks_exact(32 * 24 * 4))
        .enumerate()
    {
        let mut max = [0u16; 4];
        let mut sum = [0u64; 4];
        for (i, (&a, &b)) in actual.iter().zip(gold).enumerate() {
            let c = i % 4;
            let diff = a.abs_diff(b);
            max[c] = max[c].max(diff);
            sum[c] += u64::from(diff);
            assert!(
                diff <= u16::from(c != 3),
                "frame {frame}, sample {i}: {a}/{b}"
            );
            if c == 3 {
                let pixel = i / 4;
                let (x, y) = (pixel % 32, pixel / 32);
                // Alpha was encoded losslessly and stays straight, including
                // samples with low 8 bits that an RGBA8 round trip would lose.
                assert_eq!(a, ((x * 3457 + y * 1297 + frame * 1741) % 65536) as u16);
            }
        }
        println!("XYB16 frame {frame}: max RGBA {max:?}, sum abs error RGBA {sum:?}, 768 pixels");
    }
}

#[test]
fn xyb16_native_matches_independent_full_precision_libjxl_frames() {
    use jxl_preflight::headers::color_encoding::{
        ColorSpace, Primaries, RenderingIntent, TransferFunction, WhitePoint,
    };
    use jxl_preflight::headers::{FileHeader, JxlHeader};
    let h = FileHeader::read(&mut jxl_preflight::bit_reader::BitReader::new(XYB16)).unwrap();
    let m = &h.image_metadata;
    assert!(m.xyb_encoded);
    assert_eq!(m.bit_depth.bits_per_sample(), 16);
    assert!(!m.bit_depth.floating_point_sample());
    assert_eq!(m.extra_channel_info.len(), 1);
    let alpha = &m.extra_channel_info[0];
    assert_eq!(
        alpha.ec_type,
        jxl_preflight::headers::extra_channels::ExtraChannel::Alpha
    );
    assert_eq!(alpha.bit_depth().bits_per_sample(), 16);
    assert!(!alpha.bit_depth().floating_point_sample());
    assert!(!alpha.alpha_associated());
    assert_eq!(alpha.dim_shift(), 0);
    assert!(!m.color_encoding.want_icc);
    assert_eq!(m.color_encoding.color_space, ColorSpace::RGB);
    assert_eq!(m.color_encoding.white_point, WhitePoint::D65);
    assert_eq!(m.color_encoding.primaries, Primaries::SRGB);
    assert_eq!(
        m.color_encoding.tf.transfer_function,
        TransferFunction::SRGB
    );
    assert_eq!(m.color_encoding.rendering_intent, RenderingIntent::Relative);
    let image =
        animation_rgba16_from_with_limits_and_cancel(XYB16, Default::default(), &|| false).unwrap();
    assert_eq!(
        (image.bits_per_sample, image.alpha_bits_per_sample),
        (16, Some(16))
    );
    assert_eq!(
        (
            image.ticks_per_second_numerator,
            image.ticks_per_second_denominator,
            image.loop_count
        ),
        (1000, 1, 0)
    );
    assert_eq!(
        image
            .frames
            .iter()
            .map(|f| f.duration_ticks)
            .collect::<Vec<_>>(),
        [50, 90, 130]
    );
    assert!(!image.output_icc.is_empty());
    assert_eq!(image.source_icc, image.output_icc);
    compare_16bit(image.frames.iter().flat_map(|f| f.rgba16.iter().copied()));
}

#[cfg(feature = "image-buffer-ex")]
#[test]
fn xyb16_ex_preserves_full_precision_across_all_frames() {
    let image = image_from_ex(XYB16, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
    assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba16));
    let frames = image.animation().unwrap();
    assert_eq!(frames.len(), 3);
    assert_eq!(image.bytes().unwrap(), frames[0].pixels.bytes());
    for (frame, ms) in frames.iter().zip([50, 90, 130]) {
        assert_eq!(frame.control.await_time, ms);
    }
    compare_16bit(frames.iter().flat_map(|f| {
        f.pixels
            .bytes()
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
    }));
}
