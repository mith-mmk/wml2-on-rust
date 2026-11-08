#![cfg(feature = "jpeg-xl")]
//! Independent libjxl Gray animation oracles and the bounded RGBA contract.
use wml2::{draw::*, jpegxl::*, limits::DecodeLimits, metadata::DataMap};

type Error = Box<dyn std::error::Error>;
const W: usize = 32;
const H: usize = 24;
struct Case {
    input: &'static [u8],
    reference: &'static [u8],
    bits: u8,
    alpha: bool,
}
macro_rules! case {
    ($name:literal, $bits:literal, $alpha:literal) => {
        Case {
            input: include_bytes!(concat!("fixtures/review/jxl-animation-", $name, ".jxl")),
            reference: include_bytes!(concat!(
                "fixtures/review/jxl-animation-",
                $name,
                ".rgba16le"
            )),
            bits: $bits,
            alpha: $alpha,
        }
    };
}
const CASES: &[Case] = &[
    case!("gray8", 8, false),
    case!("grayalpha8", 8, true),
    case!("gray16", 16, false),
    case!("grayalpha16", 16, true),
];
const ICC: &[u8] = include_bytes!("fixtures/review/jxl-animation-gray16-icc-refused.jxl");
fn values(bytes: &[u8]) -> impl Iterator<Item = u16> + '_ {
    bytes
        .chunks_exact(2)
        .map(|v| u16::from_le_bytes(v.try_into().unwrap()))
}
fn rounded(v: u16) -> u8 {
    ((u32::from(v) * 255 + 32767) / 65535) as u8
}
#[test]
fn gray_native_matches_every_libjxl_sample_and_source_code() {
    use jxl_preflight::headers::{FileHeader, JxlHeader, color_encoding::ColorSpace};
    for c in CASES {
        let h = FileHeader::read(&mut jxl_preflight::bit_reader::BitReader::new(c.input)).unwrap();
        assert_eq!(
            h.image_metadata.color_encoding.color_space,
            ColorSpace::Gray
        );
        assert!(!h.image_metadata.xyb_encoded);
        assert_eq!(
            h.image_metadata.extra_channel_info.len(),
            usize::from(c.alpha)
        );
        let i =
            animation_rgba16_from_with_limits_and_cancel(c.input, Default::default(), &|| false)
                .unwrap();
        assert_eq!(
            (i.bits_per_sample, i.alpha_bits_per_sample),
            (c.bits, c.alpha.then_some(c.bits))
        );
        assert_eq!(
            (
                i.ticks_per_second_numerator,
                i.ticks_per_second_denominator,
                i.loop_count
            ),
            (1000, 1, 0)
        );
        assert_eq!(i.frames.len(), 3);
        assert_eq!(&i.source_icc[16..20], b"GRAY");
        assert_eq!(&i.output_icc[16..20], b"RGB ");
        assert_ne!(i.source_icc, i.output_icc);
        let gold: Vec<_> = values(c.reference).collect();
        let actual: Vec<_> = i
            .frames
            .iter()
            .flat_map(|f| f.rgba16.iter().copied())
            .collect();
        assert_eq!(actual, gold);
        if c.bits == 16 {
            assert!(actual.iter().filter(|&&v| v % 257 != 0).count() > 6800);
        }
        for (frame, (f, ticks)) in i.frames.iter().zip([50, 90, 130]).enumerate() {
            assert_eq!(f.duration_ticks, ticks);
            assert_eq!(f.rgba16.len(), W * H * 4);
            for (pixel, rgba) in f.rgba16.chunks_exact(4).enumerate() {
                let (x, y) = (pixel % W, pixel / W);
                let scale = if c.bits == 8 { 257 } else { 1 };
                let gray = ((x * 1703 + y * 193 + frame * 4111) % (1 << c.bits)) as u16 * scale;
                let alpha = if c.alpha {
                    ((x * 3457 + y * 1297 + frame * 1741) % (1 << c.bits)) as u16 * scale
                } else {
                    65535
                };
                assert_eq!(rgba, [gray, gray, gray, alpha]);
            }
        }
    }
}
#[test]
fn gray_common_and_ex_preserve_all_frames_timing_and_profile_tagging() {
    for c in CASES {
        let i = image_from(c.input).unwrap();
        assert_eq!((i.width, i.height, i.loop_count), (W, H, Some(0)));
        let frames = i.animation.as_ref().unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(i.buffer.as_ref().unwrap(), &frames[0].buffer);
        let metadata = i.metadata.as_ref().unwrap();
        assert!(
            matches!(&metadata["ICC Profile"], DataMap::ICCProfile(p) if &p[16..20] == b"RGB ")
        );
        assert!(
            matches!(&metadata["JPEG XL Source ICC Profile"], DataMap::ICCProfile(p) if &p[16..20] == b"GRAY")
        );
        assert!(
            matches!(&metadata["JPEG XL Reconstruction ICC Profile"], DataMap::ICCProfile(p) if &p[16..20] == b"RGB ")
        );
        assert_eq!(
            frames
                .iter()
                .flat_map(|f| f.buffer.iter().copied())
                .collect::<Vec<_>>(),
            values(c.reference).map(rounded).collect::<Vec<_>>()
        );
        for (frame, ms) in frames.iter().zip([50, 90, 130]) {
            assert_eq!(frame.control.await_time, ms);
            assert!(matches!(frame.control.blend, Some(NextBlend::Override)));
        }
        #[cfg(feature = "image-buffer-ex")]
        {
            let ex =
                image_from_ex(c.input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert_eq!(ex.source_format(), Some(PixelFormatEx::Rgba16));
            assert_eq!(ex.loop_count(), Some(0));
            let frames = ex.animation().unwrap();
            assert_eq!(frames.len(), 3);
            assert_eq!(ex.bytes().unwrap(), frames[0].pixels.bytes());
            assert_eq!(
                frames
                    .iter()
                    .flat_map(|f| f.pixels.bytes().iter().copied())
                    .collect::<Vec<_>>(),
                c.reference
            );
        }
    }
}
#[cfg(feature = "color-management")]
#[test]
fn gray_managed_srgb_matches_libjxl_with_alpha_unchanged() {
    for c in CASES {
        let i = image_from_srgb_with_limits(c.input, Default::default()).unwrap();
        let frames = i.animation.as_ref().unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(i.buffer.as_ref().unwrap(), &frames[0].buffer);
        let actual: Vec<_> = frames
            .iter()
            .flat_map(|f| f.buffer.iter().copied())
            .collect();
        let gold: Vec<_> = values(c.reference).map(rounded).collect();
        assert_eq!(actual.len(), gold.len());
        for (sample, (a, b)) in actual.iter().zip(gold).enumerate() {
            assert!(
                a.abs_diff(b) <= u8::from(sample % 4 != 3),
                "sample {sample}: {a}/{b}"
            );
        }
    }
}
#[derive(Default)]
struct Probe(Vec<&'static str>);
impl DrawCallback for Probe {
    fn begin_image(&mut self) -> Response {
        self.0.push("begin");
        Ok(None)
    }
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        self.0.push("init");
        Ok(None)
    }
    #[cfg(feature = "image-buffer-ex")]
    fn init_ex(&mut self, _: usize, _: usize, _: Option<InitOptionsEx>) -> Response {
        self.0.push("init_ex");
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
        self.0.push("draw");
        Ok(None)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.0.push("end");
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        self.0.push("next");
        Ok(None)
    }
    fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
        self.0.push("metadata");
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
}
fn run(input: &[u8], limits: DecodeLimits, p: &mut Probe, ex: bool) -> Result<(), Error> {
    let mut r = bin_rs::reader::StreamReader::new(std::io::Cursor::new(input));
    let mut o = DecodeOptions {
        debug_flag: 0,
        drawer: p,
    };
    #[cfg(feature = "image-buffer-ex")]
    if ex {
        return image_decoder_ex_with_limits(&mut r, &mut o, limits).map(|_| ());
    }
    let _ = ex;
    image_decoder_with_limits(&mut r, &mut o, limits).map(|_| ())
}
#[test]
fn gray_limits_truncation_cancellation_and_unsupported_icc_are_atomic() {
    for c in CASES {
        for limits in [
            DecodeLimits {
                frames: 2,
                ..Default::default()
            },
            DecodeLimits {
                animation_bytes: W * H * 8 * 3 - 1,
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
                pixels: W * H - 1,
                ..Default::default()
            },
        ] {
            assert!(
                animation_rgba16_from_with_limits_and_cancel(c.input, limits, &|| false).is_err()
            );
            for ex in [false, true] {
                let mut p = Probe::default();
                assert!(run(c.input, limits, &mut p, ex).is_err());
                assert!(p.0 == ["begin", "end"] || p.0 == ["begin"]);
            }
        }
        let successful_probes = std::cell::Cell::new(0);
        animation_rgba16_from_with_limits_and_cancel(c.input, Default::default(), &|| {
            successful_probes.set(successful_probes.get() + 1);
            false
        })
        .unwrap();
        for stop in 0..successful_probes.get() {
            let count = std::cell::Cell::new(0);
            let cancelled = || {
                let n = count.get();
                count.set(n + 1);
                n >= stop
            };
            assert!(
                animation_rgba16_from_with_limits_and_cancel(
                    c.input,
                    Default::default(),
                    &cancelled
                )
                .unwrap_err()
                .is::<DecodeCancelled>()
            );
        }
        for cut in 0..c.input.len() {
            assert!(image_from(&c.input[..cut]).is_err(), "truncation {cut}");
        }
        let mut trailing = c.input.to_vec();
        trailing.push(0);
        assert!(image_from(&trailing).is_err());
        let mut p = Probe::default();
        assert!(
            run(
                &c.input[..c.input.len() - 1],
                Default::default(),
                &mut p,
                false
            )
            .is_err()
        );
        assert!(p.0 == ["begin", "end"] || p.0 == ["begin"]);
        assert_eq!(image_from(c.input).unwrap().animation.unwrap().len(), 3);
    }
    assert!(
        animation_rgba16_from_with_limits_and_cancel(ICC, Default::default(), &|| false)
            .unwrap_err()
            .to_string()
            .contains("Gray ICC")
    );
    for ex in [false, true] {
        let mut p = Probe::default();
        assert!(run(ICC, Default::default(), &mut p, ex).is_err());
        assert_eq!(p.0, ["begin", "end"]);
    }
    #[cfg(feature = "color-management")]
    assert!(
        image_from_srgb_with_limits(ICC, Default::default())
            .err()
            .unwrap()
            .to_string()
            .contains("Gray ICC")
    );
}
